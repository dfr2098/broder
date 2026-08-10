-- Esquema histórico de visión para Jaiba (lab DMA_JAIVA).
-- Broder no se conecta aquí: solo entrega EventEnvelope a Jaiba.

CREATE DATABASE IF NOT EXISTS temporal;

CREATE TABLE IF NOT EXISTS temporal.vision_detection
(
    event_id String,
    event_type LowCardinality(String),
    schema_version Int16,
    occurred_at DateTime64(3, 'UTC'),
    observed_at DateTime64(3, 'UTC'),
    source_id LowCardinality(String),
    correlation_id Nullable(String),
    detection_id String,
    frame_id Int64,
    source_timestamp_ms Int64,
    class_id Int32,
    class_name LowCardinality(String),
    confidence Float32,
    bbox_x Float32,
    bbox_y Float32,
    bbox_width Float32,
    bbox_height Float32,
    persisted_at DateTime64(3, 'UTC') DEFAULT now64(3)
)
ENGINE = ReplacingMergeTree(persisted_at)
ORDER BY (source_id, occurred_at, event_id);
