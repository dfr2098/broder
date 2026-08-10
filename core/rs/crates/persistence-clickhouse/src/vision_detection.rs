use std::io::Read;
use std::time::Duration;

use event_core::EventEnvelope;
use persistence_core::{PersistenceDomain, PersistenceError, PersistenceWriter};
use serde::Serialize;
use vision_core::VisionDetection;

const DEFAULT_BATCH_SIZE: usize = 25;
const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MIGRATION: &str = include_str!("../migrations/0001_temporal_vision_detection.sql");

/// Conexión ClickHouse expuesta por el gateway Jaiva del lab `DMA_JAIVA`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JaivaClickHouseConfig {
    /// Base URL del gateway Jaiva (variable `DMA_JAIVA`).
    pub gateway_url: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub timeout_ms: u64,
}

impl JaivaClickHouseConfig {
    pub fn from_gateway_url(gateway_url: &str) -> Result<Self, PersistenceError> {
        parse_jaiva_clickhouse_url(gateway_url)
    }

    fn query_endpoint(&self) -> String {
        format!("{}/", self.gateway_url.trim_end_matches('/'))
    }
}

/// Interpreta `DMA_JAIVA` como URL del gateway Jaiva hacia ClickHouse.
///
/// Formatos admitidos:
/// - `http://host:port`
/// - `http://user:pass@host:port`
/// - `http://host:port?user=...&password=...&database=...`
pub fn parse_jaiva_clickhouse_url(raw: &str) -> Result<JaivaClickHouseConfig, PersistenceError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PersistenceError::new("DMA_JAIVA no puede estar vacío"));
    }

    let mut user = std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_owned());
    let mut password = std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default();
    let mut database =
        std::env::var("CLICKHOUSE_DATABASE").unwrap_or_else(|_| "temporal".to_owned());
    let timeout_ms = std::env::var("CLICKHOUSE_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_MS);

    let (without_query, query) = match trimmed.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (trimmed, None),
    };

    if let Some(query) = query {
        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            match key {
                "user" | "username" => user = urlencoding_decode(value),
                "password" => password = urlencoding_decode(value),
                "database" | "db" => database = urlencoding_decode(value),
                _ => {}
            }
        }
    }

    let gateway_url = if let Some((scheme, rest)) = without_query.split_once("://") {
        if let Some((auth, host)) = rest.split_once('@') {
            if let Some((auth_user, auth_password)) = auth.split_once(':') {
                user = urlencoding_decode(auth_user);
                password = urlencoding_decode(auth_password);
            } else {
                user = urlencoding_decode(auth);
            }
            format!("{scheme}://{host}")
        } else {
            format!("{scheme}://{rest}")
        }
    } else {
        without_query.to_owned()
    };

    if !(gateway_url.starts_with("http://") || gateway_url.starts_with("https://")) {
        return Err(PersistenceError::new(
            "DMA_JAIVA debe ser una URL http(s) del gateway Jaiva",
        ));
    }

    Ok(JaivaClickHouseConfig {
        gateway_url: gateway_url.trim_end_matches('/').to_owned(),
        user,
        password,
        database,
        timeout_ms,
    })
}

fn urlencoding_decode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte as char);
                    index += 3;
                } else {
                    out.push('%');
                    index += 1;
                }
            }
            byte => {
                out.push(byte as char);
                index += 1;
            }
        }
    }
    out
}

#[derive(Clone, Debug, Serialize)]
struct DetectionRow<'a> {
    event_id: &'a str,
    event_type: &'a str,
    schema_version: i16,
    occurred_at: u64,
    observed_at: u64,
    source_id: &'a str,
    correlation_id: Option<&'a str>,
    detection_id: &'a str,
    frame_id: i64,
    source_timestamp_ms: i64,
    class_id: i32,
    class_name: &'a str,
    confidence: f32,
    bbox_x: f32,
    bbox_y: f32,
    bbox_width: f32,
    bbox_height: f32,
}

