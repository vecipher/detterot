use std::collections::HashMap;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::systems::economy::CommodityId;

#[derive(Debug, Default, Resource, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cargo {
    pub capacity_mass_kg: u32,
    pub capacity_volume_l: u32,
    pub items: HashMap<CommodityId, u32>,
}

impl Cargo {
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn units(&self, com: CommodityId) -> u32 {
        self.items.get(&com).copied().unwrap_or_default()
    }
}
