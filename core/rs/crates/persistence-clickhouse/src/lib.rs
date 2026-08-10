//! Adaptador ClickHouse de Little Brother vía gateway Jaiva (`DMA_JAIVA`).
//!
//! El proceso no abre PostgreSQL ni habla con ClickHouse “en crudo” fuera del
//! gateway: todas las consultas HTTP pasan por la base URL de Jaiva definida
//! en `DMA_JAIVA` (laboratorio DMA_JAIVA).

mod vision_detection;

pub use vision_detection::{
    ClickHouseVisionDetectionWriter, JaivaClickHouseConfig, parse_jaiva_clickhouse_url,
};
