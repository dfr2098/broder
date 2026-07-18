use super::{ConnectionId, ConveyorId, PlantId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConveyorConnection {
    pub id: ConnectionId,
    pub plant_id: PlantId,
    pub origin_id: ConveyorId,
    pub destination_id: ConveyorId,
    pub active: bool,
}

impl ConveyorConnection {
    pub fn new(
        id: ConnectionId,
        plant_id: PlantId,
        origin_id: ConveyorId,
        destination_id: ConveyorId,
    ) -> Self {
        Self {
            id,
            plant_id,
            origin_id,
            destination_id,
            active: true,
        }
    }
}
