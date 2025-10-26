use crate::systems::economy::{
    bankers_round_cents, compute_price, round_down_to_cents, rulepack::PricingCfg, BasisBp,
    MoneyCents,
};

fn unlimited_pricing_cfg() -> PricingCfg {
    PricingCfg {
        min_multiplier_bp: i32::MIN,
        max_multiplier_bp: i32::MAX,
    }
}

#[test]
fn bankers_rounds_ties_to_even_positive() {
    assert_eq!(bankers_round_cents(15), MoneyCents(2));
    assert_eq!(bankers_round_cents(25), MoneyCents(2));
    assert_eq!(bankers_round_cents(35), MoneyCents(4));
}

#[test]
fn bankers_rounds_ties_to_even_negative() {
    assert_eq!(bankers_round_cents(-15), MoneyCents(-2));
    assert_eq!(bankers_round_cents(-25), MoneyCents(-2));
    assert_eq!(bankers_round_cents(-35), MoneyCents(-4));
}

#[test]
fn bankers_rounds_general_cases() {
    assert_eq!(bankers_round_cents(14), MoneyCents(1));
    assert_eq!(bankers_round_cents(16), MoneyCents(2));
    assert_eq!(bankers_round_cents(-14), MoneyCents(-1));
    assert_eq!(bankers_round_cents(-16), MoneyCents(-2));
}

#[test]
fn floor_rounds_down_to_cents() {
    assert_eq!(round_down_to_cents(123), MoneyCents(12));
    assert_eq!(round_down_to_cents(120), MoneyCents(12));
    assert_eq!(round_down_to_cents(-123), MoneyCents(-13));
    assert_eq!(round_down_to_cents(-120), MoneyCents(-12));
}

#[test]
fn compute_price_handles_half_cent_ties() {
    let base = MoneyCents(2);
    let di = BasisBp(-2000);
    let basis = BasisBp(-500);
    let pricing = unlimited_pricing_cfg();
    let rounded_up = compute_price(base, di, basis, &pricing);
    assert_eq!(rounded_up, MoneyCents(2));

    let base = MoneyCents(5);
    let di = BasisBp(-500);
    let basis = BasisBp(-500);
    let rounded_down = compute_price(base, di, basis, &pricing);
    assert_eq!(rounded_down, MoneyCents(4));
}

#[test]
fn compute_price_monotonic_in_drivers() {
    let base = MoneyCents(10_000);
    let pricing = unlimited_pricing_cfg();
    let mut previous = compute_price(base, BasisBp(-1_000), BasisBp(-1_000), &pricing);
    for delta in (-1_000..=1_000).step_by(250) {
        let price = compute_price(base, BasisBp(delta), BasisBp(-1_000), &pricing);
        assert!(price.0 >= previous.0);
        previous = price;
    }

    let mut previous = compute_price(base, BasisBp(0), BasisBp(-1_000), &pricing);
    for delta in (-1_000..=1_000).step_by(250) {
        let price = compute_price(base, BasisBp(0), BasisBp(delta), &pricing);
        assert!(price.0 >= previous.0);
        previous = price;
    }
}

#[test]
fn compute_price_saturates_on_extreme_inputs() {
    let pricing = unlimited_pricing_cfg();
    let base = MoneyCents(i64::MAX);
    let di = BasisBp(500_000);
    let basis = BasisBp(500_000);
    let price = compute_price(base, di, basis, &pricing);
    assert_eq!(price, MoneyCents(i64::MAX));

    let base = MoneyCents(i64::MAX);
    let di = BasisBp(i32::MIN / 2);
    let basis = BasisBp(i32::MIN / 2);
    let price = compute_price(base, di, basis, &pricing);
    assert_eq!(price, MoneyCents(i64::MIN));
}

#[test]
fn compute_price_respects_pricing_bounds() {
    let pricing = PricingCfg {
        min_multiplier_bp: -1_000,
        max_multiplier_bp: 1_000,
    };
    let base = MoneyCents(100);

    let price_high = compute_price(base, BasisBp(2_500), BasisBp(0), &pricing);
    assert_eq!(price_high, MoneyCents(110));

    let price_low = compute_price(base, BasisBp(-5_000), BasisBp(0), &pricing);
    assert_eq!(price_low, MoneyCents(90));
}
