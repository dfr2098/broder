use std::error::Error;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use event_core::{EventBus, EventEnvelope, EventSource, InMemoryEventBus, SourceKind};
use persistence_core::{PersistenceDomain, PersistencePolicy, PersistenceRouter};
use persistence_postgres::PostgresVisionDetectionWriter;
use vision_core::VisionDetection;

use crate::config::{PersistenceConfig, PersistenceMode};

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

enum WorkerCommand {
    Event(Box<EventEnvelope<VisionDetection>>),
    Shutdown,
}

enum WorkerStartup {
    Connected,
    Recovering(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub(crate) enum PersistenceStartup {
    Connected,
    Recovering(String),
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PersistenceSnapshot {
    pub queued: u64,
    pub persisted: u64,
    pub dropped: u64,
    pub connected: bool,
}

#[derive(Default)]
struct SharedMetrics {
    queued: AtomicU64,
    persisted: AtomicU64,
    dropped: AtomicU64,
    connected: AtomicBool,
}

impl SharedMetrics {
    fn snapshot(&self) -> PersistenceSnapshot {
        PersistenceSnapshot {
            queued: self.queued.load(Ordering::Relaxed),
            persisted: self.persisted.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            connected: self.connected.load(Ordering::Relaxed),
        }
    }
}

pub(crate) struct VisionEventPublisher {
    sender: Option<SyncSender<WorkerCommand>>,
    worker: Option<JoinHandle<Result<(), String>>>,
    source: EventSource,
    session_id: u128,
    next_sequence: u64,
    mode: PersistenceMode,
    metrics: Arc<SharedMetrics>,
}

impl VisionEventPublisher {
    pub(crate) fn start(
        database_url: &str,
        source_id: &str,
        config: PersistenceConfig,
    ) -> Result<(Self, PersistenceStartup), Box<dyn Error>> {
        let source = EventSource::new(source_id, SourceKind::Device)?;
        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity);
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let metrics = Arc::new(SharedMetrics::default());
        let worker_metrics = metrics.clone();
        let database_url = database_url.to_owned();
        let worker = thread::Builder::new()
            .name("vision-persistence".to_owned())
            .spawn(move || {
                worker_loop(
                    &database_url,
                    config,
                    receiver,
                    startup_sender,
                    worker_metrics,
                )
            })?;

        let startup = match startup_receiver.recv()? {
            WorkerStartup::Connected => PersistenceStartup::Connected,
            WorkerStartup::Recovering(message) => PersistenceStartup::Recovering(message),
            WorkerStartup::Failed(message) => {
                let _ = worker.join();
                return Err(io::Error::other(message).into());
            }
        };

        Ok((
            Self {
                sender: Some(sender),
                worker: Some(worker),
                source,
                session_id: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
                next_sequence: 1,
                mode: config.mode,
                metrics,
            },
            startup,
        ))
    }

    pub(crate) fn publish_all(
        &mut self,
        detections: &[VisionDetection],
    ) -> Result<(), Box<dyn Error>> {
        for detection in detections {
            let event = self.create_event(detection.clone())?;
            self.enqueue(event)?;
        }
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> PersistenceSnapshot {
        self.metrics.snapshot()
    }

    pub(crate) fn finish(&mut self) -> Result<PersistenceSnapshot, Box<dyn Error>> {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(WorkerCommand::Shutdown);
        }
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(message)) => return Err(io::Error::other(message).into()),
                Err(_) => return Err(io::Error::other("el worker de persistencia colapsó").into()),
            }
        }
        Ok(self.snapshot())
    }

    fn create_event(
        &mut self,
        detection: VisionDetection,
    ) -> Result<EventEnvelope<VisionDetection>, Box<dyn Error>> {
        let observed_at_ms = unix_time_ms()?;
        let event_id = format!(
            "{}:vision:{}:{}",
            self.source.id, self.session_id, self.next_sequence
        );
        self.next_sequence += 1;
        Ok(EventEnvelope::new(
            event_id,
            EVENT_TYPE,
            self.source.clone(),
            observed_at_ms,
            observed_at_ms,
            detection,
        )?)
    }

    fn enqueue(&self, event: EventEnvelope<VisionDetection>) -> Result<(), Box<dyn Error>> {
        let Some(sender) = &self.sender else {
            return Err(io::Error::other("la persistencia ya fue cerrada").into());
        };
        match self.mode {
            PersistenceMode::Required => {
                self.metrics.queued.fetch_add(1, Ordering::Relaxed);
                if sender.send(WorkerCommand::Event(Box::new(event))).is_err() {
                    self.metrics.queued.fetch_sub(1, Ordering::Relaxed);
                    return Err(io::Error::other("worker de persistencia no disponible").into());
                }
            }
            PersistenceMode::BestEffort => {
                self.metrics.queued.fetch_add(1, Ordering::Relaxed);
                match sender.try_send(WorkerCommand::Event(Box::new(event))) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => {
                        self.metrics.queued.fetch_sub(1, Ordering::Relaxed);
                        self.metrics.dropped.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        self.metrics.queued.fetch_sub(1, Ordering::Relaxed);
                        self.metrics.dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for VisionEventPublisher {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn worker_loop(
    database_url: &str,
    config: PersistenceConfig,
    receiver: Receiver<WorkerCommand>,
    startup: SyncSender<WorkerStartup>,
    metrics: Arc<SharedMetrics>,
) -> Result<(), String> {
    let mut bus = match connect_bus(database_url, config.batch_size) {
        Ok(bus) => {
            metrics.connected.store(true, Ordering::Relaxed);
            let _ = startup.send(WorkerStartup::Connected);
            Some(bus)
        }
        Err(error) if config.mode == PersistenceMode::BestEffort => {
            let message = error.to_string();
            let _ = startup.send(WorkerStartup::Recovering(message));
            None
        }
        Err(error) => {
            let message = error.to_string();
            let _ = startup.send(WorkerStartup::Failed(message.clone()));
            return Err(message);
        }
    };
    let interval = Duration::from_millis(config.flush_interval_ms);
    let mut pending = 0_u64;
    let mut last_reconnect_attempt = Instant::now();

    loop {
        match receiver.recv_timeout(interval) {
            Ok(WorkerCommand::Event(event)) => {
                metrics.queued.fetch_sub(1, Ordering::Relaxed);
                if bus.is_none() && last_reconnect_attempt.elapsed() >= interval {
                    last_reconnect_attempt = Instant::now();
                    if let Ok(new_bus) = connect_bus(database_url, config.batch_size) {
                        metrics.connected.store(true, Ordering::Relaxed);
                        bus = Some(new_bus);
                    }
                }
                let Some(active_bus) = bus.as_mut() else {
                    metrics.dropped.fetch_add(1, Ordering::Relaxed);
                    continue;
                };
                if let Err(error) = active_bus.publish(&event) {
                    let lost = pending + 1;
                    pending = 0;
                    if config.mode == PersistenceMode::Required {
                        return Err(error.to_string());
                    }
                    metrics.dropped.fetch_add(lost, Ordering::Relaxed);
                    metrics.connected.store(false, Ordering::Relaxed);
                    bus = None;
                    continue;
                }
                pending += 1;
                if pending >= config.batch_size as u64 {
                    metrics.persisted.fetch_add(pending, Ordering::Relaxed);
                    pending = 0;
                }
            }
            Ok(WorkerCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                flush_bus(&mut bus, &mut pending, config.mode, &metrics)?;
                return Ok(());
            }
            Err(RecvTimeoutError::Timeout) => {
                if bus.is_none() {
                    last_reconnect_attempt = Instant::now();
                    if let Ok(new_bus) = connect_bus(database_url, config.batch_size) {
                        metrics.connected.store(true, Ordering::Relaxed);
                        bus = Some(new_bus);
                    }
                } else {
                    flush_bus(&mut bus, &mut pending, config.mode, &metrics)?;
                }
            }
        }
    }
}

fn connect_bus(
    database_url: &str,
    batch_size: usize,
) -> Result<InMemoryEventBus<VisionDetection>, Box<dyn Error>> {
    let writer = PostgresVisionDetectionWriter::connect_with_batch_size(database_url, batch_size)?;
    let mut router = PersistenceRouter::new(VisionTemporalPolicy);
    router.register(writer);
    let mut bus = InMemoryEventBus::new();
    bus.subscribe(router);
    Ok(bus)
}

fn flush_bus(
    bus: &mut Option<InMemoryEventBus<VisionDetection>>,
    pending: &mut u64,
    mode: PersistenceMode,
    metrics: &SharedMetrics,
) -> Result<(), String> {
    if *pending == 0 {
        return Ok(());
    }
    let Some(active_bus) = bus.as_mut() else {
        metrics.dropped.fetch_add(*pending, Ordering::Relaxed);
        *pending = 0;
        return Ok(());
    };
    match active_bus.flush() {
        Ok(()) => {
            metrics.persisted.fetch_add(*pending, Ordering::Relaxed);
            *pending = 0;
            Ok(())
        }
        Err(_error) if mode == PersistenceMode::BestEffort => {
            metrics.dropped.fetch_add(*pending, Ordering::Relaxed);
            metrics.connected.store(false, Ordering::Relaxed);
            *pending = 0;
            *bus = None;
            Ok(())
        }
        Err(error) => Err(error.to_string()),
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
