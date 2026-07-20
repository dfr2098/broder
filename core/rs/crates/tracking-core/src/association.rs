use vision_core::{NormalizedBoundingBox, TimestampMs, VisionDetection};

use crate::{TrackerConfig, VisionTrack};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AssociationCandidate {
    pub track_index: usize,
    pub detection_index: usize,
    cost: f32,
}

pub(crate) fn build_association_candidates(
    active_tracks: &[VisionTrack],
    timestamp_ms: TimestampMs,
    detections: &[VisionDetection],
    config: TrackerConfig,
) -> Vec<AssociationCandidate> {
    let mut candidates = Vec::new();
    for (track_index, track) in active_tracks.iter().enumerate() {
        if timestamp_ms.saturating_sub(track.last_observed_at_ms) > config.maximum_lost_ms {
            continue;
        }
        let predicted = predicted_bounding_box(track, timestamp_ms);
        for (detection_index, detection) in detections.iter().enumerate() {
            if track.class_id != detection.class_id {
                continue;
            }
            let iou = predicted.iou(detection.bounding_box);
            let center_distance = center_distance(predicted, detection.bounding_box);
            if iou < config.minimum_iou && center_distance > config.maximum_center_distance {
                continue;
            }
            let normalized_distance = (center_distance / config.maximum_center_distance).min(1.0);
            candidates.push(AssociationCandidate {
                track_index,
                detection_index,
                cost: 0.65 * (1.0 - iou) + 0.35 * normalized_distance,
            });
        }
    }
    candidates.sort_by(|left, right| {
        left.cost
            .total_cmp(&right.cost)
            .then(left.track_index.cmp(&right.track_index))
            .then(left.detection_index.cmp(&right.detection_index))
    });
    candidates
}

fn predicted_bounding_box(track: &VisionTrack, timestamp_ms: TimestampMs) -> NormalizedBoundingBox {
    let latest = track.latest_observation();
    if track.history.len() < 2 || timestamp_ms <= latest.timestamp_ms {
        return latest.bounding_box;
    }
    let previous = &track.history[track.history.len() - 2];
    let observation_delta = latest.timestamp_ms.saturating_sub(previous.timestamp_ms);
    if observation_delta == 0 {
        return latest.bounding_box;
    }

    let elapsed = timestamp_ms.saturating_sub(latest.timestamp_ms) as f32;
    let ratio = (elapsed / observation_delta as f32).min(3.0);
    let (previous_x, previous_y) = center(previous.bounding_box);
    let (latest_x, latest_y) = center(latest.bounding_box);
    let predicted_x = latest_x + (latest_x - previous_x) * ratio;
    let predicted_y = latest_y + (latest_y - previous_y) * ratio;
    box_from_center(predicted_x, predicted_y, latest.bounding_box).unwrap_or(latest.bounding_box)
}

fn center(bounding_box: NormalizedBoundingBox) -> (f32, f32) {
    (
        bounding_box.x + bounding_box.width / 2.0,
        bounding_box.y + bounding_box.height / 2.0,
    )
}

fn center_distance(left: NormalizedBoundingBox, right: NormalizedBoundingBox) -> f32 {
    let (left_x, left_y) = center(left);
    let (right_x, right_y) = center(right);
    (left_x - right_x).hypot(left_y - right_y)
}

fn box_from_center(
    center_x: f32,
    center_y: f32,
    template: NormalizedBoundingBox,
) -> Option<NormalizedBoundingBox> {
    let x = (center_x - template.width / 2.0).clamp(0.0, 1.0 - template.width);
    let y = (center_y - template.height / 2.0).clamp(0.0, 1.0 - template.height);
    NormalizedBoundingBox::new(x, y, template.width, template.height).ok()
}
