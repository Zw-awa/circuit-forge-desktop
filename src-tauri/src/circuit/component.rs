use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use super::types::{ComponentId, ComponentKind, PinId, Signal};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    // Phase 1 fields
    pub id: ComponentId,
    pub kind: ComponentKind,
    pub x: f32,
    pub y: f32,
    pub input_pins: Vec<PinId>,
    pub output_pins: Vec<PinId>,
    pub toggle_state: Option<Signal>,

    // Phase 2 fields
    pub press_state: Option<bool>,
    pub clock_period: Option<u32>,
    pub clock_duty: Option<f32>,
    pub clock_counter: Option<u32>,
    pub random_probability: Option<f32>,
    pub constant_value: Option<Signal>,
    pub oscilloscope_channels: Option<u32>,
    pub oscilloscope_time_window: Option<u32>,
    pub delay_ticks: Option<u32>,
    #[serde(skip)]
    pub delay_buffer: Option<VecDeque<Signal>>,
    pub bus_width: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lua_state: Option<serde_json::Value>,
}
