use crate::SpatialError;

const EPSILON: f32 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedPoint {
    pub x: f32,
    pub y: f32,
}

impl NormalizedPoint {
    pub fn new(x: f32, y: f32) -> Result<Self, SpatialError> {
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..=1.0).contains(&x)
            || !(0.0..=1.0).contains(&y)
        {
            return Err(SpatialError::new(
                "el punto normalizado debe permanecer entre 0 y 1",
            ));
        }
        Ok(Self { x, y })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialPolygon {
    points: Vec<NormalizedPoint>,
}

impl SpatialPolygon {
    pub fn new(points: Vec<NormalizedPoint>) -> Result<Self, SpatialError> {
        if points.len() < 3 {
            return Err(SpatialError::new(
                "un polígono espacial requiere al menos tres puntos",
            ));
        }
        Ok(Self { points })
    }

    pub fn points(&self) -> &[NormalizedPoint] {
        &self.points
    }

    pub fn contains(&self, point: NormalizedPoint) -> bool {
        let mut inside = false;
        for index in 0..self.points.len() {
            let current = self.points[index];
            let previous = self.points[(index + self.points.len() - 1) % self.points.len()];
            if point_on_segment(point, previous, current) {
                return true;
            }
            let crosses = (current.y > point.y) != (previous.y > point.y)
                && point.x
                    < (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y)
                        + current.x;
            if crosses {
                inside = !inside;
            }
        }
        inside
    }
}

pub(crate) fn side_of_line(
    start: NormalizedPoint,
    end: NormalizedPoint,
    point: NormalizedPoint,
) -> f32 {
    (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x)
}

fn point_on_segment(point: NormalizedPoint, start: NormalizedPoint, end: NormalizedPoint) -> bool {
    if side_of_line(start, end, point).abs() > EPSILON {
        return false;
    }
    point.x >= start.x.min(end.x) - EPSILON
        && point.x <= start.x.max(end.x) + EPSILON
        && point.y >= start.y.min(end.y) - EPSILON
        && point.y <= start.y.max(end.y) + EPSILON
}
