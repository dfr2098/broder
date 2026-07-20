use spatial_core::{
    CameraSpatialModel, CrossingDirection, LineRole, NormalizedPoint, SpatialPolygon, SpatialZone,
    VirtualLine, ZoneKind,
};
use tracking_core::{MultiObjectTracker, TrackerConfig};
use vision_core::{DetectionCandidate, NormalizedBoundingBox, VisionDetection};

fn point(x: f32, y: f32) -> NormalizedPoint {
    NormalizedPoint::new(x, y).unwrap()
}

fn rectangle(left: f32, top: f32, right: f32, bottom: f32) -> SpatialPolygon {
    SpatialPolygon::new(vec![
        point(left, top),
        point(right, top),
        point(right, bottom),
        point(left, bottom),
    ])
    .unwrap()
}

fn detection(frame: u64, timestamp: u64, x: f32) -> VisionDetection {
    VisionDetection::from_candidate(
        "cam-1",
        frame,
        timestamp,
        0,
        DetectionCandidate {
            class_id: 0,
            class_name: "object".to_owned(),
            confidence: 0.9,
            bounding_box: NormalizedBoundingBox::new(x, 0.4, 0.1, 0.2).unwrap(),
        },
    )
    .unwrap()
}

fn model() -> CameraSpatialModel {
    CameraSpatialModel::new(
        "cam-1",
        rectangle(0.0, 0.0, 1.0, 1.0),
        vec![
            SpatialZone::new(
                "conveyor",
                "Transportador",
                ZoneKind::Conveyor,
                None,
                Some("norte".to_owned()),
                rectangle(0.0, 0.0, 1.0, 1.0),
            )
            .unwrap(),
            SpatialZone::new(
                "right-lane",
                "Carril derecho",
                ZoneKind::Lane,
                Some("conveyor".to_owned()),
                Some("norte".to_owned()),
                rectangle(0.5, 0.0, 1.0, 1.0),
            )
            .unwrap(),
        ],
        vec![
            VirtualLine::new(
                "entry",
                "Entrada",
                LineRole::Entry,
                point(0.5, 0.0),
                point(0.5, 1.0),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn polygon_includes_internal_and_boundary_points() {
    let polygon = rectangle(0.2, 0.2, 0.8, 0.8);

    assert!(polygon.contains(point(0.5, 0.5)));
    assert!(polygon.contains(point(0.2, 0.5)));
    assert!(!polygon.contains(point(0.1, 0.5)));
}

#[test]
fn track_is_mapped_to_nested_physical_zones() {
    let mut tracker = MultiObjectTracker::new(
        "cam-1",
        TrackerConfig {
            minimum_confirmed_hits: 1,
            ..TrackerConfig::default()
        },
    )
    .unwrap();
    tracker
        .process_frame(1, 0, &[detection(1, 0, 0.6)])
        .unwrap();

    let spatial = model().locate(&tracker.active_tracks()[0]).unwrap();

    assert!(spatial.inside_observation_region);
    assert_eq!(spatial.occupied_zones.len(), 2);
    assert_eq!(spatial.occupied_zones[1].zone_id, "right-lane");
    assert_eq!(
        spatial.occupied_zones[1].direction.as_deref(),
        Some("norte")
    );
}

#[test]
fn movement_across_a_virtual_line_is_reported() {
    let mut tracker = MultiObjectTracker::new(
        "cam-1",
        TrackerConfig {
            minimum_confirmed_hits: 1,
            maximum_center_distance: 0.5,
            ..TrackerConfig::default()
        },
    )
    .unwrap();
    tracker
        .process_frame(1, 0, &[detection(1, 0, 0.3)])
        .unwrap();
    tracker
        .process_frame(2, 200, &[detection(2, 200, 0.55)])
        .unwrap();

    let spatial = model().locate(&tracker.active_tracks()[0]).unwrap();

    assert_eq!(spatial.crossed_lines.len(), 1);
    assert_eq!(spatial.crossed_lines[0].line_id, "entry");
    assert_eq!(
        spatial.crossed_lines[0].direction,
        CrossingDirection::PositiveToNegative
    );
}
