use serde::{Deserialize, Serialize};
use super::types::{NetId, PinId, WireId};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WireEndpoint {
    Pin(PinId),
    Junction(u32),
}

impl WireEndpoint {
    pub fn as_pin(&self) -> Option<PinId> {
        match self {
            WireEndpoint::Pin(id) => Some(*id),
            WireEndpoint::Junction(_) => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wire {
    pub id: WireId,
    pub start: WireEndpoint,
    pub end: WireEndpoint,
    pub net_id: NetId,
    pub color: Option<u32>,
}
