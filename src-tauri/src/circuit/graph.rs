use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::types::{ComponentId, ComponentKind, NetId, PinId, WireId, Signal};
use super::component::Component;
use super::pin::Pin;
use super::wire::{Wire, WireEndpoint};
use super::junction::Junction;
use super::subcircuit::SubCircuitDef;
use crate::scripting::lua_engine::LuaComponentDef;

#[derive(Clone, Serialize, Deserialize)]
pub struct CircuitGraph {
    pub components: HashMap<ComponentId, Component>,
    pub pins: HashMap<PinId, Pin>,
    pub wires: HashMap<WireId, Wire>,
    pub nets: HashMap<NetId, Vec<PinId>>,
    pub junctions: HashMap<u32, Junction>,
    pub next_id: u32,
}

impl CircuitGraph {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            pins: HashMap::new(),
            wires: HashMap::new(),
            nets: HashMap::new(),
            junctions: HashMap::new(),
            next_id: 0,
        }
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_component(
        &mut self,
        kind: ComponentKind,
        x: f32,
        y: f32,
    ) -> Result<(ComponentId, Vec<PinId>, Vec<PinId>), String> {
        match &kind {
            ComponentKind::SubCircuit(_) => return Err("use add_subcircuit_component instead".into()),
            ComponentKind::LuaScript(_) => return Err("use add_lua_component instead".into()),
            _ => {}
        }
        // Prevent overlap: check if position is already occupied
        for c in self.components.values() {
            if (c.x - x).abs() < 1.05 && (c.y - y).abs() < 1.05 {
                return Err("position already occupied by another component".into());
            }
        }
        let comp_id = self.alloc_id();

        let kind_for_checks = kind.clone();
        let (input_offsets, output_offsets): (Vec<(f32, f32)>, Vec<(f32, f32)>) = match kind {
            ComponentKind::And | ComponentKind::Or | ComponentKind::Nand | ComponentKind::Xor => {
                (vec![(-1.0, 0.3), (-1.0, -0.3)], vec![(1.0, 0.0)])
            }
            ComponentKind::Not => {
                (vec![(-1.0, 0.0)], vec![(1.0, 0.0)])
            }
            ComponentKind::Switch => {
                (vec![], vec![(1.0, 0.0)])
            }
            ComponentKind::Led => {
                (vec![(-1.0, 0.0)], vec![])
            }
            ComponentKind::Button => {
                (vec![], vec![(1.0, 0.0)])
            }
            ComponentKind::Clock => {
                (vec![], vec![(1.0, 0.0)])
            }
            ComponentKind::Random => {
                (vec![], vec![(1.0, 0.0)])
            }
            ComponentKind::Constant => {
                (vec![], vec![(1.0, 0.0)])
            }
            ComponentKind::SevenSegment => {
                (
                    vec![
                        (-1.0, 0.45),
                        (-1.0, 0.15),
                        (-1.0, -0.15),
                        (-1.0, -0.45),
                    ],
                    vec![],
                )
            }
            ComponentKind::Oscilloscope => {
                (vec![(-1.0, 0.0)], vec![])
            }
            ComponentKind::DelayLine => {
                (vec![(-1.0, 0.0)], vec![(1.0, 0.0)])
            }
            ComponentKind::Splitter => {
                let width = 4_u32;
                let mut outputs = Vec::new();
                let spacing = 0.8 / width.max(1) as f32;
                let start_y = -0.4;
                for i in 0..width {
                    outputs.push((1.0, start_y + spacing * i as f32));
                }
                (vec![(-1.0, 0.0)], outputs)
            }
            ComponentKind::Merger => {
                let width = 4_u32;
                let mut inputs = Vec::new();
                let spacing = 0.8 / width.max(1) as f32;
                let start_y = -0.4;
                for i in 0..width {
                    inputs.push((-1.0, start_y + spacing * i as f32));
                }
                (inputs, vec![(1.0, 0.0)])
            }
            ComponentKind::SubCircuit(_) | ComponentKind::LuaScript(_) | ComponentKind::Plugin(_, _) => {
                (Vec::new(), Vec::new())
            }
        };

        let mut input_pins = Vec::new();
        let mut output_pins = Vec::new();

        for &(ox, oy) in &input_offsets {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: false,
                    net: None,
                    offset_x: ox,
                    offset_y: oy,
                },
            );
            input_pins.push(pin_id);
        }

        for &(ox, oy) in &output_offsets {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: true,
                    net: None,
                    offset_x: ox,
                    offset_y: oy,
                },
            );
            output_pins.push(pin_id);
        }

        let toggle_state = if kind == ComponentKind::Switch {
            Some(Signal::Low)
        } else {
            None
        };

        self.components.insert(
            comp_id,
            Component {
                id: comp_id,
                kind,
                x,
                y,
                input_pins: input_pins.clone(),
                output_pins: output_pins.clone(),
                toggle_state,
                press_state: None,
                clock_period: Some(2),
                clock_duty: Some(0.5),
                clock_counter: Some(0),
                random_probability: Some(0.5),
                constant_value: if kind_for_checks == ComponentKind::Constant {
                    Some(Signal::High)
                } else {
                    None
                },
                oscilloscope_channels: Some(1),
                oscilloscope_time_window: Some(100),
                delay_ticks: Some(1),
                delay_buffer: None,
                bus_width: if kind_for_checks == ComponentKind::Splitter || kind_for_checks == ComponentKind::Merger {
                    Some(4)
                } else {
                    Some(1)
                },
                lua_state: None,
            },
        );

        Ok((comp_id, input_pins, output_pins))
    }

    pub fn add_junction(&mut self, x: f32, y: f32, net_id: NetId) -> u32 {
        let id = self.alloc_id();
        self.junctions.insert(
            id,
            Junction {
                id,
                x,
                y,
                net_id,
            },
        );
        id
    }

    pub fn remove_junction(&mut self, id: u32) -> Result<(), String> {
        self.junctions
            .remove(&id)
            .ok_or_else(|| format!("junction {} not found", id))?;

        let junction_id = id;
        let wires_to_remove: Vec<WireId> = self
            .wires
            .iter()
            .filter(|(_, w)| {
                matches!(&w.start, WireEndpoint::Junction(j) if *j == junction_id)
                    || matches!(&w.end, WireEndpoint::Junction(j) if *j == junction_id)
            })
            .map(|(wid, _)| *wid)
            .collect();

        for wire_id in wires_to_remove {
            let wire = self.wires.remove(&wire_id).unwrap();
            let pin_ids: Vec<Option<PinId>> = vec![wire.start.as_pin(), wire.end.as_pin()];
            for pin_id in pin_ids.iter().flatten() {
                let has_other = self.wires.values().any(|w| {
                    w.start.as_pin() == Some(*pin_id) || w.end.as_pin() == Some(*pin_id)
                });
                if !has_other {
                    self.clear_pin_from_net(*pin_id, wire.net_id);
                }
            }
        }

        Ok(())
    }

    pub fn add_wire(&mut self, start: WireEndpoint, end: WireEndpoint, color: Option<u32>) -> Result<(WireId, NetId), String> {
        match (&start, &end) {
            (WireEndpoint::Pin(pa), WireEndpoint::Pin(pb)) => {
                let pa_pin = self.pins.get(pa).ok_or_else(|| format!("pin {} not found", pa))?;
                let pb_pin = self.pins.get(pb).ok_or_else(|| format!("pin {} not found", pb))?;
                if pa_pin.is_output && pb_pin.is_output {
                    return Err("cannot connect two output pins together".into());
                }
            }
            (WireEndpoint::Pin(pa), WireEndpoint::Junction(jb)) => {
                self.pins.get(pa).ok_or_else(|| format!("pin {} not found", pa))?;
                self.junctions.get(jb).ok_or_else(|| format!("junction {} not found", jb))?;
            }
            (WireEndpoint::Junction(ja), WireEndpoint::Pin(pb)) => {
                self.junctions.get(ja).ok_or_else(|| format!("junction {} not found", ja))?;
                self.pins.get(pb).ok_or_else(|| format!("pin {} not found", pb))?;
            }
            (WireEndpoint::Junction(ja), WireEndpoint::Junction(jb)) => {
                self.junctions.get(ja).ok_or_else(|| format!("junction {} not found", ja))?;
                self.junctions.get(jb).ok_or_else(|| format!("junction {} not found", jb))?;
            }
        }

        let net_a = match &start {
            WireEndpoint::Pin(pid) => self.pins.get(pid).and_then(|p| p.net),
            WireEndpoint::Junction(jid) => self.junctions.get(jid).map(|j| j.net_id),
        };
        let net_b = match &end {
            WireEndpoint::Pin(pid) => self.pins.get(pid).and_then(|p| p.net),
            WireEndpoint::Junction(jid) => self.junctions.get(jid).map(|j| j.net_id),
        };

        let net_id = match (net_a, net_b) {
            (Some(na), Some(nb)) if na == nb => {
                return Err("pins already on the same net".into());
            }
            (Some(na), Some(nb)) => {
                let pins_to_move: Vec<PinId> =
                    self.nets.get(&nb).cloned().unwrap_or_default();
                self.nets.remove(&nb);
                for pid in &pins_to_move {
                    if let Some(pin) = self.pins.get_mut(pid) {
                        pin.net = Some(na);
                    }
                }
                if let Some(net_pins) = self.nets.get_mut(&na) {
                    net_pins.extend(pins_to_move);
                }
                na
            }
            (Some(na), None) => {
                if let WireEndpoint::Pin(pid) = &end {
                    if let Some(net_pins) = self.nets.get_mut(&na) {
                        net_pins.push(*pid);
                    }
                }
                na
            }
            (None, Some(nb)) => {
                if let WireEndpoint::Pin(pid) = &start {
                    if let Some(net_pins) = self.nets.get_mut(&nb) {
                        net_pins.push(*pid);
                    }
                }
                nb
            }
            (None, None) => {
                let new_net = self.alloc_id();
                let mut net_pins = Vec::new();
                if let WireEndpoint::Pin(pid) = &start { net_pins.push(*pid); }
                if let WireEndpoint::Pin(pid) = &end { net_pins.push(*pid); }
                self.nets.insert(new_net, net_pins);
                new_net
            }
        };

        if let WireEndpoint::Pin(pid) = &start {
            if let Some(pin) = self.pins.get_mut(pid) { pin.net = Some(net_id); }
        }
        if let WireEndpoint::Pin(pid) = &end {
            if let Some(pin) = self.pins.get_mut(pid) { pin.net = Some(net_id); }
        }

        let wire_id = self.alloc_id();
        self.wires.insert(
            wire_id,
            Wire {
                id: wire_id,
                start: start.clone(),
                end: end.clone(),
                net_id,
                color,
            },
        );

        Ok((wire_id, net_id))
    }

    pub fn remove_component(&mut self, comp_id: ComponentId) -> Result<(), String> {
        let all_pins: Vec<PinId> = {
            let comp = self
                .components
                .get(&comp_id)
                .ok_or("component not found")?;
            comp.input_pins
                .iter()
                .chain(comp.output_pins.iter())
                .copied()
                .collect()
        };

        let wires_to_remove: Vec<WireId> = self
            .wires
            .iter()
            .filter(|(_, w)| {
                w.start.as_pin().map_or(false, |p| all_pins.contains(&p))
                    || w.end.as_pin().map_or(false, |p| all_pins.contains(&p))
            })
            .map(|(id, _)| *id)
            .collect();

        for wire_id in &wires_to_remove {
            let wire = self.wires.remove(wire_id).unwrap();
            let pin_ids: Vec<Option<PinId>> = vec![wire.start.as_pin(), wire.end.as_pin()];
            for pin_id in pin_ids.iter().flatten() {
                let has_other = self.wires.values().any(|w| {
                    w.start.as_pin() == Some(*pin_id) || w.end.as_pin() == Some(*pin_id)
                });
                if !has_other {
                    self.clear_pin_from_net(*pin_id, wire.net_id);
                }
            }
        }

        for pin_id in &all_pins {
            self.pins.remove(pin_id);
        }

        self.components.remove(&comp_id);

        Ok(())
    }

    pub fn remove_wire(&mut self, wire_id: WireId) -> Result<(), String> {
        let wire = self.wires.remove(&wire_id).ok_or("wire not found")?;
        let pin_ids: Vec<Option<PinId>> = vec![wire.start.as_pin(), wire.end.as_pin()];
        for pin_id in pin_ids.iter().flatten() {
            let has_other = self.wires.values().any(|w| {
                w.start.as_pin() == Some(*pin_id) || w.end.as_pin() == Some(*pin_id)
            });
            if !has_other {
                self.clear_pin_from_net(*pin_id, wire.net_id);
            }
        }
        Ok(())
    }

    fn clear_pin_from_net(&mut self, pin_id: PinId, net_id: NetId) {
        let is_empty = if let Some(net_pins) = self.nets.get_mut(&net_id) {
            net_pins.retain(|&p| p != pin_id);
            net_pins.is_empty()
        } else {
            false
        };
        if is_empty {
            self.nets.remove(&net_id);
        }
        if let Some(pin) = self.pins.get_mut(&pin_id) {
            pin.net = None;
        }
    }

    pub fn move_component(
        &mut self,
        comp_id: ComponentId,
        x: f32,
        y: f32,
    ) -> Result<(), String> {
        let _comp = self
            .components
            .get(&comp_id)
            .ok_or("component not found")?;

        for c in self.components.values() {
            if c.id != comp_id && (c.x - x).abs() < 1.05 && (c.y - y).abs() < 1.05 {
                return Err("target position already occupied".into());
            }
        }

        let comp = self
            .components
            .get_mut(&comp_id)
            .ok_or("component not found")?;
        comp.x = x;
        comp.y = y;
        Ok(())
    }

    pub fn add_subcircuit_component(
        &mut self,
        def: &SubCircuitDef,
        x: f32,
        y: f32,
    ) -> Result<(ComponentId, Vec<PinId>, Vec<PinId>), String> {
        let comp_id = self.alloc_id();
        let mut input_pins = Vec::new();
        let mut output_pins = Vec::new();

        for ext_pin in &def.external_pins {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: ext_pin.is_output,
                    net: None,
                    offset_x: ext_pin.offset_x,
                    offset_y: ext_pin.offset_y,
                },
            );
            if ext_pin.is_output {
                output_pins.push(pin_id);
            } else {
                input_pins.push(pin_id);
            }
        }

        let comp = Component {
            id: comp_id,
            kind: ComponentKind::SubCircuit(def.id),
            x,
            y,
            input_pins: input_pins.clone(),
            output_pins: output_pins.clone(),
            toggle_state: None,
            press_state: None,
            clock_period: None,
            clock_duty: None,
            clock_counter: None,
            random_probability: None,
            constant_value: None,
            oscilloscope_channels: None,
            oscilloscope_time_window: None,
            delay_ticks: None,
            delay_buffer: None,
            bus_width: None,
            lua_state: None,
        };
        self.components.insert(comp_id, comp);
        Ok((comp_id, input_pins, output_pins))
    }

    pub fn add_lua_component(
        &mut self,
        def: &LuaComponentDef,
        x: f32,
        y: f32,
    ) -> Result<(ComponentId, Vec<PinId>, Vec<PinId>), String> {
        let comp_id = self.alloc_id();
        let mut input_pins = Vec::new();
        let mut output_pins = Vec::new();

        for lua_pin in &def.input_pins {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: false,
                    net: None,
                    offset_x: lua_pin.offset_x,
                    offset_y: lua_pin.offset_y,
                },
            );
            input_pins.push(pin_id);
        }

        for lua_pin in &def.output_pins {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: true,
                    net: None,
                    offset_x: lua_pin.offset_x,
                    offset_y: lua_pin.offset_y,
                },
            );
            output_pins.push(pin_id);
        }

        let comp = Component {
            id: comp_id,
            kind: ComponentKind::LuaScript(def.id),
            x,
            y,
            input_pins: input_pins.clone(),
            output_pins: output_pins.clone(),
            toggle_state: None,
            press_state: None,
            clock_period: None,
            clock_duty: None,
            clock_counter: None,
            random_probability: None,
            constant_value: None,
            oscilloscope_channels: None,
            oscilloscope_time_window: None,
            delay_ticks: None,
            delay_buffer: None,
            bus_width: None,
            lua_state: None,
        };
        self.components.insert(comp_id, comp);
        Ok((comp_id, input_pins, output_pins))
    }

    pub fn add_plugin_component(
        &mut self,
        plugin_id: &str,
        kind_name: &str,
        input_offsets: Vec<(f32, f32)>,
        output_offsets: Vec<(f32, f32)>,
        x: f32,
        y: f32,
    ) -> Result<(ComponentId, Vec<PinId>, Vec<PinId>), String> {
        let comp_id = self.alloc_id();
        let mut input_pins = Vec::new();
        let mut output_pins = Vec::new();

        for (ox, oy) in &input_offsets {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: false,
                    net: None,
                    offset_x: *ox,
                    offset_y: *oy,
                },
            );
            input_pins.push(pin_id);
        }

        for (ox, oy) in &output_offsets {
            let pin_id = self.alloc_id();
            self.pins.insert(
                pin_id,
                Pin {
                    id: pin_id,
                    owner: comp_id,
                    is_output: true,
                    net: None,
                    offset_x: *ox,
                    offset_y: *oy,
                },
            );
            output_pins.push(pin_id);
        }

        let comp = Component {
            id: comp_id,
            kind: ComponentKind::Plugin(plugin_id.to_string(), kind_name.to_string()),
            x,
            y,
            input_pins: input_pins.clone(),
            output_pins: output_pins.clone(),
            toggle_state: None,
            press_state: None,
            clock_period: None,
            clock_duty: None,
            clock_counter: None,
            random_probability: None,
            constant_value: None,
            oscilloscope_channels: None,
            oscilloscope_time_window: None,
            delay_ticks: None,
            delay_buffer: None,
            bus_width: None,
            lua_state: None,
        };
        self.components.insert(comp_id, comp);
        Ok((comp_id, input_pins, output_pins))
    }

    pub fn extract_subgraph(
        &self,
        component_ids: &[ComponentId],
    ) -> Result<CircuitGraph, String> {
        use std::collections::HashSet;
        let mut sub = CircuitGraph::new();
        sub.next_id = self.next_id;

        for &comp_id in component_ids {
            if let Some(comp) = self.components.get(&comp_id) {
                sub.components.insert(comp_id, comp.clone());
                let all_pins: Vec<PinId> = comp
                    .input_pins
                    .iter()
                    .chain(comp.output_pins.iter())
                    .copied()
                    .collect();
                for pin_id in all_pins {
                    if let Some(pin) = self.pins.get(&pin_id) {
                        sub.pins.insert(pin_id, pin.clone());
                    }
                }
            }
        }

        let sub_pin_ids: HashSet<PinId> = sub.pins.keys().copied().collect();
        for (wire_id, wire) in &self.wires {
            let pin_a = match &wire.start {
                WireEndpoint::Pin(pid) => sub_pin_ids.contains(pid),
                _ => false,
            };
            let pin_b = match &wire.end {
                WireEndpoint::Pin(pid) => sub_pin_ids.contains(pid),
                _ => false,
            };
            if pin_a && pin_b {
                sub.wires.insert(*wire_id, wire.clone());
            }
        }

        for (pin_id, pin) in &sub.pins {
            if let Some(net_id) = pin.net {
                sub.nets.entry(net_id).or_default().push(*pin_id);
            }
        }

        sub.remap_graph_ids();

        Ok(sub)
    }

    pub fn remap_graph_ids(&mut self) {
        let mut id_map: HashMap<u32, u32> = HashMap::new();
        let mut new_next = 1u32;

        // Pass 1: assign new IDs for every entity to build a complete mapping
        let old_comps: Vec<_> = self.components.drain().collect();
        let old_pins: Vec<_> = self.pins.drain().collect();
        let old_wires: Vec<_> = self.wires.drain().collect();
        let old_junctions: Vec<_> = self.junctions.drain().collect();

        for &(old_id, _) in &old_comps {
            id_map.insert(old_id, new_next);
            new_next += 1;
        }
        for &(old_id, _) in &old_pins {
            id_map.insert(old_id, new_next);
            new_next += 1;
        }
        for &(old_id, _) in &old_wires {
            id_map.insert(old_id, new_next);
            new_next += 1;
        }
        for &(old_id, _) in &old_junctions {
            id_map.insert(old_id, new_next);
            new_next += 1;
        }

        let remap = |old: u32| id_map.get(&old).copied().unwrap_or(old);

        // Pass 2: apply mapping
        for (old_id, mut comp) in old_comps {
            comp.id = remap(old_id);
            comp.input_pins = comp.input_pins.iter().map(|p| remap(*p)).collect();
            comp.output_pins = comp.output_pins.iter().map(|p| remap(*p)).collect();
            self.components.insert(comp.id, comp);
        }

        for (old_id, mut pin) in old_pins {
            pin.id = remap(old_id);
            pin.owner = remap(pin.owner);
            self.pins.insert(pin.id, pin);
        }

        for (old_id, mut wire) in old_wires {
            wire.id = remap(old_id);
            match &mut wire.start {
                WireEndpoint::Pin(pid) => *pid = remap(*pid),
                WireEndpoint::Junction(jid) => *jid = remap(*jid),
            }
            match &mut wire.end {
                WireEndpoint::Pin(pid) => *pid = remap(*pid),
                WireEndpoint::Junction(jid) => *jid = remap(*jid),
            }
            wire.net_id = remap(wire.net_id);
            self.wires.insert(wire.id, wire);
        }

        for (old_id, mut j) in old_junctions {
            j.id = remap(old_id);
            j.net_id = remap(j.net_id);
            self.junctions.insert(j.id, j);
        }

        // Rebuild nets from pins
        self.nets.clear();
        for (pin_id, pin) in &self.pins {
            if let Some(net_id) = pin.net {
                self.nets
                    .entry(remap(net_id))
                    .or_default()
                    .push(*pin_id);
            }
        }

        self.next_id = new_next;
    }
}
