use tauri::State;
use crate::EngineState;
use crate::project;

#[tauri::command]
pub fn save_project(
    engine: State<'_, EngineState>,
) -> Result<String, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    project::save::save_project(&eng)
}

#[tauri::command]
pub fn load_project(
    engine: State<'_, EngineState>,
    json: String,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    project::load::load_project(&mut eng, &json)?;

    Ok(serde_json::json!({
        "components": eng.graph.components.values().collect::<Vec<_>>(),
        "pins": eng.graph.pins.values().collect::<Vec<_>>(),
        "wires": eng.graph.wires.values().collect::<Vec<_>>(),
    }))
}

#[tauri::command]
pub fn export_custom_component(
    engine: State<'_, EngineState>,
    def_id: u32,
    component_type: String,
) -> Result<String, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    match component_type.as_str() {
        "lua" => project::export::export_lua_component(
            &eng.lua_registry,
            &eng.truth_tables,
            def_id,
        ),
        _ => project::export::export_subcircuit(
            &eng.subcircuit_registry,
            &eng.lua_registry,
            &eng.truth_tables,
            def_id,
        ),
    }
}

#[tauri::command]
pub fn import_custom_component(
    engine: State<'_, EngineState>,
    json: String,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let (new_id, component_type, name) = project::export::import_cfcomp(&json, &mut eng)?;
    Ok(serde_json::json!({
        "id": new_id,
        "component_type": component_type,
        "name": name,
    }))
}

#[tauri::command(rename_all = "camelCase")]
pub fn export_rule_pack(
    engine: State<'_, EngineState>,
    rule_pack_id: u32,
) -> Result<String, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let pack = eng.rule_registry.packs.get(&rule_pack_id)
        .ok_or("rule pack not found")?;
    project::export::export_rule_pack(pack)
}

#[tauri::command]
pub fn import_rule_pack(
    engine: State<'_, EngineState>,
    json: String,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let pack = project::export::import_rule_pack(&json)?;
    let name = pack.name.clone();
    let new_id = eng.rule_registry.add_custom(pack);
    Ok(serde_json::json!({
        "id": new_id,
        "name": name,
    }))
}
