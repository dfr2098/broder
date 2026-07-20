use crate::TrackingError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackerConfig {
    pub minimum_confirmed_hits: u32,
    pub maximum_missed_frames: u32,
    pub maximum_lost_ms: u64,
    pub minimum_iou: f32,
    pub maximum_center_distance: f32,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            minimum_confirmed_hits: 2,
            maximum_missed_frames: 5,
            maximum_lost_ms: 1_500,
            minimum_iou: 0.05,
            maximum_center_distance: 0.25,
        }
    }
}

impl TrackerConfig {
    pub fn validate(self) -> Result<Self, TrackingError> {
        if self.minimum_confirmed_hits == 0 {
            return Err(TrackingError::new(
                "minimum_confirmed_hits debe ser mayor que cero",
            ));
        }
        if self.maximum_lost_ms == 0 {
            return Err(TrackingError::new(
                "maximum_lost_ms debe ser mayor que cero",
            ));
        }
        if !is_unit_interval(self.minimum_iou) {
            return Err(TrackingError::new("minimum_iou debe estar entre 0 y 1"));
        }
        if !self.maximum_center_distance.is_finite()
            || self.maximum_center_distance <= 0.0
            || self.maximum_center_distance > 2.0
        {
            return Err(TrackingError::new(
                "maximum_center_distance debe estar entre 0 y 2",
            ));
        }
        Ok(self)
    }
}

fn is_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
