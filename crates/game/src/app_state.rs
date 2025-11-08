use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::systems::economy::state::RngCursor;
use crate::systems::economy::{EconState, HubId, MoneyCents, RouteId};
use crate::systems::save::InventorySlot;
use crate::systems::trading::inventory::Cargo;

#[derive(Debug, Clone, Resource, Serialize, Deserialize)]
pub struct AppState {
    pub econ_version: u32,
    pub world_seed: u64,
    pub econ: EconState,
    pub last_hub: HubId,
    pub inventory: Vec<InventorySlot>,
    pub cargo: Cargo,
    pub rng_cursors: Vec<RngCursor>,
    pub wallet: MoneyCents,
    pub last_board_hash: u64,
    pub visited_links: Vec<RouteId>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            econ_version: 1,
            world_seed: 0,
            econ: EconState::default(),
            last_hub: HubId::default(),
            inventory: Vec::new(),
            cargo: Cargo::default(),
            rng_cursors: Vec::new(),
            wallet: MoneyCents::ZERO,
            last_board_hash: 0,
            visited_links: Vec::new(),
        }
    }
}

impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        self.econ_version == other.econ_version
            && self.world_seed == other.world_seed
            && self.last_hub == other.last_hub
            && self.inventory == other.inventory
            && self.cargo == other.cargo
            && self.rng_cursors == other.rng_cursors
            && self.wallet == other.wallet
            && self.last_board_hash == other.last_board_hash
            && self.visited_links == other.visited_links
            && econ_eq(&self.econ, &other.econ)
    }
}

impl Eq for AppState {}

fn econ_eq(a: &EconState, b: &EconState) -> bool {
    a.day == b.day
        && a.di_bp == b.di_bp
        && a.di_overlay_bp == b.di_overlay_bp
        && a.basis_bp == b.basis_bp
        && a.pp == b.pp
        && a.rot_u16 == b.rot_u16
        && a.pending_planting == b.pending_planting
        && a.debt_cents == b.debt_cents
}
