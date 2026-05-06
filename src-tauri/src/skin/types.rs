use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinManifest {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub component_textures: HashMap<String, String>,
    pub wire_style: WireStyle,
    pub grid_style: GridStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireStyle {
    pub thickness: f32,
    pub high_color: [f32; 3],
    pub low_color: [f32; 3],
    pub dash_length: Option<f32>,
    pub gap_length: Option<f32>,
    pub glow_intensity: Option<f32>,
}

impl Default for WireStyle {
    fn default() -> Self {
        Self {
            thickness: 2.0,
            high_color: [0.0, 1.0, 0.0],
            low_color: [0.2, 0.2, 0.2],
            dash_length: None,
            gap_length: None,
            glow_intensity: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridStyle {
    pub bg_color: [f32; 4],
    pub minor_color: [f32; 4],
    pub major_color: [f32; 4],
    pub axis_color: [f32; 4],
    pub bg_texture: Option<String>,
    pub pattern: GridPattern,
    pub opacity: f32,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum GridPattern {
    Line,
    Dot,
    Cross,
}
