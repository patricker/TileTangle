use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub fn engine_version() -> EngineVersion {
    let parts: Vec<u32> = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    EngineVersion {
        major: parts.first().copied().unwrap_or(0),
        minor: parts.get(1).copied().unwrap_or(0),
        patch: parts.get(2).copied().unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_toml() {
        let v = engine_version();
        let expected = env!("CARGO_PKG_VERSION");
        let actual = format!("{}.{}.{}", v.major, v.minor, v.patch);
        assert_eq!(actual, expected);
    }
}
