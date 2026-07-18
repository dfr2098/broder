use super::{ConveyorId, PlantId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConveyorState {
    Active,
    Inactive,
    Maintenance,
    OutOfService,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalDimensions {
    pub length_m: f32,
    pub width_m: f32,
}

impl PhysicalDimensions {
    pub fn new(length_m: f32, width_m: f32) -> Result<Self, &'static str> {
        if !length_m.is_finite() || !width_m.is_finite() {
            return Err("las dimensiones deben ser números finitos");
        }
        if length_m <= 0.0 || width_m <= 0.0 {
            return Err("las dimensiones deben ser mayores que cero");
        }
        Ok(Self { length_m, width_m })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Conveyor {
    pub id: ConveyorId,
    pub plant_id: PlantId,
    pub code: String,
    pub name: String,
    pub dimensions: PhysicalDimensions,
    pub nominal_speed_m_s: Option<f32>,
    pub state: ConveyorState,
}

impl Conveyor {
    pub fn new(
        id: ConveyorId,
        plant_id: PlantId,
        code: impl Into<String>,
        name: impl Into<String>,
        dimensions: PhysicalDimensions,
    ) -> Self {
        Self {
            id,
            plant_id,
            code: code.into(),
            name: name.into(),
            dimensions,
            nominal_speed_m_s: None,
            state: ConveyorState::Active,
        }
    }
}
