//! Interpretación espacial de tracks visuales en una cámara configurada.
//!
//! Este núcleo trabaja con geometría normalizada y no contiene velocidad física,
//! alarmas, persistencia ni integraciones industriales.

mod error;
mod geometry;
mod model;
mod spatializer;

pub use error::SpatialError;
pub use geometry::{NormalizedPoint, SpatialPolygon};
pub use model::{
    CameraSpatialModel, CrossingDirection, LineCrossing, LineRole, SpatialTrack, SpatialZone,
    VirtualLine, ZoneKind, ZoneMatch,
};
