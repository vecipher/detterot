use std::collections::HashMap;

use bevy::prelude::Resource;

use crate::systems::economy::CommodityId;

/// Ship cargo manifest tracked as a Bevy resource.
///
/// TODO(v1.1): introduce serde-friendly mirror structs so this resource can
/// round-trip through the save-game format without leaking Bevy-specific
/// internals.
#[derive(Debug, Default, Resource)]
pub struct Cargo {
    /// Total cargo capacity available to the ship.
    pub capacity_total: u32,
    /// Portion of the cargo hold currently in use.
    pub capacity_used: u32,
    /// Maximum mass that can be carried across all cargo (in arbitrary units).
    pub mass_capacity_total: u32,
    /// Mass currently consumed by the cargo manifest.
    pub mass_capacity_used: u32,
    items: HashMap<CommodityId, u32>,
}

impl Cargo {
    /// Returns the number of units carried for the requested commodity.
    pub fn units(&self, commodity: CommodityId) -> u32 {
        self.items.get(&commodity).copied().unwrap_or_default()
    }

    /// Sets the number of units for a commodity, pruning zero entries.
    pub fn set_units(&mut self, commodity: CommodityId, units: u32) {
        if units == 0 {
            self.items.remove(&commodity);
        } else {
            self.items.insert(commodity, units);
        }
    }

    /// Whether the cargo manifest currently contains any commodities.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Removes all cargo and resets capacity counters.
    pub fn clear(&mut self) {
        self.capacity_total = 0;
        self.capacity_used = 0;
        self.mass_capacity_total = 0;
        self.mass_capacity_used = 0;
        self.items.clear();
    }

    /// Snapshot of the manifest sorted by commodity identifier.
    pub fn manifest_snapshot(&self) -> Vec<(CommodityId, u32)> {
        let mut entries: Vec<_> = self
            .items
            .iter()
            .map(|(commodity, units)| (*commodity, *units))
            .collect();
        entries.sort_by_key(|(commodity, _)| commodity.0);
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_defaults_to_zero_for_missing_entries() {
        let cargo = Cargo::default();
        assert_eq!(cargo.units(CommodityId(1)), 0);
    }

    #[test]
    fn clear_resets_capacity_and_inventory() {
        let mut cargo = Cargo {
            capacity_total: 10,
            capacity_used: 6,
            ..Default::default()
        };
        cargo.set_units(CommodityId(5), 3);

        cargo.clear();

        assert_eq!(cargo.capacity_total, 0);
        assert_eq!(cargo.capacity_used, 0);
        assert_eq!(cargo.units(CommodityId(5)), 0);
        assert!(cargo.is_empty());
    }

    #[test]
    fn manifest_snapshot_returns_sorted_entries() {
        let mut cargo = Cargo::default();
        cargo.set_units(CommodityId(5), 3);
        cargo.set_units(CommodityId(2), 1);
        cargo.set_units(CommodityId(7), 0);

        let manifest = cargo.manifest_snapshot();
        assert_eq!(manifest, vec![(CommodityId(2), 1), (CommodityId(5), 3)]);
    }
}
