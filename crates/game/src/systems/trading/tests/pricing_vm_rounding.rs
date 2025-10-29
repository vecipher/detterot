use std::collections::HashMap;

use crate::systems::economy::{
    compute_price,
    rulepack::PricingCfg,
    BasisBp,
    CommodityId,
    EconState,
    HubId,
    MoneyCents,
};
use crate::systems::trading::pricing_vm::price_view;

fn unlimited_pricing_cfg() -> PricingCfg {
    PricingCfg {
        min_multiplier_bp: -3_000,
        max_multiplier_bp: 4_000,
    }
}

#[test]
fn price_view_resolves_half_cent_ties_like_compute_price() {
    let pricing = unlimited_pricing_cfg();
    let commodity = CommodityId(1);
    let hub = HubId(7);

    let mut state = EconState::default();
    state.di_bp = HashMap::from([(commodity, BasisBp(-500))]);
    state
        .basis_bp
        .insert((hub, commodity), BasisBp(-500));

    let view = price_view(&state, &pricing);

    let base = MoneyCents(5);
    let expected = compute_price(base, BasisBp(-500), BasisBp(-500), &pricing);
    let quoted = view.quote(hub, commodity, base);
    assert_eq!(quoted, expected, "ties-to-even should match compute_price");

    state.di_bp.insert(commodity, BasisBp(-2_000));
    let view = price_view(&state, &pricing);
    let base = MoneyCents(2);
    let expected = compute_price(base, BasisBp(-2_000), BasisBp(-500), &pricing);
    let quoted = view.quote(hub, commodity, base);
    assert_eq!(quoted, expected, "half-cent rounding up must match compute_price");
}

#[test]
fn price_view_preserves_final_flooring() {
    let pricing = unlimited_pricing_cfg();
    let commodity = CommodityId(2);
    let hub = HubId(3);

    let mut state = EconState::default();
    state.di_bp.insert(commodity, BasisBp(3_333));
    state
        .basis_bp
        .insert((hub, commodity), BasisBp(-444));

    let view = price_view(&state, &pricing);

    let base = MoneyCents(1_234);
    let expected = compute_price(base, BasisBp(3_333), BasisBp(-444), &pricing);
    let quoted = view.quote(hub, commodity, base);
    assert_eq!(quoted, expected, "final floor parity should match compute_price");
}
