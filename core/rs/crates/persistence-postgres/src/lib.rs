//! Adaptadores PostgreSQL de Little Brother.
//!
//! Este crate es infraestructura: conoce PostgreSQL y sus tablas. Los núcleos
//! funcionales continúan dependiendo únicamente de contratos del dominio.

mod vision_detection;

pub use vision_detection::PostgresVisionDetectionWriter;
