# Operación del prototipo

Todos los comandos de esta guía se ejecutan desde la raíz del repositorio.

## Variables de ejecución

| Variable | Predeterminado | Uso |
|---|---|---|
| `VIDEO` | video incluido en `video prueva/` | Archivo o URL RTSP |
| `FPS` | `5` | Inferencias o frames mostrados por segundo |
| `MODEL` | `core/yolo/models/yolo11n.onnx` | Modelo ONNX |
| `CONFIDENCE` | `0.25` | Confianza mínima de YOLO |
| `NMS` | `0.45` | IoU usada por NMS |
| `SOURCE_ID` | `camera-1` | Identidad lógica de cámara |
| `SPATIAL_CONFIG` | `core/vision/config/camera-1.spatial` | Geometría de cámara |
| `VISION_LOG` | `logs/vision-inference.log` | Log del motor |
| `LOG` | `logs/video-viewer.log` | Log del visor |
| `TRACK_MIN_HITS` | `2` | Detecciones para confirmar un track |
| `TRACK_MAX_MISSED` | `5` | Inferencias perdidas toleradas |
| `TRACK_MAX_LOST_MS` | `1500` | Tiempo máximo sin observación |
| `TRACK_MIN_IOU` | `0.05` | IoU mínima de asociación |
| `TRACK_MAX_DISTANCE` | `0.25` | Distancia normalizada máxima |
| `DB_PORT` | `5432` | Puerto local de PostgreSQL |
| `DATABASE_URL` | construida desde `.env` | Conexión usada por Rust |
| `PERSISTENCE_MODE` | `required` | `required` o `best-effort` |
| `PERSISTENCE_QUEUE` | `256` | Eventos máximos esperando al worker |
| `PERSISTENCE_BATCH` | `25` | Detecciones por transacción |
| `PERSISTENCE_FLUSH_MS` | `500` | Tiempo máximo antes del commit |

Las variables pueden pasarse a `make` sin modificar archivos:

```bash
make vision FPS=5 CONFIDENCE=0.40 SOURCE_ID=cam-entrada
```

## Infraestructura PostgreSQL

Iniciar, inspeccionar logs y detener:

```bash
make infra-up
make infra-logs
make infra-down
```

`make infra-down` conserva el volumen. El siguiente comando elimina el
contenedor y también todos los datos del volumen PostgreSQL:

```bash
make infra-reset
```

Use `infra-reset` únicamente cuando la pérdida total de los datos sea
intencional.

Comprobar instalación, archivos y coherencia del puerto PostgreSQL:

```bash
make doctor
make doctor DB_PORT=55432
```

## Visor provisional

El video local configurado de manera predeterminada es:

```text
video prueba/WhatsApp Video 2026-07-20 at 10.07.07 AM.mp4
```

Es un MP4 de aproximadamente 1.9 MB y 14 segundos. La carpeta `video prueba/`
está excluida de Git, por lo que cada instalación debe copiar este archivo o
indicar otra fuente mediante `VIDEO="ruta/al/video.mp4"`.

Mostrar el video a 5 FPS:

```bash
make viewer
```

Obtener metadatos sin abrir ventana:

```bash
make viewer-info
```

Para validar explícitamente otro archivo:

```bash
make viewer-info VIDEO="ruta/al/video.mp4"
```

Controles:

| Tecla | Acción |
|---|---|
| Espacio | Pausar o continuar |
| `N` | Avanzar un frame estando pausado |
| `R` | Reiniciar el video |
| `Q` o `Esc` | Cerrar |

Seguir el log:

```bash
make viewer-logs
```

## Motor YOLO, tracking y modelo espacial

Ejecución con ventana:

```bash
make vision
```

Ejecución sin ventana:

```bash
make vision-headless
```

Prueba corta de seis inferencias:

```bash
make vision-smoke
```

Si PostgreSQL todavía no está disponible y solo se quiere comprobar el video y
el motor, se puede permitir que la prueba continúe sin conexión:

```bash
make vision-smoke PERSISTENCE_MODE=best-effort
```

La prueba requiere OpenCV 4 y el archivo
`core/yolo/models/yolo11n.onnx`. `make doctor` informa si falta alguno de estos
recursos antes de iniciar la inferencia.

Seguir los logs desde otra terminal:

```bash
make vision-logs
```

## Panel web por WebSocket

El visualizador se ejecuta en un contenedor Nginx. Sirve la interfaz y hace
proxy de `/ws` y `/health` hacia el backend nativo de `vision-inference`, que
transmite cada frame procesado como JPEG junto con detecciones, tracking, zonas
y métricas. El flujo no consulta PostgreSQL.

Iniciar la demostración local con el video incluido:

```bash
make demo-web
```

Este comando construye e inicia `little-brother-web` y después ejecuta el motor
Rust nativo exclusivamente con el MP4 de `video prueba/`. Al detener el motor,
el contenedor permanece disponible. El video vuelve al inicio automáticamente
hasta que se presiona `Ctrl+C`. Gestión independiente del visualizador:

```bash
make web-up
make web-logs
make web-down
```

Abrir en el navegador:

