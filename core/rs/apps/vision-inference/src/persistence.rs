use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use event_core::{EventBus, EventEnvelope, EventSource, InMemoryEventBus, SourceKind};
use persistence_core::{PersistenceDomain, PersistencePolicy, PersistenceRouter};
use persistence_postgres::PostgresVisionDetectionWriter;
use vision_core::VisionDetection;

const EVENT_TYPE: &str = "vision.detection.observed";

struct VisionTemporalPolicy;

impl PersistencePolicy<VisionDetection> for VisionTemporalPolicy {
    fn targets(&self, event: &EventEnvelope<VisionDetection>) -> Vec<PersistenceDomain> {
        if event.event_type == EVENT_TYPE {
            vec![PersistenceDomain::Temporal]
        } else {
            Vec::new()
        }
    }
}

pub(crate) struct VisionEventPublisher {
    bus: InMemoryEventBus<VisionDetection>,
    source: EventSource,
    session_id: u128,
    next_sequence: u64,
}

impl VisionEventPublisher {
    pub(crate) fn connect(database_url: &str, source_id: &str) -> Result<Self, Box<dyn Error>> {
        let writer = PostgresVisionDetectionWriter::connect(database_url)?;
        let mut router = PersistenceRouter::new(VisionTemporalPolicy);
        router.register(writer);

        let mut bus = InMemoryEventBus::new();
        bus.subscribe(router);

        Ok(Self {
            bus,
            source: EventSource::new(source_id, SourceKind::Device)?,
            session_id: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            next_sequence: 1,
        })
    }

    pub(crate) fn publish_all(
        &mut self,
        detections: &[VisionDetection],
    ) -> Result<(), Box<dyn Error>> {
        for detection in detections {
            let observed_at_ms = unix_time_ms()?;
            let event_id = format!(
                "{}:vision:{}:{}",
                self.source.id, self.session_id, self.next_sequence
            );
            self.next_sequence += 1;
            let event = EventEnvelope::new(
                event_id,
                EVENT_TYPE,
                self.source.clone(),
                observed_at_ms,
                observed_at_ms,
                detection.clone(),
            )?;
            self.bus.publish(&event)?;
        }
        Ok(())
    }
}

fn unix_time_ms() -> Result<u64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use vision_core::{DetectionCandidate, NormalizedBoundingBox};

    use super::*;

    fn event(event_type: &str) -> EventEnvelope<VisionDetection> {
        let detection = VisionDetection::from_candidate(
            "camera-1",
            1,
            0,
            0,
            DetectionCandidate {
                class_id: 0,
                class_name: "box".to_owned(),
                confidence: 0.9,
                bounding_box: NormalizedBoundingBox::new(0.1, 0.2, 0.3, 0.4).unwrap(),
            },
        )
        .unwrap();
        EventEnvelope::new(
            "event-1",
            event_type,
            EventSource::new("camera-1", SourceKind::Device).unwrap(),
            100,
            101,
            detection,
        )
        .unwrap()
    }

    #[test]
    fn routes_vision_detections_only_to_temporal_storage() {
        let policy = VisionTemporalPolicy;
        assert_eq!(
            policy.targets(&event(EVENT_TYPE)),
            [PersistenceDomain::Temporal]
        );
        assert!(policy.targets(&event("other.event")).is_empty());
    }
}
