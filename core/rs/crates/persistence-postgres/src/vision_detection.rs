use event_core::EventEnvelope;
use persistence_core::{PersistenceDomain, PersistenceError, PersistenceWriter};
use postgres::{Client, NoTls, Statement};
use vision_core::VisionDetection;

const MIGRATION: &str = include_str!("../migrations/0001_temporal_vision_detection.sql");

const INSERT_DETECTION: &str = "
    INSERT INTO temporal.vision_detection (
        event_id,
        event_type,
        schema_version,
        occurred_at,
        observed_at,
        source_id,
        correlation_id,
        detection_id,
        frame_id,
        source_timestamp_ms,
        class_id,
        class_name,
        confidence,
        bbox_x,
        bbox_y,
        bbox_width,
        bbox_height
    ) VALUES (
        $1, $2, $3,
        TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond',
        TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond',
        $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
    )
    ON CONFLICT (event_id) DO NOTHING
";

pub struct PostgresVisionDetectionWriter {
    client: Client,
    insert_detection: Statement,
}

impl PostgresVisionDetectionWriter {
    /// Abre la conexión, aplica el esquema idempotente y prepara la inserción.
    pub fn connect(database_url: &str) -> Result<Self, PersistenceError> {
        let mut client = Client::connect(database_url, NoTls)
            .map_err(|error| PersistenceError::new(format!("conexión PostgreSQL: {error}")))?;
        client
            .batch_execute(MIGRATION)
            .map_err(|error| PersistenceError::new(format!("migración PostgreSQL: {error}")))?;
        let insert_detection = client
            .prepare(INSERT_DETECTION)
            .map_err(|error| PersistenceError::new(format!("prepare PostgreSQL: {error}")))?;

        Ok(Self {
            client,
            insert_detection,
        })
    }
}

impl PersistenceWriter<VisionDetection> for PostgresVisionDetectionWriter {
    fn name(&self) -> &'static str {
        "postgres-vision-detection-writer"
    }

    fn domain(&self) -> PersistenceDomain {
        PersistenceDomain::Temporal
    }

    fn persist(&mut self, event: &EventEnvelope<VisionDetection>) -> Result<(), PersistenceError> {
        let detection = &event.payload;
        let schema_version = i16::try_from(event.schema_version)
            .map_err(|_| PersistenceError::new("schema_version excede SMALLINT"))?;
        let occurred_at_ms = to_i64(event.occurred_at_ms, "occurred_at_ms")?;
        let observed_at_ms = to_i64(event.observed_at_ms, "observed_at_ms")?;
        let frame_id = to_i64(detection.frame_id, "frame_id")?;
        let source_timestamp_ms = to_i64(detection.timestamp_ms, "source_timestamp_ms")?;
        let class_id = i32::try_from(detection.class_id)
            .map_err(|_| PersistenceError::new("class_id excede INTEGER"))?;
        let correlation_id = event.correlation_id.as_deref();
        let bbox = detection.bounding_box;

        self.client
            .execute(
                &self.insert_detection,
                &[
                    &event.id,
                    &event.event_type,
                    &schema_version,
                    &occurred_at_ms,
                    &observed_at_ms,
                    &event.source.id,
                    &correlation_id,
                    &detection.detection_id,
                    &frame_id,
                    &source_timestamp_ms,
                    &class_id,
                    &detection.class_name,
                    &detection.confidence,
                    &bbox.x,
                    &bbox.y,
                    &bbox.width,
                    &bbox.height,
                ],
            )
            .map_err(|error| PersistenceError::new(format!("insert PostgreSQL: {error}")))?;
        Ok(())
    }
}

fn to_i64(value: u64, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::new(format!("{field} excede BIGINT")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_values_that_do_not_fit_postgresql_bigint() {
        assert!(to_i64(i64::MAX as u64, "frame_id").is_ok());
        assert!(to_i64(i64::MAX as u64 + 1, "frame_id").is_err());
    }
}
