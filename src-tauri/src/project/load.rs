use serde::Deserialize;
use crate::circuit::component::Component;
use crate::circuit::pin::Pin;
use crate::circuit::wire::Wire;
use crate::circuit::junction::Junction;
use crate::circuit::types::SimMode;
use crate::circuit::subcircuit::SubCircuitDef;
use crate::scripting::lua_engine::LuaComponentDef;
use crate::rules::presets::RulePack;
use crate::verification::truth_table::TruthTable;
use crate::simulation::engine::SimulationEngine;
use crate::debugging::breakpoint::Breakpoint;

#[derive(Deserialize)]
struct LoadData {
    version: u32,
    components: Vec<Component>,
    pins: Vec<Pin>,
    wires: Vec<Wire>,
    #[serde(default)]
    junctions: Vec<Junction>,
    next_id: u32,
    #[serde(default)]
    canvas_mode: Option<String>,
    #[serde(default)]
    sim_mode: Option<String>,
    #[serde(default)]
    tick_rate: Option<u32>,
    #[serde(default)]
    speed_multiplier: Option<f32>,
    #[serde(default)]
    subcircuit_defs: Vec<SubCircuitDef>,
    #[serde(default)]
    lua_defs: Vec<LuaComponentDef>,
    #[serde(default)]
    rule_packs: Vec<RulePack>,
    #[serde(default)]
    active_rule_pack_id: Option<u32>,
    #[serde(default)]
    truth_tables: Vec<TruthTable>,
    #[serde(default)]
    subcircuit_registry_next_id: Option<u32>,
    #[serde(default)]
    lua_registry_next_id: Option<u32>,
    #[serde(default)]
    rule_registry_next_id: Option<u32>,
    #[serde(default)]
    breakpoints: Vec<Breakpoint>,
}

pub fn load_project(engine: &mut SimulationEngine, json: &str) -> Result<(), String> {
    let data: LoadData = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if data.version < 1 || data.version > 3 {
        return Err(format!("unsupported version: {}", data.version));
    }

    let graph = &mut engine.graph;
    graph.components.clear();
    graph.pins.clear();
    graph.wires.clear();
    graph.nets.clear();
    graph.junctions.clear();
    graph.next_id = data.next_id;

    engine.signals.clear();
    engine.event_queue.clear();
    engine.tick_engine.reset();
    engine.signal_history.clear();

    for comp in data.components {
        graph.components.insert(comp.id, comp);
    }
    for pin in data.pins {
        graph.pins.insert(pin.id, pin);
    }
    for wire in data.wires {
        graph.wires.insert(wire.id, wire);
    }
    for junction in data.junctions {
        graph.junctions.insert(junction.id, junction);
    }

    for (pin_id, pin) in &graph.pins {
        if let Some(net_id) = pin.net {
            graph
                .nets
                .entry(net_id)
                .or_insert_with(Vec::new)
                .push(*pin_id);
        }
    }

    engine.sim_mode = match data.sim_mode.as_deref() {
        Some("tick") => SimMode::TickDriven,
        _ => SimMode::EventDriven,
    };
    engine.tick_rate = data.tick_rate.unwrap_or(10);
    engine.speed_multiplier = data.speed_multiplier.unwrap_or(1.0);

    // Restore subcircuit defs
    for def in data.subcircuit_defs {
        engine.subcircuit_registry.defs.insert(def.id, def);
    }
    // Restore Lua defs
    for def in data.lua_defs {
        engine.lua_registry.defs.insert(def.id, def);
    }
    // Restore rule packs
    for pack in data.rule_packs {
        engine.rule_registry.packs.insert(pack.id, pack);
    }
    if let Some(id) = data.active_rule_pack_id {
        engine.rule_registry.active_id = id;
    }
    // Restore truth tables
    for tt in data.truth_tables {
        engine.truth_tables.insert(tt.id, tt);
    }

    // Restore registry next_id counters (validate against max existing ID)
    if let Some(nid) = data.subcircuit_registry_next_id {
        let max_id = engine.subcircuit_registry.defs.keys().max().copied().unwrap_or(0);
        engine.subcircuit_registry.next_id = nid.max(max_id + 1);
    }
    if let Some(nid) = data.lua_registry_next_id {
        let max_id = engine.lua_registry.defs.keys().max().copied().unwrap_or(0);
        engine.lua_registry.next_id = nid.max(max_id + 1);
    }
    if let Some(nid) = data.rule_registry_next_id {
        let max_id = engine.rule_registry.packs.keys().max().copied().unwrap_or(0);
        engine.rule_registry.next_id = nid.max(max_id + 1);
    }

    engine.breakpoint_manager.restore_from_save(data.breakpoints, &engine.graph);

    Ok(())
}
