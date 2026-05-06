use std::path::Path;
use tauri::State;
use chrono::Utc;
use crate::EngineState;
use crate::packaging;
use crate::packaging::snapshot::SnapshotInfo;
use crate::project;

#[tauri::command]
pub fn export_circuitforge(
    engine: State<'_, EngineState>,
    name: String,
    path: String,
) -> Result<String, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    packaging::circuitforge::pack_circuitforge(&eng, &name, Path::new(&path))?;
    Ok(path)
}

#[tauri::command]
pub fn import_circuitforge(
    engine: State<'_, EngineState>,
    path: String,
) -> Result<serde_json::Value, String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    packaging::circuitforge::import_circuitforge(&mut eng, Path::new(&path))?;

    Ok(serde_json::json!({
        "success": true,
        "componentCount": eng.graph.components.len(),
        "customComponentCount": eng.subcircuit_registry.defs.len() + eng.lua_registry.defs.len(),
        "rulePackCount": eng.rule_registry.packs.len(),
        "snapshotCount": eng.snapshots.len(),
    }))
}

#[tauri::command]
pub fn create_snapshot_cmd(
    engine: State<'_, EngineState>,
    name: String,
) -> Result<SnapshotInfo, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let circuit_json = project::save::save_project(&eng)?;
    drop(eng);
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    let id = eng.next_snapshot_id;
    let created_at = Utc::now().to_rfc3339();
    eng.snapshots.push((id, name.clone(), created_at.clone(), circuit_json));
    eng.next_snapshot_id += 1;
    Ok(SnapshotInfo { id, name, created_at })
}

#[tauri::command]
pub fn list_snapshots(
    engine: State<'_, EngineState>,
) -> Result<Vec<SnapshotInfo>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let snapshots: Vec<SnapshotInfo> = eng.snapshots.iter().map(|(id, name, created_at, _)| {
        SnapshotInfo {
            id: *id,
            name: name.clone(),
            created_at: created_at.clone(),
        }
    }).collect();
    Ok(snapshots)
}

#[tauri::command]
pub fn restore_snapshot(
    engine: State<'_, EngineState>,
    id: u32,
) -> Result<serde_json::Value, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let json = eng.snapshots.iter()
        .find(|(sid, _, _, _)| *sid == id)
        .map(|(_, _, _, json)| json.clone())
        .ok_or("snapshot not found")?;
    drop(eng);
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    project::load::load_project(&mut eng, &json)?;

    Ok(serde_json::json!({
        "components": eng.graph.components.values().collect::<Vec<_>>(),
        "pins": eng.graph.pins.values().collect::<Vec<_>>(),
        "wires": eng.graph.wires.values().collect::<Vec<_>>(),
    }))
}
