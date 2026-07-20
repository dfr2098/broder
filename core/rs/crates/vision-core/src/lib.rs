//! Contratos neutrales del núcleo de visión.
//!
//! Este crate no depende de OpenCV, YOLO, cámaras ni persistencia. Representa
//! detecciones normalizadas y las operaciones puras necesarias para producirlas.

mod detection;
mod nms;
mod sampler;

pub use detection::{
    DetectionCandidate, FrameId, NormalizedBoundingBox, TimestampMs, VisionDetection,
};
pub use nms::class_aware_nms;
pub use sampler::FrameSampler;
