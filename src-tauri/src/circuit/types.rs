use serde::{Deserialize, Serialize};

pub type ComponentId = u32;
pub type PinId = u32;
pub type WireId = u32;
pub type NetId = u32;
pub type SubCircuitDefId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Signal {
    Low,
    High,
    Bus(u8),
    Integer(i32),
    Float(f64),
}

impl Signal {
    pub fn is_high(&self) -> bool {
        matches!(self, Signal::High | Signal::Bus(_))
    }
    pub fn is_low(&self) -> bool {
        matches!(self, Signal::Low)
    }
    pub fn to_bool(&self) -> bool {
        match self {
            Signal::High => true,
            Signal::Low => false,
            Signal::Bus(v) => *v != 0,
            Signal::Integer(v) => *v != 0,
            Signal::Float(v) => *v != 0.0,
        }
    }
    pub fn to_integer(&self) -> i32 {
        match self {
            Signal::High => 1,
            Signal::Low => 0,
            Signal::Bus(v) => *v as i32,
            Signal::Integer(v) => *v,
            Signal::Float(v) => v.round() as i32,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimMode {
    EventDriven,
    TickDriven,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentKind {
    // Phase 1 existing
    And,
    Or,
    Not,
    Nand,
    Xor,
    Switch,
    Led,
    // Phase 2 new
    Button,
    Clock,
    Random,
    Constant,
    SevenSegment,
    Oscilloscope,
    DelayLine,
    Splitter,
    Merger,
    SubCircuit(SubCircuitDefId),
    LuaScript(u32),
    Plugin(String, String),
}

impl ComponentKind {
    #[allow(dead_code)]
    pub fn pin_counts(&self) -> (usize, usize) {
        match self {
            ComponentKind::And => (2, 1),
            ComponentKind::Or => (2, 1),
            ComponentKind::Not => (1, 1),
            ComponentKind::Nand => (2, 1),
            ComponentKind::Xor => (2, 1),
            ComponentKind::Switch => (0, 1),
            ComponentKind::Led => (1, 0),
            ComponentKind::Button => (0, 1),
            ComponentKind::Clock => (0, 1),
            ComponentKind::Random => (0, 1),
            ComponentKind::Constant => (0, 1),
            ComponentKind::SevenSegment => (4, 0),
            ComponentKind::Oscilloscope => (1, 0),
            ComponentKind::DelayLine => (1, 1),
            ComponentKind::Splitter => (1, 4),
            ComponentKind::Merger => (4, 1),
            ComponentKind::SubCircuit(_) => (0, 0),
            ComponentKind::LuaScript(_) => (0, 0),
            ComponentKind::Plugin(_, _) => (0, 0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SignalType {
    Bit,
    Bus { width: u8 },
    Integer { min: i32, max: i32 },
    Float { min: f64, max: f64 },
}

impl SignalType {
    pub fn default_signal(&self) -> Signal {
        match self {
            SignalType::Bit => Signal::Low,
            SignalType::Bus { .. } => Signal::Bus(0),
            SignalType::Integer { min, .. } => Signal::Integer(*min),
            SignalType::Float { min, .. } => Signal::Float(*min),
        }
    }
    pub fn clamp(&self, signal: Signal) -> Signal {
        match (self, signal) {
            (SignalType::Integer { min, max }, Signal::Integer(v)) => {
                Signal::Integer(v.clamp(*min, *max))
            }
            (SignalType::Float { min, max }, Signal::Float(v)) => {
                Signal::Float(v.clamp(*min, *max))
            }
            _ => signal,
        }
    }
    pub fn range(&self) -> (i32, i32) {
        match self {
            SignalType::Integer { min, max } => (*min, *max),
            _ => (0, 15),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropagationMode {
    EventDriven,
    TickDriven,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AttenuationModel {
    None,
    Linear { loss_per_unit: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TickBehavior {
    Synchronous,
    Asynchronous,
}
