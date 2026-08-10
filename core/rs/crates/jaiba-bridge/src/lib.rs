//! Puente Broder → Jaiba.
//!
//! Broder **no** conoce drivers, credenciales ni motores de base de datos.
//! Solo entrega `EventEnvelope` al gateway Jaiba (`DMA_JAIVA`). Jaiba bufferiza,
//! enruta y persiste únicamente hacia **ClickHouse** (histórico) y
//! **PostgreSQL** (configuración / estado) según el lab `DMA_JAIVA`.

mod vision_detection;

pub use vision_detection::{JaibaBridgeConfig, JaibaVisionDetectionWriter, parse_dma_jaiva_url};