pub struct ClickHouseVisionDetectionWriter {
    config: JaivaClickHouseConfig,
    agent: ureq::Agent,
    pending: Vec<EventEnvelope<VisionDetection>>,
    batch_size: usize,
}

impl ClickHouseVisionDetectionWriter {
    pub fn connect(gateway_url: &str) -> Result<Self, PersistenceError> {
        Self::connect_with_batch_size(gateway_url, DEFAULT_BATCH_SIZE)
    }

    /// Abre el gateway Jaiva→ClickHouse, aplica el esquema idempotente y configura lotes.
    pub fn connect_with_batch_size(
        gateway_url: &str,
        batch_size: usize,
    ) -> Result<Self, PersistenceError> {
        if batch_size == 0 {
            return Err(PersistenceError::new(
                "el tamaño de lote debe ser mayor que cero",
            ));
        }
        let config = JaivaClickHouseConfig::from_gateway_url(gateway_url)?;
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build();
        let writer = Self {
            config,
            agent,
            pending: Vec::with_capacity(batch_size),
            batch_size,
        };
        writer.migrate()?;
        writer.ping()?;
        Ok(writer)
    }

    fn migrate(&self) -> Result<(), PersistenceError> {
        for statement in split_sql_statements(MIGRATION) {
            // CREATE DATABASE debe ir sin database= temporal (aún puede no existir).
            let use_database = !statement.to_ascii_uppercase().contains("CREATE DATABASE");
            self.execute_sql(statement, use_database)?;
        }
        Ok(())
    }

    fn ping(&self) -> Result<(), PersistenceError> {
        let body = self.execute_sql("SELECT 1", true)?;
        if body.trim() != "1" {
            return Err(PersistenceError::new(format!(
                "ping ClickHouse vía Jaiva inesperado: {body}"
            )));
        }
        Ok(())
    }

    fn execute_sql(&self, sql: &str, use_database: bool) -> Result<String, PersistenceError> {
        // El gateway Jaiva del lab DMA_JAIVA reenvía el protocolo HTTP de ClickHouse.
        let mut request = self
            .agent
            .post(&self.config.query_endpoint())
            .set("X-ClickHouse-User", &self.config.user)
            .set("X-ClickHouse-Key", &self.config.password);
        if use_database {
            request = request.query("database", &self.config.database);
        }
        let response = request
            .query("query", sql)
            .call()
            .map_err(|error| PersistenceError::new(format!("gateway Jaiva/ClickHouse: {error}")))?;
        read_body(response)
    }

    fn insert_rows(&self, rows: &[DetectionRow<'_>]) -> Result<(), PersistenceError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut payload = String::new();
        for row in rows {
            payload.push_str(
                &serde_json::to_string(row)
                    .map_err(|error| PersistenceError::new(format!("JSON ClickHouse: {error}")))?,
            );
            payload.push('\n');
        }
        let query = format!(
            "INSERT INTO {}.vision_detection \
             (event_id, event_type, schema_version, occurred_at, observed_at, source_id, \
              correlation_id, detection_id, frame_id, source_timestamp_ms, class_id, class_name, \
              confidence, bbox_x, bbox_y, bbox_width, bbox_height) \
             FORMAT JSONEachRow",
            self.config.database
        );
        let response = self
            .agent
            .post(&self.config.query_endpoint())
            .set("X-ClickHouse-User", &self.config.user)
            .set("X-ClickHouse-Key", &self.config.password)
            .query("database", &self.config.database)
            .query("query", &query)
            .send_string(&payload)
            .map_err(|error| {
                PersistenceError::new(format!("insert ClickHouse vía Jaiva: {error}"))
            })?;
        let _ = read_body(response)?;
        Ok(())
    }

    fn flush_once(&mut self) -> Result<(), PersistenceError> {
        let rows = self
            .pending
            .iter()
            .map(row_from_event)
            .collect::<Result<Vec<_>, _>>()?;
        self.insert_rows(&rows)
    }

    fn flush_pending(&mut self) -> Result<(), PersistenceError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if self.flush_once().is_err() {
            self.migrate()?;
            self.flush_once().map_err(|error| {
                PersistenceError::new(format!("reintento después de reconectar: {error}"))
            })?;
        }
        self.pending.clear();
        Ok(())
    }
}

