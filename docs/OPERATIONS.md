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

## Visor provisional

Mostrar el video a 5 FPS:

```bash
make viewer
```

Obtener metadatos sin abrir ventana:

```bash
make viewer-info
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

Seguir los logs desde otra terminal:

```bash
make vision-logs
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
| `--max-inferences N` | Finaliza después de N inferencias |
| `--track-min-hits N` | Observaciones para confirmar identidad |
| `--track-max-missed N` | Pérdidas consecutivas toleradas |
| `--track-max-lost-ms N` | Tiempo máximo sin observación |
| `--track-min-iou N` | IoU mínima para asociación |
| `--track-max-distance N` | Distancia normalizada máxima |
| `--database-url URL` | Conexión PostgreSQL explícita |
| `--no-persistence` | Omite la conexión y las escrituras |

El visor provisional admite `--fps`, `--log`, `--info` y un único archivo de
video. Su ayuda se consulta con:

```bash
cargo run --manifest-path core/rs/Cargo.toml -p video-viewer -- --help
```

Cuando `DATABASE_URL` está configurada, el proceso exige que PostgreSQL esté
disponible. Si falla una escritura, el proceso termina con un error visible en
vez de perder la detección silenciosamente.

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

```bash
make vision \
  VIDEO="rtsp://usuario:clave@192.168.1.20/stream" \
  SOURCE_ID="cam-entrada" \
  SPATIAL_CONFIG="core/vision/config/cam-entrada.spatial"
```

La fuente RTSP se captura continuamente, pero sólo se procesa a la frecuencia
configurada. En esta fase la fuente completa se escribe en el log inicial; no
se deben usar credenciales de producción embebidas en la URL hasta incorporar
redacción de secretos o un mecanismo seguro de configuración.

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
```

El resumen final incluye frames capturados, inferencias, detecciones,
detecciones persistidas y tracks finalizados.

## Verificación del proyecto

```bash
make test
make lint
make verify-model
make check
make release
```

Las pruebas cubren bus y router, topología y movimientos, muestreo y NMS,
tracking, geometría espacial, política de persistencia y conversiones de tipos
PostgreSQL. La integración real con la base se valida mediante
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
