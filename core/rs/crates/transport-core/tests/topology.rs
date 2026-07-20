use transport_core::{
    ConnectionId, Conveyor, ConveyorConnection, ConveyorId, ConveyorNetwork, PhysicalDimensions,
    PlantId, TopologyError,
};

fn plant_id() -> PlantId {
    PlantId::new("PLANT-1").unwrap()
}

fn conveyor(code: &str) -> Conveyor {
    Conveyor::new(
        ConveyorId::new(code).unwrap(),
        plant_id(),
        code,
        code,
        PhysicalDimensions::new(1.0, 0.8).unwrap(),
    )
}

#[test]
fn finds_a_route_through_a_branch() {
    let mut network = ConveyorNetwork::new();
    for code in ["TR001", "TR002", "TR003", "TR004"] {
        network.add_conveyor(conveyor(code)).unwrap();
    }
    for (id, origin, destination) in [
        ("CN1", "TR001", "TR002"),
        ("CN2", "TR002", "TR003"),
        ("CN3", "TR002", "TR004"),
    ] {
        network
            .add_connection(ConveyorConnection::new(
                ConnectionId::new(id).unwrap(),
                plant_id(),
                ConveyorId::new(origin).unwrap(),
                ConveyorId::new(destination).unwrap(),
            ))
            .unwrap();
    }

    let route = network
        .find_route(
            &ConveyorId::new("TR001").unwrap(),
            &ConveyorId::new("TR004").unwrap(),
        )
        .unwrap();
    let codes: Vec<_> = route.iter().map(ConveyorId::as_str).collect();
    assert_eq!(codes, ["TR001", "TR002", "TR004"]);
}

#[test]
fn ignores_inactive_connections_when_routing() {
    let mut network = ConveyorNetwork::new();
    for code in ["TR001", "TR002"] {
        network.add_conveyor(conveyor(code)).unwrap();
    }
    let mut connection = ConveyorConnection::new(
        ConnectionId::new("CN1").unwrap(),
        plant_id(),
        ConveyorId::new("TR001").unwrap(),
        ConveyorId::new("TR002").unwrap(),
    );
    connection.active = false;
    network.add_connection(connection).unwrap();

    let route = network.find_route(
        &ConveyorId::new("TR001").unwrap(),
        &ConveyorId::new("TR002").unwrap(),
    );
    assert!(route.is_none());
}

#[test]
fn rejects_a_connection_to_itself() {
    let mut network = ConveyorNetwork::new();
    network.add_conveyor(conveyor("TR001")).unwrap();
    let error = network
        .add_connection(ConveyorConnection::new(
            ConnectionId::new("CN1").unwrap(),
            plant_id(),
            ConveyorId::new("TR001").unwrap(),
            ConveyorId::new("TR001").unwrap(),
        ))
        .unwrap_err();
    assert!(matches!(error, TopologyError::SelfConnection(_)));
}
