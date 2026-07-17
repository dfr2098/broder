"""Worker de visión: OpenCV + YOLO (tracking) -> objetos / posiciones / eventos.

Sigue el mismo patrón que ``local_worker.py`` pero, en lugar de leer tareas
de Redis, procesa el video de una cámara:

    captura (OpenCV)  ->  detección + tracking (YOLO)  ->  base de datos

Cada objeto rastreado (track_id) se registra una sola vez en ``objetos`` y
va acumulando filas en ``posiciones``; los hitos relevantes (aparición,
cambio de zona) se registran en ``eventos``.

Configuración por variables de entorno (ver los valores por defecto abajo).

Uso:
    python vision_worker.py
    # o
    make vision
"""
import json
import logging
import os
import time
import uuid
from datetime import datetime

from dotenv import load_dotenv
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from backend.app.models import (
    Dispositivo,
    Evento,
    Objeto,
    Posicion,
    TipoDispositivo,
    TipoEvento,
    Transportador,
)

# Las dependencias pesadas de visión son opcionales: si no están instaladas,
# el módulo se puede importar igual y avisamos al ejecutar.
try:
    import cv2
    import numpy as np
    from ultralytics import YOLO

    _DEPS_OK = True
    _IMPORT_ERROR = None
except ImportError as exc:  # pragma: no cover - depende del entorno
    _DEPS_OK = False
    _IMPORT_ERROR = exc

load_dotenv()

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("vision_worker")

# --- Configuración -----------------------------------------------------------
DATABASE_URL = os.getenv(
    "DATABASE_URL_HOST", "postgresql://appuser:password@localhost:5432/appdb"
)
CAMERA_SOURCE = os.getenv("CAMERA_SOURCE", "0")  # índice, ruta o URL RTSP
YOLO_MODEL = os.getenv("YOLO_MODEL", "yolo11n.pt")
YOLO_DEVICE = os.getenv("YOLO_DEVICE", "cpu")  # "cpu" o "cuda"
CONF_THRESHOLD = float(os.getenv("CONF_THRESHOLD", "0.4"))

# A qué transportador / cámara se asocia lo detectado.
TRANSPORTADOR_CODIGO = os.getenv("TRANSPORTADOR_CODIGO", "")
DISPOSITIVO_CODIGO = os.getenv("DISPOSITIVO_CODIGO", "cam-01")
ZONA_DEFAULT = os.getenv("ZONA_DEFAULT", "")

# Solo guardamos una posición cuando pasó suficiente tiempo o el objeto se
# movió lo suficiente (evita una fila por cada fotograma).
POSITION_MIN_INTERVAL = float(os.getenv("POSITION_MIN_INTERVAL", "0.5"))  # segundos
POSITION_MIN_DISTANCE = float(os.getenv("POSITION_MIN_DISTANCE", "15"))  # px o unidades

# Homografía opcional (matriz 3x3 en JSON) para pasar de pixeles a coordenadas
# reales de planta. Ej: HOMOGRAPHY_MATRIX="[[..],[..],[..]]"
_HOMOGRAPHY_RAW = os.getenv("HOMOGRAPHY_MATRIX", "")

engine = create_engine(DATABASE_URL, future=True)
SessionLocal = sessionmaker(bind=engine, autoflush=False, autocommit=False, future=True)


# --- Utilidades de dominio ---------------------------------------------------
def _tipo_objeto(class_name: str):
    """Traduce el nombre de clase de YOLO a un TipoObjeto del dominio."""
    from backend.app.models import TipoObjeto

    mapping = {
        "caja": TipoObjeto.caja,
        "box": TipoObjeto.caja,
        "tarima": TipoObjeto.tarima,
        "pallet": TipoObjeto.tarima,
        "contenedor": TipoObjeto.contenedor,
        "container": TipoObjeto.contenedor,
        "producto": TipoObjeto.producto,
        "product": TipoObjeto.producto,
    }
    return mapping.get((class_name or "").lower())


def get_or_create_tipo_evento(session, codigo: str, nombre: str) -> TipoEvento:
    tipo = session.query(TipoEvento).filter_by(codigo=codigo).one_or_none()
    if tipo is None:
        tipo = TipoEvento(codigo=codigo, nombre=nombre)
        session.add(tipo)
        session.commit()
        session.refresh(tipo)
    return tipo


def resolve_transportador(session):
    if not TRANSPORTADOR_CODIGO:
        return None
    return (
        session.query(Transportador)
        .filter_by(codigo=TRANSPORTADOR_CODIGO)
        .one_or_none()
    )


def get_or_create_dispositivo(session, transportador):
    disp = (
        session.query(Dispositivo).filter_by(codigo=DISPOSITIVO_CODIGO).one_or_none()
    )
    if disp is None and transportador is not None:
        disp = Dispositivo(
            transportador_id=transportador.id,
            tipo=TipoDispositivo.camara,
            codigo=DISPOSITIVO_CODIGO,
            nombre=f"Cámara {DISPOSITIVO_CODIGO}",
        )
        session.add(disp)
        session.commit()
        session.refresh(disp)
    return disp


def _load_homography():
    if not _HOMOGRAPHY_RAW:
        return None
    try:
        return np.array(json.loads(_HOMOGRAPHY_RAW), dtype="float64")
    except (ValueError, TypeError):
        logger.warning("HOMOGRAPHY_MATRIX inválida; se usarán coordenadas en píxeles.")
        return None


def _to_world(homography, x, y):
    """Convierte (x, y) de píxeles a coordenadas de planta si hay homografía."""
    if homography is None:
        return x, y
    pt = np.array([[[float(x), float(y)]]], dtype="float64")
    dst = cv2.perspectiveTransform(pt, homography)
    return float(dst[0][0][0]), float(dst[0][0][1])


