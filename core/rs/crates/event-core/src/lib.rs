//! Contratos tecnológicos neutrales para el bus de eventos de Little Brother.
//!
//! Un adaptador futuro podrá implementar estos contratos con NATS, Kafka,
//! Redis Streams u otra tecnología sin modificar a los productores.

use std::error::Error;
use std::fmt::{self, Display};

pub type TimestampMs = u64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceKind {
    Device,
    ExternalSystem,
    Human,
    Simulation,
    Other(String),
}

/// Describe de dónde provino una observación, sin introducir conceptos como
/// PLC, WMS, fabricante o protocolo en el modelo persistido.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSource {
    pub id: String,
    pub kind: SourceKind,
}

impl EventSource {
    pub fn new(id: impl Into<String>, kind: SourceKind) -> Result<Self, &'static str> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err("la fuente no puede estar vacía");
        }
        Ok(Self { id, kind })
    }
}

/// Metadatos comunes a cualquier evento normalizado.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope<E> {
    pub id: String,
    pub event_type: String,
    pub schema_version: u16,
    pub source: EventSource,
    pub occurred_at_ms: TimestampMs,
    pub observed_at_ms: TimestampMs,
    pub correlation_id: Option<String>,
    pub payload: E,
}

impl<E> EventEnvelope<E> {
    pub fn new(
        id: impl Into<String>,
        event_type: impl Into<String>,
        source: EventSource,
        occurred_at_ms: TimestampMs,
        observed_at_ms: TimestampMs,
        payload: E,
    ) -> Result<Self, &'static str> {
        let id = id.into();
        let event_type = event_type.into();
        if id.trim().is_empty() || event_type.trim().is_empty() {
            return Err("el evento debe tener identificador y tipo");
        }
        Ok(Self {
            id,
            event_type,
            schema_version: 1,
            source,
            occurred_at_ms,
            observed_at_ms,
            correlation_id: None,
            payload,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerError {
    pub handler: &'static str,
    pub message: String,
}

impl HandlerError {
    pub fn new(handler: &'static str, message: impl Into<String>) -> Self {
        Self {
            handler,
            message: message.into(),
        }
    }
}

impl Display for HandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.handler, self.message)
    }
}

impl Error for HandlerError {}

pub trait EventHandler<E> {
    fn name(&self) -> &'static str;
    fn handle(&mut self, event: &EventEnvelope<E>) -> Result<(), HandlerError>;

    /// Fuerza la entrega de buffers internos. Los consumidores sin buffers no
    /// necesitan sobrescribir este método.
    fn flush(&mut self) -> Result<(), HandlerError> {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishError {
    pub failures: Vec<HandlerError>,
}

impl Display for PublishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "fallaron {} consumidores", self.failures.len())
    }
}

impl Error for PublishError {}

pub trait EventBus<E> {
    fn publish(&mut self, event: &EventEnvelope<E>) -> Result<(), PublishError>;
    fn flush(&mut self) -> Result<(), PublishError>;
}

/// Implementación síncrona para pruebas y prototipos. Los fallos se recopilan
/// después de notificar a todos los consumidores para evitar que uno bloquee a
/// los demás.
#[derive(Default)]
pub struct InMemoryEventBus<E> {
    handlers: Vec<Box<dyn EventHandler<E>>>,
}

impl<E> InMemoryEventBus<E> {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, handler: impl EventHandler<E> + 'static) {
        self.handlers.push(Box::new(handler));
    }

    pub fn subscriber_count(&self) -> usize {
        self.handlers.len()
    }
}

impl<E> EventBus<E> for InMemoryEventBus<E> {
    fn publish(&mut self, event: &EventEnvelope<E>) -> Result<(), PublishError> {
        let failures = self
            .handlers
            .iter_mut()
            .filter_map(|handler| {
                handler.handle(event).err().map(|mut error| {
                    error.handler = handler.name();
                    error
                })
            })
            .collect::<Vec<_>>();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(PublishError { failures })
        }
    }

    fn flush(&mut self) -> Result<(), PublishError> {
        let failures = self
            .handlers
            .iter_mut()
            .filter_map(|handler| {
                handler.flush().err().map(|mut error| {
                    error.handler = handler.name();
                    error
                })
            })
            .collect::<Vec<_>>();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(PublishError { failures })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    struct Collector {
        values: Rc<RefCell<Vec<String>>>,
        flushes: Rc<RefCell<u32>>,
    }

    impl EventHandler<String> for Collector {
        fn name(&self) -> &'static str {
            "collector"
        }

        fn handle(&mut self, event: &EventEnvelope<String>) -> Result<(), HandlerError> {
            self.values.borrow_mut().push(event.payload.clone());
            Ok(())
        }

        fn flush(&mut self) -> Result<(), HandlerError> {
            *self.flushes.borrow_mut() += 1;
            Ok(())
        }
    }

    #[test]
    fn publishes_the_same_normalized_event_to_multiple_consumers() {
        let first = Rc::new(RefCell::new(Vec::new()));
        let second = Rc::new(RefCell::new(Vec::new()));
        let flushes = Rc::new(RefCell::new(0));
        let mut bus = InMemoryEventBus::new();
        bus.subscribe(Collector {
            values: first.clone(),
            flushes: flushes.clone(),
        });
        bus.subscribe(Collector {
            values: second.clone(),
            flushes: flushes.clone(),
        });
        let event = EventEnvelope::new(
            "EV1",
            "object.entered",
            EventSource::new("SIM-1", SourceKind::Simulation).unwrap(),
            100,
            101,
            "BOX-1".to_owned(),
        )
        .unwrap();

        bus.publish(&event).unwrap();
        bus.flush().unwrap();

        assert_eq!(first.borrow().as_slice(), ["BOX-1"]);
        assert_eq!(second.borrow().as_slice(), ["BOX-1"]);
        assert_eq!(*flushes.borrow(), 2);
    }
}
