use std::io::Read;
use std::time::Duration;

use event_core::EventEnvelope;
use persistence_core::{PersistenceDomain, PersistenceError, PersistenceWriter};
use serde::Serialize;
use vision_core::VisionDetection;

const DEFAULT_BATCH_SIZE: usize = 25;
const DEFAULT_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_INGEST_PATH: &str = "/api/v1/ingest/events";

/// Configuración del puente hacia Jaiba. Sin URLs ni secretos de bases de datos.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JaibaBridgeConfig {
    pub base_url: String,
    pub ingest_path: String,
    pub token: Option<String>,
    pub timeout_ms: u64,
}

impl JaibaBridgeConfig {
    pub fn from_dma_jaiva(dma_jaiva: &str) -> Result<Self, PersistenceError> {
        parse_dma_jaiva_url(dma_jaiva)
    }

    fn ingest_url(&self) -> String {
        format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            if self.ingest_path.starts_with('/') {
                self.ingest_path.clone()
            } else {
                format!("/{}", self.ingest_path)
            }
        )
    }
}

/// Interpreta `DMA_JAIVA` como base URL del servidor Jaiba (lab DMA_JAIVA).
pub fn parse_dma_jaiva_url(raw: &str) -> Result<JaibaBridgeConfig, PersistenceError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PersistenceError::new("DMA_JAIVA no puede estar vacío"));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(PersistenceError::new(
            "DMA_JAIVA debe ser una URL http(s) del servidor Jaiba",
        ));
    }

    let ingest_path = std::env::var("JAIBA_INGEST_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_INGEST_PATH.to_owned());
    let token = std::env::var("JAIBA_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("DMA_JAIVA_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    let timeout_ms = std::env::var("JAIBA_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_MS);

    Ok(JaibaBridgeConfig {
        base_url: trimmed.trim_end_matches('/').to_owned(),
        ingest_path,
        token,
        timeout_ms,
    })
}

#[derive(Clone, Debug, Serialize)]
struct IngestBatch<'a> {
    source: &'a str,
    events: Vec<IngestEvent<'a>>,
}

#[derive(Clone, Debug, Serialize)]
struct IngestEvent<'a> {
    id: &'a str,
    event_type: &'a str,
    schema_version: u16,
    source_id: &'a str,
    source_kind: &'a str,
    occurred_at_ms: u64,
    observed_at_ms: u64,
    correlation_id: Option<&'a str>,
    payload: IngestVisionPayload<'a>,
}

#[derive(Clone, Debug, Serialize)]
struct IngestVisionPayload<'a> {
    detection_id: &'a str,
    source_id: &'a str,
    frame_id: u64,
    timestamp_ms: u64,
    class_id: u32,
    class_name: &'a str,
    confidence: f32,
    bbox_x: f32,
    bbox_y: f32,
    bbox_width: f32,
    bbox_height: f32,
}

/// Entrega detecciones a Jaiba. No habla con ninguna base de datos.
pub struct JaibaVisionDetectionWriter {
    config: JaibaBridgeConfig,
    agent: ureq::Agent,
    pending: Vec<EventEnvelope<VisionDetection>>,
    batch_size: usize,
}

impl JaibaVisionDetectionWriter {
    pub fn connect(dma_jaiva: &str) -> Result<Self, PersistenceError> {
        Self::connect_with_batch_size(dma_jaiva, DEFAULT_BATCH_SIZE)
    }

    pub fn connect_with_batch_size(
        dma_jaiva: &str,
        batch_size: usize,
    ) -> Result<Self, PersistenceError> {
        if batch_size == 0 {
            return Err(PersistenceError::new(
                "el tamaño de lote debe ser mayor que cero",
            ));
        }
        let config = JaibaBridgeConfig::from_dma_jaiva(dma_jaiva)?;
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build();
        let writer = Self {
            config,
            agent,
            pending: Vec::with_capacity(batch_size),
            batch_size,
        };
        writer.ping()?;
        Ok(writer)
    }

