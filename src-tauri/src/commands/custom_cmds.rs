use tauri::State;
use crate::EngineState;
use crate::circuit::types::ComponentKind;
use crate::circuit::subcircuit::{
    SubCircuitDef, ExternalPin,
    check_circular_reference,
};
use crate::scripting::lua_engine::LuaComponentDef;
use crate::scripting::validator::validate_script;
use crate::verification::truth_table::{
    TruthTable, TruthTableRow, VerificationResult,
};
use crate::verification::verifier::verify_truth_table;
use std::collections::HashSet;

#[tauri::command]
pub fn create_subcircuit_def(
    engine: State<'_, EngineState>,
    name: String,
    component_ids: Vec<u32>,
    external_pins: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let inner_graph = eng.graph.extract_subgraph(&component_ids)?;

    let ext_pins: Vec<ExternalPin> = external_pins
        .iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    // Check for circular references: inner graph must not contain SubCircuit(def_id) where def_id == the new def
    let mut visited = HashSet::new();
    // Temporarily insert a stub to check. We use id=0 as a sentinel.
    let temp_def = SubCircuitDef {
        id: 0,
        name: name.clone(),
        description: String::new(),
        inner_graph: inner_graph.clone(),
        external_pins: ext_pins.clone(),
        width: 200.0,
        height: 150.0,
        icon_label: name.chars().take(3).collect(),
    };
    eng.subcircuit_registry.defs.insert(0, temp_def);
    let has_circular = check_circular_reference(&eng.subcircuit_registry, 0, &mut visited);
    eng.subcircuit_registry.defs.remove(&0);
    if has_circular {
        return Err("circular reference detected".into());
    }

    let def = SubCircuitDef {
        id: 0,
        name: name.clone(),
        description: String::new(),
        inner_graph,
        external_pins: ext_pins,
        width: 200.0,
        height: 150.0,
        icon_label: name.chars().take(3).collect(),
    };

    let def_id = eng.subcircuit_registry.create(def);
    Ok(serde_json::json!({ "defId": def_id }))
}

#[tauri::command]
pub fn update_subcircuit_def(
    engine: State<'_, EngineState>,
    def_id: u32,
    changes: serde_json::Value,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let existing = eng.subcircuit_registry.get(def_id)
        .ok_or("definition not found")?
        .clone();

    let mut updated = existing.clone();

    if let Some(name) = changes.get("name").and_then(|v| v.as_str()) {
        updated.name = name.to_string();
    }
    if let Some(desc) = changes.get("description").and_then(|v| v.as_str()) {
        updated.description = desc.to_string();
    }
    if let Some(width) = changes.get("width").and_then(|v| v.as_f64()) {
        updated.width = width as f32;
    }
    if let Some(height) = changes.get("height").and_then(|v| v.as_f64()) {
        updated.height = height as f32;
    }
    if let Some(label) = changes.get("iconLabel").and_then(|v| v.as_str()) {
        updated.icon_label = label.to_string();
    }
    if let Some(ext_pins) = changes.get("externalPins").and_then(|v| v.as_array()) {
        updated.external_pins = ext_pins
            .iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
    }

    let mut visited = HashSet::new();
    if check_circular_reference(&eng.subcircuit_registry, def_id, &mut visited) {
        return Err("circular reference detected".into());
    }

    eng.subcircuit_registry.update(def_id, updated)
}

#[tauri::command]
pub fn delete_subcircuit_def(
    engine: State<'_, EngineState>,
    def_id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    // Cascade-delete instances: remove all components on the graph that reference this def
    let comps_to_remove: Vec<u32> = eng.graph.components.iter()
        .filter_map(|(id, c)| {
            if let ComponentKind::SubCircuit(did) = c.kind.clone() {
                if did == def_id { Some(*id) } else { None }
            } else {
                None
            }
        })
        .collect();
    for comp_id in comps_to_remove {
        let _ = eng.graph.remove_component(comp_id);
    }

    eng.subcircuit_registry.remove(def_id)
}

#[tauri::command]
pub fn get_subcircuit_defs(
    engine: State<'_, EngineState>,
) -> Result<Vec<SubCircuitDef>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(eng.subcircuit_registry.defs.values().cloned().collect())
}

