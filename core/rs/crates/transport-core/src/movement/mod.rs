use std::error::Error;
use std::fmt::{self, Display};

use crate::domain::{
    ConveyorId, CurrentLocation, EventId, MovementEvent, MovementEventKind, MovementState,
    ObjectId, ObjectState, TimestampMs,
};
use crate::repository::{MovementRepository, RepositoryError};
use crate::topology::ConveyorNetwork;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MovementError {
    ObjectNotFound(ObjectId),
    ConveyorNotFound(ConveyorId),
    ObjectAlreadyInside(ObjectId),
    ObjectOutsideNetwork(ObjectId),
    InvalidConnection {
        origin_id: ConveyorId,
        destination_id: ConveyorId,
    },
    InvalidState(&'static str),
    Repository(RepositoryError),
}

impl Display for MovementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectNotFound(id) => write!(formatter, "no existe el objeto {id}"),
            Self::ConveyorNotFound(id) => write!(formatter, "no existe el transportador {id}"),
            Self::ObjectAlreadyInside(id) => {
                write!(formatter, "el objeto {id} ya está dentro de la red")
            }
            Self::ObjectOutsideNetwork(id) => {
                write!(formatter, "el objeto {id} está fuera de la red")
            }
            Self::InvalidConnection {
                origin_id,
                destination_id,
            } => {
                write!(
                    formatter,
                    "no existe una conexión activa {origin_id} -> {destination_id}"
                )
            }
            Self::InvalidState(message) => message.fmt(formatter),
            Self::Repository(error) => error.fmt(formatter),
        }
    }
}

impl Error for MovementError {}

impl From<RepositoryError> for MovementError {
    fn from(value: RepositoryError) -> Self {
        Self::Repository(value)
    }
}

pub struct MovementService<'a, R: MovementRepository> {
    network: &'a ConveyorNetwork,
    repository: &'a mut R,
}

impl<'a, R: MovementRepository> MovementService<'a, R> {
    pub fn new(network: &'a ConveyorNetwork, repository: &'a mut R) -> Self {
        Self {
            network,
            repository,
        }
    }

    pub fn enter(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        conveyor_id: ConveyorId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        self.require_object(&object_id)?;
        self.require_conveyor(&conveyor_id)?;
        if self.repository.current_location(&object_id).is_some() {
            return Err(MovementError::ObjectAlreadyInside(object_id));
        }

        let location = CurrentLocation {
            object_id: object_id.clone(),
            conveyor_id: conveyor_id.clone(),
            entered_at_ms: occurred_at_ms,
            updated_at_ms: occurred_at_ms,
            state: MovementState::Moving,
        };
        self.commit(
            event_id,
            object_id,
            MovementEventKind::Entered { conveyor_id },
            occurred_at_ms,
            recorded_at_ms,
            Some(location),
            ObjectState::Moving,
        )
    }

    pub fn transfer(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        destination_id: ConveyorId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        self.require_object(&object_id)?;
        self.require_conveyor(&destination_id)?;
        let current = self
            .repository
            .current_location(&object_id)
            .cloned()
            .ok_or_else(|| MovementError::ObjectOutsideNetwork(object_id.clone()))?;

        if !self
            .network
            .has_active_connection(&current.conveyor_id, &destination_id)
        {
            return Err(MovementError::InvalidConnection {
                origin_id: current.conveyor_id,
                destination_id,
            });
        }

        let origin_id = current.conveyor_id;
        let location = CurrentLocation {
            object_id: object_id.clone(),
            conveyor_id: destination_id.clone(),
            entered_at_ms: occurred_at_ms,
            updated_at_ms: occurred_at_ms,
            state: MovementState::Moving,
        };
        self.commit(
            event_id,
            object_id,
            MovementEventKind::Transferred {
                origin_id,
                destination_id,
            },
            occurred_at_ms,
            recorded_at_ms,
            Some(location),
            ObjectState::Moving,
        )
    }

