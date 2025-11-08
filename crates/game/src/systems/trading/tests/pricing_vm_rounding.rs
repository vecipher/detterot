use std::path::PathBuf;

use crate::systems::economy::rulepack::load_rulepack;
use crate::systems::economy::{compute_price, BasisBp, CommodityId, EconState, HubId, Rulepack};
use crate::systems::trading::pricing_vm::{price_view, DEFAULT_QUOTE_BASE};

fn workspace_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

fn setup_state(di: BasisBp, basis: BasisBp) -> EconState {
    let mut state = EconState::default();
    state.di_bp.insert(CommodityId(1), di);
    state.basis_bp.insert((HubId(1), CommodityId(1)), basis);
    state
}

fn load_rulepack_fixture() -> Rulepack {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack")
}

#[test]
fn view_matches_compute_price_across_range() {
    let rp = load_rulepack_fixture();
    for di in (-1_200..=1_200).step_by(150) {
        for basis in (-1_200..=1_200).step_by(150) {
            let di_bp = BasisBp(di);
            let basis_bp = BasisBp(basis);
            let econ = setup_state(di_bp, basis_bp);
            let view = price_view(HubId(1), CommodityId(1), &econ, &rp);
            let expected = compute_price(DEFAULT_QUOTE_BASE, di_bp, basis_bp, &rp.pricing);
            assert_eq!(view.price_cents, expected, "di={di} basis={basis}");
        }
    }
}

#[test]
fn ties_round_to_even_cent() {
    let rp = load_rulepack_fixture();
    let min = i64::from(rp.pricing.min_multiplier_bp);
    let max = i64::from(rp.pricing.max_multiplier_bp);
    let mut found = None;
    'outer: for di in (-4_000..=4_000).step_by(25) {
        for basis in (-4_000..=4_000).step_by(25) {
            let drivers = i64::from(di) + i64::from(basis);
            let clamped = drivers.clamp(min, max);
            let multiplier = 10_000 + clamped;
            let intermediate = i128::from(DEFAULT_QUOTE_BASE.as_i64())
                .saturating_mul(i128::from(multiplier))
                .saturating_mul(10);
            let remainder = intermediate % 10_000;
            if remainder == 0 {
                continue;
            }
            let milli = intermediate / 10_000;
            let unit = milli % 10;
            if unit == 5 || unit == -5 {
                found = Some((di, basis));
                break 'outer;
            }
        }
    }

    let (di, basis) = found.expect("tie case");
    let di_bp = BasisBp(di);
    let basis_bp = BasisBp(basis);
    let econ = setup_state(di_bp, basis_bp);
    let rp = load_rulepack_fixture();
    let view = price_view(HubId(1), CommodityId(1), &econ, &rp);
    let expected = compute_price(DEFAULT_QUOTE_BASE, di_bp, basis_bp, &rp.pricing);
    assert_eq!(view.price_cents, expected);
    assert_eq!(
        view.price_cents.as_i64() % 2,
        0,
        "expected even-cent rounding for tie"
    );
}

#[test]
fn floor_applied_to_final_price() {
    let rp = load_rulepack_fixture();
    // Construct a case where the unclamped price would include fractional cents
    let di_bp = BasisBp(3_333);
    let basis_bp = BasisBp(1_111);
    let econ = setup_state(di_bp, basis_bp);
    let view = price_view(HubId(1), CommodityId(1), &econ, &rp);
    let expected = compute_price(DEFAULT_QUOTE_BASE, di_bp, basis_bp, &rp.pricing);
    assert_eq!(view.price_cents, expected);
    // Floor ensures we never exceed theoretical value even if rounding nudges up
    let base_cents = DEFAULT_QUOTE_BASE.as_i64();
    let drivers = i64::from(di_bp.0) + i64::from(basis_bp.0);
    let clamped = drivers.clamp(
        i64::from(rp.pricing.min_multiplier_bp),
        i64::from(rp.pricing.max_multiplier_bp),
    );
    let multiplier = 10_000 + clamped;
    let theoretical = (i128::from(base_cents)
        .saturating_mul(i128::from(multiplier))
        .saturating_mul(10))
        / 10_000;
    let floored = theoretical / 10;
    assert!(i64::from(view.price_cents) <= floored as i64);
}
