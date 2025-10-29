use bevy::prelude::*;

use crate::scheduling::sets;
use crate::systems::save::{CargoSlot, SaveV1_1};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::{TradingView, TradingViewState};
use crate::ui::hub_trade::{ActiveHub, WalletBalance};

#[derive(Resource, Clone)]
pub struct AppSaveState {
    snapshot: SaveV1_1,
    last_view: Option<TradingView>,
}

impl Default for AppSaveState {
    fn default() -> Self {
        Self {
            snapshot: SaveV1_1::default(),
            last_view: None,
        }
    }
}

impl AppSaveState {
    pub fn from_snapshot(snapshot: SaveV1_1) -> Self {
        Self {
            snapshot,
            last_view: None,
        }
    }

    pub fn snapshot(&self) -> &SaveV1_1 {
        &self.snapshot
    }

    pub fn snapshot_mut(&mut self) -> &mut SaveV1_1 {
        &mut self.snapshot
    }

    pub fn clone_snapshot(&self) -> SaveV1_1 {
        self.snapshot.clone()
    }
}

pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppSaveState>()
            .add_systems(Startup, hydrate_hub_resources)
            .add_systems(
                FixedUpdate,
                persist_hub_resources.in_set(sets::DETTEROT_Cleanup),
            );
    }
}

fn hydrate_hub_resources(
    mut save_state: ResMut<AppSaveState>,
    mut cargo: ResMut<Cargo>,
    mut wallet: ResMut<WalletBalance>,
    mut active_hub: ResMut<ActiveHub>,
    view_state: Option<Res<TradingViewState>>,
) {
    let snapshot = save_state.clone_snapshot();
    snapshot.hydrate_cargo(&mut cargo);
    wallet.0 = snapshot.wallet_balance();
    if let Some(last_hub) = snapshot.last_hub {
        active_hub.0 = last_hub;
    }
    save_state.last_view = view_state.map(|state| state.current());
}

fn persist_hub_resources(
    mut save_state: ResMut<AppSaveState>,
    view_state: Option<Res<TradingViewState>>,
    cargo: Option<Res<Cargo>>,
    wallet: Option<Res<WalletBalance>>,
    active_hub: Option<Res<ActiveHub>>,
) {
    let Some(view_state) = view_state else {
        return;
    };

    let current_view = view_state.current();
    let previous_view = save_state.last_view.unwrap_or(current_view);

    if previous_view == TradingView::Trading && current_view != TradingView::Trading {
        let cargo_data = cargo.as_ref().map(|cargo| {
            (
                cargo.capacity_total,
                cargo.capacity_used,
                cargo.mass_capacity_total,
                cargo.mass_capacity_used,
                cargo.manifest_snapshot(),
            )
        });
        let wallet_balance = wallet.map(|wallet| wallet.0);
        let hub_value = active_hub.map(|hub| hub.0);

        let snapshot = save_state.snapshot_mut();
        if let Some((capacity_total, capacity_used, mass_total, mass_used, manifest)) = cargo_data {
            snapshot.cargo.capacity_total = capacity_total;
            snapshot.cargo.capacity_used = capacity_used;
            snapshot.cargo.mass_capacity_total = mass_total;
            snapshot.cargo.mass_capacity_used = mass_used;
            snapshot.cargo.manifest = manifest
                .into_iter()
                .map(|(commodity, units)| CargoSlot { commodity, units })
                .collect();
        }

        if let Some(balance) = wallet_balance {
            snapshot.set_wallet_balance(balance);
        }

        snapshot.set_last_hub(hub_value);
    }

    save_state.last_view = Some(current_view);
}
