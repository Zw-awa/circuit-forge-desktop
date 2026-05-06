use std::path::Path;
use tauri::State;
use crate::EngineState;
use crate::plugins::manifest::PluginInfo;
use crate::plugins::loader::ComponentRegistration;
use crate::circuit::types::Signal;

#[tauri::command]
pub fn plugin_load(
    engine: State<'_, EngineState>,
    path: String,
) -> Result<PluginInfo, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.plugin_manager.load(Path::new(&path))
}

#[tauri::command]
pub fn plugin_unload(
    engine: State<'_, EngineState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.plugin_manager.unload(&plugin_id)
}

#[tauri::command]
pub fn plugin_list(
    engine: State<'_, EngineState>,
) -> Result<Vec<PluginInfo>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(eng.plugin_manager.list())
}

#[tauri::command]
pub fn plugin_set_enabled(
    engine: State<'_, EngineState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.plugin_manager.set_enabled(&plugin_id, enabled)
}

#[tauri::command]
pub fn plugin_get_components(
    engine: State<'_, EngineState>,
    plugin_id: String,
) -> Result<Vec<ComponentRegistration>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let plugin = eng.plugin_manager.get(&plugin_id).ok_or("plugin not found")?;
    Ok(plugin.registered_components.clone())
}

#[tauri::command]
pub fn plugin_get_menu_items(
    engine: State<'_, EngineState>,
    plugin_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let _eng = engine.lock().map_err(|e| e.to_string())?;
    let _plugin = _eng.plugin_manager.get(&plugin_id).ok_or("plugin not found")?;
    Ok(vec![])
}

#[tauri::command]
pub fn plugin_get_export_formats(
    engine: State<'_, EngineState>,
) -> Result<Vec<serde_json::Value>, String> {
    let _eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(vec![])
}

#[tauri::command]
pub fn plugin_call_menu_item(
    engine: State<'_, EngineState>,
    plugin_id: String,
    menu_item_id: u32,
) -> Result<(), String> {
    let _eng = engine.lock().map_err(|e| e.to_string())?;
    let _plugin = _eng.plugin_manager.get(&plugin_id).ok_or("plugin not found")?;
    let _ = menu_item_id;
    Ok(())
}

#[tauri::command]
pub fn plugin_evaluate(
    engine: State<'_, EngineState>,
    plugin_id: String,
    kind_name: String,
    inputs: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let signals: Vec<Signal> = inputs
        .iter()
        .map(|v| {
            if let Some(b) = v.as_bool() {
                if b { Signal::High } else { Signal::Low }
            } else if let Some(n) = v.as_f64() {
                if n.fract() == 0.0 {
                    Signal::Integer(n as i32)
                } else {
                    Signal::Float(n)
                }
            } else if let Some(n) = v.as_i64() {
                Signal::Integer(n as i32)
            } else {
                Signal::Low
            }
        })
        .collect();

    let lua_state = serde_json::json!({});
    let (outputs, _) = eng.plugin_manager.evaluate(&plugin_id, &kind_name, &signals, &lua_state)?;

    let result: Vec<serde_json::Value> = outputs
        .iter()
        .map(|s| match s {
            Signal::Low => serde_json::Value::Number(0.into()),
            Signal::High => serde_json::Value::Number(1.into()),
            Signal::Bus(v) => serde_json::Value::Number((*v).into()),
            Signal::Integer(v) => serde_json::Value::Number((*v).into()),
            Signal::Float(v) => serde_json::json!(*v),
        })
        .collect();
    Ok(result)
}
