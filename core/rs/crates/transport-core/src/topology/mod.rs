use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display};

use crate::domain::{Conveyor, ConveyorConnection, ConveyorId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopologyError {
    ConveyorAlreadyExists(ConveyorId),
    ConveyorNotFound(ConveyorId),
    ConnectionAlreadyExists {
        origin_id: ConveyorId,
        destination_id: ConveyorId,
    },
    SelfConnection(ConveyorId),
    DifferentPlants,
}

impl Display for TopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConveyorAlreadyExists(id) => write!(formatter, "el transportador {id} ya existe"),
            Self::ConveyorNotFound(id) => write!(formatter, "no existe el transportador {id}"),
            Self::ConnectionAlreadyExists {
                origin_id,
                destination_id,
            } => {
                write!(
                    formatter,
                    "la conexión {origin_id} -> {destination_id} ya existe"
                )
            }
            Self::SelfConnection(id) => {
                write!(
                    formatter,
                    "el transportador {id} no puede conectarse consigo mismo"
                )
            }
            Self::DifferentPlants => write!(
                formatter,
                "los transportadores pertenecen a plantas diferentes"
            ),
        }
    }
}

impl Error for TopologyError {}

#[derive(Default)]
pub struct ConveyorNetwork {
    conveyors: HashMap<ConveyorId, Conveyor>,
    outgoing: HashMap<ConveyorId, Vec<ConveyorId>>,
    connections: Vec<ConveyorConnection>,
}

impl ConveyorNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_conveyor(&mut self, conveyor: Conveyor) -> Result<(), TopologyError> {
        if self.conveyors.contains_key(&conveyor.id) {
            return Err(TopologyError::ConveyorAlreadyExists(conveyor.id));
        }
        self.outgoing.entry(conveyor.id.clone()).or_default();
        self.conveyors.insert(conveyor.id.clone(), conveyor);
        Ok(())
    }

    pub fn add_connection(&mut self, connection: ConveyorConnection) -> Result<(), TopologyError> {
        if connection.origin_id == connection.destination_id {
            return Err(TopologyError::SelfConnection(connection.origin_id));
        }

        let origin = self
            .conveyors
            .get(&connection.origin_id)
            .ok_or_else(|| TopologyError::ConveyorNotFound(connection.origin_id.clone()))?;
        let destination = self
            .conveyors
            .get(&connection.destination_id)
            .ok_or_else(|| TopologyError::ConveyorNotFound(connection.destination_id.clone()))?;

        if origin.plant_id != destination.plant_id || origin.plant_id != connection.plant_id {
            return Err(TopologyError::DifferentPlants);
        }

        let destinations = self
            .outgoing
            .entry(connection.origin_id.clone())
            .or_default();
        if destinations.contains(&connection.destination_id) {
            return Err(TopologyError::ConnectionAlreadyExists {
                origin_id: connection.origin_id,
                destination_id: connection.destination_id,
            });
        }

        destinations.push(connection.destination_id.clone());
        self.connections.push(connection);
        Ok(())
    }

    pub fn conveyor(&self, id: &ConveyorId) -> Option<&Conveyor> {
        self.conveyors.get(id)
    }

    pub fn destinations(&self, origin_id: &ConveyorId) -> &[ConveyorId] {
        self.outgoing
            .get(origin_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Destinos alcanzables solo a través de conexiones activas. El cálculo de
    /// rutas debe coincidir con lo que `has_active_connection` permite mover.
    fn active_destinations(&self, origin_id: &ConveyorId) -> Vec<ConveyorId> {
        self.connections
            .iter()
            .filter(|connection| connection.active && &connection.origin_id == origin_id)
            .map(|connection| connection.destination_id.clone())
            .collect()
    }

    pub fn has_active_connection(
        &self,
        origin_id: &ConveyorId,
        destination_id: &ConveyorId,
    ) -> bool {
        self.connections.iter().any(|connection| {
            connection.active
                && &connection.origin_id == origin_id
                && &connection.destination_id == destination_id
        })
    }

    pub fn find_route(
        &self,
        origin_id: &ConveyorId,
        destination_id: &ConveyorId,
    ) -> Option<Vec<ConveyorId>> {
        if !self.conveyors.contains_key(origin_id) || !self.conveyors.contains_key(destination_id) {
            return None;
        }

        let mut pending = VecDeque::from([origin_id.clone()]);
        let mut visited = HashSet::from([origin_id.clone()]);
        let mut previous: HashMap<ConveyorId, ConveyorId> = HashMap::new();

        while let Some(current) = pending.pop_front() {
            if &current == destination_id {
                let mut route = vec![current.clone()];
                let mut cursor = current;
                while let Some(parent) = previous.get(&cursor) {
                    route.push(parent.clone());
                    cursor = parent.clone();
                }
                route.reverse();
                return Some(route);
            }

            for next in self.active_destinations(&current) {
                if visited.insert(next.clone()) {
                    previous.insert(next.clone(), current.clone());
                    pending.push_back(next);
                }
            }
        }
        None
    }
}