    pub fn stop(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        let mut location = self.location(&object_id)?;
        if location.state == MovementState::Stopped {
            return Err(MovementError::InvalidState("el objeto ya está detenido"));
        }
        location.state = MovementState::Stopped;
        location.updated_at_ms = occurred_at_ms;
        self.commit(
            event_id,
            object_id,
            MovementEventKind::Stopped {
                conveyor_id: location.conveyor_id.clone(),
            },
            occurred_at_ms,
            recorded_at_ms,
            Some(location),
            ObjectState::Stopped,
        )
    }

    pub fn resume(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        let mut location = self.location(&object_id)?;
        if location.state != MovementState::Stopped {
            return Err(MovementError::InvalidState("el objeto no está detenido"));
        }
        location.state = MovementState::Moving;
        location.updated_at_ms = occurred_at_ms;
        self.commit(
            event_id,
            object_id,
            MovementEventKind::Resumed {
                conveyor_id: location.conveyor_id.clone(),
            },
            occurred_at_ms,
            recorded_at_ms,
            Some(location),
            ObjectState::Moving,
        )
    }

    pub fn change_direction(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        let mut location = self.location(&object_id)?;
        location.updated_at_ms = occurred_at_ms;
        let object_state = self
            .repository
            .object(&object_id)
            .map(|object| object.state)
            .ok_or_else(|| MovementError::ObjectNotFound(object_id.clone()))?;
        self.commit(
            event_id,
            object_id,
            MovementEventKind::DirectionChanged {
                conveyor_id: location.conveyor_id.clone(),
            },
            occurred_at_ms,
            recorded_at_ms,
            Some(location),
            object_state,
        )
    }

    pub fn exit(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        let location = self.location(&object_id)?;
        self.commit(
            event_id,
            object_id,
            MovementEventKind::Exited {
                conveyor_id: location.conveyor_id,
            },
            occurred_at_ms,
            recorded_at_ms,
            None,
            ObjectState::Completed,
        )
    }

    pub fn disappear(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
    ) -> Result<(), MovementError> {
        let location = self.location(&object_id)?;
        self.commit(
            event_id,
            object_id,
            MovementEventKind::Disappeared {
                last_conveyor_id: location.conveyor_id,
            },
            occurred_at_ms,
            recorded_at_ms,
            None,
            ObjectState::Missing,
        )
    }

    fn require_object(&self, object_id: &ObjectId) -> Result<(), MovementError> {
        self.repository
            .object(object_id)
            .map(|_| ())
            .ok_or_else(|| MovementError::ObjectNotFound(object_id.clone()))
    }

    fn require_conveyor(&self, conveyor_id: &ConveyorId) -> Result<(), MovementError> {
        self.network
            .conveyor(conveyor_id)
            .map(|_| ())
            .ok_or_else(|| MovementError::ConveyorNotFound(conveyor_id.clone()))
    }

    fn location(&self, object_id: &ObjectId) -> Result<CurrentLocation, MovementError> {
        self.require_object(object_id)?;
        self.repository
            .current_location(object_id)
            .cloned()
            .ok_or_else(|| MovementError::ObjectOutsideNetwork(object_id.clone()))
    }

