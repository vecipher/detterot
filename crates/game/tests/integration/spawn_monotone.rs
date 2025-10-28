use game::systems::director::compute_spawn_budget;
use game::systems::director::config::load_director_cfg;
use game::systems::economy::{Pp, Weather};
use std::path::Path;

#[test]
fn spawn_budget_monotonic_with_pp() {
    let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/director/m2.toml");
    let cfg = load_director_cfg(cfg_path.to_str().expect("cfg path")).expect("load config");
    let weathers = [Weather::Clear, Weather::Rains, Weather::Fog, Weather::Windy];

    for weather in weathers {
        let mut prior = None;
        for band in 0..=4 {
            let pp = Pp((band * 100) as u16);
            let budget = compute_spawn_budget(pp, weather, prior, &cfg);
            if let Some(prev) = prior {
                assert!(
                    budget.enemies >= prev,
                    "enemies should be monotonic for {:?} at {} vs {}",
                    weather,
                    budget.enemies,
                    prev
                );
            }
            prior = Some(budget.enemies);
        }
    }
}
