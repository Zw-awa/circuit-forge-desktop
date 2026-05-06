use std::collections::{HashMap, VecDeque};
use crate::circuit::types::{ComponentId, NetId, Signal, ComponentKind, SignalType};
use crate::circuit::component::Component;
use crate::circuit::graph::CircuitGraph;
use super::evaluator::evaluate_gate;

pub struct TickEngine {
    pub current_signals: HashMap<NetId, Signal>,
    pub next_signals: HashMap<NetId, Signal>,
    pub tick_count: u64,
    delay_buffers: HashMap<ComponentId, VecDeque<Signal>>,
}

impl TickEngine {
    pub fn new() -> Self {
        Self {
            current_signals: HashMap::new(),
            next_signals: HashMap::new(),
            tick_count: 0,
            delay_buffers: HashMap::new(),
        }
    }

    pub fn tick(&mut self, graph: &CircuitGraph, signal_type: &SignalType) -> HashMap<NetId, Signal> {
        self.tick_count += 1;
        self.next_signals = self.current_signals.clone();
        let mut comp_ids: Vec<ComponentId> = graph.components.keys().copied().collect();
        comp_ids.sort();
        for comp_id in comp_ids {
            if let Some(comp) = graph.components.get(&comp_id) {
                self.evaluate_component_tick(comp, graph, signal_type);
            }
        }
        let mut changed = HashMap::new();
        for (net_id, new_sig) in &self.next_signals {
            let old_sig = self
                .current_signals
                .get(net_id)
                .copied()
                .unwrap_or(Signal::Low);
            if old_sig != *new_sig {
                changed.insert(*net_id, *new_sig);
            }
        }
        std::mem::swap(&mut self.current_signals, &mut self.next_signals);
        changed
    }

    fn evaluate_component_tick(
        &mut self,
        comp: &Component,
        graph: &CircuitGraph,
        signal_type: &SignalType,
    ) {
        match comp.kind.clone() {
            ComponentKind::And
            | ComponentKind::Or
            | ComponentKind::Not
            | ComponentKind::Nand
            | ComponentKind::Xor => {
                let inputs: Vec<Signal> = comp
                    .input_pins
                    .iter()
                    .filter_map(|pid| graph.pins.get(pid))
                    .filter_map(|p| p.net)
                    .map(|nid| {
                        self.current_signals
                            .get(&nid)
                            .copied()
                            .unwrap_or(Signal::Low)
                    })
                    .collect();
                let output = evaluate_gate(comp.kind.clone(), &inputs, signal_type);
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, output);
                        }
                    }
                }
            }
            ComponentKind::Switch => {
                let signal = comp.toggle_state.unwrap_or(Signal::Low);
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, signal);
                        }
                    }
                }
            }
            ComponentKind::Button => {
                let signal = if comp.press_state.unwrap_or(false) {
                    Signal::High
                } else {
                    Signal::Low
                };
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, signal);
                        }
                    }
                }
            }
            ComponentKind::Constant => {
                let signal = comp.constant_value.unwrap_or(Signal::High);
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, signal);
                        }
                    }
                }
            }
            ComponentKind::Clock => {
                let period = comp.clock_period.unwrap_or(2);
                let duty = comp.clock_duty.unwrap_or(0.5);
                let counter = comp.clock_counter.unwrap_or(0);
                let high_ticks = (period as f32 * duty) as u32;
                let signal = if counter < high_ticks {
                    Signal::High
                } else {
                    Signal::Low
                };
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, signal);
                        }
                    }
                }
            }
            ComponentKind::Random => {
                let prob = comp.random_probability.unwrap_or(0.5);
                let val =
                    ((self.tick_count.wrapping_mul(comp.id as u64)) % 100) as f32 / 100.0;
                let signal = if val < prob {
                    Signal::High
                } else {
                    Signal::Low
                };
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, signal);
                        }
                    }
                }
            }
            ComponentKind::DelayLine => {
                let delay = comp.delay_ticks.unwrap_or(1) as usize;
                let input = comp
                    .input_pins
                    .iter()
                    .filter_map(|pid| graph.pins.get(pid))
                    .filter_map(|p| p.net)
                    .map(|nid| {
                        self.current_signals
                            .get(&nid)
                            .copied()
                            .unwrap_or(Signal::Low)
                    })
                    .next()
                    .unwrap_or(Signal::Low);
                let buffer = self
                    .delay_buffers
                    .entry(comp.id)
                    .or_insert_with(|| VecDeque::from(vec![Signal::Low; delay]));
                buffer.push_back(input);
                let output = buffer.pop_front().unwrap_or(Signal::Low);
                let out_pins = comp.output_pins.clone();
                for out_pin_id in &out_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, output);
                        }
                    }
                }
            }
            ComponentKind::Splitter => {
                let input_signal = comp.input_pins.iter()
                    .filter_map(|pid| graph.pins.get(pid))
                    .filter_map(|p| p.net)
                    .filter_map(|n| self.current_signals.get(&n).copied())
                    .next()
                    .unwrap_or(Signal::Low);
                let value = match input_signal {
                    Signal::Bus(v) => v,
                    Signal::High => 1,
                    Signal::Low => 0,
                    Signal::Integer(v) => v as u8,
                    Signal::Float(v) => v.round() as u8,
                };
                for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                    let bit_signal = if (value >> i) & 1 == 1 { Signal::High } else { Signal::Low };
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, bit_signal);
                        }
                    }
                }
            }
            ComponentKind::Merger => {
                let mut value: u8 = 0;
                for (i, in_pin_id) in comp.input_pins.iter().enumerate() {
                    let sig = graph.pins.get(in_pin_id)
                        .and_then(|p| p.net)
                        .and_then(|n| self.current_signals.get(&n).copied())
                        .unwrap_or(Signal::Low);
                    if sig == Signal::High {
                        value |= 1 << i;
                    }
                }
                let output = Signal::Bus(value);
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            self.next_signals.insert(net_id, output);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn get_signals(&self) -> &HashMap<NetId, Signal> {
        &self.current_signals
    }

    pub fn reset(&mut self) {
        self.current_signals.clear();
        self.next_signals.clear();
        self.tick_count = 0;
        self.delay_buffers.clear();
    }
}
