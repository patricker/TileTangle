use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub fn engine_version() -> EngineVersion {
    EngineVersion {
        major: 0,
        minor: 1,
        patch: 0,
    }
}
