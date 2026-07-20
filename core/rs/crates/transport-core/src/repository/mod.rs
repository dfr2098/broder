mod memory;

use std::error::Error;
use std::fmt::{self, Display};

use crate::domain::{CurrentLocation, MovementEvent, ObjectId, ObjectState, TransportObject};

pub use memory::InMemoryMovementRepository;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    ObjectAlreadyExists(ObjectId),
    ObjectNotFound(ObjectId),
    InvalidSequence { expected: u64, received: u64 },
}

impl Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectAlreadyExists(id) => write!(formatter, "el objeto {id} ya existe"),
            Self::ObjectNotFound(id) => write!(formatter, "no existe el objeto {id}"),
            Self::InvalidSequence { expected, received } => write!(
                formatter,
                "secuencia inválida: se esperaba {expected} y se recibió {received}"
            ),
        }
    }
}

impl Error for RepositoryError {}

/// Puerto de persistencia. Una implementación SQL deberá ejecutar `commit`
/// dentro de una única transacción.
pub trait MovementRepository {
    fn register_object(&mut self, object: TransportObject) -> Result<(), RepositoryError>;
    fn object(&self, object_id: &ObjectId) -> Option<&TransportObject>;
    fn current_location(&self, object_id: &ObjectId) -> Option<&CurrentLocation>;
    fn events_for(&self, object_id: &ObjectId) -> &[MovementEvent];
    fn next_sequence(&self, object_id: &ObjectId) -> Result<u64, RepositoryError>;
    fn commit(
        &mut self,
        event: MovementEvent,
        location: Option<CurrentLocation>,
        object_state: ObjectState,
    ) -> Result<(), RepositoryError>;
}
