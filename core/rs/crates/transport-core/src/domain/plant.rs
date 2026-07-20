use super::PlantId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlantState {
    Active,
    Inactive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plant {
    pub id: PlantId,
    pub code: String,
    pub name: String,
    pub state: PlantState,
}

impl Plant {
    pub fn new(id: PlantId, code: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id,
            code: code.into(),
            name: name.into(),
            state: PlantState::Active,
        }
    }
}
