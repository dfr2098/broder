use event_core::EventEnvelope;
use persistence_core::{PersistenceDomain, PersistenceError, PersistenceWriter};
use postgres::{Client, GenericClient, NoTls, Statement};
use vision_core::VisionDetection;

const DEFAULT_BATCH_SIZE: usize = 25;
const MIGRATION: &str = include_str!("../migrations/0001_temporal_vision_detection.sql");

const INSERT_DETECTION: &str = "
    INSERT INTO temporal.vision_detection (
        event_id, event_type, schema_version, occurred_at, observed_at,
        source_id, correlation_id, detection_id, frame_id, source_timestamp_ms,
        class_id, class_name, confidence, bbox_x, bbox_y, bbox_width, bbox_height
    ) VALUES (
        $1, $2, $3,
        TIMESTAMPTZ 'epoch' + $4::bigint * INTERVAL '1 millisecond',
        TIMESTAMPTZ 'epoch' + $5::bigint * INTERVAL '1 millisecond',
        $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
    )
    ON CONFLICT (event_id) DO NOTHING
";

pub struct PostgresVisionDetectionWriter {
    database_url: String,
    client: Client,
    insert_detection: Statement,
    pending: Vec<EventEnvelope<VisionDetection>>,
    batch_size: usize,
}

impl PostgresVisionDetectionWriter {
    pub fn connect(database_url: &str) -> Result<Self, PersistenceError> {
        Self::connect_with_batch_size(database_url, DEFAULT_BATCH_SIZE)
    }

    /// Abre la conexión, aplica el esquema idempotente y configura los lotes.
    pub fn connect_with_batch_size(
        database_url: &str,
        batch_size: usize,
    ) -> Result<Self, PersistenceError> {
        if batch_size == 0 {
            return Err(PersistenceError::new(
                "el tamaño de lote debe ser mayor que cero",
            ));
        }
        let (client, insert_detection) = connect_parts(database_url)?;
        Ok(Self {
            database_url: database_url.to_owned(),
            client,
            insert_detection,
            pending: Vec::with_capacity(batch_size),
            batch_size,
        })
    }

    fn reconnect(&mut self) -> Result<(), PersistenceError> {
        let (client, insert_detection) = connect_parts(&self.database_url)?;
        self.client = client;
        self.insert_detection = insert_detection;
        Ok(())
    }

    fn flush_once(&mut self) -> Result<(), PersistenceError> {
        let pending = &self.pending;
        let statement = &self.insert_detection;
        let mut transaction = self
            .client
            .transaction()
            .map_err(|error| postgres_error("inicio de transacción", &error))?;
        for event in pending {
            insert_event(&mut transaction, statement, event)?;
        }
        transaction
            .commit()
            .map_err(|error| postgres_error("commit PostgreSQL", &error))
    }

    fn flush_pending(&mut self) -> Result<(), PersistenceError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if self.flush_once().is_err() {
            self.reconnect()?;
            self.flush_once().map_err(|error| {
                PersistenceError::new(format!("reintento después de reconectar: {error}"))
            })?;
        }
        self.pending.clear();
        Ok(())
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
        validate_event(event)?;
        self.pending.push(event.clone());
        if self.pending.len() >= self.batch_size {
            self.flush_pending()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), PersistenceError> {
        self.flush_pending()
    }
}

fn connect_parts(database_url: &str) -> Result<(Client, Statement), PersistenceError> {
    let mut client = Client::connect(database_url, NoTls)
        .map_err(|error| postgres_error("conexión PostgreSQL", &error))?;
    client
        .batch_execute(MIGRATION)
        .map_err(|error| postgres_error("migración PostgreSQL", &error))?;
    let statement = client
        .prepare(INSERT_DETECTION)
        .map_err(|error| postgres_error("prepare PostgreSQL", &error))?;
    Ok((client, statement))
}

fn insert_event(
    client: &mut impl GenericClient,
    statement: &Statement,
    event: &EventEnvelope<VisionDetection>,
) -> Result<(), PersistenceError> {
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

    client
        .execute(
            statement,
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
        .map_err(|error| postgres_error("insert PostgreSQL", &error))?;
    Ok(())
}

fn validate_event(event: &EventEnvelope<VisionDetection>) -> Result<(), PersistenceError> {
    i16::try_from(event.schema_version)
        .map_err(|_| PersistenceError::new("schema_version excede SMALLINT"))?;
    to_i64(event.occurred_at_ms, "occurred_at_ms")?;
    to_i64(event.observed_at_ms, "observed_at_ms")?;
    to_i64(event.payload.frame_id, "frame_id")?;
    to_i64(event.payload.timestamp_ms, "source_timestamp_ms")?;
    i32::try_from(event.payload.class_id)
        .map_err(|_| PersistenceError::new("class_id excede INTEGER"))?;
    Ok(())
}

fn postgres_error(context: &str, error: &postgres::Error) -> PersistenceError {
    if let Some(database_error) = error.as_db_error() {
        PersistenceError::new(format!(
            "{context}: {} (SQLSTATE {})",
            database_error.message(),
            database_error.code().code()
        ))
    } else {
        PersistenceError::new(format!("{context}: {error}"))
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

    #[test]
    fn rejects_an_empty_batch_size_before_connecting() {
        let result = PostgresVisionDetectionWriter::connect_with_batch_size("unused", 0);
        assert!(result.is_err());
    }
}