    #[allow(clippy::too_many_arguments)]
    fn commit(
        &mut self,
        event_id: EventId,
        object_id: ObjectId,
        kind: MovementEventKind,
        occurred_at_ms: TimestampMs,
        recorded_at_ms: TimestampMs,
        location: Option<CurrentLocation>,
        object_state: ObjectState,
    ) -> Result<(), MovementError> {
        let sequence = self.repository.next_sequence(&object_id)?;
        self.repository.commit(
            MovementEvent {
                id: event_id,
                sequence,
                object_id,
                kind,
                occurred_at_ms,
                recorded_at_ms,
            },
            location,
            object_state,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::*;
    use crate::repository::{InMemoryMovementRepository, MovementRepository};
    use crate::topology::ConveyorNetwork;

    use super::{MovementError, MovementService};

    fn id<T>(value: &str) -> T
    where
        T: TryFrom<&'static str>,
        <T as TryFrom<&'static str>>::Error: std::fmt::Debug,
    {
        // Los literales de las pruebas tienen vida estática.
        let leaked: &'static str = Box::leak(value.to_owned().into_boxed_str());
        T::try_from(leaked).unwrap()
    }

    fn network() -> ConveyorNetwork {
        let plant_id: PlantId = id("PLANT-1");
        let dimensions = PhysicalDimensions::new(1.0, 0.8).unwrap();
        let mut network = ConveyorNetwork::new();
        for code in ["TR001", "TR002", "TR003", "TR004"] {
            network
                .add_conveyor(Conveyor::new(
                    id(code),
                    plant_id.clone(),
                    code,
                    code,
                    dimensions,
                ))
                .unwrap();
        }
        for (connection, origin, destination) in [
            ("CN1", "TR001", "TR002"),
            ("CN2", "TR002", "TR003"),
            ("CN3", "TR002", "TR004"),
        ] {
            network
                .add_connection(ConveyorConnection::new(
                    id(connection),
                    plant_id.clone(),
                    id(origin),
                    id(destination),
                ))
                .unwrap();
        }
        network
    }

    #[test]
    fn records_a_complete_route_and_current_location() {
        let network = network();
        let object_id: ObjectId = id("BOX-001");
        let mut repository = InMemoryMovementRepository::new();
        repository
            .register_object(TransportObject::new(
                object_id.clone(),
                "BOX-001",
                ObjectType::Box,
            ))
            .unwrap();

        {
            let mut service = MovementService::new(&network, &mut repository);
            service
                .enter(id("EV1"), object_id.clone(), id("TR001"), 100, 101)
                .unwrap();
            service
                .transfer(id("EV2"), object_id.clone(), id("TR002"), 200, 201)
                .unwrap();
            service
                .transfer(id("EV3"), object_id.clone(), id("TR004"), 300, 302)
                .unwrap();
        }

        assert_eq!(repository.events_for(&object_id).len(), 3);
        assert_eq!(repository.events_for(&object_id)[2].sequence, 3);
        assert_eq!(
            repository.current_location(&object_id).unwrap().conveyor_id,
            id("TR004")
        );
    }

    #[test]
    fn rejects_a_transfer_without_a_physical_connection() {
        let network = network();
        let object_id: ObjectId = id("BOX-001");
        let mut repository = InMemoryMovementRepository::new();
        repository
            .register_object(TransportObject::new(
                object_id.clone(),
                "BOX-001",
                ObjectType::Box,
            ))
            .unwrap();

        let mut service = MovementService::new(&network, &mut repository);
        service
            .enter(id("EV1"), object_id.clone(), id("TR001"), 100, 100)
            .unwrap();
        let error = service
            .transfer(id("EV2"), object_id, id("TR004"), 200, 200)
            .unwrap_err();

        assert!(matches!(error, MovementError::InvalidConnection { .. }));
    }

    #[test]
    fn stop_and_resume_preserve_the_single_location() {
        let network = network();
        let object_id: ObjectId = id("BOX-001");
        let mut repository = InMemoryMovementRepository::new();
        repository
            .register_object(TransportObject::new(
                object_id.clone(),
                "BOX-001",
                ObjectType::Box,
            ))
            .unwrap();

        {
            let mut service = MovementService::new(&network, &mut repository);
            service
                .enter(id("EV1"), object_id.clone(), id("TR001"), 100, 100)
                .unwrap();
            service
                .stop(id("EV2"), object_id.clone(), 150, 151)
                .unwrap();
            service
                .resume(id("EV3"), object_id.clone(), 180, 181)
                .unwrap();
        }

        let location = repository.current_location(&object_id).unwrap();
        assert_eq!(location.conveyor_id, id("TR001"));
        assert_eq!(location.state, MovementState::Moving);
        assert_eq!(repository.events_for(&object_id).len(), 3);
    }
}
