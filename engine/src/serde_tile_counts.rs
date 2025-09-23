use crate::TileKind;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

pub fn serialize<S>(value: &HashMap<TileKind, u32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let vec: Vec<(TileKind, u32)> = value.iter().map(|(k, v)| (k.clone(), *v)).collect();
    vec.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<TileKind, u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Vec::<(TileKind, u32)>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

