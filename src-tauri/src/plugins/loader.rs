use std::collections::HashMap;
use std::path::{Path, PathBuf};
use mlua::{Lua, Function, Table, Value, LuaSerdeExt};
use serde_json;
use crate::plugins::manifest::{PluginManifest, PluginInfo};
use crate::circuit::types::Signal;

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub lua: Lua,
    pub enabled: bool,
    pub registered_components: Vec<ComponentRegistration>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ComponentRegistration {
    pub kind_name: String,
    pub input_pins: Vec<PinDef>,
    pub output_pins: Vec<PinDef>,
    pub icon_label: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PinDef {
    pub name: String,
    pub is_output: bool,
    pub offset_x: f32,
    pub offset_y: f32,
}

use serde::{Deserialize, Serialize};

pub struct PluginManager {
    pub plugins: HashMap<String, LoadedPlugin>,
    pub plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dir: PathBuf::from("plugins"),
        }
    }

    pub fn get(&self, plugin_id: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(plugin_id)
    }

    pub fn load(&mut self, path: &Path) -> Result<PluginInfo, String> {
        let manifest_path = path.join("plugin.json");
        let manifest_str =
            std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let manifest: PluginManifest =
            serde_json::from_str(&manifest_str).map_err(|e| e.to_string())?;

        let lua = Lua::new();
        // Sandbox: disable dangerous APIs
        lua.globals()
            .set("io", Value::Nil)
            .map_err(|e| e.to_string())?;
        lua.globals()
            .set("os", Value::Nil)
            .map_err(|e| e.to_string())?;
        lua.globals()
            .set("debug", Value::Nil)
            .map_err(|e| e.to_string())?;
        lua.globals()
            .set("loadfile", Value::Nil)
            .map_err(|e| e.to_string())?;
        lua.globals()
            .set("dofile", Value::Nil)
            .map_err(|e| e.to_string())?;
        lua.globals()
            .set("require", Value::Nil)
            .map_err(|e| e.to_string())?;
        lua.set_memory_limit(16 * 1024 * 1024)
            .map_err(|e| e.to_string())?;
        let total = std::cell::Cell::new(0u64);
        let batch: u64 = 100_000;
        lua.set_hook(
            mlua::HookTriggers::default().every_nth_instruction(batch as u32),
            move |_lua, _debug| {
                let t = total.get() + batch;
                total.set(t);
                if t >= 1_000_000 {
                    return Err(mlua::Error::external("instruction limit exceeded"));
                }
                Ok(mlua::VmState::Continue)
            },
        ).map_err(|e| e.to_string())?;

        let main_path = path.join(&manifest.main);
        let script =
            std::fs::read_to_string(&main_path).map_err(|e| e.to_string())?;
        register_cf_api(&lua)?;

        lua.load(&script)
            .exec()
            .map_err(|e| format!("Lua error: {}", e))?;

        let registered = collect_registrations(&lua)?;

        let info = PluginInfo {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            author: manifest.author.clone(),
            enabled: true,
        };

        self.plugins.insert(
            manifest.id.clone(),
            LoadedPlugin {
                manifest,
                lua,
                enabled: true,
                registered_components: registered,
            },
        );

        Ok(info)
    }

    pub fn unload(&mut self, plugin_id: &str) -> Result<(), String> {
        self.plugins
            .remove(plugin_id)
            .ok_or("plugin not found")?;
        Ok(())
    }

    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins
            .values()
            .map(|p| PluginInfo {
                id: p.manifest.id.clone(),
                name: p.manifest.name.clone(),
                version: p.manifest.version.clone(),
                description: p.manifest.description.clone(),
                author: p.manifest.author.clone(),
                enabled: p.enabled,
            })
            .collect()
    }

    pub fn set_enabled(&mut self, plugin_id: &str, enabled: bool) -> Result<(), String> {
        let p = self.plugins.get_mut(plugin_id).ok_or("not found")?;
        p.enabled = enabled;
        Ok(())
    }

    /// Call the evaluate function registered for a plugin component kind.
    /// Returns outputs as Vec<Signal>.
    pub fn evaluate(
        &self,
        plugin_id: &str,
        kind_name: &str,
        inputs: &[Signal],
        lua_state: &serde_json::Value,
    ) -> Result<(Vec<Signal>, serde_json::Value), String> {
        let plugin = self.plugins.get(plugin_id).ok_or("plugin not found")?;
        if !plugin.enabled {
            return Err("plugin disabled".into());
        }

        let eval_table: Table = plugin
            .lua
            .globals()
            .get("__plugin_evaluators")
            .map_err(|e| format!("no evaluators table: {}", e))?;

        let eval_fn: Function = eval_table
            .get(kind_name)
            .map_err(|e| format!("no evaluator for kind '{}': {}", kind_name, e))?;

        let input_table = plugin.lua.create_table().map_err(|e| e.to_string())?;
        for (i, signal) in inputs.iter().enumerate() {
            let val: f64 = match signal {
                Signal::Low => 0.0,
                Signal::High => 1.0,
                Signal::Bus(v) => *v as f64,
                Signal::Integer(v) => *v as f64,
                Signal::Float(v) => *v,
            };
            input_table.set(i + 1, val).map_err(|e| e.to_string())?;
        }

        let state_val: Value = plugin
            .lua
            .to_value(lua_state)
            .map_err(|e| format!("state conversion: {}", e))?;

        let result: (Table, Value) = eval_fn
            .call((input_table, state_val))
            .map_err(|e| format!("plugin evaluate error: {}", e))?;

        let outputs_table = result.0;
        let new_state_value = result.1;

        let new_state: serde_json::Value = plugin
            .lua
            .from_value(new_state_value)
            .map_err(|e| format!("new state conversion: {}", e))?;

        let mut outputs = Vec::new();
        let len: usize = outputs_table.raw_len();
        for i in 0..len {
            let val: f64 = outputs_table.get(i + 1).unwrap_or(0.0);
            outputs.push(if val == 0.0 {
                Signal::Low
            } else if val == 1.0 {
                Signal::High
            } else if val.fract() == 0.0 {
                Signal::Integer(val as i32)
            } else {
                Signal::Float(val)
            });
        }

        Ok((outputs, new_state))
    }
}

