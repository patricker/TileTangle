use crate::CellId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

type RawAdjEntry = (u32, Vec<(u32, String)>);
type RawAdjOpt = Option<Vec<RawAdjEntry>>;
type CellAdj = Option<HashMap<CellId, Vec<(CellId, String)>>>;

pub fn serialize<S>(value: &CellAdj, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let opt: RawAdjOpt = value.as_ref().map(|map| {
        let mut entries: Vec<RawAdjEntry> = map
            .iter()
            .map(|(cid, vec)| {
                let inner = vec.iter().map(|(other, dir)| (other.0, dir.clone())).collect();
                (cid.0, inner)
            })
            .collect();
        entries.sort_by_key(|(id, _)| *id);
        entries
    });
    opt.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<CellAdj, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = RawAdjOpt::deserialize(deserializer)?;
    Ok(opt.map(|entries| {
        entries
            .into_iter()
            .map(|(id, vec)| {
                (
                    CellId(id),
                    vec.into_iter().map(|(other, dir)| (CellId(other), dir)).collect(),
                )
            })
            .collect()
    }))
}

