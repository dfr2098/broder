use opencv::{
    core::{Mat, Point, Rect, Scalar},
    imgproc,
    prelude::*,
};
use spatial_core::{
    CameraSpatialModel, LineRole, NormalizedPoint, SpatialPolygon, SpatialTrack, ZoneKind,
};
use tracking_core::TrackAssignment;
use vision_core::VisionDetection;

pub(crate) struct DisplayContext<'a> {
    pub detections: &'a [VisionDetection],
    pub assignments: &'a [TrackAssignment],
    pub spatial_tracks: &'a [SpatialTrack],
    pub spatial_model: Option<&'a CameraSpatialModel>,
    pub active_track_count: usize,
    pub inference_ms: f64,
    pub processing_fps: f64,
}

pub(crate) fn draw_detections(frame: &mut Mat, context: &DisplayContext<'_>) -> opencv::Result<()> {
    let frame_width = frame.cols();
    let frame_height = frame.rows();
    if let Some(model) = context.spatial_model {
        draw_spatial_model(frame, model)?;
    }
    for detection in context.detections {
        let bounding_box = detection.bounding_box;
        let x = (bounding_box.x * frame_width as f32).round() as i32;
        let y = (bounding_box.y * frame_height as f32).round() as i32;
        let width = (bounding_box.width * frame_width as f32).round().max(1.0) as i32;
        let height = (bounding_box.height * frame_height as f32).round().max(1.0) as i32;
        let color = class_color(detection.class_id);
        imgproc::rectangle(
            frame,
            Rect::new(x, y, width, height),
            color,
            2,
            imgproc::LINE_AA,
            0,
        )?;
        let track_label = context
            .assignments
            .iter()
            .find(|assignment| assignment.detection_id == detection.detection_id)
            .map(|assignment| short_track_id(&assignment.track_id))
            .unwrap_or("T?");
        let zone_label = context
            .assignments
            .iter()
            .find(|assignment| assignment.detection_id == detection.detection_id)
            .and_then(|assignment| {
                context
                    .spatial_tracks
                    .iter()
                    .find(|track| track.track_id == assignment.track_id)
            })
            .and_then(|track| track.occupied_zones.last())
            .map(|zone| zone.name.as_str());
        let label = format!(
            "{track_label} {} {:.0}%{}",
            detection.class_name,
            detection.confidence * 100.0,
            zone_label
                .map(|zone| format!(" | {zone}"))
                .unwrap_or_default()
        );
        imgproc::put_text(
            frame,
            &label,
            Point::new(x, (y - 7).max(18)),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.55,
            color,
            2,
            imgproc::LINE_AA,
            false,
        )?;
    }

    let status = format!(
        "YOLO {:.1} FPS | inferencia {:.1} ms | detecciones {} | tracks {}",
        context.processing_fps,
        context.inference_ms,
        context.detections.len(),
        context.active_track_count
    );
    imgproc::put_text(
        frame,
        &status,
        Point::new(16, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.62,
        Scalar::new(80.0, 255.0, 80.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )
}

fn draw_spatial_model(frame: &mut Mat, model: &CameraSpatialModel) -> opencv::Result<()> {
    draw_polygon(
        frame,
        &model.observation_region,
        Scalar::new(220.0, 220.0, 220.0, 0.0),
        1,
    )?;
    for zone in &model.zones {
        let color = match &zone.kind {
            ZoneKind::Conveyor => Scalar::new(255.0, 180.0, 40.0, 0.0),
            ZoneKind::Lane => Scalar::new(80.0, 255.0, 80.0, 0.0),
            ZoneKind::Entry => Scalar::new(80.0, 220.0, 255.0, 0.0),
            ZoneKind::Exit => Scalar::new(80.0, 80.0, 255.0, 0.0),
            ZoneKind::Custom(_) => Scalar::new(220.0, 120.0, 220.0, 0.0),
        };
        draw_polygon(frame, &zone.polygon, color, 1)?;
        let label_point = to_pixel(zone.polygon.points()[0], frame.cols(), frame.rows());
        imgproc::put_text(
            frame,
            &zone.name,
            Point::new(label_point.x + 4, (label_point.y + 18).max(18)),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.42,
            color,
            1,
            imgproc::LINE_AA,
            false,
        )?;
    }
    for line in &model.lines {
        let color = match line.role {
            LineRole::Entry => Scalar::new(0.0, 255.0, 255.0, 0.0),
            LineRole::Exit => Scalar::new(0.0, 0.0, 255.0, 0.0),
            LineRole::Boundary => Scalar::new(255.0, 255.0, 0.0, 0.0),
        };
        let start = to_pixel(line.start, frame.cols(), frame.rows());
        let end = to_pixel(line.end, frame.cols(), frame.rows());
        imgproc::line(frame, start, end, color, 2, imgproc::LINE_AA, 0)?;
    }
    Ok(())
}

fn draw_polygon(
    frame: &mut Mat,
    polygon: &SpatialPolygon,
    color: Scalar,
    thickness: i32,
) -> opencv::Result<()> {
    for index in 0..polygon.points().len() {
        let start = to_pixel(polygon.points()[index], frame.cols(), frame.rows());
        let end = to_pixel(
            polygon.points()[(index + 1) % polygon.points().len()],
            frame.cols(),
            frame.rows(),
        );
        imgproc::line(frame, start, end, color, thickness, imgproc::LINE_AA, 0)?;
    }
    Ok(())
}

fn to_pixel(point: NormalizedPoint, width: i32, height: i32) -> Point {
    Point::new(
        (point.x * (width - 1) as f32).round() as i32,
        (point.y * (height - 1) as f32).round() as i32,
    )
}

fn short_track_id(track_id: &str) -> &str {
    track_id.rsplit(':').next().unwrap_or(track_id)
}

fn class_color(class_id: u32) -> Scalar {
    let blue = ((class_id * 67 + 80) % 255) as f64;
    let green = ((class_id * 29 + 180) % 255) as f64;
    let red = ((class_id * 97 + 120) % 255) as f64;
    Scalar::new(blue, green, red, 0.0)
}
