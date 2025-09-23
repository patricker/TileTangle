use crate::{Bonus, CellId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

pub fn serialize<S>(value: &HashMap<CellId, Bonus>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let vec: Vec<(u32, &Bonus)> = value.iter().map(|(cid, bonus)| (cid.0, bonus)).collect();
    vec.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<CellId, Bonus>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Vec::<(u32, Bonus)>::deserialize(deserializer)?;
    Ok(vec.into_iter().map(|(id, bonus)| (CellId(id), bonus)).collect())
}