# --- Núcleo ------------------------------------------------------------------
def process_stream():
    homography = _load_homography()
    model = YOLO(YOLO_MODEL)

    source = int(CAMERA_SOURCE) if CAMERA_SOURCE.isdigit() else CAMERA_SOURCE
    logger.info("Abriendo fuente de video: %s", source)

    with SessionLocal() as session:
        transportador = resolve_transportador(session)
        if transportador is None:
            logger.warning(
                "No se encontró el transportador '%s'; los registros no quedarán "
                "asociados a un transportador.",
                TRANSPORTADOR_CODIGO,
            )
        dispositivo = get_or_create_dispositivo(session, transportador)
        ev_detectado = get_or_create_tipo_evento(
            session, "objeto_detectado", "Objeto detectado"
        )
        ev_zona = get_or_create_tipo_evento(
            session, "cambio_zona", "Cambio de zona"
        )

        transportador_id = transportador.id if transportador else None
        dispositivo_id = dispositivo.id if dispositivo else None

        # Estado en memoria por track_id.
        track_objeto: dict[int, uuid.UUID] = {}
        track_last: dict[int, tuple[float, float, float]] = {}  # x, y, t
        track_zona: dict[int, str] = {}

        # model.track hace streaming frame a frame con seguimiento persistente.
        results = model.track(
            source=source,
            stream=True,
            persist=True,
            conf=CONF_THRESHOLD,
            device=YOLO_DEVICE,
            verbose=False,
        )

        for result in results:
            ahora = time.time()
            boxes = getattr(result, "boxes", None)
            if boxes is None or boxes.id is None:
                continue

            names = result.names
            ids = boxes.id.int().tolist()
            clss = boxes.cls.int().tolist()
            xywh = boxes.xywh.tolist()  # centro x, centro y, ancho, alto

            for track_id, cls_idx, (cx, cy, _w, _h) in zip(ids, clss, xywh):
                class_name = names.get(cls_idx, str(cls_idx)) if isinstance(names, dict) else str(cls_idx)
                wx, wy = _to_world(homography, cx, cy)

                # 1) Objeto nuevo -> crear objeto + evento de aparición.
                if track_id not in track_objeto:
                    objeto = Objeto(
                        codigo=f"{DISPOSITIVO_CODIGO}-{track_id}-{int(ahora)}",
                        tipo=_tipo_objeto(class_name),
                        estado="en_transito",
                        fecha_creacion=datetime.utcnow(),
                    )
                    session.add(objeto)
                    session.commit()
                    session.refresh(objeto)
                    track_objeto[track_id] = objeto.id

                    session.add(
                        Evento(
                            tipo_evento_id=ev_detectado.id,
                            fecha_hora=datetime.utcnow(),
                            transportador_id=transportador_id,
                            dispositivo_id=dispositivo_id,
                            objeto_id=objeto.id,
                            prioridad=3,
                            estado="nuevo",
                        )
                    )
                    session.commit()
                    logger.info(
                        "Objeto detectado track=%s clase=%s -> %s",
                        track_id,
                        class_name,
                        objeto.codigo,
                    )

                objeto_id = track_objeto[track_id]

                # 2) Posición: solo si pasó tiempo o distancia suficiente.
                last = track_last.get(track_id)
                velocidad = None
                direccion = None
                guardar = last is None
                if last is not None:
                    lx, ly, lt = last
                    dt = ahora - lt
                    dist = ((wx - lx) ** 2 + (wy - ly) ** 2) ** 0.5
                    if dt >= POSITION_MIN_INTERVAL or dist >= POSITION_MIN_DISTANCE:
                        guardar = True
                        if dt > 0:
                            velocidad = dist / dt
                            direccion = "adelante" if (wx - lx) >= 0 else "atras"

                if guardar:
                    session.add(
                        Posicion(
                            objeto_id=objeto_id,
                            transportador_id=transportador_id,
                            zona=ZONA_DEFAULT or None,
                            fecha_hora=datetime.utcnow(),
                            posicion_x=wx,
                            posicion_y=wy,
                            velocidad=velocidad,
                            direccion=direccion,
                        )
                    )
                    session.commit()
                    track_last[track_id] = (wx, wy, ahora)

                # 3) Cambio de zona -> evento.
                if ZONA_DEFAULT and track_zona.get(track_id) != ZONA_DEFAULT:
                    if track_zona.get(track_id) is not None:
                        session.add(
                            Evento(
                                tipo_evento_id=ev_zona.id,
                                fecha_hora=datetime.utcnow(),
                                transportador_id=transportador_id,
                                dispositivo_id=dispositivo_id,
                                objeto_id=objeto_id,
                                prioridad=4,
                                estado="nuevo",
                            )
                        )
                        session.commit()
                    track_zona[track_id] = ZONA_DEFAULT


def main():
    if not _DEPS_OK:
        logger.error(
            "Faltan dependencias de visión (%s). Instálalas con: "
            "pip install -r requirements-vision.txt",
            _IMPORT_ERROR,
        )
        raise SystemExit(1)

    logger.info("Vision worker iniciado (modelo=%s, device=%s)", YOLO_MODEL, YOLO_DEVICE)
    while True:
        try:
            process_stream()
            logger.warning("El stream terminó; reintentando en 3s...")
        except KeyboardInterrupt:
            logger.info("Detenido por el usuario.")
            break
        except Exception:  # noqa: BLE001 - el worker debe seguir vivo
            logger.exception("Error en el pipeline de visión; reintentando en 3s...")
        time.sleep(3)


if __name__ == "__main__":
    main()
