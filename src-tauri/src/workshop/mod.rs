pub mod types;
pub mod fetch;

use std::path::Path;
use crate::simulation::engine::SimulationEngine;
use crate::skin;

pub fn import_item(
    engine: &mut SimulationEngine,
    bytes: &[u8],
    file_type: &str,
) -> Result<serde_json::Value, String> {
    match file_type {
        "cfcomp" => {
            let json = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
            let (new_id, component_type, name) =
                crate::project::export::import_cfcomp(&json, engine)?;
            Ok(serde_json::json!({
                "success": true,
                "defId": new_id,
                "componentType": component_type,
                "name": name,
            }))
        }
        "cfskin" => {
            let uniq = format!(
                "cf_workshop_skin_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            let tmp = std::env::temp_dir().join(uniq);
            std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
            let (manifest, assets) = skin::pack::unpack_skin(Path::new(&tmp))?;
            engine.active_skin = Some(manifest.clone());
            engine.skin_assets = assets.clone();
            let _ = std::fs::remove_file(&tmp);

            use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
            let base64_assets: std::collections::HashMap<String, String> = assets
                .iter()
                .map(|(k, v)| (k.clone(), BASE64.encode(v)))
                .collect();

            Ok(serde_json::json!({
                "success": true,
                "manifest": manifest,
                "assets": base64_assets,
            }))
        }
        "cfrule" => {
            let json = String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())?;
            let pack = crate::project::export::import_rule_pack(&json)?;
            let name = pack.name.clone();
            let new_id = engine.rule_registry.add_custom(pack);
            Ok(serde_json::json!({
                "success": true,
                "defId": new_id,
                "name": name,
            }))
        }
        "circuitforge" => {
            let uniq = format!(
                "cf_workshop_proj_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            let tmp = std::env::temp_dir().join(uniq);
            std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
            let result = crate::packaging::circuitforge::import_circuitforge(
                engine,
                Path::new(&tmp),
            );
            let _ = std::fs::remove_file(&tmp);
            result?;
            Ok(serde_json::json!({
                "success": true,
                "componentCount": engine.graph.components.len(),
                "customComponentCount": engine.subcircuit_registry.defs.len() + engine.lua_registry.defs.len(),
                "rulePackCount": engine.rule_registry.packs.len(),
            }))
        }
        _ => Err(format!("unsupported file type: {}", file_type)),
    }
}
