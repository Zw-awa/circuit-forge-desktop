use std::collections::HashMap;
use std::path::Path;
use tauri::State;
use crate::EngineState;
use crate::skin::types::SkinManifest;
use crate::skin::pack;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[tauri::command]
pub fn load_skin_pack(
    engine: State<'_, EngineState>,
    path: String,
) -> Result<serde_json::Value, String> {
    let (manifest, assets) = pack::unpack_skin(Path::new(&path))?;
    let mut eng = engine.lock().map_err(|e| e.to_string())?;

    let base64_assets: HashMap<String, String> = assets
        .iter()
        .map(|(k, v)| (k.clone(), BASE64.encode(v)))
        .collect();

    eng.active_skin = Some(manifest.clone());
    eng.skin_assets = assets;

    Ok(serde_json::json!({
        "manifest": manifest,
        "assets": base64_assets,
    }))
}

#[tauri::command]
pub fn get_active_skin(
    engine: State<'_, EngineState>,
) -> Result<Option<SkinManifest>, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(eng.active_skin.clone())
}

#[tauri::command]
pub fn set_active_skin(
    engine: State<'_, EngineState>,
    manifest_json: String,
) -> Result<(), String> {
    let manifest: SkinManifest = serde_json::from_str(&manifest_json).map_err(|e| e.to_string())?;
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.active_skin = Some(manifest);
    Ok(())
}

#[tauri::command]
pub fn get_skin_asset(
    engine: State<'_, EngineState>,
    name: String,
) -> Result<String, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    let data = eng.skin_assets.get(&name).ok_or("asset not found")?;
    Ok(BASE64.encode(data))
}

#[tauri::command]
pub fn clear_skin(
    engine: State<'_, EngineState>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.active_skin = None;
    eng.skin_assets.clear();
    Ok(())
}

#[tauri::command]
pub fn export_skin_pack(
    engine: State<'_, EngineState>,
    path: String,
    manifest_json: String,
) -> Result<(), String> {
    let manifest: SkinManifest = serde_json::from_str(&manifest_json)
        .map_err(|e| format!("invalid manifest: {}", e))?;

    let (assets, tmp) = {
        let eng = engine.lock().map_err(|e| e.to_string())?;
        let assets = eng.skin_assets.clone();
        let uniq = format!(
            "cf_skin_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let tmp = std::env::temp_dir().join(uniq);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        (assets, tmp)
    };

    for (name, data) in &assets {
        let asset_path = tmp.join(name);
        std::fs::write(&asset_path, data).map_err(|e| e.to_string())?;
    }

    let result = pack::pack_skin(&manifest, &tmp, Path::new(&path));
    let _ = std::fs::remove_dir_all(&tmp);
    result
}
