use serde::{Deserialize, Serialize};
use super::types::{ComponentId, NetId, PinId};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub id: PinId,
    pub owner: ComponentId,
    pub is_output: bool,
    pub net: Option<NetId>,
    pub offset_x: f32,
    pub offset_y: f32,
}
