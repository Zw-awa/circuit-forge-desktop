use std::collections::{HashMap, VecDeque};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use crate::circuit::types::{
    ComponentKind, NetId, Signal, ComponentId, SimMode, SignalType, AttenuationModel,
};
use crate::circuit::graph::CircuitGraph;
use crate::circuit::subcircuit::{SubCircuitDefRegistry, SubCircuitDef, ExternalPin};
use crate::circuit::component::Component;
use crate::rules::presets::RulePackRegistry;
use crate::scripting::lua_engine::LuaComponentDefRegistry;
use crate::verification::truth_table::TruthTable;
use crate::skin::types::SkinManifest;
use crate::debugging::breakpoint::{BreakpointManager, BreakpointHitInfo};
use crate::plugins::PluginManager;
use super::evaluator::evaluate_gate;
use super::tick_engine::TickEngine;

const MAX_HISTORY_LENGTH: usize = 8192;

#[derive(Clone, Serialize)]
pub struct StepResult {
    pub changed: HashMap<NetId, Signal>,
    pub breakpoint_hit: Option<BreakpointHitInfo>,
    pub events_remaining: usize,
}

#[derive(Clone)]
pub struct SignalHistory {
    data: VecDeque<(u64, Signal)>,
    net_id: NetId,
}

impl SignalHistory {
    pub fn new(net_id: NetId) -> Self {
        Self {
            data: VecDeque::new(),
            net_id,
        }
    }
    pub fn record(&mut self, tick: u64, signal: Signal) {
        if self.data.len() >= MAX_HISTORY_LENGTH {
            self.data.pop_front();
        }
        self.data.push_back((tick, signal));
    }
    pub fn get_data(&self) -> &VecDeque<(u64, Signal)> {
        &self.data
    }
}

pub struct SimEvent {
    pub net_id: NetId,
    pub new_signal: Signal,
}

struct DelayedEvent {
    net_id: NetId,
    new_signal: Signal,
    remaining_ticks: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimStatus {
    Stopped,
    Running,
    Paused,
}

pub struct SimulationEngine {
    pub graph: CircuitGraph,
    pub signals: HashMap<NetId, Signal>,
    pub event_queue: VecDeque<SimEvent>,
    pub status: SimStatus,
    pub tick_count: u64,
    pub sim_mode: SimMode,
    pub tick_rate: u32,
    pub speed_multiplier: f32,
    pub signal_history: HashMap<NetId, SignalHistory>,
    pub tick_engine: TickEngine,
    pub rule_registry: RulePackRegistry,
    pub subcircuit_registry: SubCircuitDefRegistry,
    pub lua_registry: LuaComponentDefRegistry,
    pub truth_tables: HashMap<u32, TruthTable>,
    pub active_skin: Option<SkinManifest>,
    pub skin_assets: HashMap<String, Vec<u8>>,
    pub snapshots: Vec<(u32, String, String, String)>,
    pub next_snapshot_id: u32,
    pub breakpoint_manager: BreakpointManager,
    pub plugin_manager: PluginManager,
    delayed_events: Vec<DelayedEvent>,
    next_truth_table_id: u32,
}

const MAX_EVENTS_PER_STEP: usize = 10_000;

impl SimulationEngine {
    pub fn new() -> Self {
        Self {
            graph: CircuitGraph::new(),
            signals: HashMap::new(),
            event_queue: VecDeque::new(),
            status: SimStatus::Stopped,
            tick_count: 0,
            sim_mode: SimMode::EventDriven,
            tick_rate: 10,
            speed_multiplier: 1.0,
            signal_history: HashMap::new(),
            tick_engine: TickEngine::new(),
            rule_registry: RulePackRegistry::new(),
            subcircuit_registry: SubCircuitDefRegistry::new(),
            lua_registry: LuaComponentDefRegistry::new(),
            truth_tables: HashMap::new(),
            active_skin: None,
            skin_assets: HashMap::new(),
            snapshots: Vec::new(),
            next_snapshot_id: 1,
            breakpoint_manager: BreakpointManager::new(),
            plugin_manager: PluginManager::new(),
            delayed_events: Vec::new(),
            next_truth_table_id: 1,
        }
    }

    pub fn alloc_truth_table_id(&mut self) -> u32 {
        let id = self.next_truth_table_id;
        self.next_truth_table_id += 1;
        id
    }

