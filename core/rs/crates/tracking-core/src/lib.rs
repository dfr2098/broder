//! Seguimiento visual independiente del motor de inferencia.
//!
//! Este crate asocia `VisionDetection` en el tiempo y produce `VisionTrack`.
//! Trabaja únicamente con coordenadas normalizadas; no calcula velocidad física,
//! no conoce transportadores y no genera decisiones operativas.

mod association;
mod config;
mod error;
mod model;
mod tracker;

pub use config::TrackerConfig;
pub use error::TrackingError;
pub use model::{TrackAssignment, TrackObservation, TrackState, TrackingUpdate, VisionTrack};
pub use tracker::MultiObjectTracker;