```text
http://127.0.0.1:8088
```

Rutas disponibles:

| Ruta | Función |
|---|---|
| `/` | Interfaz de visualización |
| `/ws` | Flujo WebSocket de frames y metadatos |
| `/health` | Estado HTTP y cantidad de clientes conectados |

El comando `demo-web` utiliza `--no-persistence` para que la demostración no
dependa de PostgreSQL. Para combinar el panel con persistencia se puede
ejecutar directamente `vision-inference` con `--web-bind` y la configuración
normal de base de datos.

Puertos predeterminados:

| Componente | Dirección |
|---|---|
| Visualizador Nginx en Docker | `127.0.0.1:8088` |
| Backend HTTP/WebSocket nativo | `0.0.0.0:8081` |

Para escuchar en todas las interfaces de red:

```bash
make vision-web WEB_HOST="0.0.0.0" WEB_PORT=8088
```

> **Seguridad:** esta primera versión no implementa autenticación ni TLS. No
> exponga `0.0.0.0:8088` fuera de una red de planta controlada. Para acceso
> remoto se debe colocar un proxy autenticado con HTTPS delante del servicio.

El mensaje WebSocket `frame` contiene:

```text
source_id / frame_id / timestamp_ms
image              JPEG como data URL
width / height
inference_ms / processing_fps / browser_clients
detections[]       clase, confianza, caja, track_id, estado y zonas
active_tracks[]    identidad, clase, estado y observaciones
geometry           región útil, polígonos de zona y líneas virtuales
```

### Opciones directas de vision-inference

La ayuda siempre refleja el binario instalado:

```bash
cargo run --manifest-path core/rs/Cargo.toml -p vision-inference -- --help
```

| Opción | Descripción |
|---|---|
| argumento `<archivo o rtsp://...>` | Fuente obligatoria |
| `--model RUTA` | Modelo YOLO 11 ONNX |
| `--classes RUTA` | Archivo con una clase por línea |
| `--spatial-config RUTA` | Geometría normalizada de la cámara |
| `--source-id ID` | Identidad lógica de la fuente |
| `--fps N` | Frecuencia de inferencia |
| `--confidence N` | Confianza mínima entre 0 y 1 |
| `--nms N` | IoU de NMS entre 0 y 1 |
| `--log RUTA` | Archivo de log |
| `--display` | Abre la ventana de visualización |
| `--loop-video` | Repite continuamente una fuente de archivo |
| `--web-bind IP:PUERTO` | Sirve el panel HTTP/WebSocket |
| `--max-inferences N` | Finaliza después de N inferencias |
| `--track-min-hits N` | Observaciones para confirmar identidad |
| `--track-max-missed N` | Pérdidas consecutivas toleradas |
| `--track-max-lost-ms N` | Tiempo máximo sin observación |
| `--track-min-iou N` | IoU mínima para asociación |
| `--track-max-distance N` | Distancia normalizada máxima |
| `--database-url URL` | Conexión PostgreSQL explícita |
| `--no-persistence` | Omite la conexión y las escrituras |
| `--persistence-mode MODO` | `required` o `best-effort` |
| `--persistence-queue N` | Capacidad acotada de la cola |
| `--persistence-batch N` | Eventos por transacción |
| `--persistence-flush-ms N` | Intervalo máximo para confirmar |

El visor provisional admite `--fps`, `--log`, `--info` y un único archivo de
video. Su ayuda se consulta con:

```bash
cargo run --manifest-path core/rs/Cargo.toml -p video-viewer -- --help
```

La persistencia se ejecuta en un worker independiente. `required` aplica
backpressure cuando se llena la cola y termina con error si PostgreSQL no se
recupera. `best-effort` mantiene la visión activa, reintenta la conexión y
contabiliza los eventos descartados:

```bash
make vision PERSISTENCE_MODE=best-effort
```

Para ejecutar una inspección sin persistencia se puede llamar directamente al
binario:

```bash
cargo run --manifest-path core/rs/Cargo.toml -p vision-inference -- \
  --no-persistence \
  --fps 5 \
  --source-id camera-1 \
  --model core/yolo/models/yolo11n.onnx \
  --spatial-config core/vision/config/camera-1.spatial \
  "video prueva/Sistema de Transportadores de Pallets conformado de 19 transportes, 18 de ellos motorizados..mp4"
```

## Cámara RTSP

### Inventario de múltiples cámaras

El archivo [`core/vision/config/cameras.toml`](../core/vision/config/cameras.toml)
centraliza las IP y los parámetros operativos de todas las cámaras. Cada bloque
`[[cameras]]` define una fuente con un `id` único y estable.

```toml
[[cameras]]
id = "camera-1"
enabled = true
name = "Transportador de entrada"
location = "Línea 1"
ip = "192.168.10.101"
rtsp_port = 554
rtsp_path = "/Streaming/Channels/101"
rtsp_url_env = "CAMERA_1_RTSP_URL"
transport = "tcp"
spatial_config = "core/vision/config/camera-1.spatial"
```

La dirección completa, incluyendo usuario y contraseña, se configura en el
archivo local `.env`, que está excluido de Git:

