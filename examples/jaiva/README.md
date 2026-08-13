# Arquitectura Broder ↔ Jaiba

Regla de Broder: **no conoce drivers, credenciales ni particularidades de DB**.
Broder puede vivir **sin DB** y sin lab DMA: solo produce eventos.

```text
BRODER ──► JAIBA ──┬──► LLM (opcional)
                   ├──► ClickHouse (histórico, si se persiste)
                   ├──► PostgreSQL / Oracle / Odoo-WMS (sinks opcionales)
                   └──► alertas / drop (si no es relevante)
```

Jaiba (OSS, DAG YAML) decide persistir / analizar / ignorar / drop. Broder
nunca habla con bases de datos directamente.

## Distinción: Jaiba vs DMA_JAIVA

| Pieza | Rol en este camino |
| --- | --- |
| **Jaiba** | Motor de conectividad/enrutamiento que recibe `EventEnvelope` de Broder. |
| **DMA_JAIVA** | Lab KPI aparte (`Oracle → Jaiba → Postgres DMA`). **No** es el camino Broder ni el esqueleto de Broder. |
| **Broder** | Productor fire-and-forget. Env var histórica `DMA_JAIVA` = URL base del *ingest Jaiba* (nombre legado; no implica el lab DMA). |

Ver [`docs/BRIEF_DMA_JAIVA_JAIBA_BRODER.md`](../../docs/BRIEF_DMA_JAIVA_JAIBA_BRODER.md).

## Sinks recomendados (opcionales)

ClickHouse (histórico) y PostgreSQL (config/estado) son los sinks **recomendados**
cuando hace falta persistir. Otros destinos (Oracle, Odoo-WMS, LLM, alertas,
drop) son opcionales vía DAG de Jaiba — Broder no los fuerza ni los conoce.

Infra local opcional (`make infra-up`): contenedores **ClickHouse** + **PostgreSQL**
para que *Jaiba* los use como sinks de laboratorio. No son el runtime skeleton
de Broder: visión/app funcionan sin ellos y sin ingest.

## Contrato HTTP (Broder → Jaiba)

`POST {DMA_JAIVA}/api/v1/ingest/events`

(`DMA_JAIVA` aquí es solo el nombre de la variable de entorno = base URL de Jaiba.)

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
**No** significa “ya está en ClickHouse/Postgres u otro sink”.

## Variables en Broder

| Variable | Rol |
| --- | --- |
| `DMA_JAIVA` | Base URL del ingest Jaiba (nombre legado) |
| `JAIBA_TOKEN` | Bearer opcional |
| `JAIBA_INGEST_PATH` | Default `/api/v1/ingest/events` |
| `JAIBA_TIMEOUT_MS` | Timeout HTTP del puente (default 2000) |

## Roles típicos detrás de Jaiba

| Camino | Destino típico | Notas |
| --- | --- | --- |
| Hot | Memoria (Broder/Jaiba) | No congelar inferencia |
| Histórico | **ClickHouse** (recomendado) | Append-only / time-series cuando se persiste |
| Config / operacional | **PostgreSQL** (recomendado) | Estado mutable si el DAG lo pide |
| Otros | Oracle / Odoo-WMS / LLM / alertas / drop | Opcionales vía DAG; Broder no los abre |
