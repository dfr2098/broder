#[derive(Clone, Debug)]
pub struct FrameSampler {
    period_ms: f64,
    next_due_ms: Option<f64>,
    last_timestamp_ms: Option<u64>,
}

impl FrameSampler {
    pub fn new(processing_fps: f64) -> Result<Self, &'static str> {
        if !processing_fps.is_finite() || processing_fps <= 0.0 || processing_fps > 120.0 {
            return Err("los FPS de procesamiento deben estar entre 0 y 120");
        }
        Ok(Self {
            period_ms: 1_000.0 / processing_fps,
            next_due_ms: None,
            last_timestamp_ms: None,
        })
    }

    pub fn should_process(&mut self, timestamp_ms: u64) -> bool {
        if self
            .last_timestamp_ms
            .is_some_and(|previous| timestamp_ms < previous)
        {
            self.next_due_ms = None;
        }
        self.last_timestamp_ms = Some(timestamp_ms);

        let timestamp = timestamp_ms as f64;
        let Some(mut due) = self.next_due_ms else {
            self.next_due_ms = Some(timestamp + self.period_ms);
            return true;
        };
        if timestamp + 0.5 < due {
            return false;
        }

        while due <= timestamp + 0.5 {
            due += self.period_ms;
        }
        self.next_due_ms = Some(due);
        true
    }
}
