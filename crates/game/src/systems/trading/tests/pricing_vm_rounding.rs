use std::collections::HashMap;

use crate::systems::economy::{
    compute_price,
    rulepack::{
        BasisCfg, BasisWeatherCfg, DiCfg, InterestCfg, PpCfg, PricingCfg, RotCfg, Rulepack,
        TradingCfg,
    },
    BasisBp, CommodityId, EconState, HubId, MoneyCents,
};
use crate::systems::trading::pricing_vm::price_view;
use crate::systems::trading::types::{self, CommoditySpec};

fn unlimited_pricing_cfg() -> PricingCfg {
    PricingCfg {
        min_multiplier_bp: -3_000,
        max_multiplier_bp: 4_000,
    }
}

fn stub_rulepack(pricing: PricingCfg) -> Rulepack {
    Rulepack {
        di: DiCfg {
            long_run_mean_bp: 0,
            retention_bp: 0,
            noise_sigma_bp: 0,
            noise_clamp_bp: 0,
            per_day_clamp_bp: 0,
            absolute_min_bp: -10_000,
            absolute_max_bp: 10_000,
            overlay_decay_bp: 0,
            overlay_min_bp: -10_000,
            overlay_max_bp: 10_000,
        },
        basis: BasisCfg {
            beta_pp_bp: 0,
            beta_routes_bp: 0,
            beta_stock_bp: 0,
            noise_sigma_bp: 0,
            noise_clamp_bp: 0,
            per_day_clamp_bp: 0,
            absolute_min_bp: -10_000,
            absolute_max_bp: 10_000,
            weather: BasisWeatherCfg {
                clear_bp: 0,
                rains_bp: 0,
                fog_bp: 0,
                windy_bp: 0,
            },
        },
        interest: InterestCfg {
            base_leg_bp: 0,
            linear_leg_bp: 0,
            linear_scale_cents: 1,
            convex_leg_bp: 0,
            convex_gamma_q16: 0,
            per_leg_cap_bp: 0,
        },
        rot: RotCfg {
            rot_floor: 0,
            rot_ceiling: 0,
            rot_decay_per_day: 0,
            conversion_chunk: 1,
            debt_per_chunk_cents: 0,
        },
        pp: PpCfg {
            min_pp: 0,
            max_pp: 100,
            neutral_pp: 50,
            planting_size_to_pp_bp: 0,
            planting_max_age_days: 0,
            decay_per_day_bp: 0,
            pull_strength_bp: 0,
            pull_decay_bp: 0,
        },
        pricing,
        trading: TradingCfg {
            transaction_fee_bp: 0,
        },
    }
}

#[test]
fn price_view_resolves_half_cent_ties_like_compute_price() {
    let _guard = types::global_commodities_guard();
    let pricing = unlimited_pricing_cfg();
    let rulepack = stub_rulepack(pricing.clone());
    let commodity = CommodityId(1);
    let hub = HubId(7);

    let mut state = EconState {
        di_bp: HashMap::from([(commodity, BasisBp(-500))]),
        basis_bp: HashMap::from([((hub, commodity), BasisBp(-500))]),
        ..EconState::default()
    };

    types::clear_global_commodities();
    let spec = CommoditySpec {
        id: commodity,
        slug: "test".to_string(),
        display_name: "Test".to_string(),
        base_price_cents: 5,
        mass_per_unit_kg: 1,
        volume_per_unit_l: 1,
    };
    types::set_global_commodities(types::commodities_from_specs(vec![spec]));
    let view = price_view(hub, commodity, &state, &rulepack).expect("price view");

    let base = MoneyCents(5);
    let expected = compute_price(base, BasisBp(-500), BasisBp(-500), &rulepack.pricing);
    let quoted = view.price_cents();
    assert_eq!(quoted, expected, "ties-to-even should match compute_price");

    state.di_bp.insert(commodity, BasisBp(-2_000));
    let spec = CommoditySpec {
        id: commodity,
        slug: "test".to_string(),
        display_name: "Test".to_string(),
        base_price_cents: 2,
        mass_per_unit_kg: 1,
        volume_per_unit_l: 1,
    };
    types::set_global_commodities(types::commodities_from_specs(vec![spec]));
    let view = price_view(hub, commodity, &state, &rulepack).expect("price view");
    let base = MoneyCents(2);
    let expected = compute_price(base, BasisBp(-2_000), BasisBp(-500), &rulepack.pricing);
    let quoted = view.price_cents();
    assert_eq!(
        quoted, expected,
        "half-cent rounding up must match compute_price"
    );
}

#[test]
fn price_view_preserves_final_flooring() {
    let _guard = types::global_commodities_guard();
    let pricing = unlimited_pricing_cfg();
    let rulepack = stub_rulepack(pricing.clone());
    let commodity = CommodityId(2);
    let hub = HubId(3);

    let state = EconState {
        di_bp: HashMap::from([(commodity, BasisBp(3_333))]),
        basis_bp: HashMap::from([((hub, commodity), BasisBp(-444))]),
        ..EconState::default()
    };

    let base = MoneyCents(1_234);
    let spec = CommoditySpec {
        id: commodity,
        slug: "test".to_string(),
        display_name: "Test".to_string(),
        base_price_cents: base.as_i64(),
        mass_per_unit_kg: 1,
        volume_per_unit_l: 1,
    };
    types::set_global_commodities(types::commodities_from_specs(vec![spec]));
    let view = price_view(hub, commodity, &state, &rulepack).expect("price view");
    let expected = compute_price(base, BasisBp(3_333), BasisBp(-444), &rulepack.pricing);
    let quoted = view.price_cents();
    assert_eq!(
        quoted, expected,
        "final floor parity should match compute_price",
    );
}
