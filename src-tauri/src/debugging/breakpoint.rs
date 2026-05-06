use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::circuit::types::{NetId, ComponentId, Signal};

pub type BreakpointId = u32;

#[derive(Clone, Serialize, Deserialize)]
pub enum BreakpointTarget {
    Net(NetId),
    Component(ComponentId),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum BreakpointCondition {
    SignalEquals(Signal),
    SignalChanges,
    RisingEdge,
    FallingEdge,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: BreakpointId,
    pub target: BreakpointTarget,
    pub condition: BreakpointCondition,
    pub enabled: bool,
    #[serde(default)]
    pub hit_count: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BreakpointHitInfo {
    pub breakpoint_id: BreakpointId,
    pub net_id: NetId,
    pub old_signal: Signal,
    pub new_signal: Signal,
    pub tick: u64,
}

pub struct BreakpointManager {
    pub breakpoints: HashMap<BreakpointId, Breakpoint>,
    net_index: HashMap<NetId, Vec<BreakpointId>>,
    next_id: BreakpointId,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            net_index: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add(
        &mut self,
        target: BreakpointTarget,
        condition: BreakpointCondition,
        enabled: bool,
        graph: &crate::circuit::graph::CircuitGraph,
    ) -> BreakpointId {
        let id = self.next_id;
        self.next_id += 1;
        let bp = Breakpoint {
            id,
            target: target.clone(),
            condition,
            enabled,
            hit_count: 0,
        };
        self.breakpoints.insert(id, bp);
        match &target {
            BreakpointTarget::Net(net_id) => {
                self.net_index.entry(*net_id).or_default().push(id);
            }
            BreakpointTarget::Component(comp_id) => {
                if let Some(comp) = graph.components.get(comp_id) {
                    for out_pin_id in &comp.output_pins {
                        if let Some(pin) = graph.pins.get(out_pin_id) {
                            if let Some(net_id) = pin.net {
                                self.net_index.entry(net_id).or_default().push(id);
                            }
                        }
                    }
                }
            }
        }
        id
    }

    pub fn remove(&mut self, id: BreakpointId) {
        if let Some(bp) = self.breakpoints.remove(&id) {
            match bp.target {
                BreakpointTarget::Net(net_id) => {
                    if let Some(list) = self.net_index.get_mut(&net_id) {
                        list.retain(|x| *x != id);
                    }
                }
                BreakpointTarget::Component(_comp_id) => {
                    for list in self.net_index.values_mut() {
                        list.retain(|x| *x != id);
                    }
                }
            }
        }
    }

    pub fn check(
        &self,
        net_id: NetId,
        old_signal: Signal,
        new_signal: Signal,
    ) -> Option<BreakpointHitInfo> {
        if let Some(bp_ids) = self.net_index.get(&net_id) {
            for bp_id in bp_ids {
                if let Some(bp) = self.breakpoints.get(bp_id) {
                    if !bp.enabled {
                        continue;
                    }
                    let hit = match &bp.condition {
                        BreakpointCondition::SignalChanges => true,
                        BreakpointCondition::SignalEquals(expected) => new_signal == *expected,
                        BreakpointCondition::RisingEdge => {
                            !old_signal.to_bool() && new_signal.to_bool()
                        }
                        BreakpointCondition::FallingEdge => {
                            old_signal.to_bool() && !new_signal.to_bool()
                        }
                    };
                    if hit {
                        return Some(BreakpointHitInfo {
                            breakpoint_id: *bp_id,
                            net_id,
                            old_signal,
                            new_signal,
                            tick: 0,
                        });
                    }
                }
            }
        }
        None
    }

    pub fn list(&self) -> Vec<&Breakpoint> {
        self.breakpoints.values().collect()
    }

    pub fn set_enabled(&mut self, id: BreakpointId, enabled: bool) -> Option<()> {
        self.breakpoints.get_mut(&id).map(|bp| bp.enabled = enabled)
    }

    pub fn restore_from_save(
        &mut self,
        saved: Vec<Breakpoint>,
        graph: &crate::circuit::graph::CircuitGraph,
    ) {
        for bp in saved {
            let bp_id = bp.id;
            match &bp.target {
                BreakpointTarget::Net(net_id) => {
                    self.net_index.entry(*net_id).or_default().push(bp_id);
                }
                BreakpointTarget::Component(comp_id) => {
                    if let Some(comp) = graph.components.get(comp_id) {
                        for out_pin_id in &comp.output_pins {
                            if let Some(pin) = graph.pins.get(out_pin_id) {
                                if let Some(net_id) = pin.net {
                                    self.net_index.entry(net_id).or_default().push(bp_id);
                                }
                            }
                        }
                    }
                }
            }
            self.breakpoints.insert(bp_id, bp);
            if bp_id >= self.next_id {
                self.next_id = bp_id + 1;
            }
        }
    }
}
