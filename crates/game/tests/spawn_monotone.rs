use std::path::Path;

use game::systems::director::config::load_director_cfg;
use game::systems::director::spawn::compute_spawn_budget;
use game::systems::economy::{Pp, Weather};

#[test]
fn spawn_budget_monotonic_in_pp() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/director/m2.toml");
    let cfg = load_director_cfg(path.to_str().unwrap()).unwrap();
    for weather in [Weather::Clear, Weather::Rains, Weather::Fog, Weather::Windy] {
        let mut prior = None;
        let mut last = 0;
        for step in 0..=4 {
            let pp = Pp((step * 100) as u16);
            let budget = compute_spawn_budget(pp, weather, prior, &cfg);
            assert!(
                budget.enemies >= last,
                "weather {weather:?} failed monotonicity"
            );
            assert!(budget.enemies <= cfg.spawn.clamp_max);
            assert!(budget.enemies >= cfg.spawn.clamp_min);
            prior = Some(budget.enemies);
            last = budget.enemies;
        }
    }
}
