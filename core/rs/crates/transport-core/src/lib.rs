//! Núcleo del dominio físico de transportadores.
//!
//! Este crate no conoce PLC, cámaras, WMS, interfaces gráficas ni reglas de
//! enrutamiento. Solo representa la topología y registra movimientos.

pub mod domain;
pub mod movement;
pub mod repository;
pub mod topology;

pub use domain::*;
pub use movement::{MovementError, MovementService};
pub use repository::{InMemoryMovementRepository, MovementRepository, RepositoryError};
pub use topology::{ConveyorNetwork, TopologyError};
