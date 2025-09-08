//! TileTangle Engine (skeleton)
//!
//! Phase 0: scaffolding with a minimal lib and smoke test.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_smoke() {
        let v = engine_version();
        assert_eq!(v.major, 0);
    }
}
