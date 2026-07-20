use tracking_core::VisionTrack;

use crate::geometry::side_of_line;
use crate::{
    CameraSpatialModel, CrossingDirection, LineCrossing, NormalizedPoint, SpatialError,
    SpatialTrack, ZoneMatch,
};

impl CameraSpatialModel {
    pub fn locate(&self, track: &VisionTrack) -> Result<SpatialTrack, SpatialError> {
        if track.camera_id != self.camera_id {
            return Err(SpatialError::new(format!(
                "el track {} pertenece a otra cámara",
                track.track_id
            )));
        }
        let latest = track.latest_observation();
        let anchor = bottom_center(latest.bounding_box)?;
        let occupied_zones = self
            .zones
            .iter()
            .filter(|zone| zone.polygon.contains(anchor))
            .map(|zone| ZoneMatch {
                zone_id: zone.zone_id.clone(),
                name: zone.name.clone(),
                kind: zone.kind.clone(),
                parent_id: zone.parent_id.clone(),
                direction: zone.direction.clone(),
            })
            .collect();
        let crossed_lines = if track.history.len() >= 2 {
            let previous = bottom_center(track.history[track.history.len() - 2].bounding_box)?;
            self.lines
                .iter()
                .filter_map(|line| {
                    let before = side_of_line(line.start, line.end, previous);
                    let after = side_of_line(line.start, line.end, anchor);
                    if before < 0.0 && after > 0.0 {
                        Some(LineCrossing {
                            line_id: line.line_id.clone(),
                            name: line.name.clone(),
                            role: line.role,
                            direction: CrossingDirection::NegativeToPositive,
                        })
                    } else if before > 0.0 && after < 0.0 {
                        Some(LineCrossing {
                            line_id: line.line_id.clone(),
                            name: line.name.clone(),
                            role: line.role,
                            direction: CrossingDirection::PositiveToNegative,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(SpatialTrack {
            track_id: track.track_id.clone(),
            camera_id: track.camera_id.clone(),
            timestamp_ms: latest.timestamp_ms,
            anchor,
            inside_observation_region: self.observation_region.contains(anchor),
            occupied_zones,
            crossed_lines,
        })
    }
}

fn bottom_center(
    bounding_box: vision_core::NormalizedBoundingBox,
) -> Result<NormalizedPoint, SpatialError> {
    NormalizedPoint::new(
        bounding_box.x + bounding_box.width / 2.0,
        bounding_box.y + bounding_box.height,
    )
}