#[tauri::command]
pub fn add_subcircuit_instance(
    engine: State<'_, EngineState>,
    def_id: u32,
    x: f32,
    y: f32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let def = eng.subcircuit_registry.get(def_id)
        .ok_or("definition not found")?
        .clone();

    let (comp_id, input_pins, output_pins) = eng.graph.add_subcircuit_component(&def, x, y)?;

    let to_pin_json = |pid: &u32| {
        let pin = &eng.graph.pins[pid];
        serde_json::json!({ "id": pin.id, "offsetX": pin.offset_x, "offsetY": pin.offset_y })
    };

    Ok(serde_json::json!({
        "componentId": comp_id,
        "inputPins": input_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
        "outputPins": output_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
    }))
}

#[tauri::command]
pub fn enter_subcircuit(
    engine: State<'_, EngineState>,
    component_id: u32,
) -> Result<serde_json::Value, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;

    let comp = eng.graph.components.get(&component_id)
        .ok_or("component not found")?;

    let def_id = match comp.kind.clone() {
        ComponentKind::SubCircuit(id) => id,
        _ => return Err("component is not a subcircuit".into()),
    };

    let def = eng.subcircuit_registry.get(def_id)
        .ok_or("subcircuit definition not found")?;

    Ok(serde_json::json!({
        "components": def.inner_graph.components.values().collect::<Vec<_>>(),
        "wires": def.inner_graph.wires.values().collect::<Vec<_>>(),
        "pins": def.inner_graph.pins.values().collect::<Vec<_>>(),
        "defName": def.name,
    }))
}

#[tauri::command]
pub fn exit_subcircuit(
    _engine: State<'_, EngineState>,
) -> Result<(), String> {
    // Frontend manages subcircuit navigation state; Rust validates and holds persistent data.
    // The engine's graph is updated by individual component/wire commands from the frontend.
    Ok(())
}

// ─── Lua Scripting Commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn get_lua_component_defs(
    engine: State<'_, EngineState>,
) -> Result<Vec<LuaComponentDef>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(eng.lua_registry.defs.values().cloned().collect())
}

#[tauri::command]
pub fn create_lua_component_def(
    engine: State<'_, EngineState>,
    name: String,
    script: String,
    input_pins: Vec<serde_json::Value>,
    output_pins: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let in_pins: Vec<crate::scripting::lua_engine::LuaPinDef> = input_pins
        .iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    let out_pins: Vec<crate::scripting::lua_engine::LuaPinDef> = output_pins
        .iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    crate::scripting::validator::validate_script(&script, in_pins.len(), out_pins.len())
        .map_err(|errors| errors.join("; "))?;

    let def = LuaComponentDef {
        id: 0,
        name,
        description: String::new(),
        script_source: script,
        input_pins: in_pins,
        output_pins: out_pins,
        icon_label: String::new(),
        width: 200.0,
        height: 150.0,
    };
    let def_id = eng.lua_registry.create(def);
    Ok(serde_json::json!({ "defId": def_id }))
}

#[tauri::command]
pub fn update_lua_component_def(
    engine: State<'_, EngineState>,
    def_id: u32,
    changes: serde_json::Value,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let existing = eng.lua_registry.get(def_id)
        .ok_or("definition not found")?
        .clone();

    let mut updated = existing.clone();

    if let Some(name) = changes.get("name").and_then(|v| v.as_str()) {
        updated.name = name.to_string();
    }
    if let Some(desc) = changes.get("description").and_then(|v| v.as_str()) {
        updated.description = desc.to_string();
    }
    if let Some(script) = changes.get("script").and_then(|v| v.as_str()) {
        updated.script_source = script.to_string();
    }
    if let Some(in_pins) = changes.get("inputPins").and_then(|v| v.as_array()) {
        updated.input_pins = in_pins
            .iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
    }
    if let Some(out_pins) = changes.get("outputPins").and_then(|v| v.as_array()) {
        updated.output_pins = out_pins
            .iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
    }
    if let Some(width) = changes.get("width").and_then(|v| v.as_f64()) {
        updated.width = width as f32;
    }
    if let Some(height) = changes.get("height").and_then(|v| v.as_f64()) {
        updated.height = height as f32;
    }
    if let Some(label) = changes.get("iconLabel").and_then(|v| v.as_str()) {
        updated.icon_label = label.to_string();
    }

    eng.lua_registry.update(def_id, updated)
}

