use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct LuaPinDef {
    pub name: String,
    pub is_output: bool,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LuaComponentDef {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub script_source: String,
    pub input_pins: Vec<LuaPinDef>,
    pub output_pins: Vec<LuaPinDef>,
    pub icon_label: String,
    pub width: f32,
    pub height: f32,
}

pub struct LuaComponentDefRegistry {
    pub defs: HashMap<u32, LuaComponentDef>,
    pub next_id: u32,
}

impl LuaComponentDefRegistry {
    pub fn new() -> Self { Self { defs: HashMap::new(), next_id: 1 } }
    pub fn create(&mut self, mut def: LuaComponentDef) -> u32 {
        let id = self.next_id; self.next_id += 1;
        def.id = id; self.defs.insert(id, def); id
    }
    pub fn get(&self, id: u32) -> Option<&LuaComponentDef> { self.defs.get(&id) }
    pub fn update(&mut self, id: u32, def: LuaComponentDef) -> Result<(), String> {
        if !self.defs.contains_key(&id) { return Err("not found".into()); }
        self.defs.insert(id, def); Ok(())
    }
    pub fn remove(&mut self, id: u32) -> Result<(), String> {
        self.defs.remove(&id).ok_or("not found")?; Ok(())
    }
    pub fn list_all(&self) -> Vec<&LuaComponentDef> { self.defs.values().collect() }
}
