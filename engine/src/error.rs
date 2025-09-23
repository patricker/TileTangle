use thiserror::Error;

use crate::CellId;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid coordinates")]
    InvalidCoordinates,
    #[error("invalid cell id")]
    InvalidCell,
    #[error("collision at cell {0:?}")]
    Collision(CellId),
    #[error("rack capacity exceeded")]
    RackCapacity,
    #[error("bag is empty")]
    BagEmpty,
    #[error("config error: {0}")]
    Config(&'static str),
    #[error("serialization error: {0}")]
    Serialization(String),
}

