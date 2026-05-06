use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::circuit::types::{SignalType, PropagationMode, AttenuationModel, TickBehavior};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePack {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_preset: bool,
    pub signal_type: SignalType,
    pub propagation_mode: PropagationMode,
    pub attenuation: AttenuationModel,
    pub tick_behavior: TickBehavior,
    pub gate_delay: u32,
}

pub struct RulePackRegistry {
    pub packs: HashMap<u32, RulePack>,
    pub active_id: u32,
    pub next_id: u32,
}

impl RulePackRegistry {
    pub fn new() -> Self {
        let mut packs = HashMap::new();
        packs.insert(
            1,
            RulePack {
                id: 1,
                name: "MC Redstone".into(),
                description: "Minecraft-style: 0-15 integer, linear decay".into(),
                is_preset: true,
                signal_type: SignalType::Integer { min: 0, max: 15 },
                propagation_mode: PropagationMode::TickDriven,
                attenuation: AttenuationModel::Linear { loss_per_unit: 1 },
                tick_behavior: TickBehavior::Synchronous,
                gate_delay: 1,
            },
        );
        packs.insert(
            2,
            RulePack {
                id: 2,
                name: "Terraria".into(),
                description: "Terraria-style: digital 0/1, event-driven".into(),
                is_preset: true,
                signal_type: SignalType::Bit,
                propagation_mode: PropagationMode::EventDriven,
                attenuation: AttenuationModel::None,
                tick_behavior: TickBehavior::Asynchronous,
                gate_delay: 0,
            },
        );
        packs.insert(
            3,
            RulePack {
                id: 3,
                name: "Standard Digital".into(),
                description: "Standard digital logic: 0/1, event-driven, 1-tick gate delay".into(),
                is_preset: true,
                signal_type: SignalType::Bit,
                propagation_mode: PropagationMode::EventDriven,
                attenuation: AttenuationModel::None,
                tick_behavior: TickBehavior::Asynchronous,
                gate_delay: 1,
            },
        );
        Self {
            packs,
            active_id: 3,
            next_id: 100,
        }
    }

    pub fn active(&self) -> &RulePack {
        &self.packs[&self.active_id]
    }

    pub fn set_active(&mut self, id: u32) -> Result<(), String> {
        if !self.packs.contains_key(&id) {
            return Err("rule pack not found".into());
        }
        self.active_id = id;
        Ok(())
    }

    pub fn add_custom(&mut self, mut pack: RulePack) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        pack.id = id;
        pack.is_preset = false;
        self.packs.insert(id, pack);
        id
    }

    pub fn remove_custom(&mut self, id: u32) -> Result<(), String> {
        let pack = self.packs.get(&id).ok_or("not found")?;
        if pack.is_preset {
            return Err("cannot delete preset".into());
        }
        self.packs.remove(&id);
        Ok(())
    }

    pub fn list_all(&self) -> Vec<&RulePack> {
        self.packs.values().collect()
    }
}
