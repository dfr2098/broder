use std::error::Error;

use transport_core::{
    ConnectionId, Conveyor, ConveyorConnection, ConveyorId, ConveyorNetwork, EventId,
    InMemoryMovementRepository, MovementRepository, MovementService, ObjectId, ObjectType,
    PhysicalDimensions, PlantId, TransportObject,
};

fn main() -> Result<(), Box<dyn Error>> {
    let plant_id = PlantId::new("PLANT-1")?;
    let dimensions = PhysicalDimensions::new(1.0, 0.8)?;
    let mut network = ConveyorNetwork::new();

    for code in ["TR001", "TR002", "TR003", "TR004"] {
        network.add_conveyor(Conveyor::new(
            ConveyorId::new(code)?,
            plant_id.clone(),
            code,
            code,
            dimensions,
        ))?;
    }

    for (id, origin, destination) in [
        ("CN1", "TR001", "TR002"),
        ("CN2", "TR002", "TR003"),
        ("CN3", "TR002", "TR004"),
    ] {
        network.add_connection(ConveyorConnection::new(
            ConnectionId::new(id)?,
            plant_id.clone(),
            ConveyorId::new(origin)?,
            ConveyorId::new(destination)?,
        ))?;
    }

    let object_id = ObjectId::new("BOX-001")?;
    let mut repository = InMemoryMovementRepository::new();
    repository.register_object(TransportObject::new(
        object_id.clone(),
        "BOX-001",
        ObjectType::Box,
    ))?;

    {
        let mut movement = MovementService::new(&network, &mut repository);
        movement.enter(
            EventId::new("EV1")?,
            object_id.clone(),
            ConveyorId::new("TR001")?,
            1_000,
            1_001,
        )?;
        movement.transfer(
            EventId::new("EV2")?,
            object_id.clone(),
            ConveyorId::new("TR002")?,
            2_000,
            2_002,
        )?;
        movement.transfer(
            EventId::new("EV3")?,
            object_id.clone(),
            ConveyorId::new("TR004")?,
            3_000,
            3_003,
        )?;
    }

    println!("Ruta registrada para {object_id}:");
    for event in repository.events_for(&object_id) {
        println!("  #{} {:?}", event.sequence, event.kind);
    }
    if let Some(location) = repository.current_location(&object_id) {
        println!("Ubicación actual: {}", location.conveyor_id);
    }

    Ok(())
}
