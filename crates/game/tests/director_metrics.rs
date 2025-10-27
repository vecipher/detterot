use std::path::PathBuf;

use game::systems::director::{
    config::load_director_cfg, danger_diff_sign, danger_score, spawn::compute_spawn_budget,
};
use game::systems::economy::types::{Pp, Weather};

fn cfg_path() -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "assets",
        "director",
        "m2.toml",
    ]
    .into_iter()
    .collect()
}

#[test]
fn spawn_budget_monotonic_and_capped() {
    let cfg = load_director_cfg(cfg_path()).expect("director config loads");
    let weathers = [Weather::Clear, Weather::Rains, Weather::Fog, Weather::Windy];
    for weather in weathers {
        let mut prior = None;
        for pp in (0..=400).step_by(100) {
            let budget = compute_spawn_budget(Pp(pp), weather, prior, &cfg);
            assert!(
                budget.enemies >= cfg.spawn.clamp_min,
                "enemies below clamp for weather {:?} at pp {}",
                weather,
                pp
            );
            assert!(
                budget.enemies <= cfg.spawn.clamp_max,
                "enemies above clamp for weather {:?} at pp {}",
                weather,
                pp
            );
            if let Some(prev) = prior {
                assert!(
                    budget.enemies >= prev,
                    "spawn budget decreased for weather {:?} at pp {}",
                    weather,
                    pp
                );
                let delta = budget.enemies - prev;
                assert!(
                    delta <= cfg.spawn.growth_cap_per_leg,
                    "spawn budget exceeded growth cap for weather {:?} at pp {}",
                    weather,
                    pp
                );
            }
            prior = Some(budget.enemies);
        }
    }
}

#[test]
fn danger_diff_matches_score_delta() {
    let cfg = load_director_cfg(cfg_path()).expect("director config loads");

    let budgets = [
        compute_spawn_budget(Pp(0), Weather::Clear, None, &cfg),
        compute_spawn_budget(Pp(100), Weather::Clear, None, &cfg),
        compute_spawn_budget(Pp(200), Weather::Rains, None, &cfg),
    ];

    let scenarios = [
        (budgets[0], 8, 55, 4, 40u8),
        (budgets[1], 10, 65, 5, 50u8),
        (budgets[2], 12, 75, 6, 60u8),
    ];

    let mut prior_score = None;
    for (budget, minutes, density, cadence, rating) in scenarios {
        let score = danger_score(&budget, minutes, density, cadence, rating);
        if let Some(prev) = prior_score {
            let expected = score.cmp(&prev);
            let diff = danger_diff_sign(score, prev);
            let diff_ordering = match diff {
                1 => std::cmp::Ordering::Greater,
                -1 => std::cmp::Ordering::Less,
                0 => std::cmp::Ordering::Equal,
                _ => unreachable!(),
            };
            assert_eq!(diff_ordering, expected);
        }
        prior_score = Some(score);
    }
}
