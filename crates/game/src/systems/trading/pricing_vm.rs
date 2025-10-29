use std::collections::HashMap;

use crate::systems::economy::{
    compute_price,
    rulepack::PricingCfg,
    BasisBp,
    CommodityId,
    EconState,
    HubId,
    MoneyCents,
    Pp,
    Weather,
};

/// Read-only view over the economy pricing inputs required to quote trades.
///
/// The view borrows the large DI/Basis hash maps from the [`EconState`] so
/// that downstream systems can perform pricing lookups without cloning the
/// underlying state. Driver style scalar inputs (PP, weather, route closures,
/// stock deviation) are exposed via accessors to emphasise the read-only
/// nature of the view.
pub struct PriceView<'a> {
    di_bp: &'a HashMap<CommodityId, BasisBp>,
    basis_bp: &'a HashMap<(HubId, CommodityId), BasisBp>,
    pricing: &'a PricingCfg,
    pp: Pp,
    weather: Weather,
    closed_routes: u8,
    stock_dev: i32,
}

impl<'a> PriceView<'a> {
    /// Returns the global daily index (DI) multiplier for a commodity.
    pub fn di_bp(&self, commodity: CommodityId) -> BasisBp {
        self.di_bp
            .get(&commodity)
            .copied()
            .unwrap_or(BasisBp(0))
    }

    /// Returns the hub-specific basis multiplier for a commodity quote.
    pub fn basis_bp(&self, hub: HubId, commodity: CommodityId) -> BasisBp {
        self.basis_bp
            .get(&(hub, commodity))
            .copied()
            .unwrap_or(BasisBp(0))
    }

    /// Player power driver extracted from the economy state.
    pub fn pp(&self) -> Pp {
        self.pp
    }

    /// Active weather driver affecting basis calculations.
    pub fn weather(&self) -> Weather {
        self.weather
    }

    /// Number of currently closed trade routes acting as a driver.
    pub fn closed_routes(&self) -> u8 {
        self.closed_routes
    }

    /// Warehouse stock deviation driver contributing to basis moves.
    pub fn stock_dev(&self) -> i32 {
        self.stock_dev
    }

    /// Computes a quoted transaction price by applying the current DI and basis
    /// multipliers to the provided base price.
    pub fn quote(
        &self,
        hub: HubId,
        commodity: CommodityId,
        base_price: MoneyCents,
    ) -> MoneyCents {
        let di = self.di_bp(commodity);
        let basis = self.basis_bp(hub, commodity);
        compute_price(base_price, di, basis, self.pricing)
    }
}

/// Creates a [`PriceView`] borrowing the underlying [`EconState`] so that
/// pricing lookups can be performed without copying large state maps.
pub fn price_view<'a>(state: &'a EconState, pricing: &'a PricingCfg) -> PriceView<'a> {
    PriceView {
        di_bp: &state.di_bp,
        basis_bp: &state.basis_bp,
        pricing,
        pp: state.pp,
        // Weather/route closures/stock deviation are currently sourced from the
        // deterministic economy step, which feeds clear weather and zeroed
        // modifiers into the basis dynamics. They are surfaced here so that
        // trading systems can treat them as read-only drivers when richer data
        // becomes available.
        weather: Weather::Clear,
        closed_routes: 0,
        stock_dev: 0,
    }
}
