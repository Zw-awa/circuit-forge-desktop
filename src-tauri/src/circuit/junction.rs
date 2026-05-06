use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Junction {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub net_id: u32,
}
