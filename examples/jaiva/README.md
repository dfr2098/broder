# Arquitectura Broder ↔ Jaiba (lab DMA_JAIVA)

Regla de Broder: **no conoce drivers, credenciales ni particularidades de DB**.

```text
BRODER
   │  EventEnvelope (fire-and-forget)
   ▼
JAIBA  (DMA_JAIVA)
   │
   ├─ Hot path / buffer en memoria
   ├─ Histórico ──────────► ClickHouse (recomendado para visión)
   ├─ Config / estado ────► PostgreSQL
   ├─ Legacy / ERP ───────► Oracle
   └─ Entity store ───────► ScyllaDB / otras
```

## Contrato HTTP (Broder → Jaiba)

`POST {DMA_JAIVA}/api/v1/ingest/events`

```json
{
  "source": "broder",
  "events": [
    {
      "id": "camera-1:vision:…:1",
      "event_type": "vision.detection.observed",
      "schema_version": 1,
      "source_id": "camera-1",
      "source_kind": "device",
      "occurred_at_ms": 0,
      "observed_at_ms": 0,
      "correlation_id": null,
      "payload": {
        "detection_id": "camera-1:1:0",
        "source_id": "camera-1",
        "frame_id": 1,
        "timestamp_ms": 0,
        "class_id": 0,
        "class_name": "box",
        "confidence": 0.9,
        "bbox_x": 0.1,
        "bbox_y": 0.2,
        "bbox_width": 0.3,
        "bbox_height": 0.4
      }
    }
  ]
}
```

Respuesta esperada: **202 Accepted** (o 2xx). Eso significa “Jaiba bufferizó”.
**No** significa “ya está en ClickHouse/Postgres”.

## Variables en Broder

| Variable | Rol |
| --- | --- |
| `DMA_JAIVA` | Base URL del servidor Jaiba |
| `JAIBA_TOKEN` | Bearer opcional |
| `JAIBA_INGEST_PATH` | Default `/api/v1/ingest/events` |
| `JAIBA_TIMEOUT_MS` | Timeout HTTP del puente (default 2000) |

## Elección de DB detrás de Jaiba (visión)

| Camino | Motor | Por qué |
| --- | --- | --- |
| Hot | Memoria (Broder/Jaiba) | No congelar inferencia |
| Histórico de detecciones | **ClickHouse** | Append-only, time-series, agregaciones |
| Config / operacional | PostgreSQL | Estado mutável, relaciones |
| Entity keyed alta QPS | ScyllaDB | Si hace falta lookup por id a escala |
| ERP / legacy | Oracle | Solo vía Jaiba, fuera del hot path |

ClickHouse es la mejor opción para el histórico de `VisionDetection` **detrás de Jaiba**, precisamente porque Broder no espera el INSERT.
