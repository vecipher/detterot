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
        self.items.clear();
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
        let mut cargo = Cargo::default();
        cargo.capacity_total = 10;
        cargo.capacity_used = 6;
        cargo.set_units(CommodityId(5), 3);

        cargo.clear();

        assert_eq!(cargo.capacity_total, 0);
        assert_eq!(cargo.capacity_used, 0);
        assert_eq!(cargo.units(CommodityId(5)), 0);
        assert!(cargo.is_empty());
    }
}
