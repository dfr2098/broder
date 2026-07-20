use std::cmp::Ordering;

use crate::DetectionCandidate;

/// NMS por clase: conserva primero la detección de mayor confianza y elimina
/// cajas muy solapadas de la misma clase.
pub fn class_aware_nms(
    mut candidates: Vec<DetectionCandidate>,
    iou_threshold: f32,
) -> Vec<DetectionCandidate> {
    candidates.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(Ordering::Equal)
    });

    let mut selected: Vec<DetectionCandidate> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let duplicate = selected.iter().any(|existing| {
            existing.class_id == candidate.class_id
                && existing.bounding_box.iou(candidate.bounding_box) > iou_threshold
        });
        if !duplicate {
            selected.push(candidate);
        }
    }
    selected
}
