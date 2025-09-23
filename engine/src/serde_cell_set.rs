use crate::CellId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;

pub fn serialize<S>(value: &Option<HashSet<CellId>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let opt: Option<Vec<u32>> = value.as_ref().map(|set| {
        let mut vec: Vec<u32> = set.iter().map(|cid| cid.0).collect();
        vec.sort_unstable();
        vec
    });
    opt.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<HashSet<CellId>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Vec<u32>>::deserialize(deserializer)?;
    Ok(opt.map(|vec| vec.into_iter().map(CellId).collect()))
}

