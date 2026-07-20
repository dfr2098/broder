use vision_core::{DetectionCandidate, FrameSampler, NormalizedBoundingBox, class_aware_nms};

fn candidate(class_id: u32, confidence: f32, x: f32) -> DetectionCandidate {
    DetectionCandidate {
        class_id,
        class_name: format!("class-{class_id}"),
        confidence,
        bounding_box: NormalizedBoundingBox::new(x, 0.1, 0.4, 0.4).unwrap(),
    }
}

#[test]
fn sampler_selects_five_frames_per_second_from_thirty_fps() {
    let mut sampler = FrameSampler::new(5.0).unwrap();
    let selected = (0..30)
        .map(|frame| ((frame as f64 * 1_000.0 / 30.0).round()) as u64)
        .filter(|timestamp| sampler.should_process(*timestamp))
        .collect::<Vec<_>>();

    assert_eq!(selected, [0, 200, 400, 600, 800]);
}

#[test]
fn nms_removes_duplicates_only_within_the_same_class() {
    let selected = class_aware_nms(
        vec![
            candidate(0, 0.95, 0.1),
            candidate(0, 0.80, 0.12),
            candidate(1, 0.70, 0.12),
        ],
        0.45,
    );

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].confidence, 0.95);
    assert_eq!(selected[1].class_id, 1);
}

#[test]
fn pixel_coordinates_are_clipped_and_normalized() {
    let bounding_box =
        NormalizedBoundingBox::from_pixel_edges(-10.0, 25.0, 250.0, 125.0, 200, 100).unwrap();

    assert_eq!(bounding_box.x, 0.0);
    assert_eq!(bounding_box.y, 0.25);
    assert_eq!(bounding_box.width, 1.0);
    assert_eq!(bounding_box.height, 0.75);
}
