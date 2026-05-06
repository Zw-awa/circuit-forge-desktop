use crate::keybindings::{self, KeyBinding, KeyBindingConfig};

#[tauri::command]
pub fn get_keybindings() -> Result<Vec<KeyBinding>, String> {
    let config = keybindings::load_bindings()?;
    Ok(config.bindings)
}

#[tauri::command]
pub fn set_keybinding(action: String, key: String) -> Result<serde_json::Value, String> {
    let mut config = keybindings::load_bindings()?;
    let old_action = config.bindings.iter().find(|b| b.key == key && b.action != action).map(|b| b.action.clone());
    config.bindings.retain(|b| b.key != key || b.action == action);
    if let Some(binding) = config.bindings.iter_mut().find(|b| b.action == action) {
        binding.key = key.clone();
        keybindings::save_bindings(&config)?;
        Ok(serde_json::json!({ "removedAction": old_action }))
    } else {
        Err(format!("unknown action: {}", action))
    }
}

#[tauri::command]
pub fn reset_keybindings() -> Result<Vec<KeyBinding>, String> {
    let default = keybindings::default_bindings();
    keybindings::save_bindings(&default)?;
    Ok(default.bindings)
}

#[tauri::command]
pub fn export_keybindings() -> Result<String, String> {
    let config = keybindings::load_bindings()?;
    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_keybindings(json: String) -> Result<Vec<KeyBinding>, String> {
    let config: KeyBindingConfig =
        serde_json::from_str(&json).map_err(|e| format!("invalid JSON: {}", e))?;
    keybindings::save_bindings(&config)?;
    Ok(config.bindings)
}
