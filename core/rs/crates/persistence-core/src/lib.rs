//! Enrutamiento de persistencia sin dependencias de motores de base de datos.
//!
//! PostgreSQL, TimescaleDB o ClickHouse se implementarán como adaptadores de
//! `PersistenceWriter`, fuera de los núcleos funcionales.

use std::error::Error;
use std::fmt::{self, Display};

use event_core::{EventEnvelope, EventHandler, HandlerError};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PersistenceDomain {
    Operational,
    Temporal,
    Relational,
}

/// La política vive en la capa de aplicación/infraestructura, no dentro de las
/// entidades del dominio.
pub trait PersistencePolicy<E> {
    fn targets(&self, event: &EventEnvelope<E>) -> Vec<PersistenceDomain>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistenceError {
    pub message: String,
}

impl PersistenceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl Error for PersistenceError {}

/// Puerto que implementarán los adaptadores concretos. El evento sigue siendo
/// una entidad del dominio; el escritor decide tablas, lotes y transacciones.
pub trait PersistenceWriter<E> {
    fn name(&self) -> &'static str;
    fn domain(&self) -> PersistenceDomain;
    fn persist(&mut self, event: &EventEnvelope<E>) -> Result<(), PersistenceError>;

    fn flush(&mut self) -> Result<(), PersistenceError> {
        Ok(())
    }
}

pub struct PersistenceRouter<E, P: PersistencePolicy<E>> {
    policy: P,
    writers: Vec<Box<dyn PersistenceWriter<E>>>,
}

impl<E, P: PersistencePolicy<E>> PersistenceRouter<E, P> {
    pub fn new(policy: P) -> Self {
        Self {
            policy,
            writers: Vec::new(),
        }
    }

    pub fn register(&mut self, writer: impl PersistenceWriter<E> + 'static) {
        self.writers.push(Box::new(writer));
    }
}

impl<E, P: PersistencePolicy<E>> EventHandler<E> for PersistenceRouter<E, P> {
    fn name(&self) -> &'static str {
        "persistence-router"
    }

    fn handle(&mut self, event: &EventEnvelope<E>) -> Result<(), HandlerError> {
        let targets = self.policy.targets(event);
        for writer in self
            .writers
            .iter_mut()
            .filter(|writer| targets.contains(&writer.domain()))
        {
            writer.persist(event).map_err(|error| {
                HandlerError::new(writer.name(), format!("no se pudo persistir: {error}"))
            })?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), HandlerError> {
        for writer in &mut self.writers {
            writer.flush().map_err(|error| {
                HandlerError::new(
                    writer.name(),
                    format!("no se pudo vaciar el buffer: {error}"),
                )
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use event_core::{EventBus, EventSource, InMemoryEventBus, SourceKind};

    use super::*;

    struct StateAndHistory;

    impl PersistencePolicy<String> for StateAndHistory {
        fn targets(&self, _event: &EventEnvelope<String>) -> Vec<PersistenceDomain> {
            vec![PersistenceDomain::Operational, PersistenceDomain::Temporal]
        }
    }

    struct Collector {
        domain: PersistenceDomain,
        values: Rc<RefCell<Vec<String>>>,
    }

    impl PersistenceWriter<String> for Collector {
        fn name(&self) -> &'static str {
            "test-writer"
        }

        fn domain(&self) -> PersistenceDomain {
            self.domain
        }

        fn persist(&mut self, event: &EventEnvelope<String>) -> Result<(), PersistenceError> {
            self.values.borrow_mut().push(event.id.clone());
            Ok(())
        }
    }

    #[test]
    fn routes_one_event_to_operational_and_temporal_storage() {
        let operational = Rc::new(RefCell::new(Vec::new()));
        let temporal = Rc::new(RefCell::new(Vec::new()));
        let relational = Rc::new(RefCell::new(Vec::new()));
        let mut router = PersistenceRouter::new(StateAndHistory);
        for (domain, values) in [
            (PersistenceDomain::Operational, operational.clone()),
            (PersistenceDomain::Temporal, temporal.clone()),
            (PersistenceDomain::Relational, relational.clone()),
        ] {
            router.register(Collector { domain, values });
        }
        let mut bus = InMemoryEventBus::new();
        bus.subscribe(router);
        let event = EventEnvelope::new(
            "EV1",
            "object.transferred",
            EventSource::new("SIM-1", SourceKind::Simulation).unwrap(),
            100,
            101,
            "BOX-1".to_owned(),
        )
        .unwrap();

        bus.publish(&event).unwrap();

        assert_eq!(operational.borrow().as_slice(), ["EV1"]);
        assert_eq!(temporal.borrow().as_slice(), ["EV1"]);
        assert!(relational.borrow().is_empty());
    }
}
