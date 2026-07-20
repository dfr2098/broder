CREATE SCHEMA IF NOT EXISTS temporal;

CREATE TABLE IF NOT EXISTS temporal.vision_detection (
    event_id             TEXT PRIMARY KEY,
    event_type           TEXT NOT NULL,
    schema_version       SMALLINT NOT NULL CHECK (schema_version > 0),
    occurred_at          TIMESTAMPTZ NOT NULL,
    observed_at          TIMESTAMPTZ NOT NULL,
    source_id            TEXT NOT NULL,
    correlation_id       TEXT,
    detection_id         TEXT NOT NULL,
    frame_id             BIGINT NOT NULL CHECK (frame_id >= 0),
    source_timestamp_ms  BIGINT NOT NULL CHECK (source_timestamp_ms >= 0),
    class_id             INTEGER NOT NULL CHECK (class_id >= 0),
    class_name           TEXT NOT NULL,
    confidence           REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    bbox_x               REAL NOT NULL CHECK (bbox_x >= 0 AND bbox_x <= 1),
    bbox_y               REAL NOT NULL CHECK (bbox_y >= 0 AND bbox_y <= 1),
    bbox_width           REAL NOT NULL CHECK (bbox_width > 0 AND bbox_width <= 1),
    bbox_height          REAL NOT NULL CHECK (bbox_height > 0 AND bbox_height <= 1),
    persisted_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (bbox_x + bbox_width <= 1.000001),
    CHECK (bbox_y + bbox_height <= 1.000001)
);

CREATE INDEX IF NOT EXISTS vision_detection_occurred_at_idx
    ON temporal.vision_detection (occurred_at DESC);

CREATE INDEX IF NOT EXISTS vision_detection_source_time_idx
    ON temporal.vision_detection (source_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS vision_detection_class_time_idx
    ON temporal.vision_detection (class_id, occurred_at DESC);
