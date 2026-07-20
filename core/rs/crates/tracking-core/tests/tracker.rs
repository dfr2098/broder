use tracking_core::{MultiObjectTracker, TrackState, TrackerConfig};
use vision_core::{DetectionCandidate, NormalizedBoundingBox, VisionDetection};

fn detection(
    frame_id: u64,
    timestamp_ms: u64,
    sequence: usize,
    class_id: u32,
    x: f32,
    confidence: f32,
) -> VisionDetection {
    VisionDetection::from_candidate(
        "cam-1",
        frame_id,
        timestamp_ms,
        sequence,
        DetectionCandidate {
            class_id,
            class_name: format!("class-{class_id}"),
            confidence,
            bounding_box: NormalizedBoundingBox::new(x, 0.2, 0.15, 0.2).unwrap(),
        },
    )
    .unwrap()
}

#[test]
fn consecutive_detections_create_one_confirmed_track() {
    let mut tracker = MultiObjectTracker::new("cam-1", TrackerConfig::default()).unwrap();
    let first = tracker
        .process_frame(1, 0, &[detection(1, 0, 0, 1, 0.1, 0.8)])
        .unwrap();
    let track_id = first.assignments[0].track_id.clone();
    assert_eq!(tracker.active_tracks()[0].state, TrackState::Tentative);

    let second = tracker
        .process_frame(2, 200, &[detection(2, 200, 0, 1, 0.13, 0.6)])
        .unwrap();

    assert_eq!(second.assignments[0].track_id, track_id);
    assert_eq!(tracker.active_tracks()[0].state, TrackState::Confirmed);
    assert_eq!(tracker.active_tracks()[0].history.len(), 2);
    assert!((tracker.active_tracks()[0].accumulated_confidence - 0.7).abs() < 0.0001);
}

#[test]
fn identity_survives_a_temporary_detection_loss() {
    let mut tracker = MultiObjectTracker::new("cam-1", TrackerConfig::default()).unwrap();
    let first = tracker
        .process_frame(1, 0, &[detection(1, 0, 0, 1, 0.1, 0.9)])
        .unwrap();
    let track_id = first.assignments[0].track_id.clone();
    tracker
        .process_frame(2, 200, &[detection(2, 200, 0, 1, 0.15, 0.9)])
        .unwrap();
    tracker.process_frame(3, 400, &[]).unwrap();
    assert_eq!(tracker.active_tracks()[0].state, TrackState::Lost);

    let recovered = tracker
        .process_frame(4, 600, &[detection(4, 600, 0, 1, 0.25, 0.9)])
        .unwrap();

    assert_eq!(recovered.assignments[0].track_id, track_id);
    assert_eq!(tracker.active_tracks()[0].state, TrackState::Confirmed);
    assert_eq!(tracker.active_tracks()[0].history.len(), 3);
}

#[test]
fn lost_track_finishes_after_the_configured_tolerance() {
    let config = TrackerConfig {
        maximum_missed_frames: 1,
        ..TrackerConfig::default()
    };
    let mut tracker = MultiObjectTracker::new("cam-1", config).unwrap();
    tracker
        .process_frame(1, 0, &[detection(1, 0, 0, 1, 0.1, 0.9)])
        .unwrap();
    tracker.process_frame(2, 200, &[]).unwrap();
    let finished = tracker.process_frame(3, 400, &[]).unwrap();

    assert!(tracker.active_tracks().is_empty());
    assert_eq!(finished.finished_tracks.len(), 1);
    assert_eq!(finished.finished_tracks[0].state, TrackState::Finished);
}

#[test]
fn an_observation_after_the_time_limit_creates_a_new_identity() {
    let config = TrackerConfig {
        minimum_confirmed_hits: 1,
        maximum_lost_ms: 500,
        ..TrackerConfig::default()
    };
    let mut tracker = MultiObjectTracker::new("cam-1", config).unwrap();
    let first = tracker
        .process_frame(1, 0, &[detection(1, 0, 0, 1, 0.1, 0.9)])
        .unwrap();
    let first_track_id = first.assignments[0].track_id.clone();

    let late = tracker
        .process_frame(2, 800, &[detection(2, 800, 0, 1, 0.1, 0.9)])
        .unwrap();

    assert_eq!(late.finished_tracks[0].track_id, first_track_id);
    assert_ne!(late.assignments[0].track_id, first_track_id);
}

#[test]
fn association_is_independent_of_detection_order() {
    let config = TrackerConfig {
        minimum_confirmed_hits: 1,
        ..TrackerConfig::default()
    };
    let mut tracker = MultiObjectTracker::new("cam-1", config).unwrap();
    let first = tracker
        .process_frame(
            1,
            0,
            &[
                detection(1, 0, 0, 1, 0.1, 0.9),
                detection(1, 0, 1, 1, 0.7, 0.9),
            ],
        )
        .unwrap();
    let left_track = first.assignments[0].track_id.clone();
    let right_track = first.assignments[1].track_id.clone();

    let second = tracker
        .process_frame(
            2,
            200,
            &[
                detection(2, 200, 0, 1, 0.68, 0.9),
                detection(2, 200, 1, 1, 0.12, 0.9),
            ],
        )
        .unwrap();

    let assignment_for = |detection_id: &str| {
        second
            .assignments
            .iter()
            .find(|assignment| assignment.detection_id == detection_id)
            .unwrap()
            .track_id
            .clone()
    };
    assert_eq!(assignment_for("cam-1:2:0"), right_track);
    assert_eq!(assignment_for("cam-1:2:1"), left_track);
}

#[test]
fn different_classes_never_share_a_track() {
    let config = TrackerConfig {
        minimum_confirmed_hits: 1,
        ..TrackerConfig::default()
    };
    let mut tracker = MultiObjectTracker::new("cam-1", config).unwrap();
    let first = tracker
        .process_frame(1, 0, &[detection(1, 0, 0, 1, 0.1, 0.9)])
        .unwrap();
    let second = tracker
        .process_frame(2, 200, &[detection(2, 200, 0, 2, 0.1, 0.9)])
        .unwrap();

    assert_ne!(
        first.assignments[0].track_id,
        second.assignments[0].track_id
    );
    assert_eq!(tracker.active_tracks().len(), 2);
}
