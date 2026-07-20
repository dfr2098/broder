use std::fmt::{self, Display};

use vision_core::{FrameId, NormalizedBoundingBox, TimestampMs};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackState {
    Tentative,
    Confirmed,
    Lost,
    Finished,
}

impl Display for TrackState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Tentative => "tentative",
            Self::Confirmed => "confirmed",
            Self::Lost => "lost",
            Self::Finished => "finished",
        };
        formatter.write_str(name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackObservation {
    pub detection_id: String,
    pub frame_id: FrameId,
    pub timestamp_ms: TimestampMs,
    pub bounding_box: NormalizedBoundingBox,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisionTrack {
    pub track_id: String,
    pub camera_id: String,
    pub class_id: u32,
    pub class_name: String,
    pub history: Vec<TrackObservation>,
    pub started_at_ms: TimestampMs,
    pub last_observed_at_ms: TimestampMs,
    pub state: TrackState,
    pub accumulated_confidence: f32,
    pub missed_frames: u32,
}

impl VisionTrack {
    pub fn latest_observation(&self) -> &TrackObservation {
        self.history
            .last()
            .expect("un track siempre contiene al menos una observación")
    }

    pub fn observation_count(&self) -> usize {
        self.history.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackAssignment {
    pub detection_id: String,
    pub track_id: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackingUpdate {
    pub assignments: Vec<TrackAssignment>,
    pub finished_tracks: Vec<VisionTrack>,
}
