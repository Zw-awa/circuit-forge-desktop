use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use crate::simulation::engine::SimulationEngine;
use crate::project;

pub fn pack_circuitforge(engine: &SimulationEngine, project_name: &str, output_path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let circuit_json = project::save::save_project(engine)?;

    let component_count = engine.graph.components.len() as u32;
    let custom_component_count = (engine.subcircuit_registry.defs.len() + engine.lua_registry.defs.len()) as u32;

    let manifest = serde_json::json!({
        "format": "circuitforge",
        "version": 1,
        "name": project_name,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "circuitForgeVersion": env!("CARGO_PKG_VERSION"),
        "componentCount": component_count,
        "customComponentCount": custom_component_count,
        "activeRulePackId": engine.rule_registry.active_id,
    });
    zip.start_file("manifest.json", options).map_err(|e| e.to_string())?;
    zip.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes()).map_err(|e| e.to_string())?;

    zip.start_file("circuit.json", options).map_err(|e| e.to_string())?;
    zip.write_all(circuit_json.as_bytes()).map_err(|e| e.to_string())?;

    for def in engine.subcircuit_registry.defs.values() {
        let json = project::export::export_subcircuit(
            &engine.subcircuit_registry,
            &engine.lua_registry,
            &engine.truth_tables,
            def.id,
        )?;
        let name = format!("components/{}.cfcomp", def.id);
        zip.start_file(&name, options).map_err(|e| e.to_string())?;
        zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    }
    for def in engine.lua_registry.defs.values() {
        let json = project::export::export_lua_component(
            &engine.lua_registry,
            &engine.truth_tables,
            def.id,
        )?;
        let name = format!("components/{}.cfcomp", def.id);
        zip.start_file(&name, options).map_err(|e| e.to_string())?;
        zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    }

    for pack in engine.rule_registry.packs.values() {
        if !pack.is_preset {
            let json = project::export::export_rule_pack(pack)?;
            let name = format!("rules/{}.cfrule", pack.id);
            zip.start_file(&name, options).map_err(|e| e.to_string())?;
            zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    if let Some(skin) = &engine.active_skin {
        let skin_json = serde_json::to_string_pretty(skin).map_err(|e| e.to_string())?;
        zip.start_file("assets/manifest.json", options).map_err(|e| e.to_string())?;
        zip.write_all(skin_json.as_bytes()).map_err(|e| e.to_string())?;
        for (name, data) in &engine.skin_assets {
            zip.start_file(&format!("assets/{}", name), options).map_err(|e| e.to_string())?;
            zip.write_all(data).map_err(|e| e.to_string())?;
        }
    }

    if !engine.snapshots.is_empty() {
        let snapshot_list: Vec<serde_json::Value> = engine.snapshots.iter().map(|(id, name, created_at, _)| {
            serde_json::json!({
                "id": id,
                "name": name,
                "createdAt": created_at,
            })
        }).collect();
        let index_json = serde_json::to_string_pretty(&snapshot_list).map_err(|e| e.to_string())?;
        zip.start_file("snapshots/index.json", options).map_err(|e| e.to_string())?;
        zip.write_all(index_json.as_bytes()).map_err(|e| e.to_string())?;

        for (id, _name, _created_at, data) in &engine.snapshots {
            let name = format!("snapshots/{}.json", id);
            zip.start_file(&name, options).map_err(|e| e.to_string())?;
            zip.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn import_circuitforge(engine: &mut SimulationEngine, path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut manifest: Option<serde_json::Value> = None;
    let mut circuit_json: Option<String> = None;
    let mut component_jsons: Vec<String> = Vec::new();
    let mut rule_jsons: Vec<String> = Vec::new();
    let mut skin_data: Option<String> = None;
    let mut skin_assets: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;

        if name == "manifest.json" {
            manifest = Some(serde_json::from_slice(&buf).map_err(|e| e.to_string())?);
        } else if name == "circuit.json" {
            circuit_json = Some(String::from_utf8(buf).map_err(|e| e.to_string())?);
        } else if name.starts_with("components/") && name.ends_with(".cfcomp") {
            component_jsons.push(String::from_utf8(buf).map_err(|e| e.to_string())?);
        } else if name.starts_with("rules/") && name.ends_with(".cfrule") {
            rule_jsons.push(String::from_utf8(buf).map_err(|e| e.to_string())?);
        } else if name == "assets/manifest.json" || name == "skin/manifest.json" {
            skin_data = Some(String::from_utf8(buf).map_err(|e| e.to_string())?);
        } else if name.starts_with("assets/") {
            let asset_name = name.strip_prefix("assets/").unwrap_or(&name).to_string();
            if asset_name != "manifest.json" {
                skin_assets.insert(asset_name, buf);
            }
        } else if name.starts_with("skin/") {
            let asset_name = name.strip_prefix("skin/").unwrap_or(&name).to_string();
            if asset_name != "manifest.json" {
                skin_assets.insert(asset_name, buf);
            }
        }
    }

    if manifest.is_none() {
        return Err("manifest.json not found in .circuitforge package".into());
    }

    let circuit = circuit_json.ok_or("circuit.json not found")?;
    project::load::load_project(engine, &circuit)?;

    for comp_json in &component_jsons {
        project::export::import_cfcomp(comp_json, engine)?;
    }
    for rule_json in &rule_jsons {
        let pack = project::export::import_rule_pack(rule_json)?;
        let _ = engine.rule_registry.add_custom(pack);
    }

    if let Some(skin_json) = skin_data {
        if let Ok(skin_manifest) = serde_json::from_str::<crate::skin::types::SkinManifest>(&skin_json) {
            engine.active_skin = Some(skin_manifest);
            engine.skin_assets = skin_assets;
        }
    }

    Ok(())
}
