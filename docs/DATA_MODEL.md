# Modelo de datos y contratos

## Principio

Little Brother almacena observaciones del mundo físico y evita modelar marcas,
protocolos o productos externos como entidades centrales. Un PLC, una cámara o
un WMS son fuentes; lo persistido son eventos, detecciones, estados y relaciones
normalizadas.

## Dominio de transportadores

`transport-core` representa:

- plantas;
- transportadores con dimensiones y estado operativo;
- conexiones físicas dirigidas;
- cajas, pallets, contenedores u otros objetos;
- ubicación actual de cada objeto;
- historial de entrada, transferencia, paro, reanudación, salida o desaparición.

Una conexión sólo representa un flujo físicamente posible. No contiene reglas
PLC, decisiones de ruteo ni lógica WMS. El repositorio implementado actualmente
es en memoria y sirve para validar el modelo.

## EventEnvelope

Todo evento publicable contiene:

| Campo | Significado |
|---|---|
| `id` | Identificador del evento |
| `event_type` | Tipo normalizado |
| `schema_version` | Versión del contrato |
| `source` | Fuente neutral y su tipo |
| `occurred_at_ms` | Tiempo del hecho |
| `observed_at_ms` | Tiempo de observación |
| `correlation_id` | Relación opcional con otros eventos |
| `payload` | Entidad específica del evento |

El evento persistido en la Fase 5 usa el tipo
`vision.detection.observed` y un payload `VisionDetection`.

## VisionDetection

```text
VisionDetection
├── detection_id
├── source_id
├── frame_id
├── timestamp_ms
├── class_id
├── class_name
├── confidence
└── bounding_box
    ├── x
    ├── y
    ├── width
    └── height
```

La caja usa coordenadas relativas al ancho y alto de la imagen. Los valores
permanecen en el intervalo `0..1`, por lo que el contrato no depende de una
resolución específica.

## VisionTrack

```text
VisionTrack
├── track_id / camera_id
├── class_id / class_name
├── history[]
│   └── detection_id, frame_id, timestamp_ms, bounding_box, confidence
├── started_at_ms / last_observed_at_ms
├── state
├── accumulated_confidence
└── missed_frames
```

Estados:

```text
tentative → confirmed ↔ lost → finished
```

El tracker conserva la identidad dentro de una ejecución. No garantiza que un
objeto reciba el mismo `track_id` después de reiniciar el proceso.

## SpatialTrack

```text
SpatialTrack
├── track_id / camera_id / timestamp_ms
├── anchor
├── inside_observation_region
├── occupied_zones[]
└── crossed_lines[]
```

`SpatialTrack` interpreta píxeles respecto a regiones configuradas, pero todavía
no transforma esas coordenadas a metros ni calcula velocidad física.

## Enrutamiento de persistencia

```text
EventEnvelope<VisionDetection>
        ↓
PersistencePolicy
        ↓
PersistenceDomain::Temporal
        ↓
JaibaVisionDetectionWriter  →  POST DMA_JAIVA/api/v1/ingest/events
        ↓
Jaiba (buffer / hot path) → motores detrás de Jaiba
```

`vision-core` no conoce el bus, Jaiba, SQL ni ningún motor. La composición
ocurre en `vision-inference` y el puente HTTP vive en `jaiba-bridge`.

Broder **no** define ni migra tablas. El esquema histórico (p. ej. ClickHouse
`temporal.vision_detection`) pertenece a Jaiba / DMA_JAIVA.

## Entrega asíncrona

Antes del puente existe una cola acotada. El hilo de video solo hace
`try_send` (fire-and-forget): nunca espera a Jaiba ni a un INSERT. El worker
agrupa lotes, hace flush por tamaño/intervalo y reintenta el puente si Jaiba
cae. `persisted` en métricas significa “aceptado por Jaiba (2xx)”, no
“escrito en ClickHouse/Postgres”.

## Semántica de tiempos

Para no confundir la posición de un video con una fecha real:

- `occurred_at` y `observed_at` usan el reloj Unix del SP;
- `source_timestamp_ms` conserva la posición relativa del archivo o flujo;
- cualquier `persisted_at` lo asigna el motor detrás de Jaiba.

## Identificadores

- `detection_id`: `source_id:frame_id:secuencia_en_frame`;
- `track_id`: `camera_id:track:secuencia`;
- `event_id`: `source_id:vision:session_id:secuencia`.

El componente de sesión evita colisiones al reiniciar una cámara o reprocesar
un archivo. El `event_id` es la clave idempotente de persistencia.

## Datos que todavía no se persisten

La tabla actual no guarda:

- video ni imágenes;
- `VisionTrack`;
- `SpatialTrack`;
- estado operativo de transportadores;
- alarmas;
- telegramas PLC o mensajes WMS;
- métricas de rendimiento o logs estructurados.

Tampoco existe aún una política de retención o particionamiento. Antes de una
operación continua se deberán definir conservación, limpieza, respaldo y, si el
volumen lo exige, particiones o un motor temporal especializado.
