//! Puente Broder → Jaiba.
//!
//! Broder **no** conoce drivers, credenciales ni motores de base de datos.
//! Solo entrega `EventEnvelope` al ingest Jaiba (env `DMA_JAIVA` = URL base,
//! nombre legado). Jaiba bufferiza y enruta según su DAG: sinks recomendados
//! ClickHouse (histórico) y PostgreSQL (config); otros (LLM, alertas, drop,
//! Oracle/Odoo-WMS, …) son opcionales. Broder funciona sin DB.
//!
//! `DMA_JAIVA` el lab KPI es un circuito aparte — no es este puente.

mod vision_detection;

pub use vision_detection::{JaibaBridgeConfig, JaibaVisionDetectionWriter, parse_dma_jaiva_url};