#[tauri::command]
pub fn delete_lua_component_def(
    engine: State<'_, EngineState>,
    def_id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.lua_registry.remove(def_id)
}

#[tauri::command]
pub fn add_lua_component_instance(
    engine: State<'_, EngineState>,
    def_id: u32,
    x: f32,
    y: f32,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let def = eng.lua_registry.get(def_id)
        .ok_or("definition not found")?
        .clone();

    let (comp_id, input_pins, output_pins) = eng.graph.add_lua_component(&def, x, y)?;

    let to_pin_json = |pid: &u32| {
        let pin = &eng.graph.pins[pid];
        serde_json::json!({ "id": pin.id, "offsetX": pin.offset_x, "offsetY": pin.offset_y })
    };

    Ok(serde_json::json!({
        "componentId": comp_id,
        "inputPins": input_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
        "outputPins": output_pins.iter().map(to_pin_json).collect::<Vec<_>>(),
    }))
}

#[tauri::command]
pub fn validate_lua_script(
    _engine: State<'_, EngineState>,
    source: String,
    input_count: usize,
    output_count: usize,
) -> Result<serde_json::Value, String> {
    match validate_script(&source, input_count, output_count) {
        Ok(()) => Ok(serde_json::json!({ "valid": true, "errors": [] })),
        Err(errors) => Ok(serde_json::json!({ "valid": false, "errors": errors })),
    }
}

// ─── Truth Table Commands ────────────────────────────────────────────────────

#[tauri::command]
pub fn create_truth_table(
    engine: State<'_, EngineState>,
    target_def_id: u32,
    target_type: String,
    rows: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let tt_type = match target_type.as_str() {
        "SubCircuit" => crate::verification::truth_table::TargetType::SubCircuit,
        "LuaScript" => crate::verification::truth_table::TargetType::LuaScript,
        _ => return Err("invalid targetType".into()),
    };

    let parsed_rows: Vec<TruthTableRow> = rows
        .iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    let id = eng.alloc_truth_table_id();
    let table = TruthTable {
        id,
        target_def_id,
        target_type: tt_type,
        input_names: Vec::new(),
        output_names: Vec::new(),
        rows: parsed_rows,
    };
    eng.truth_tables.insert(id, table);
    Ok(serde_json::json!({ "id": id }))
}

#[tauri::command]
pub fn update_truth_table(
    engine: State<'_, EngineState>,
    id: u32,
    rows: Vec<serde_json::Value>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let existing = eng.truth_tables.get(&id)
        .ok_or("truth table not found")?
        .clone();

    let parsed_rows: Vec<TruthTableRow> = rows
        .iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;

    let mut updated = existing.clone();
    updated.rows = parsed_rows;
    eng.truth_tables.insert(id, updated);
    Ok(())
}

#[tauri::command]
pub fn delete_truth_table(
    engine: State<'_, EngineState>,
    id: u32,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.truth_tables.remove(&id).ok_or("truth table not found")?;
    Ok(())
}

#[tauri::command]
pub fn get_truth_table(
    engine: State<'_, EngineState>,
    target_def_id: u32,
    target_type: String,
) -> Result<Option<TruthTable>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;

    let tt_type = match target_type.as_str() {
        "SubCircuit" => crate::verification::truth_table::TargetType::SubCircuit,
        "LuaScript" => crate::verification::truth_table::TargetType::LuaScript,
        _ => return Err("invalid targetType".into()),
    };

    let found = eng.truth_tables.values()
        .find(|t| t.target_def_id == target_def_id && t.target_type == tt_type)
        .cloned();
    Ok(found)
}

#[tauri::command]
pub fn verify_truth_table_cmd(
    engine: State<'_, EngineState>,
    target_def_id: u32,
    target_type: String,
) -> Result<VerificationResult, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let tt_type = match target_type.as_str() {
        "SubCircuit" => crate::verification::truth_table::TargetType::SubCircuit,
        "LuaScript" => crate::verification::truth_table::TargetType::LuaScript,
        _ => return Err("invalid targetType".into()),
    };

    let table = eng.truth_tables.values()
        .find(|t| t.target_def_id == target_def_id && t.target_type == tt_type)
        .ok_or("truth table not found")?
        .clone();

    Ok(verify_truth_table(&mut eng, &table))
}
