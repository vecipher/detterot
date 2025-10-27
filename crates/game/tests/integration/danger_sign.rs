use game::systems::director::{danger_diff_sign, danger_score, SpawnBudget};

#[test]
fn danger_diff_matches_sign() {
    let budget_low = SpawnBudget::new(10, 0);
    let budget_high = SpawnBudget::new(20, 0);
    let danger_low = danger_score(&budget_low, 5, 3, 2, 40);
    let danger_high = danger_score(&budget_high, 5, 3, 2, 40);
    assert_eq!(danger_diff_sign(danger_high, danger_low), 1);
    assert_eq!(danger_diff_sign(danger_low, danger_high), -1);
    assert_eq!(danger_diff_sign(danger_low, danger_low), 0);
}
