use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use super::types::PinId;
use super::graph::CircuitGraph;

pub type SubCircuitDefId = u32;

#[derive(Clone, Serialize, Deserialize)]
pub struct ExternalPin {
    pub name: String,
    pub is_output: bool,
    pub internal_pin_id: PinId,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SubCircuitDef {
    pub id: SubCircuitDefId,
    pub name: String,
    pub description: String,
    pub inner_graph: CircuitGraph,
    pub external_pins: Vec<ExternalPin>,
    pub width: f32,
    pub height: f32,
    pub icon_label: String,
}

pub struct SubCircuitDefRegistry {
    pub defs: HashMap<SubCircuitDefId, SubCircuitDef>,
    pub next_id: u32,
}

impl SubCircuitDefRegistry {
    pub fn new() -> Self {
        Self {
            defs: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create(&mut self, mut def: SubCircuitDef) -> SubCircuitDefId {
        let id = self.next_id;
        self.next_id += 1;
        def.id = id;
        self.defs.insert(id, def);
        id
    }

    pub fn get(&self, id: SubCircuitDefId) -> Option<&SubCircuitDef> {
        self.defs.get(&id)
    }

    pub fn update(&mut self, id: SubCircuitDefId, def: SubCircuitDef) -> Result<(), String> {
        if !self.defs.contains_key(&id) {
            return Err("definition not found".into());
        }
        self.defs.insert(id, def);
        Ok(())
    }

    pub fn remove(&mut self, id: SubCircuitDefId) -> Result<(), String> {
        self.defs.remove(&id).ok_or("definition not found")?;
        Ok(())
    }
}

pub fn check_circular_reference(
    registry: &SubCircuitDefRegistry,
    def_id: SubCircuitDefId,
    visited: &mut HashSet<SubCircuitDefId>,
) -> bool {
    if !visited.insert(def_id) {
        return true;
    }
    if let Some(def) = registry.get(def_id) {
        for comp in def.inner_graph.components.values() {
            if let super::types::ComponentKind::SubCircuit(inner_id) = comp.kind.clone() {
                if check_circular_reference(registry, inner_id, visited) {
                    return true;
                }
            }
        }
    }
    visited.remove(&def_id);
    false
}
