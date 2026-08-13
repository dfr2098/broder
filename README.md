# Little Brother

Little Brother es una plataforma de observabilidad industrial. El prototipo
actual cubre el modelo físico de transportadores, inferencia YOLO, tracking,
interpretación espacial y entrega fire-and-forget de detecciones a **Jaiba**;
las DB son sinks opcionales detrás de Jaiba, no del runtime de Broder.
Permanece independiente de PLC, WMS y fabricantes específicos.

Alcance de conectividad: ver [`docs/BRIEF_DMA_JAIVA_JAIBA_BRODER.md`](docs/BRIEF_DMA_JAIVA_JAIBA_BRODER.md)
(Jaiba ≠ lab DMA_JAIVA ≠ Broder).

## Documentación

La guía completa comienza en [`docs/README.md`](docs/README.md):

- [diagramas de flujo](docs/FLOWS.md);
- [instalación](docs/INSTALLATION.md);
- [operación y diagnóstico](docs/OPERATIONS.md);
- [modelo de datos](docs/DATA_MODEL.md);
- [fases y estado](docs/PHASES.md);
- [arquitectura](core/rs/ARCHITECTURE.md).

## Componentes actuales

```text
core/rs/crates/event-core        Contratos del bus de eventos
core/rs/crates/persistence-core  Puertos y router de persistencia
core/rs/crates/jaiba-bridge            Puente Broder → Jaiba (sin drivers de DB)
core/rs/crates/transport-core    Dominio físico de transportadores
core/rs/crates/vision-core       Detecciones, muestreo y NMS neutrales
core/rs/crates/tracking-core     Identidad temporal y trayectorias visuales
core/rs/crates/spatial-core      Regiones, zonas y líneas virtuales
core/rs/apps/transport-simulator Simulador local
core/rs/apps/video-viewer        Visor provisional de videos
core/rs/apps/vision-inference    Motor YOLO 11 con OpenCV DNN
core/yolo/models                 Modelos ONNX locales
docker-compose.yml               Nginx + sinks opcionales CH/PG para Jaiba
```

Los procesos Rust se ejecutan directamente en el SP o equipo de planta.
El visualizador Nginx corre en contenedor. ClickHouse/PostgreSQL en Compose
son **opcionales** (sinks para Jaiba vía DAG). Broder solo entrega
`EventEnvelope` al ingest Jaiba (env `DMA_JAIVA` = URL legado). El lab KPI
`DMA_JAIVA` es un circuito aparte. El panel recibe WebSockets vía proxy hacia
`vision-inference`. Los núcleos no conocen SQL, drivers ni Nginx.

## Comprobar el proyecto

Diagnosticar dependencias, archivos y Jaiba:

```bash
make doctor
make doctor  # comprueba DMA_JAIVA si está definida
```

Ejecutar todas las pruebas:

```bash
make check
```

Ejecutar el simulador:

```bash
make run
```

Abrir el video de prueba a 5 FPS:

```bash
make viewer
```

Controles del visor: `Espacio` pausa, `N` avanza un frame, `R` reinicia y
`Q`/`Esc` cierra. Para consultar metadatos sin abrir una ventana:

```bash
make viewer-info
```

El visor muestra sus eventos en la terminal y también los agrega a
`logs/video-viewer.log`. Registra apertura, metadatos, progreso cada 10 segundos,
pausas, reinicios, fin y cierre; no escribe una línea por frame. Para usar otro
archivo:

```bash
make viewer LOG="logs/prueba-01.log"
```

Para seguir el log en vivo desde otra terminal:

```bash
make viewer-logs
```

## Inferencia YOLO

Ejecutar el motor a 5 FPS y mostrar las detecciones:

```bash
make vision
```

Construir el contenedor del panel y reproducir el video de demostración:

```bash
make demo-web
```

Después abra `http://127.0.0.1:8088`. El comando utiliza exclusivamente el MP4
incluido en `video prueba/`; el panel muestra el video, cajas, identificadores
de track, zonas y métricas sin requerir ClickHouse. El MP4 se repite hasta que
el usuario detiene el motor con `Ctrl+C`.

Ejecutarlo sin ventana o realizar una prueba corta de seis inferencias:

```bash
make vision-headless
make vision-smoke
```

El motor acepta tanto archivos como direcciones RTSP. Por ejemplo:

```bash
make vision VIDEO="rtsp://usuario:clave@192.168.1.20/stream" SOURCE_ID="cam-entrada"
```

