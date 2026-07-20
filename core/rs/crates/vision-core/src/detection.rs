pub type FrameId = u64;
pub type TimestampMs = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedBoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NormalizedBoundingBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, &'static str> {
        let values = [x, y, width, height];
        if values.iter().any(|value| !value.is_finite()) {
            return Err("las coordenadas deben ser números finitos");
        }
        if x < 0.0 || y < 0.0 || width <= 0.0 || height <= 0.0 {
            return Err("la caja debe tener origen válido y dimensiones positivas");
        }
        if x + width > 1.000_001 || y + height > 1.000_001 {
            return Err("la caja debe permanecer dentro de la imagen normalizada");
        }

        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn from_pixel_edges(
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        image_width: u32,
        image_height: u32,
    ) -> Option<Self> {
        if image_width == 0 || image_height == 0 {
            return None;
        }

        let max_x = image_width as f32;
        let max_y = image_height as f32;
        let left = left.clamp(0.0, max_x);
        let top = top.clamp(0.0, max_y);
        let right = right.clamp(0.0, max_x);
        let bottom = bottom.clamp(0.0, max_y);
        if right <= left || bottom <= top {
            return None;
        }

        Self::new(
            left / max_x,
            top / max_y,
            (right - left) / max_x,
            (bottom - top) / max_y,
        )
        .ok()
    }

    pub fn iou(self, other: Self) -> f32 {
        let intersection_left = self.x.max(other.x);
        let intersection_top = self.y.max(other.y);
        let intersection_right = (self.x + self.width).min(other.x + other.width);
        let intersection_bottom = (self.y + self.height).min(other.y + other.height);
        let intersection_width = (intersection_right - intersection_left).max(0.0);
        let intersection_height = (intersection_bottom - intersection_top).max(0.0);
        let intersection = intersection_width * intersection_height;
        let union = self.width * self.height + other.width * other.height - intersection;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DetectionCandidate {
    pub class_id: u32,
    pub class_name: String,
    pub confidence: f32,
    pub bounding_box: NormalizedBoundingBox,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisionDetection {
    pub detection_id: String,
    pub source_id: String,
    pub frame_id: FrameId,
    pub timestamp_ms: TimestampMs,
    pub class_id: u32,
    pub class_name: String,
    pub confidence: f32,
    pub bounding_box: NormalizedBoundingBox,
}

impl VisionDetection {
    pub fn from_candidate(
        source_id: &str,
        frame_id: FrameId,
        timestamp_ms: TimestampMs,
        sequence: usize,
        candidate: DetectionCandidate,
    ) -> Result<Self, &'static str> {
        if source_id.trim().is_empty() {
            return Err("la fuente de visión no puede estar vacía");
        }
        if !candidate.confidence.is_finite()
            || candidate.confidence < 0.0
            || candidate.confidence > 1.0
        {
            return Err("la confianza debe estar entre 0 y 1");
        }

        Ok(Self {
            detection_id: format!("{source_id}:{frame_id}:{sequence}"),
            source_id: source_id.to_owned(),
            frame_id,
            timestamp_ms,
            class_id: candidate.class_id,
            class_name: candidate.class_name,
            confidence: candidate.confidence,
            bounding_box: candidate.bounding_box,
        })
    }
}