fn register_cf_api(lua: &Lua) -> Result<(), String> {
    let cf_table = lua.create_table().map_err(|e| e.to_string())?;

    cf_table
        .set(
            "register_component",
            lua.create_function(
                |lua, (kind, inputs, outputs, label, eval): (String, Table, Table, String, Function)| {
                    let reg_table: Table = lua
                        .globals()
                        .get("__plugin_registrations")
                        .unwrap_or_else(|_| lua.create_table().unwrap());
                    let comps: Table = reg_table
                        .get("components")
                        .unwrap_or_else(|_| lua.create_table().unwrap());
                    let entry = lua.create_table()?;
                    entry.set("kind", kind.clone())?;
                    entry.set("inputs", inputs)?;
                    entry.set("outputs", outputs)?;
                    entry.set("label", label)?;
                    comps.raw_push(entry)?;
                    reg_table.set("components", comps)?;
                    lua.globals()
                        .set("__plugin_registrations", reg_table)?;

                    let eval_table: Table = lua
                        .globals()
                        .get("__plugin_evaluators")
                        .unwrap_or_else(|_| lua.create_table().unwrap());
                    eval_table.set(kind, eval)?;
                    lua.globals().set("__plugin_evaluators", eval_table)?;

                    Ok(())
                },
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

    lua.globals()
        .set("cf", cf_table)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn collect_registrations(lua: &Lua) -> Result<Vec<ComponentRegistration>, String> {
    let reg_table: Table = lua
        .globals()
        .get("__plugin_registrations")
        .unwrap_or_else(|_| lua.create_table().unwrap());
    let comps: Table = reg_table
        .get("components")
        .unwrap_or_else(|_| lua.create_table().unwrap());
    let mut result = Vec::new();
    let len: usize = comps.raw_len();
    for i in 1..=len {
        if let Ok(entry) = comps.get::<Table>(i) {
            let kind: String = entry.get("kind").map_err(|e| e.to_string())?;
            let label: String = entry.get("label").map_err(|e| e.to_string())?;

            let input_pins = extract_pins(&entry, "inputs")?;
            let output_pins = extract_pins(&entry, "outputs")?;

            result.push(ComponentRegistration {
                kind_name: kind,
                input_pins,
                output_pins,
                icon_label: label,
            });
        }
    }
    Ok(result)
}

fn extract_pins(entry: &Table, key: &str) -> Result<Vec<PinDef>, String> {
    let pins_table: Table = match entry.get(key) {
        Ok(t) => t,
        Err(_) => return Ok(Vec::new()),
    };
    let mut pins = Vec::new();
    let len: usize = pins_table.raw_len();
    for i in 1..=len {
        if let Ok(pin_entry) = pins_table.get::<Table>(i) {
            let name: String = pin_entry.get("name").unwrap_or_else(|_| String::from("unnamed"));
            let is_output: bool = pin_entry.get("is_output").unwrap_or(false);
            let offset_x: f32 = pin_entry.get("offset_x").unwrap_or(0.0);
            let offset_y: f32 = pin_entry.get("offset_y").unwrap_or(0.0);
            pins.push(PinDef { name, is_output, offset_x, offset_y });
        }
    }
    Ok(pins)
}