    pub fn propagate(&mut self) -> (HashMap<NetId, Signal>, Option<BreakpointHitInfo>) {
        self.process_delayed_events();
        let mut changed: HashMap<NetId, Signal> = HashMap::new();
        let mut iterations = 0;

        while let Some(event) = self.event_queue.pop_front() {
            if iterations >= MAX_EVENTS_PER_STEP {
                break;
            }
            iterations += 1;

            let old = self
                .signals
                .get(&event.net_id)
                .copied()
                .unwrap_or(Signal::Low);
            let signal = self.apply_attenuation(event.new_signal);
            if old == signal {
                continue;
            }

            self.signals.insert(event.net_id, signal);
            changed.insert(event.net_id, signal);

            self.record_signal_history(event.net_id, signal);

            if let Some(mut hit) = self.breakpoint_manager.check(event.net_id, old, signal) {
                self.status = SimStatus::Paused;
                hit.tick = self.tick_count;
                return (changed, Some(hit));
            }

            if let Some(pin_ids) = self.graph.nets.get(&event.net_id) {
                let pin_ids: Vec<_> = pin_ids.clone();
                for pin_id in pin_ids {
                    if let Some(pin) = self.graph.pins.get(&pin_id) {
                        if !pin.is_output {
                            self.evaluate_component(pin.owner, 0);
                        }
                    }
                }
            }
        }

        self.tick_count += 1;
        (changed, None)
    }

    /// Identify connected component groups using Union-Find.
    /// Returns independent subgraphs that can be evaluated in parallel.
    pub fn identify_independent_subgraphs(&self) -> Vec<Vec<ComponentId>> {
        let mut parent: HashMap<ComponentId, ComponentId> = HashMap::new();
        for comp_id in self.graph.components.keys() {
            parent.insert(*comp_id, *comp_id);
        }

        fn find(
            parent: &mut HashMap<ComponentId, ComponentId>,
            x: ComponentId,
        ) -> ComponentId {
            let px = *parent.get(&x).unwrap_or(&x);
            if px != x {
                let root = find(parent, px);
                parent.insert(x, root);
                root
            } else {
                x
            }
        }

        fn union(
            parent: &mut HashMap<ComponentId, ComponentId>,
            x: ComponentId,
            y: ComponentId,
        ) {
            let rx = find(parent, x);
            let ry = find(parent, y);
            if rx != ry {
                parent.insert(rx, ry);
            }
        }

        for pin_ids in self.graph.nets.values() {
            let owners: Vec<ComponentId> = pin_ids
                .iter()
                .filter_map(|pid| self.graph.pins.get(pid).map(|p| p.owner))
                .collect();
            for i in 1..owners.len() {
                union(&mut parent, owners[0], owners[i]);
            }
        }

        let mut groups: HashMap<ComponentId, Vec<ComponentId>> = HashMap::new();
        for comp_id in self.graph.components.keys() {
            let root = find(&mut parent, *comp_id);
            groups.entry(root).or_default().push(*comp_id);
        }
        groups.into_values().collect()
    }

