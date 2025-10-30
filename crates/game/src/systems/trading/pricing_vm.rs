use anyhow::{anyhow, Result};

use crate::systems::economy::{
    compute_price,
    rulepack::{BasisCfg, Rulepack},
    BasisBp, CommodityId, EconState, HubId, MoneyCents, Pp, Weather,
};

use super::types;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradingDrivers {
    pp: Pp,
    weather: Weather,
    closed_routes: u8,
    stock_dev: i32,
}

impl TradingDrivers {
    pub fn pp(&self) -> Pp {
        self.pp
    }

    pub fn weather(&self) -> Weather {
        self.weather
    }

    pub fn closed_routes(&self) -> u8 {
        self.closed_routes
    }

    pub fn stock_dev(&self) -> i32 {
        self.stock_dev
    }
}

/// Read-only view over the pricing inputs relevant to a single
/// `(hub, commodity)` quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceView {
    di_bp: BasisBp,
    basis_bp: BasisBp,
    price_cents: MoneyCents,
    drivers: TradingDrivers,
}

impl PriceView {
    pub fn di_bp(&self) -> BasisBp {
        self.di_bp
    }

    pub fn basis_bp(&self) -> BasisBp {
        self.basis_bp
    }

    pub fn price_cents(&self) -> MoneyCents {
        self.price_cents
    }

    pub fn drivers(&self) -> TradingDrivers {
        self.drivers
    }
}

fn derive_drivers(state: &EconState, _basis_cfg: &BasisCfg) -> TradingDrivers {
    TradingDrivers {
        pp: state.pp,
        weather: Weather::Clear,
        closed_routes: 0,
        stock_dev: 0,
    }
}

pub fn trading_drivers(state: &EconState, rulepack: &Rulepack) -> TradingDrivers {
    derive_drivers(state, &rulepack.basis)
}

pub fn price_view(
    hub: HubId,
    commodity: CommodityId,
    state: &EconState,
    rulepack: &Rulepack,
) -> Result<PriceView> {
    let Some(spec) = types::commodity_spec(commodity) else {
        return Err(anyhow!("missing commodity metadata for {:?}", commodity));
    };

    let di_bp = state.di_bp.get(&commodity).copied().unwrap_or(BasisBp(0));
    let basis_bp = state
        .basis_bp
        .get(&(hub, commodity))
        .copied()
        .unwrap_or(BasisBp(0));

    let drivers = trading_drivers(state, rulepack);

    let price_cents = compute_price(spec.base_price(), di_bp, basis_bp, &rulepack.pricing);

    Ok(PriceView {
        di_bp,
        basis_bp,
        price_cents,
        drivers,
    })
}
