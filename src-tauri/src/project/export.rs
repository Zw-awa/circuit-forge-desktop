use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::circuit::subcircuit::{SubCircuitDef, SubCircuitDefRegistry};
use crate::circuit::types::ComponentKind;
use crate::scripting::lua_engine::{LuaComponentDef, LuaComponentDefRegistry};
use crate::verification::truth_table::TruthTable;
use crate::rules::presets::RulePack;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfcompMetadata {
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub created_at: String,
    pub circuit_forge_version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfcompNestedDef {
    pub component_type: String,
    pub definition: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfcompFile {
    pub format: String,
    pub version: u32,
    pub component_type: String,
    pub metadata: CfcompMetadata,
    pub definition: serde_json::Value,
    pub truth_table: Option<TruthTable>,
    pub nested_defs: Vec<CfcompNestedDef>,
}

pub fn export_subcircuit(
    subcircuit_registry: &SubCircuitDefRegistry,
    lua_registry: &LuaComponentDefRegistry,
    truth_tables: &HashMap<u32, TruthTable>,
    def_id: u32,
) -> Result<String, String> {
    let def = subcircuit_registry.get(def_id).ok_or("subcircuit def not found")?;
    let mut visited_sub: HashSet<u32> = HashSet::new();
    let mut visited_lua: HashSet<u32> = HashSet::new();
    let mut nested = Vec::new();
    collect_nested_deps(def, subcircuit_registry, lua_registry, &mut nested, &mut visited_sub, &mut visited_lua);

    let truth_table = truth_tables.values().find(|t| t.target_def_id == def_id).cloned();

    let cfcomp = CfcompFile {
        format: "cfcomp".into(),
        version: 1,
        component_type: "subcircuit".into(),
        metadata: CfcompMetadata {
            name: def.name.clone(),
            description: def.description.clone(),
            author: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            circuit_forge_version: env!("CARGO_PKG_VERSION").into(),
        },
        definition: serde_json::to_value(def).map_err(|e| e.to_string())?,
        truth_table,
        nested_defs: nested,
    };
    serde_json::to_string_pretty(&cfcomp).map_err(|e| e.to_string())
}

pub fn export_lua_component(
    lua_registry: &LuaComponentDefRegistry,
    truth_tables: &HashMap<u32, TruthTable>,
    def_id: u32,
) -> Result<String, String> {
    let def = lua_registry.get(def_id).ok_or("lua component def not found")?;

    let truth_table = truth_tables.values().find(|t| t.target_def_id == def_id).cloned();

    let cfcomp = CfcompFile {
        format: "cfcomp".into(),
        version: 1,
        component_type: "lua".into(),
        metadata: CfcompMetadata {
            name: def.name.clone(),
            description: def.description.clone(),
            author: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            circuit_forge_version: env!("CARGO_PKG_VERSION").into(),
        },
        definition: serde_json::to_value(def).map_err(|e| e.to_string())?,
        truth_table,
        nested_defs: Vec::new(),
    };
    serde_json::to_string_pretty(&cfcomp).map_err(|e| e.to_string())
}

fn collect_nested_deps(
    def: &SubCircuitDef,
    sub_reg: &SubCircuitDefRegistry,
    lua_reg: &LuaComponentDefRegistry,
    nested: &mut Vec<CfcompNestedDef>,
    visited_sub: &mut HashSet<u32>,
    visited_lua: &mut HashSet<u32>,
) {
    for comp in def.inner_graph.components.values() {
        match comp.kind.clone() {
            ComponentKind::SubCircuit(inner_id) => {
                if visited_sub.insert(inner_id) {
                    if let Some(inner_def) = sub_reg.get(inner_id) {
                        nested.push(CfcompNestedDef {
                            component_type: "subcircuit".into(),
                            definition: serde_json::to_value(inner_def).unwrap_or_default(),
                        });
                        collect_nested_deps(inner_def, sub_reg, lua_reg, nested, visited_sub, visited_lua);
                    }
                }
            }
            ComponentKind::LuaScript(inner_id) => {
                if visited_lua.insert(inner_id) {
                    if let Some(inner_def) = lua_reg.get(inner_id) {
                        nested.push(CfcompNestedDef {
                            component_type: "lua".into(),
                            definition: serde_json::to_value(inner_def).unwrap_or_default(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn import_cfcomp(
    json: &str,
    engine: &mut crate::simulation::engine::SimulationEngine,
) -> Result<(u32, String, String), String> {
    let cfcomp: CfcompFile = serde_json::from_str(json).map_err(|e| format!("invalid format: {}", e))?;
    if cfcomp.format != "cfcomp" {
        return Err("not a cfcomp file".into());
    }
    if cfcomp.version != 1 {
        return Err("unsupported version".into());
    }

    // Import nested defs first
    let mut id_remap: HashMap<u32, u32> = HashMap::new();
    for nested in &cfcomp.nested_defs {
        match nested.component_type.as_str() {
            "subcircuit" => {
                let mut def: SubCircuitDef = serde_json::from_value(nested.definition.clone()).map_err(|e| e.to_string())?;
                def.inner_graph.remap_graph_ids();
                let old_id = def.id;
                let new_id = engine.subcircuit_registry.create(def);
                id_remap.insert(old_id, new_id);
            }
            "lua" => {
                let def: LuaComponentDef = serde_json::from_value(nested.definition.clone()).map_err(|e| e.to_string())?;
                let old_id = def.id;
                let new_id = engine.lua_registry.create(def);
                id_remap.insert(old_id, new_id);
            }
            _ => {}
        }
    }

    // Import main definition
    match cfcomp.component_type.as_str() {
        "subcircuit" => {
            let mut def: SubCircuitDef = serde_json::from_value(cfcomp.definition).map_err(|e| e.to_string())?;
            def.inner_graph.remap_graph_ids();
            let base_name = def.name.clone();
            let mut name = base_name.clone();
            let mut suffix = 2;
            while engine.subcircuit_registry.defs.values().any(|d| d.name == name) {
                name = format!("{} ({})", base_name, suffix);
                suffix += 1;
            }
            def.name = name.clone();
            let new_id = engine.subcircuit_registry.create(def);
            if let Some(mut tt) = cfcomp.truth_table {
                tt.target_def_id = new_id;
                tt.id = engine.alloc_truth_table_id();
                engine.truth_tables.insert(tt.id, tt);
            }
            Ok((new_id, "subcircuit".into(), name))
        }
        "lua" => {
            let mut def: LuaComponentDef = serde_json::from_value(cfcomp.definition).map_err(|e| e.to_string())?;
            let base_name = def.name.clone();
            let mut name = base_name.clone();
            let mut suffix = 2;
            while engine.lua_registry.defs.values().any(|d| d.name == name) {
                name = format!("{} ({})", base_name, suffix);
                suffix += 1;
            }
            def.name = name.clone();
            let new_id = engine.lua_registry.create(def);
            if let Some(mut tt) = cfcomp.truth_table {
                tt.target_def_id = new_id;
                tt.id = engine.alloc_truth_table_id();
                engine.truth_tables.insert(tt.id, tt);
            }
            Ok((new_id, "lua".into(), name))
        }
        _ => Err("unknown component type".into()),
    }
}

pub fn export_rule_pack(pack: &RulePack) -> Result<String, String> {
    let envelope = serde_json::json!({
        "format": "cfrule",
        "version": 1,
        "metadata": {
            "name": pack.name,
            "description": pack.description,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        },
        "pack": pack,
    });
    serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())
}

pub fn import_rule_pack(json: &str) -> Result<RulePack, String> {
    let envelope: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("invalid .cfrule: {}", e))?;
    let format = envelope.get("format")
        .and_then(|v| v.as_str())
        .ok_or("missing format field")?;
    if format != "cfrule" {
        return Err(format!("unsupported format: {}", format));
    }
    let version = envelope.get("version")
        .and_then(|v| v.as_u64())
        .ok_or("missing version field")?;
    if version != 1 {
        return Err(format!("unsupported version: {}", version));
    }
    let mut pack: RulePack = serde_json::from_value(
        envelope.get("pack").ok_or("missing pack field")?.clone()
    ).map_err(|e| format!("invalid pack data: {}", e))?;
    pack.id = 0;
    pack.is_preset = false;
    Ok(pack)
}
