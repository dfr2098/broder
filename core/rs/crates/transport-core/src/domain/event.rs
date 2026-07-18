use super::{ConveyorId, EventId, ObjectId};

/// Milisegundos desde una época acordada por el adaptador de infraestructura.
pub type TimestampMs = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MovementState {
    Moving,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentLocation {
    pub object_id: ObjectId,
    pub conveyor_id: ConveyorId,
    pub entered_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub state: MovementState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MovementEventKind {
    Entered {
        conveyor_id: ConveyorId,
    },
    Transferred {
        origin_id: ConveyorId,
        destination_id: ConveyorId,
    },
    Stopped {
        conveyor_id: ConveyorId,
    },
    Resumed {
        conveyor_id: ConveyorId,
    },
    DirectionChanged {
        conveyor_id: ConveyorId,
    },
    Exited {
        conveyor_id: ConveyorId,
    },
    Disappeared {
        last_conveyor_id: ConveyorId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementEvent {
    pub id: EventId,
    pub sequence: u64,
    pub object_id: ObjectId,
    pub kind: MovementEventKind,
    pub occurred_at_ms: TimestampMs,
    pub recorded_at_ms: TimestampMs,
}