    /// Pure-function gate evaluation for a subgraph (no &mut self, safe for rayon).
    /// Only handles basic logic gates. Complex components are skipped.
    pub(crate) fn propagate_subgraph_pure(
        graph: &crate::circuit::graph::CircuitGraph,
        signal_snapshot: &HashMap<NetId, Signal>,
        comp_ids: &[ComponentId],
    ) -> HashMap<NetId, Signal> {
        let mut local_signals: HashMap<NetId, Signal> = HashMap::new();
        for comp_id in comp_ids {
            if let Some(comp) = graph.components.get(comp_id) {
                match comp.kind {
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
                                signal_snapshot.get(&nid).copied().unwrap_or(Signal::Low)
                            })
                            .collect();
                        let output = evaluate_gate(
                            comp.kind.clone(),
                            &inputs,
                            &SignalType::Bit,
                        );
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    local_signals.insert(net_id, output);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        local_signals
    }

    pub fn propagate_parallel(
        &mut self,
    ) -> (HashMap<NetId, Signal>, Option<BreakpointHitInfo>) {
        self.process_delayed_events();

        while let Some(event) = self.event_queue.pop_front() {
            let old = self.signals.get(&event.net_id).copied().unwrap_or(Signal::Low);
            let signal = self.apply_attenuation(event.new_signal);
            if old != signal {
                self.signals.insert(event.net_id, signal);
            }
        }

        let subgraphs = self.identify_independent_subgraphs();
        let signal_snapshot = self.signals.clone();
        let graph_ref = &self.graph;

        // Phase 2: parallel evaluation of each subgraph
        let results: Vec<HashMap<NetId, Signal>> = subgraphs
            .par_iter()
            .map(|comp_ids| {
                Self::propagate_subgraph_pure(graph_ref, &signal_snapshot, comp_ids)
            })
            .collect();

        // Phase 3: merge results (single-threaded)
        let mut all_changed = HashMap::new();
        for r in results {
            for (net_id, new_signal) in r {
                let old = signal_snapshot
                    .get(&net_id)
                    .copied()
                    .unwrap_or(Signal::Low);
                if old != new_signal {
                    self.signals.insert(net_id, new_signal);
                    self.record_signal_history(net_id, new_signal);
                    all_changed.insert(net_id, new_signal);
                }
            }
        }

        // Breakpoint check (single-threaded, after merge)
        let bp_hit = if !all_changed.is_empty() {
            let mut hit = None;
            for (&net_id, &new_signal) in &all_changed {
                let old = signal_snapshot
                    .get(&net_id)
                    .copied()
                    .unwrap_or(Signal::Low);
                if let Some(h) = self.breakpoint_manager.check(net_id, old, new_signal) {
                    self.status = SimStatus::Paused;
                    hit = Some(h);
                    break;
                }
            }
            hit
        } else {
            None
        };

        self.tick_count += 1;
        (all_changed, bp_hit)
    }

    pub fn step_single_event(&mut self) -> Result<StepResult, String> {
        if let Some(event) = self.event_queue.pop_front() {
            let old = self.signals.get(&event.net_id).copied().unwrap_or(Signal::Low);
            let signal = self.apply_attenuation(event.new_signal);
            if old != signal {
                self.signals.insert(event.net_id, signal);
                self.record_signal_history(event.net_id, signal);
                if let Some(pin_ids) = self.graph.nets.get(&event.net_id).cloned() {
                    for pin_id in pin_ids {
                        if let Some(pin) = self.graph.pins.get(&pin_id) {
                            if !pin.is_output {
                                self.evaluate_component(pin.owner, 0);
                            }
                        }
                    }
                }
                let mut bp_hit = self.breakpoint_manager.check(event.net_id, old, signal);
                if let Some(ref mut hit) = bp_hit {
                    hit.tick = self.tick_count;
                    self.status = SimStatus::Paused;
                }
                let mut changed = HashMap::new();
                changed.insert(event.net_id, signal);
                return Ok(StepResult {
                    changed,
                    breakpoint_hit: bp_hit,
                    events_remaining: self.event_queue.len(),
                });
            }
        }
        Ok(StepResult {
            changed: HashMap::new(),
            breakpoint_hit: None,
            events_remaining: 0,
        })
    }

    pub fn debug_continue(&mut self) {
        self.status = SimStatus::Running;
    }

    fn evaluate_component(&mut self, comp_id: ComponentId, depth: u32) {
        let comp = match self.graph.components.get(&comp_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let kind_for_gate = comp.kind.clone();
        match comp.kind {
            ComponentKind::Switch
            | ComponentKind::Led
            | ComponentKind::Button
            | ComponentKind::Clock
            | ComponentKind::Random
            | ComponentKind::Constant
            | ComponentKind::SevenSegment
            | ComponentKind::Oscilloscope
            | ComponentKind::DelayLine => return,
            ComponentKind::Splitter => {
                let input_signal = comp.input_pins.iter()
                    .filter_map(|pid| self.graph.pins.get(pid))
                    .filter_map(|p| p.net)
                    .filter_map(|n| self.signals.get(&n).copied())
                    .next()
                    .unwrap_or(Signal::Low);
                let value = match input_signal {
                    Signal::Bus(v) => v,
                    Signal::High => 1,
                    Signal::Low => 0,
                    Signal::Integer(v) => v as u8,
                    Signal::Float(v) => v.round() as u8,
                };
                let delay = self.rule_registry.active().gate_delay;
                for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                    let bit_signal = if (value >> i) & 1 == 1 { Signal::High } else { Signal::Low };
                    if let Some(pin) = self.graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            let current = self.signals.get(&net_id).copied().unwrap_or(Signal::Low);
                            if current != bit_signal {
                                self.queue_gate_event(net_id, bit_signal, delay);
                            }
                        }
                    }
                }
                return;
            }
            ComponentKind::Merger => {
                let mut value: u8 = 0;
                for (i, in_pin_id) in comp.input_pins.iter().enumerate() {
                    let sig = self.graph.pins.get(in_pin_id)
                        .and_then(|p| p.net)
                    .and_then(|n| self.signals.get(&n).copied())
                        .unwrap_or(Signal::Low);
                    if sig == Signal::High {
                        value |= 1 << i;
                    }
                }
                let output = Signal::Bus(value);
                let delay = self.rule_registry.active().gate_delay;
                for out_pin_id in &comp.output_pins {
                    if let Some(pin) = self.graph.pins.get(out_pin_id) {
                        if let Some(net_id) = pin.net {
                            let current = self.signals.get(&net_id).copied().unwrap_or(Signal::Low);
                            if current != output {
                                self.queue_gate_event(net_id, output, delay);
                            }
                        }
                    }
                }
                return;
            }
            ComponentKind::SubCircuit(def_id) => {
                if depth > 16 {
                    eprintln!("Warning: max subcircuit recursion depth exceeded for component {}", comp_id);
                    for out_pin_id in &comp.output_pins {
                        if let Some(pin) = self.graph.pins.get(out_pin_id) {
                            if let Some(net_id) = pin.net {
                                self.queue_gate_event(net_id, Signal::Low, 0);
                            }
                        }
                    }
                    return;
                }
                if let Some(def) = self.subcircuit_registry.get(def_id) {
                    let def = def.clone();
                    Self::evaluate_subcircuit(
                        &self.graph, &self.subcircuit_registry, &self.lua_registry,
                        &self.rule_registry, &comp, &def, depth,
                        &mut self.signals, &mut self.event_queue,
                    );
                }
                return;
            }
            ComponentKind::LuaScript(def_id) => {
                if let Some(def) = self.lua_registry.get(def_id) {
                    let def = def.clone();
                    let inputs: Vec<Signal> = comp.input_pins.iter()
                        .filter_map(|pid| self.graph.pins.get(pid))
                        .filter_map(|p| p.net)
                        .filter_map(|nid| self.signals.get(&nid).copied())
                        .collect();
                    let state = comp.lua_state.clone().unwrap_or(serde_json::json!({}));
                    match crate::scripting::sandbox::LuaSandbox::new() {
                        Ok(sandbox) => {
                            match sandbox.evaluate(&def.script_source, &inputs, &state, false) {
                                Ok((outputs, new_state)) => {
                                    if let Some(comp_mut) = self.graph.components.get_mut(&comp.id) {
                                        comp_mut.lua_state = Some(new_state);
                                    }
                                    for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                                        let out_signal = if i < outputs.len() { outputs[i] } else { Signal::Low };
                                        if let Some(pin) = self.graph.pins.get(out_pin_id) {
                                            if let Some(net_id) = pin.net {
                                                self.queue_gate_event(net_id, out_signal, 0);
                                            }
                                        }
                                    }
                                }
                                Err(e) => { eprintln!("Lua eval error: {}", e); }
                            }
                        }
                        Err(e) => { eprintln!("Lua sandbox error: {}", e); }
                    }
                }
                return;
            }
            ComponentKind::Plugin(ref plugin_id, ref kind_name) => {
                let inputs: Vec<Signal> = comp.input_pins.iter()
                    .filter_map(|pid| self.graph.pins.get(pid))
                    .filter_map(|p| p.net)
                    .filter_map(|nid| self.signals.get(&nid).copied())
                    .collect();
                let state = comp.lua_state.clone().unwrap_or(serde_json::json!({}));
                match self.plugin_manager.evaluate(plugin_id, kind_name, &inputs, &state) {
                    Ok((outputs, new_state)) => {
                        if let Some(comp_mut) = self.graph.components.get_mut(&comp.id) {
                            comp_mut.lua_state = Some(new_state);
                        }
                        for (i, out_pin_id) in comp.output_pins.iter().enumerate() {
                            let out_signal = if i < outputs.len() { outputs[i] } else { Signal::Low };
                            if let Some(pin) = self.graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    self.queue_gate_event(net_id, out_signal, 0);
                                }
                            }
                        }
                    }
                    Err(e) => { eprintln!("Plugin eval error: {}", e); }
                }
                return;
            }
            _ => {}
        }
        let delay = self.rule_registry.active().gate_delay;
        let inputs: Vec<Signal> = comp
            .input_pins
            .iter()
            .map(|pid| {
                self.graph
                    .pins
                    .get(pid)
                    .and_then(|p| p.net)
                    .and_then(|n| self.signals.get(&n).copied())
                    .unwrap_or(Signal::Low)
            })
            .collect();
        let output = evaluate_gate(kind_for_gate, &inputs, &self.rule_registry.active().signal_type);
        for out_pin_id in &comp.output_pins {
            if let Some(pin) = self.graph.pins.get(out_pin_id) {
                if let Some(net_id) = pin.net {
                    let current = self
                        .signals
                        .get(&net_id)
                        .copied()
                        .unwrap_or(Signal::Low);
                    if current != output {
                        self.queue_gate_event(net_id, output, delay);
                    }
                }
            }
        }
    }

    fn evaluate_subcircuit(
        graph: &CircuitGraph,
        subcircuit_registry: &SubCircuitDefRegistry,
        lua_registry: &LuaComponentDefRegistry,
        rule_registry: &RulePackRegistry,
        comp: &Component,
        def: &SubCircuitDef,
        depth: u32,
        signal_map: &mut HashMap<NetId, Signal>,
        event_queue: &mut VecDeque<SimEvent>,
    ) {
        let mut inner_signals: HashMap<NetId, Signal> = HashMap::new();

        // Propagate outer input signals → inner input nets (index-based matching)
        let input_ext_pins: Vec<&ExternalPin> = def.external_pins.iter()
            .filter(|ep| !ep.is_output)
            .collect();
        for (i, ext_pin) in input_ext_pins.iter().enumerate() {
            if let Some(outer_pin_id) = comp.input_pins.get(i) {
                let outer_signal = graph.pins.get(outer_pin_id)
                    .and_then(|p| p.net)
                    .and_then(|n| signal_map.get(&n).copied())
                    .unwrap_or(Signal::Low);
                let inner_pin = def.inner_graph.pins.get(&ext_pin.internal_pin_id);
                if let Some(ip) = inner_pin {
                    if let Some(inner_net) = ip.net {
                        inner_signals.insert(inner_net, outer_signal);
                    }
                }
            }
        }

        // Evaluate all inner components with fix-point iteration (max 100 rounds)
        for _iteration in 0..100 {
            let before = inner_signals.clone();
            let inner_comp_ids: Vec<ComponentId> = def.inner_graph.components.keys().copied().collect();
            let mut sorted_ids = inner_comp_ids;
            sorted_ids.sort();
            for inner_comp_id in sorted_ids {
            if let Some(inner_comp) = def.inner_graph.components.get(&inner_comp_id) {
                match inner_comp.kind.clone() {
                    ComponentKind::And | ComponentKind::Or | ComponentKind::Not
                    | ComponentKind::Nand | ComponentKind::Xor => {
                        let inputs: Vec<Signal> = inner_comp.input_pins.iter()
                            .filter_map(|pid| def.inner_graph.pins.get(pid))
                            .filter_map(|p| p.net)
                            .filter_map(|n| inner_signals.get(&n).copied())
                            .collect();
                        if inputs.len() == inner_comp.input_pins.len() {
                            let output = evaluate_gate(inner_comp.kind.clone(), &inputs, &rule_registry.active().signal_type);
                            for out_pin_id in &inner_comp.output_pins {
                                if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                    if let Some(net_id) = pin.net {
                                        inner_signals.insert(net_id, output);
                                    }
                                }
                            }
                        }
                    }
                    ComponentKind::Switch | ComponentKind::Button => {
                        let state = inner_comp.toggle_state.unwrap_or(Signal::Low);
                        for out_pin_id in &inner_comp.output_pins {
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, state);
                                }
                            }
                        }
                    }
                    ComponentKind::Constant => {
                        let val = inner_comp.constant_value.unwrap_or(Signal::Low);
                        for out_pin_id in &inner_comp.output_pins {
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, val);
                                }
                            }
                        }
                    }
                    ComponentKind::Clock => {
                        let period = inner_comp.clock_period.unwrap_or(2);
                        let counter = inner_comp.clock_counter.unwrap_or(0);
                        let high_ticks = (period as f32 * inner_comp.clock_duty.unwrap_or(0.5)) as u32;
                        let out = if counter < high_ticks { Signal::High } else { Signal::Low };
                        for out_pin_id in &inner_comp.output_pins {
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, out);
                                }
                            }
                        }
                    }
                    ComponentKind::Random => {
                        let prob = inner_comp.random_probability.unwrap_or(0.5);
                        let val = ((inner_comp.id as u64) % 100) as f32 / 100.0;
                        let out = if val < prob { Signal::High } else { Signal::Low };
                        for out_pin_id in &inner_comp.output_pins {
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, out);
                                }
                            }
                        }
                    }
                    ComponentKind::DelayLine => {
                        let input = inner_comp.input_pins.iter()
                            .filter_map(|pid| def.inner_graph.pins.get(pid))
                            .filter_map(|p| p.net)
                            .filter_map(|n| inner_signals.get(&n).copied())
                            .next()
                            .unwrap_or(Signal::Low);
                        for out_pin_id in &inner_comp.output_pins {
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, input);
                                }
                            }
                        }
                    }
                    ComponentKind::Splitter => {
                        let input = inner_comp.input_pins.iter()
                            .filter_map(|pid| def.inner_graph.pins.get(pid))
                            .filter_map(|p| p.net)
                            .filter_map(|n| inner_signals.get(&n).copied())
                            .next()
                            .unwrap_or(Signal::Low);
                        let value = match input {
                            Signal::Bus(v) => v,
                            Signal::High => 1,
                            Signal::Low => 0,
                            Signal::Integer(v) => v as u8,
                            Signal::Float(v) => v.round() as u8,
                        };
                        for (i, out_pin_id) in inner_comp.output_pins.iter().enumerate() {
                            let bit = if (value >> i) & 1 == 1 { Signal::High } else { Signal::Low };
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, bit);
                                }
                            }
                        }
                    }
                    ComponentKind::Merger => {
                        let mut value: u8 = 0;
                        for (i, in_pin_id) in inner_comp.input_pins.iter().enumerate() {
                            let sig = def.inner_graph.pins.get(in_pin_id)
                                .and_then(|p| p.net)
                                .and_then(|n| inner_signals.get(&n).copied())
                                .unwrap_or(Signal::Low);
                            if sig == Signal::High {
                                value |= 1 << i;
                            }
                        }
                        for out_pin_id in &inner_comp.output_pins {
                            if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    inner_signals.insert(net_id, Signal::Bus(value));
                                }
                            }
                        }
                    }
                    ComponentKind::SubCircuit(inner_def_id) => {
                        if depth < 16 {
                            if let Some(inner_def) = subcircuit_registry.get(inner_def_id) {
                                let inner_def = inner_def.clone();
                                Self::evaluate_subcircuit(
                                    graph, subcircuit_registry, lua_registry, rule_registry,
                                    inner_comp, &inner_def, depth + 1, &mut inner_signals,
                                    event_queue,
                                );
                            }
                        }
                    }
                    ComponentKind::LuaScript(inner_def_id) => {
                        if let Some(lua_def) = lua_registry.get(inner_def_id) {
                            let lua_def = lua_def.clone();
                            let lua_inputs: Vec<Signal> = inner_comp.input_pins.iter()
                                .filter_map(|pid| def.inner_graph.pins.get(pid))
                                .filter_map(|p| p.net)
                                .filter_map(|n| inner_signals.get(&n).copied())
                                .collect();
                            let state = inner_comp.lua_state.clone().unwrap_or(serde_json::json!({}));
                            match crate::scripting::sandbox::LuaSandbox::new() {
                                Ok(sandbox) => {
                                    match sandbox.evaluate(&lua_def.script_source, &lua_inputs, &state, false) {
                                        Ok((outputs, _new_state)) => {
                                            for (i, out_pin_id) in inner_comp.output_pins.iter().enumerate() {
                                                if i < outputs.len() {
                                                    if let Some(pin) = def.inner_graph.pins.get(out_pin_id) {
                                                        if let Some(net_id) = pin.net {
                                                            inner_signals.insert(net_id, outputs[i]);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => { eprintln!("Lua eval error in subcircuit: {}", e); }
                                    }
                                }
                                Err(e) => { eprintln!("Lua sandbox error in subcircuit: {}", e); }
                            }
                        }
                    }
                    ComponentKind::Led | ComponentKind::SevenSegment | ComponentKind::Oscilloscope
                    | ComponentKind::Plugin(_, _) => {
                        // Output-only components — no action needed
                    }
                }
            }
            if inner_signals == before { break; }
            }
        }

        // Propagate inner output signals → outer output nets (index-based matching)
        let output_ext_pins: Vec<&ExternalPin> = def.external_pins.iter()
            .filter(|ep| ep.is_output)
            .collect();
        for (i, ext_pin) in output_ext_pins.iter().enumerate() {
            let inner_signal = def.inner_graph.pins.get(&ext_pin.internal_pin_id)
                .and_then(|p| p.net)
                .and_then(|n| inner_signals.get(&n).copied())
                .unwrap_or(Signal::Low);

            if let Some(outer_pin_id) = comp.output_pins.get(i) {
                if let Some(pin) = graph.pins.get(outer_pin_id) {
                    if let Some(net_id) = pin.net {
                        let current = signal_map.get(&net_id).copied().unwrap_or(Signal::Low);
                        if current != inner_signal {
                            signal_map.insert(net_id, inner_signal);
                            event_queue.push_back(SimEvent { net_id, new_signal: inner_signal });
                        }
                    }
                }
            }
        }
    }

    fn queue_gate_event(&mut self, net_id: NetId, new_signal: Signal, delay: u32) {
        if delay > 0 {
            self.delayed_events.push(DelayedEvent {
                net_id,
                new_signal,
                remaining_ticks: delay,
            });
        } else {
            self.event_queue.push_back(SimEvent { net_id, new_signal });
        }
    }

    fn apply_attenuation(&self, signal: Signal) -> Signal {
        let rule = self.rule_registry.active();
        match (&rule.attenuation, signal) {
            (AttenuationModel::Linear { loss_per_unit }, Signal::Integer(v)) => {
                let min_val = match rule.signal_type {
                    SignalType::Integer { min, .. } => min,
                    _ => 0,
                };
                Signal::Integer((v - loss_per_unit).max(min_val))
            }
            (AttenuationModel::Linear { loss_per_unit }, Signal::Float(v)) => {
                let min_val = match rule.signal_type {
                    SignalType::Float { min, .. } => min,
                    _ => 0.0,
                };
                Signal::Float((v - *loss_per_unit as f64).max(min_val))
            }
            _ => signal,
        }
    }

    fn process_delayed_events(&mut self) {
        let mut ready = Vec::new();
        for ev in self.delayed_events.iter_mut() {
            ev.remaining_ticks -= 1;
            if ev.remaining_ticks == 0 {
                ready.push(SimEvent {
                    net_id: ev.net_id,
                    new_signal: ev.new_signal,
                });
            }
        }
        self.delayed_events.retain(|ev| ev.remaining_ticks > 0);
        for ev in ready {
            self.event_queue.push_back(ev);
        }
    }

    pub fn toggle_switch(
        &mut self,
        comp_id: ComponentId,
    ) -> Result<HashMap<NetId, Signal>, String> {
        let comp = self
            .graph
            .components
            .get_mut(&comp_id)
            .ok_or("component not found")?;
        if comp.kind != ComponentKind::Switch {
            return Err("not a switch".into());
        }
        let new_state = match comp.toggle_state {
            Some(Signal::Low) => Signal::High,
            _ => Signal::Low,
        };
        comp.toggle_state = Some(new_state);
        let out_pins = comp.output_pins.clone();
        for out_pin_id in &out_pins {
            if let Some(pin) = self.graph.pins.get(out_pin_id) {
                if let Some(net_id) = pin.net {
                    self.event_queue.push_back(SimEvent {
                        net_id,
                        new_signal: new_state,
                    });
                }
            }
        }
        let (changed, _) = self.propagate();
        Ok(changed)
    }

    pub fn step(&mut self) -> (HashMap<NetId, Signal>, Option<BreakpointHitInfo>) {
        match self.sim_mode {
            SimMode::EventDriven => {
                self.advance_clocks();
                self.advance_randoms();
                self.advance_delay_lines();
                if self.graph.components.len() > 200 {
                    self.propagate_parallel()
                } else {
                    self.propagate()
                }
            }
            SimMode::TickDriven => {
                self.tick_driven_step()
            }
        }
    }

    pub fn reset(&mut self) {
        self.signals.clear();
        self.event_queue.clear();
        self.tick_count = 0;
        self.status = SimStatus::Stopped;
        self.tick_engine.reset();
        for comp in self.graph.components.values_mut() {
            if comp.kind == ComponentKind::Switch {
                comp.toggle_state = Some(Signal::Low);
            }
            if comp.kind == ComponentKind::Constant {
                comp.constant_value = Some(Signal::Low);
            }
        }
    }

    pub fn get_signals(&self) -> &HashMap<NetId, Signal> {
        &self.signals
    }

    pub fn set_mode(&mut self, mode: SimMode) {
        match (self.sim_mode, mode) {
            (SimMode::EventDriven, SimMode::TickDriven) => {
                self.tick_engine.current_signals = self.signals.clone();
                self.tick_engine.next_signals = self.signals.clone();
            }
            (SimMode::TickDriven, SimMode::EventDriven) => {
                self.signals = self.tick_engine.current_signals.clone();
                self.event_queue.clear();
            }
            _ => {}
        }
        self.sim_mode = mode;
    }

    pub fn record_signal_history(&mut self, net_id: NetId, signal: Signal) {
        self.signal_history
            .entry(net_id)
            .or_insert_with(|| SignalHistory::new(net_id))
            .record(self.tick_count, signal);
    }

    pub fn get_bulk_signal_history(&self, net_ids: &[NetId], from_tick: Option<u64>, to_tick: Option<u64>) -> HashMap<NetId, Vec<(u64, Signal)>> {
        let mut result = HashMap::new();
        for &net_id in net_ids {
            if let Some(history) = self.signal_history.get(&net_id) {
                let data: Vec<(u64, Signal)> = history.data.iter()
                    .filter(|(tick, _)| {
                        if let Some(from) = from_tick { if *tick < from { return false; } }
                        if let Some(to) = to_tick { if *tick > to { return false; } }
                        true
                    })
                    .cloned()
                    .collect();
                result.insert(net_id, data);
            }
        }
        result
    }

    pub fn export_waveform_csv(&self, net_ids: &[NetId]) -> Result<String, String> {
        use std::collections::BTreeSet;
        let mut all_ticks = BTreeSet::new();
        let histories = self.get_bulk_signal_history(net_ids, None, None);
        for data in histories.values() {
            for (tick, _) in data { all_ticks.insert(*tick); }
        }
        let net_list: Vec<NetId> = net_ids.to_vec();
        let mut csv = format!("tick,{}\n", net_list.iter().map(|n| format!("net_{}", n)).collect::<Vec<_>>().join(","));
        for tick in all_ticks {
            let mut row = format!("{}", tick);
            for net_id in &net_list {
                let val = histories.get(net_id)
                    .and_then(|data| data.iter().rev().find(|(t, _)| *t <= tick))
                    .map(|(_, s)| match s {
                        Signal::Float(v) => format!("{}", v),
                        _ => s.to_integer().to_string(),
                    })
                    .unwrap_or_else(|| "0".to_string());
                row.push_str(&format!(",{}", val));
            }
            csv.push_str(&row); csv.push('\n');
        }
        Ok(csv)
    }

    pub fn advance_clocks(&mut self) {
        for (_, comp) in self.graph.components.iter_mut() {
            if comp.kind != ComponentKind::Clock {
                continue;
            }
            let period = comp.clock_period.unwrap_or(2);
            let mut counter = comp.clock_counter.unwrap_or(0);
            counter = (counter + 1) % period;
            let high_ticks = (period as f32 * comp.clock_duty.unwrap_or(0.5)) as u32;
            let new_signal = if counter < high_ticks {
                Signal::High
            } else {
                Signal::Low
            };
            comp.clock_counter = Some(counter);
            for out_pin_id in comp.output_pins.clone() {
                if let Some(pin) = self.graph.pins.get(&out_pin_id) {
                    if let Some(net_id) = pin.net {
                        self.event_queue.push_back(SimEvent {
                            net_id,
                            new_signal,
                        });
                    }
                }
            }
        }
    }

    pub fn advance_randoms(&mut self) {
        let tick = self.tick_count;
        for (_, comp) in self.graph.components.iter_mut() {
            if comp.kind != ComponentKind::Random {
                continue;
            }
            let prob = comp.random_probability.unwrap_or(0.5);
            let val = ((tick.wrapping_mul(comp.id as u64)) % 100) as f32 / 100.0;
            let signal = if val < prob {
                Signal::High
            } else {
                Signal::Low
            };
            for out_pin_id in &comp.output_pins {
                if let Some(pin) = self.graph.pins.get(out_pin_id) {
                    if let Some(net_id) = pin.net {
                        self.event_queue.push_back(SimEvent {
                            net_id,
                            new_signal: signal,
                        });
                    }
                }
            }
        }
    }

    pub fn tick_driven_step(&mut self) -> (HashMap<NetId, Signal>, Option<BreakpointHitInfo>) {
        let changed = self.tick_engine.tick(&self.graph, &self.rule_registry.active().signal_type);
        self.tick_count += 1;
        let old_signals = std::mem::replace(&mut self.signals, self.tick_engine.current_signals.clone());
        for (&net_id, &new_signal) in &changed {
            let old_signal = old_signals.get(&net_id).copied().unwrap_or(Signal::Low);
            self.record_signal_history(net_id, new_signal);
            if let Some(mut hit) = self.breakpoint_manager.check(net_id, old_signal, new_signal) {
                hit.tick = self.tick_count;
                self.status = SimStatus::Paused;
                return (changed, Some(hit));
            }
        }
        (changed, None)
    }

    pub fn advance_delay_lines(&mut self) {
        for (_, comp) in self.graph.components.iter_mut() {
            if comp.kind != ComponentKind::DelayLine {
                continue;
            }
            let delay = comp.delay_ticks.unwrap_or(1) as usize;
            let input = comp
                .input_pins
                .iter()
                .filter_map(|pid| self.graph.pins.get(pid))
                .filter_map(|p| p.net)
                .filter_map(|nid| self.signals.get(&nid).copied())
                .next()
                .unwrap_or(Signal::Low);
            let buffer = comp
                .delay_buffer
                .get_or_insert_with(|| VecDeque::from(vec![Signal::Low; delay]));
            buffer.push_back(input);
            let output = buffer.pop_front().unwrap_or(Signal::Low);
            let out_pins = comp.output_pins.clone();
            for out_pin_id in &out_pins {
                if let Some(pin) = self.graph.pins.get(out_pin_id) {
                    if let Some(net_id) = pin.net {
                        self.event_queue.push_back(SimEvent {
                            net_id,
                            new_signal: output,
                        });
                    }
                }
            }
        }
    }
}
