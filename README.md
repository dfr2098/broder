# Little Brother

Little Brother es una plataforma de observabilidad industrial. El prototipo
actual cubre el modelo físico de transportadores, inferencia YOLO, tracking,
interpretación espacial y persistencia temporal de detecciones en PostgreSQL.
Permanece independiente de PLC, WMS y fabricantes específicos.

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
core/rs/crates/persistence-postgres  Adaptador temporal PostgreSQL
core/rs/crates/transport-core    Dominio físico de transportadores
core/rs/crates/vision-core       Detecciones, muestreo y NMS neutrales
core/rs/crates/tracking-core     Identidad temporal y trayectorias visuales
core/rs/crates/spatial-core      Regiones, zonas y líneas virtuales
core/rs/apps/transport-simulator Simulador local
core/rs/apps/video-viewer        Visor provisional de videos
core/rs/apps/vision-inference    Motor YOLO 11 con OpenCV DNN
core/yolo/models                 Modelos ONNX locales
docker-compose.yml               PostgreSQL de infraestructura
```

Los procesos Rust se ejecutan directamente en el SP o equipo de planta.
PostgreSQL y el visualizador Nginx se ejecutan en contenedores. PostgreSQL se
conecta al proceso mediante el bus de eventos y `PersistenceRouter`; el panel
recibe los WebSockets mediante un proxy hacia `vision-inference`. Los núcleos
visuales no conocen SQL ni Nginx.

## Comprobar el proyecto

Diagnosticar dependencias, archivos y PostgreSQL:

```bash
make doctor
make doctor DB_PORT=55432  # cuando se usa el puerto alternativo
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
de track, zonas y métricas sin requerir PostgreSQL. El MP4 se repite hasta que
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

## Fase 5: persistencia PostgreSQL

Crear la configuración local e iniciar PostgreSQL:

```bash
cp .env.example .env
make infra-up
```

`make vision`, `make vision-headless` y `make vision-smoke` leen
`DATABASE_URL` y envían cada `VisionDetection` a un worker de persistencia. El
worker usa una cola acotada, transacciones por lotes, flush periódico y
reconexión. El esquema y los índices se crean de manera idempotente.

```text
mode=required queue=256 batch=25 flush_ms=500
```

Para mantener la visión activa cuando PostgreSQL no esté disponible:

```bash
make vision PERSISTENCE_MODE=best-effort
```

Consultar las últimas veinte detecciones:

```bash
make vision-query
```

O ejecutar directamente:

```sql
SELECT *
FROM temporal.vision_detection
ORDER BY occurred_at DESC;
```

Si `5432` ya está ocupado, se puede cambiar el puerto sin modificar archivos:

```bash
make infra-up DB_PORT=55432
make vision DB_PORT=55432
```

Para ejecutar el motor provisionalmente sin PostgreSQL, se puede usar el
binario con `--no-persistence`.

Detenerla sin eliminar sus datos:

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
