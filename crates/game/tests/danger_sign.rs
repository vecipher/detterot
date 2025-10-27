use game::systems::director::spawn::{danger_diff_sign, danger_score, SpawnBudget};

#[test]
fn danger_diff_matches_delta() {
    let prev = danger_score(
        &SpawnBudget {
            enemies: 10,
            obstacles: 0,
        },
        10,
        5,
        3,
        50,
    );
    let next = danger_score(
        &SpawnBudget {
            enemies: 12,
            obstacles: 0,
        },
        10,
        5,
        3,
        50,
    );
    assert_eq!(danger_diff_sign(next, prev), 1);
    let lower = danger_score(
        &SpawnBudget {
            enemies: 8,
            obstacles: 0,
        },
        10,
        5,
        3,
        50,
    );
    assert_eq!(danger_diff_sign(lower, next), -1);
    assert_eq!(danger_diff_sign(prev, prev), 0);
}
