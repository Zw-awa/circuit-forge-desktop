use serde::{Deserialize, Serialize};
use crate::circuit::component::Component;
use crate::circuit::pin::Pin;
use crate::circuit::wire::Wire;
use crate::circuit::junction::Junction;
use crate::circuit::subcircuit::SubCircuitDef;
use crate::scripting::lua_engine::LuaComponentDef;
use crate::rules::presets::RulePack;
use crate::verification::truth_table::TruthTable;
use crate::simulation::engine::SimulationEngine;
use crate::debugging::breakpoint::Breakpoint;

#[derive(Serialize, Deserialize)]
struct SaveData {
    version: u32,
    components: Vec<Component>,
    pins: Vec<Pin>,
    wires: Vec<Wire>,
    junctions: Vec<Junction>,
    next_id: u32,
    canvas_mode: Option<String>,
    sim_mode: Option<String>,
    tick_rate: Option<u32>,
    speed_multiplier: Option<f32>,
    // Phase 3 fields
    subcircuit_defs: Vec<SubCircuitDef>,
    lua_defs: Vec<LuaComponentDef>,
    rule_packs: Vec<RulePack>,
    active_rule_pack_id: Option<u32>,
    truth_tables: Vec<TruthTable>,
    subcircuit_registry_next_id: u32,
    lua_registry_next_id: u32,
    rule_registry_next_id: u32,
    #[serde(default)]
    breakpoints: Vec<Breakpoint>,
}

pub fn save_project(engine: &SimulationEngine) -> Result<String, String> {
    let graph = &engine.graph;
    let sim_mode = match engine.sim_mode {
        crate::circuit::types::SimMode::EventDriven => "event",
        crate::circuit::types::SimMode::TickDriven => "tick",
    };
    let data = SaveData {
        version: 3,
        components: graph.components.values().cloned().collect(),
        pins: graph.pins.values().cloned().collect(),
        wires: graph.wires.values().cloned().collect(),
        junctions: graph.junctions.values().cloned().collect(),
        next_id: graph.next_id,
        canvas_mode: None,
        sim_mode: Some(sim_mode.to_string()),
        tick_rate: Some(engine.tick_rate),
        speed_multiplier: Some(engine.speed_multiplier),
        subcircuit_defs: engine.subcircuit_registry.defs.values().cloned().collect(),
        lua_defs: engine.lua_registry.defs.values().cloned().collect(),
        rule_packs: engine.rule_registry.packs.values().cloned().collect(),
        active_rule_pack_id: Some(engine.rule_registry.active_id),
        truth_tables: engine.truth_tables.values().cloned().collect(),
        subcircuit_registry_next_id: engine.subcircuit_registry.next_id,
        lua_registry_next_id: engine.lua_registry.next_id,
        rule_registry_next_id: engine.rule_registry.next_id,
        breakpoints: engine.breakpoint_manager.breakpoints.values().cloned().collect(),
    };
    serde_json::to_string(&data).map_err(|e| e.to_string())
}
