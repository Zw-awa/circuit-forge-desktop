use crate::circuit::types::{ComponentKind, Signal, SignalType};

pub fn evaluate_gate(kind: ComponentKind, inputs: &[Signal], signal_type: &SignalType) -> Signal {
    if inputs.iter().any(|s| matches!(s, Signal::Integer(_))) {
        return evaluate_gate_integer(kind, inputs, signal_type);
    }
    match kind {
        ComponentKind::And => {
            if inputs.iter().all(|s| *s == Signal::High) {
                Signal::High
            } else {
                Signal::Low
            }
        }
        ComponentKind::Or => {
            if inputs.iter().any(|s| *s == Signal::High) {
                Signal::High
            } else {
                Signal::Low
            }
        }
        ComponentKind::Not => match inputs.first() {
            Some(Signal::High) => Signal::Low,
            _ => Signal::High,
        },
        ComponentKind::Nand => {
            if inputs.iter().all(|s| *s == Signal::High) {
                Signal::Low
            } else {
                Signal::High
            }
        }
        ComponentKind::Xor => {
            let high_count = inputs.iter().filter(|s| **s == Signal::High).count();
            if high_count % 2 == 1 {
                Signal::High
            } else {
                Signal::Low
            }
        }
        ComponentKind::Switch => Signal::Low,
        ComponentKind::Led => Signal::Low,
        ComponentKind::Button => Signal::Low,
        ComponentKind::Clock => Signal::Low,
        ComponentKind::Random => Signal::Low,
        ComponentKind::Constant => Signal::Low,
        ComponentKind::SevenSegment => Signal::Low,
        ComponentKind::Oscilloscope => Signal::Low,
        ComponentKind::DelayLine => Signal::Low,
        ComponentKind::Splitter => Signal::Low,
        ComponentKind::Merger => Signal::Low,
        ComponentKind::SubCircuit(_) => Signal::Low,
        ComponentKind::LuaScript(_) => Signal::Low,
        ComponentKind::Plugin(_, _) => Signal::Low,
    }
}

fn evaluate_gate_integer(kind: ComponentKind, inputs: &[Signal], signal_type: &SignalType) -> Signal {
    let (min_val, max_val) = signal_type.range();
    let vals: Vec<i32> = inputs.iter().map(|s| s.to_integer()).collect();
    match kind {
        ComponentKind::And => Signal::Integer(*vals.iter().min().unwrap_or(&min_val)),
        ComponentKind::Or => Signal::Integer(*vals.iter().max().unwrap_or(&max_val)),
        ComponentKind::Not => {
            let val = vals[0].clamp(min_val, max_val);
            Signal::Integer(max_val - val)
        }
        ComponentKind::Nand => Signal::Integer(max_val - *vals.iter().min().unwrap_or(&min_val)),
        ComponentKind::Xor => {
            Signal::Integer(vals.iter().fold(0, |a, b| a ^ b))
        }
        _ => Signal::Low,
    }
}
