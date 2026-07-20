use std::collections::HashSet;

use vision_core::{FrameId, TimestampMs, VisionDetection};

use crate::association::build_association_candidates;
use crate::{
    TrackAssignment, TrackObservation, TrackState, TrackerConfig, TrackingError, TrackingUpdate,
    VisionTrack,
};

pub struct MultiObjectTracker {
    camera_id: String,
    config: TrackerConfig,
    active_tracks: Vec<VisionTrack>,
    next_track_sequence: u64,
    last_frame_id: Option<FrameId>,
    last_timestamp_ms: Option<TimestampMs>,
}

impl MultiObjectTracker {
    pub fn new(camera_id: impl Into<String>, config: TrackerConfig) -> Result<Self, TrackingError> {
        let camera_id = camera_id.into();
        if camera_id.trim().is_empty() {
            return Err(TrackingError::new("camera_id no puede estar vacío"));
        }
        Ok(Self {
            camera_id,
            config: config.validate()?,
            active_tracks: Vec::new(),
            next_track_sequence: 1,
            last_frame_id: None,
            last_timestamp_ms: None,
        })
    }

    pub fn process_frame(
        &mut self,
        frame_id: FrameId,
        timestamp_ms: TimestampMs,
        detections: &[VisionDetection],
    ) -> Result<TrackingUpdate, TrackingError> {
        self.validate_frame(frame_id, timestamp_ms, detections)?;

        let association_candidates = build_association_candidates(
            &self.active_tracks,
            timestamp_ms,
            detections,
            self.config,
        );
        let mut matched_tracks = HashSet::new();
        let mut matched_detections = HashSet::new();
        let mut assignments = Vec::new();

        for candidate in association_candidates {
            if matched_tracks.contains(&candidate.track_index)
                || matched_detections.contains(&candidate.detection_index)
            {
                continue;
            }
            let detection = &detections[candidate.detection_index];
            let track = &mut self.active_tracks[candidate.track_index];
            update_track(track, detection, self.config.minimum_confirmed_hits);
            assignments.push(TrackAssignment {
                detection_id: detection.detection_id.clone(),
                track_id: track.track_id.clone(),
            });
            matched_tracks.insert(candidate.track_index);
            matched_detections.insert(candidate.detection_index);
        }

        let mut finished_tracks = Vec::new();
        for (track_index, track) in self.active_tracks.iter_mut().enumerate() {
            if matched_tracks.contains(&track_index) {
                continue;
            }
            track.missed_frames += 1;
            let lost_ms = timestamp_ms.saturating_sub(track.last_observed_at_ms);
            if track.missed_frames > self.config.maximum_missed_frames
                || lost_ms > self.config.maximum_lost_ms
            {
                track.state = TrackState::Finished;
                finished_tracks.push(track.clone());
            } else {
                track.state = TrackState::Lost;
            }
        }
        self.active_tracks
            .retain(|track| track.state != TrackState::Finished);

        for (detection_index, detection) in detections.iter().enumerate() {
            if matched_detections.contains(&detection_index) {
                continue;
            }
            let track = self.create_track(detection);
            assignments.push(TrackAssignment {
                detection_id: detection.detection_id.clone(),
                track_id: track.track_id.clone(),
            });
            self.active_tracks.push(track);
        }

        assignments.sort_by(|left, right| left.detection_id.cmp(&right.detection_id));
        self.last_frame_id = Some(frame_id);
        self.last_timestamp_ms = Some(timestamp_ms);
        Ok(TrackingUpdate {
            assignments,
            finished_tracks,
        })
    }

    pub fn finish_all(&mut self) -> Vec<VisionTrack> {
        self.active_tracks
            .drain(..)
            .map(|mut track| {
                track.state = TrackState::Finished;
                track
            })
            .collect()
    }

    pub fn active_tracks(&self) -> &[VisionTrack] {
        &self.active_tracks
    }

    fn validate_frame(
        &self,
        frame_id: FrameId,
        timestamp_ms: TimestampMs,
        detections: &[VisionDetection],
    ) -> Result<(), TrackingError> {
        if self.last_frame_id.is_some_and(|last| frame_id <= last) {
            return Err(TrackingError::new(
                "los frames deben procesarse en orden estrictamente creciente",
            ));
        }
        if self
            .last_timestamp_ms
            .is_some_and(|last| timestamp_ms < last)
        {
            return Err(TrackingError::new(
                "las marcas de tiempo no pueden retroceder",
            ));
        }

        let mut detection_ids = HashSet::new();
        for detection in detections {
            if detection.source_id != self.camera_id {
                return Err(TrackingError::new(format!(
                    "la detección {} pertenece a otra cámara",
                    detection.detection_id
                )));
            }
            if detection.frame_id != frame_id || detection.timestamp_ms != timestamp_ms {
                return Err(TrackingError::new(format!(
                    "la detección {} no pertenece al lote actual",
                    detection.detection_id
                )));
            }
            if !detection_ids.insert(&detection.detection_id) {
                return Err(TrackingError::new(
                    "el lote contiene detecciones duplicadas",
                ));
            }
        }
        Ok(())
    }

    fn create_track(&mut self, detection: &VisionDetection) -> VisionTrack {
        let track_id = format!("{}:track:{:06}", self.camera_id, self.next_track_sequence);
        self.next_track_sequence += 1;
        let state = if self.config.minimum_confirmed_hits <= 1 {
            TrackState::Confirmed
        } else {
            TrackState::Tentative
        };
        VisionTrack {
            track_id,
            camera_id: self.camera_id.clone(),
            class_id: detection.class_id,
            class_name: detection.class_name.clone(),
            history: vec![observation_from_detection(detection)],
            started_at_ms: detection.timestamp_ms,
            last_observed_at_ms: detection.timestamp_ms,
            state,
            accumulated_confidence: detection.confidence,
            missed_frames: 0,
        }
    }
}

fn update_track(track: &mut VisionTrack, detection: &VisionDetection, minimum_hits: u32) {
    let previous_count = track.history.len() as f32;
    track.accumulated_confidence = (track.accumulated_confidence * previous_count
        + detection.confidence)
        / (previous_count + 1.0);
    track.history.push(observation_from_detection(detection));
    track.last_observed_at_ms = detection.timestamp_ms;
    track.missed_frames = 0;
    track.state = if track.history.len() as u32 >= minimum_hits {
        TrackState::Confirmed
    } else {
        TrackState::Tentative
    };
}

fn observation_from_detection(detection: &VisionDetection) -> TrackObservation {
    TrackObservation {
        detection_id: detection.detection_id.clone(),
        frame_id: detection.frame_id,
        timestamp_ms: detection.timestamp_ms,
        bounding_box: detection.bounding_box,
        confidence: detection.confidence,
    }
}
