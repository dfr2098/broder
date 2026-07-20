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
PostgresVisionDetectionWriter
```

`vision-core` no conoce el bus, PostgreSQL ni SQL. La composición ocurre en la
aplicación `vision-inference` y el SQL permanece dentro del adaptador de
infraestructura `persistence-postgres`.

## Tabla temporal.vision_detection

| Columna | Tipo | Descripción |
|---|---|---|
| `event_id` | `TEXT` PK | ID único por sesión y secuencia |
| `event_type` | `TEXT` | `vision.detection.observed` |
| `schema_version` | `SMALLINT` | Versión del contrato |
| `occurred_at` | `TIMESTAMPTZ` | Tiempo real de publicación |
| `observed_at` | `TIMESTAMPTZ` | Tiempo real de observación |
| `source_id` | `TEXT` | Cámara lógica |
| `correlation_id` | `TEXT` | Correlación opcional |
| `detection_id` | `TEXT` | ID producido por visión |
| `frame_id` | `BIGINT` | Frame de la fuente |
| `source_timestamp_ms` | `BIGINT` | Posición temporal dentro del flujo |
| `class_id` | `INTEGER` | Clase numérica del modelo |
| `class_name` | `TEXT` | Nombre de clase |
| `confidence` | `REAL` | Confianza entre 0 y 1 |
| `bbox_x`, `bbox_y` | `REAL` | Origen normalizado |
| `bbox_width`, `bbox_height` | `REAL` | Dimensiones normalizadas |
| `persisted_at` | `TIMESTAMPTZ` | Tiempo de inserción PostgreSQL |

Índices actuales:

- `occurred_at DESC`;
- `(source_id, occurred_at DESC)`;
- `(class_id, occurred_at DESC)`.

Las restricciones validan rangos, dimensiones, versión y valores no negativos.
La migración se ejecuta automáticamente al conectar y usa operaciones
idempotentes. Las inserciones usan una sentencia preparada dentro de una
transacción por lotes y `ON CONFLICT (event_id) DO NOTHING`.

Antes del adaptador existe una cola acotada. El worker hace flush al completar
el lote, vencer el intervalo o cerrar el motor. Si una transacción falla, abre
una conexión nueva y reintenta una vez. En modo `required` se aplica
backpressure; en `best-effort` el análisis continúa y contabiliza las pérdidas.

## Semántica de tiempos

Para no confundir la posición de un video con una fecha real:

- `occurred_at` y `observed_at` usan el reloj Unix del SP;
- `source_timestamp_ms` conserva la posición relativa del archivo o flujo;
- `persisted_at` lo asigna PostgreSQL al insertar.

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
