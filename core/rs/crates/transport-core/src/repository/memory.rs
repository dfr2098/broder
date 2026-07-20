use std::collections::HashMap;

use crate::domain::{CurrentLocation, MovementEvent, ObjectId, ObjectState, TransportObject};

use super::{MovementRepository, RepositoryError};

#[derive(Default)]
pub struct InMemoryMovementRepository {
    objects: HashMap<ObjectId, TransportObject>,
    locations: HashMap<ObjectId, CurrentLocation>,
    events: HashMap<ObjectId, Vec<MovementEvent>>,
}

impl InMemoryMovementRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MovementRepository for InMemoryMovementRepository {
    fn register_object(&mut self, object: TransportObject) -> Result<(), RepositoryError> {
        if self.objects.contains_key(&object.id) {
            return Err(RepositoryError::ObjectAlreadyExists(object.id));
        }
        self.events.insert(object.id.clone(), Vec::new());
        self.objects.insert(object.id.clone(), object);
        Ok(())
    }

    fn object(&self, object_id: &ObjectId) -> Option<&TransportObject> {
        self.objects.get(object_id)
    }

    fn current_location(&self, object_id: &ObjectId) -> Option<&CurrentLocation> {
        self.locations.get(object_id)
    }

    fn events_for(&self, object_id: &ObjectId) -> &[MovementEvent] {
        self.events.get(object_id).map(Vec::as_slice).unwrap_or(&[])
    }

    fn next_sequence(&self, object_id: &ObjectId) -> Result<u64, RepositoryError> {
        if !self.objects.contains_key(object_id) {
            return Err(RepositoryError::ObjectNotFound(object_id.clone()));
        }
        Ok(self.events_for(object_id).len() as u64 + 1)
    }

    fn commit(
        &mut self,
        event: MovementEvent,
        location: Option<CurrentLocation>,
        object_state: ObjectState,
    ) -> Result<(), RepositoryError> {
        let expected = self.next_sequence(&event.object_id)?;
        if event.sequence != expected {
            return Err(RepositoryError::InvalidSequence {
                expected,
                received: event.sequence,
            });
        }

        let object = self
            .objects
            .get_mut(&event.object_id)
            .ok_or_else(|| RepositoryError::ObjectNotFound(event.object_id.clone()))?;
        object.state = object_state;

        if let Some(location) = location {
            self.locations.insert(event.object_id.clone(), location);
        } else {
            self.locations.remove(&event.object_id);
        }

        self.events
            .entry(event.object_id.clone())
            .or_default()
            .push(event);
        Ok(())
    }
}