```dotenv
CAMERA_1_RTSP_URL=rtsp://usuario:contrasena@192.168.10.101:554/Streaming/Channels/101
```

No se deben guardar credenciales reales en `cameras.toml`. Para incorporar una
cámara se copia un bloque, se asignan un `id`, una IP y una variable de entorno
únicos, y se crea su archivo `.spatial`. Una cámara con `enabled = false`
permanece inventariada, pero no debe ser iniciada por el supervisor.

> El motor actual ejecuta una fuente por proceso. `cameras.toml` establece el
> contrato para el futuro supervisor multi-cámara; cada proceso debe recibir el
> RTSP, `id` y archivo espacial del bloque correspondiente.

```bash
make vision \
  VIDEO="rtsp://usuario:clave@192.168.1.20/stream" \
  SOURCE_ID="cam-entrada" \
  SPATIAL_CONFIG="core/vision/config/cam-entrada.spatial"
```

La fuente RTSP se captura continuamente, pero sólo se procesa a la frecuencia
configurada. El usuario y contraseña se sustituyen por `***` en logs y errores;
OpenCV recibe internamente la URL original para conectarse.

## Configuración espacial

El formato es texto plano y todas las coordenadas están normalizadas entre
`0` y `1`:

```text
camera=camera-1
observation=x,y;x,y;x,y;x,y
zone=id|nombre|tipo|parent_id|dirección|x,y;x,y;x,y;x,y
line=id|nombre|rol|x,y|x,y
```

Tipos de zona:

- `conveyor`;
- `lane`;
- `entry`;
- `exit`;
- `custom:nombre`.

Roles de línea:

- `entry`;
- `exit`;
- `boundary`.

La posición de un objeto se calcula con el punto inferior central de su caja.
Las zonas pueden anidarse con `parent_id`. La cámara configurada debe coincidir
con `SOURCE_ID`.

La configuración incluida es DEMO. Como el video de prueba mueve la cámara,
sus regiones no representan posiciones físicas permanentes. Una instalación
real requiere cámara fija y polígonos calibrados para esa escena.

## Logs

El motor genera cuatro registros principales:

```text
DETECTION       observación independiente de YOLO
TRACK           identidad activa y trayectoria acumulada
TRACK_FINISHED  identidad terminada
SPATIAL         zona ocupada y cruces de líneas
PERSISTENCE     estado periódico de cola y confirmaciones
PERSISTENCE_FINAL  métricas después del flush de cierre
METRICS         FPS efectivo y latencia media/máxima de inferencia
```

El resumen final incluye frames capturados, inferencias, detecciones,
detecciones persistidas, descartadas y tracks finalizados. Una detección sólo
cuenta como persistida después del commit PostgreSQL.

## Verificación del proyecto

```bash
make test
make lint
make verify-model
make check
make release
```

Las pruebas cubren bus, flush y router, topología y movimientos, muestreo y
NMS, tracking, geometría espacial, política de persistencia, redacción de
secretos y conversiones PostgreSQL. La integración real se valida mediante
`make vision-smoke` seguido de `make vision-query`.

## Consultas PostgreSQL

Últimas veinte detecciones:

```bash
make vision-query
```

Entrar a `psql`:

```bash
docker compose exec db psql -U little_brother -d little_brother
```

Consultas útiles:

```sql
-- Más recientes
SELECT *
FROM temporal.vision_detection
ORDER BY occurred_at DESC
LIMIT 20;

-- Por cámara
SELECT occurred_at, frame_id, class_name, confidence
FROM temporal.vision_detection
WHERE source_id = 'camera-1'
ORDER BY occurred_at DESC;

-- Conteo por clase
SELECT class_id, class_name, count(*) AS detections
FROM temporal.vision_detection
GROUP BY class_id, class_name
ORDER BY detections DESC;

-- Rendimiento aproximado por minuto
SELECT date_trunc('minute', occurred_at) AS minute, count(*) AS detections
FROM temporal.vision_detection
GROUP BY minute
ORDER BY minute DESC;
```

## Diagnóstico

### El puerto 5432 está ocupado

```bash
make infra-up DB_PORT=55432
make vision DB_PORT=55432
```

### No se encuentra OpenCV

Compruebe:

```bash
pkg-config --modversion opencv4
pkg-config --cflags --libs opencv4
```

Si `pkg-config` no encuentra `opencv4.pc`, corrija la instalación local o
`PKG_CONFIG_PATH` antes de compilar.

### No se abre una ventana

Use `make vision-headless` o `make viewer-info` en un SP sin sesión gráfica.

### No se encuentra el modelo

```bash
ls -lh core/yolo/models/yolo11n.onnx
make verify-model
```

### PostgreSQL no responde

```bash
docker compose ps
make infra-logs
```

Confirme que `DB_PORT` y el puerto contenido en `DATABASE_URL` sean iguales.

### YOLO identifica una clase incorrecta

El modelo actual es COCO genérico. La prueba ha llegado a clasificar la portada
del video como `stop sign`; esto demuestra el flujo técnico, no precisión
industrial. Se requiere un modelo entrenado con imágenes y clases de la planta.