    fn ping(&self) -> Result<(), PersistenceError> {
        // Solo comprobamos que el host Jaiba responde. Auth y routing son de Jaiba.
        let health = format!(
            "{}/api/v1/whoami",
            self.config.base_url.trim_end_matches('/')
        );
        let mut request = self.agent.get(&health);
        if let Some(token) = &self.config.token {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        match request.call() {
            Ok(_) => Ok(()),
            Err(ureq::Error::Status(_, _)) => Ok(()),
            Err(error) => Err(PersistenceError::new(format!(
                "Jaiba no alcanzable en {}: {error}",
                self.config.base_url
            ))),
        }
    }

    fn post_batch(
        &self,
        events: &[EventEnvelope<VisionDetection>],
    ) -> Result<(), PersistenceError> {
        let body = IngestBatch {
            source: "broder",
            events: events.iter().map(to_ingest_event).collect(),
        };
        let payload = serde_json::to_string(&body)
            .map_err(|error| PersistenceError::new(format!("JSON Jaiba: {error}")))?;
        let mut request = self
            .agent
            .post(&self.config.ingest_url())
            .set("Content-Type", "application/json")
            .set("Accept", "application/json");
        if let Some(token) = &self.config.token {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        let response = request
            .send_string(&payload)
            .map_err(|error| PersistenceError::new(format!("entrega a Jaiba: {error}")))?;
        let status = response.status();
        let mut response_body = String::new();
        let _ = response.into_reader().read_to_string(&mut response_body);
        // 202 Accepted: Jaiba bufferizó; Broder no espera persistencia en DB.
        if !(200..300).contains(&status) {
            return Err(PersistenceError::new(format!(
                "Jaiba ingest HTTP {status}: {}",
                response_body.trim()
            )));
        }
        Ok(())
    }

    fn flush_pending(&mut self) -> Result<(), PersistenceError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if let Err(error) = self.post_batch(&self.pending) {
            // Un reintento corto: Jaiba puede haber reiniciado el listener.
            self.post_batch(&self.pending).map_err(|retry| {
                PersistenceError::new(format!("reintento Jaiba tras {error}: {retry}"))
            })?;
        }
        self.pending.clear();
        Ok(())
    }
}

impl PersistenceWriter<VisionDetection> for JaibaVisionDetectionWriter {
    fn name(&self) -> &'static str {
        "jaiba-vision-detection-bridge"
    }

    fn domain(&self) -> PersistenceDomain {
        PersistenceDomain::Temporal
    }

    fn persist(&mut self, event: &EventEnvelope<VisionDetection>) -> Result<(), PersistenceError> {
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

fn to_ingest_event(event: &EventEnvelope<VisionDetection>) -> IngestEvent<'_> {
    let detection = &event.payload;
    let bbox = detection.bounding_box;
    IngestEvent {
        id: &event.id,
        event_type: &event.event_type,
        schema_version: event.schema_version,
        source_id: &event.source.id,
        source_kind: match &event.source.kind {
            event_core::SourceKind::Device => "device",
            event_core::SourceKind::ExternalSystem => "external_system",
            event_core::SourceKind::Human => "human",
            event_core::SourceKind::Simulation => "simulation",
            event_core::SourceKind::Other(value) => value.as_str(),
        },
        occurred_at_ms: event.occurred_at_ms,
        observed_at_ms: event.observed_at_ms,
        correlation_id: event.correlation_id.as_deref(),
        payload: IngestVisionPayload {
            detection_id: &detection.detection_id,
            source_id: &detection.source_id,
            frame_id: detection.frame_id,
            timestamp_ms: detection.timestamp_ms,
            class_id: detection.class_id,
            class_name: &detection.class_name,
            confidence: detection.confidence,
            bbox_x: bbox.x,
            bbox_y: bbox.y,
            bbox_width: bbox.width,
            bbox_height: bbox.height,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_batch_size_before_connecting() {
        let result = JaibaVisionDetectionWriter::connect_with_batch_size("http://127.0.0.1:9", 0);
        assert!(result.is_err());
    }

    #[test]
    fn parses_dma_jaiva_base_url() {
        let config = parse_dma_jaiva_url("http://127.0.0.1:19090/").unwrap();
        assert_eq!(config.base_url, "http://127.0.0.1:19090");
        assert_eq!(config.ingest_path, DEFAULT_INGEST_PATH);
        assert_eq!(
            config.ingest_url(),
            "http://127.0.0.1:19090/api/v1/ingest/events"
        );
    }

    #[test]
    fn rejects_non_http_dma_jaiva() {
        assert!(parse_dma_jaiva_url("jaiba://localhost").is_err());
    }
}