impl PersistenceWriter<VisionDetection> for ClickHouseVisionDetectionWriter {
    fn name(&self) -> &'static str {
        "jaiva-clickhouse-vision-detection-writer"
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

fn row_from_event(
    event: &EventEnvelope<VisionDetection>,
) -> Result<DetectionRow<'_>, PersistenceError> {
    let detection = &event.payload;
    let schema_version = i16::try_from(event.schema_version)
        .map_err(|_| PersistenceError::new("schema_version excede Int16"))?;
    let frame_id = to_i64(detection.frame_id, "frame_id")?;
    let source_timestamp_ms = to_i64(detection.timestamp_ms, "source_timestamp_ms")?;
    let class_id = i32::try_from(detection.class_id)
        .map_err(|_| PersistenceError::new("class_id excede Int32"))?;
    let bbox = detection.bounding_box;
    Ok(DetectionRow {
        event_id: &event.id,
        event_type: &event.event_type,
        schema_version,
        occurred_at: event.occurred_at_ms,
        observed_at: event.observed_at_ms,
        source_id: &event.source.id,
        correlation_id: event.correlation_id.as_deref(),
        detection_id: &detection.detection_id,
        frame_id,
        source_timestamp_ms,
        class_id,
        class_name: &detection.class_name,
        confidence: detection.confidence,
        bbox_x: bbox.x,
        bbox_y: bbox.y,
        bbox_width: bbox.width,
        bbox_height: bbox.height,
    })
}

fn validate_event(event: &EventEnvelope<VisionDetection>) -> Result<(), PersistenceError> {
    i16::try_from(event.schema_version)
        .map_err(|_| PersistenceError::new("schema_version excede Int16"))?;
    to_i64(event.occurred_at_ms, "occurred_at_ms")?;
    to_i64(event.observed_at_ms, "observed_at_ms")?;
    to_i64(event.payload.frame_id, "frame_id")?;
    to_i64(event.payload.timestamp_ms, "source_timestamp_ms")?;
    i32::try_from(event.payload.class_id)
        .map_err(|_| PersistenceError::new("class_id excede Int32"))?;
    Ok(())
}

fn to_i64(value: u64, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::new(format!("{field} excede Int64")))
}

fn split_sql_statements(sql: &str) -> Vec<&str> {
    sql.split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .collect()
}

fn read_body(response: ureq::Response) -> Result<String, PersistenceError> {
    let status = response.status();
    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .map_err(|error| PersistenceError::new(format!("lectura respuesta Jaiva: {error}")))?;
    if !(200..300).contains(&status) {
        return Err(PersistenceError::new(format!(
            "gateway Jaiva/ClickHouse HTTP {status}: {}",
            body.trim()
        )));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_values_that_do_not_fit_clickhouse_int64() {
        assert!(to_i64(i64::MAX as u64, "frame_id").is_ok());
        assert!(to_i64(i64::MAX as u64 + 1, "frame_id").is_err());
    }

    #[test]
    fn rejects_an_empty_batch_size_before_connecting() {
        let result = ClickHouseVisionDetectionWriter::connect_with_batch_size("unused", 0);
        assert!(result.is_err());
    }

    #[test]
    fn parses_dma_jaiva_gateway_url_with_auth_and_database() {
        let config =
            parse_jaiva_clickhouse_url("http://jaiva:secret@127.0.0.1:19090?database=temporal")
                .unwrap();
        assert_eq!(config.gateway_url, "http://127.0.0.1:19090");
        assert_eq!(config.user, "jaiva");
        assert_eq!(config.password, "secret");
        assert_eq!(config.database, "temporal");
    }

    #[test]
    fn rejects_non_http_dma_jaiva() {
        assert!(parse_jaiva_clickhouse_url("clickhouse://localhost").is_err());
    }
}
