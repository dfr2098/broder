use super::ObjectId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectType {
    Box,
    Pallet,
    Container,
    Other(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectState {
    OutsideNetwork,
    Moving,
    Stopped,
    Completed,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportObject {
    pub id: ObjectId,
    pub code: String,
    pub object_type: ObjectType,
    pub state: ObjectState,
}

impl TransportObject {
    pub fn new(id: ObjectId, code: impl Into<String>, object_type: ObjectType) -> Self {
        Self {
            id,
            code: code.into(),
            object_type,
            state: ObjectState::OutsideNetwork,
        }
    }
}