Las detecciones se muestran en la terminal y se agregan a
`logs/vision-inference.log`. Cada registro `DETECTION` contiene frame, marca de
tiempo, clase, confianza y una caja normalizada `[x,y,width,height]`. Para seguir
el archivo desde otra terminal:

```bash
make vision-logs
```

El modelo `yolo11n.onnx` incluido usa las clases COCO y no contiene una clase
industrial específica para pallet. Sirve para validar el motor; la precisión
sobre pallets requerirá un modelo entrenado con imágenes de la planta.

## Seguimiento de objetos

La ejecución de `make vision` también asocia detecciones consecutivas y muestra
un identificador como `000001` sobre cada objeto. Los logs `TRACK` contienen el
estado, número de observaciones, pérdidas consecutivas, confianza promedio y
última posición. Al terminar aparece un registro `TRACK_FINISHED`.

Estados posibles:

```text
tentative -> confirmed <-> lost -> finished
```

Los valores predeterminados confirman un track después de dos observaciones y
toleran cinco inferencias perdidas o 1500 ms sin detección. Se pueden ajustar:

```bash
make vision \
  TRACK_MIN_HITS=2 \
  TRACK_MAX_MISSED=5 \
  TRACK_MAX_LOST_MS=1500 \
  TRACK_MIN_IOU=0.05 \
  TRACK_MAX_DISTANCE=0.25
```

Las distancias son relativas a la imagen, no metros. Este módulo no calcula
velocidad física, no identifica transportadores y no genera alarmas.

## Modelo espacial

`make vision` carga la geometría DEMO de
`core/vision/config/camera-1.spatial`. La etiqueta visual agrega la zona más
específica, y el visor dibuja los polígonos y líneas virtuales configurados. Los
logs generan registros `SPATIAL`:

```text
SPATIAL track=camera-1:track:000001 ...
zones=[transportador-demo,carril-central] crossings=[]
```

La posición espacial usa el punto inferior central de la caja detectada. La
configuración admite:

```text
camera=camera-1
observation=x,y;x,y;x,y;x,y
zone=id|nombre|tipo|parent_id|dirección|x,y;x,y;x,y;x,y
line=id|nombre|rol|x,y|x,y
```

Tipos de zona: `conveyor`, `lane`, `entry`, `exit` o `custom:nombre`. Roles de
línea: `entry`, `exit` y `boundary`. Todas las coordenadas están normalizadas
entre `0` y `1`.

Para usar otra calibración:

```bash
make vision SPATIAL_CONFIG="core/vision/config/camara-entrada.spatial"
```

La geometría incluida es únicamente demostrativa. El video de prueba mueve la
cámara, por lo que no permite representar posiciones físicas estables; una
instalación real necesita cámara fija y una calibración propia.

Compilar los binarios optimizados para el SP:

```bash
make release
```

## Fase 5: puente Jaiba (fire-and-forget)

Opcional: levantar sinks locales para que Jaiba pueda persistir, o apuntar
`DMA_JAIVA` (env legado) al ingest Jaiba:

```bash
cp .env.example .env
make infra-up   # CH + PG opcionales; Broder no los requiere para correr
```

`make vision`, `make vision-headless` y `make vision-smoke` leen
`DMA_JAIVA` y entregan cada `EventEnvelope` a Jaiba sin esperar confirmación
de base de datos. El hilo de video nunca se bloquea por Jaiba ni por un INSERT.

```text
mode=required queue=256 batch=25 flush_ms=500
```

Por defecto el modo es `best-effort`. Si Jaiba no está disponible:

```bash
make vision PERSISTENCE_MODE=best-effort
```

Broder no consulta bases de datos. El histórico, si existe, se lee desde el
sink que Jaiba haya escrito (p. ej. ClickHouse local vía `make vision-query`).

```bash
make vision-query
```

Para ejecutar sin entregar eventos a Jaiba: `--no-persistence`.

Detener sinks opcionales sin eliminar sus datos:

```bash
make infra-down
```

## Modelo YOLO local

El modelo esperado es `core/yolo/models/yolo11n.onnx`. No se versiona en Git.
Para comprobar que corresponde al modelo aprobado:

```bash
make verify-model
```

La arquitectura y los diagramas están documentados en
[`core/rs/ARCHITECTURE.md`](core/rs/ARCHITECTURE.md).
