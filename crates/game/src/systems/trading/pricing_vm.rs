use crate::systems::economy::{
    basis::BasisDrivers, compute_price, BasisBp, CommodityId, EconState, HubId, MoneyCents,
    Rulepack, Weather,
};

/// Base price in cents used for quote construction.
pub const DEFAULT_QUOTE_BASE: MoneyCents = MoneyCents(12_345);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradingDrivers {
    pub pp: crate::systems::economy::Pp,
    pub weather: Weather,
    pub closed_routes: u8,
    pub stock_dev: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceView {
    pub di_bp: BasisBp,
    pub basis_bp: BasisBp,
    pub price_cents: MoneyCents,
    pub drivers: TradingDrivers,
}

pub fn price_view(hub: HubId, com: CommodityId, econ: &EconState, rp: &Rulepack) -> PriceView {
    let di_bp = econ.di_bp.get(&com).copied().unwrap_or(BasisBp(0));
    let basis_bp = econ
        .basis_bp
        .get(&(hub, com))
        .copied()
        .unwrap_or(BasisBp(0));

    let price_cents = compute_price(DEFAULT_QUOTE_BASE, di_bp, basis_bp, &rp.pricing);
    let drivers_snapshot = econ
        .basis_drivers
        .get(&hub)
        .copied()
        .unwrap_or(BasisDrivers {
            pp: econ.pp,
            weather: Weather::Clear,
            closed_routes: 0,
            stock_dev: 0,
        });
    let drivers = TradingDrivers {
        pp: drivers_snapshot.pp,
        weather: drivers_snapshot.weather,
        closed_routes: drivers_snapshot.closed_routes,
        stock_dev: drivers_snapshot.stock_dev,
    };

    PriceView {
        di_bp,
        basis_bp,
        price_cents,
        drivers,
    }
}
