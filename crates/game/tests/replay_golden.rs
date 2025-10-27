use std::path::Path;

use game::systems::director::{DirectorInputs, DirectorState};
use game::{build_headless_app, collect_commands};
use repro::{Command, Record, RecordMeta};

use game::systems::economy::{Pp, RouteId, Weather};

struct GoldenSpec {
    name: &'static str,
    world_seed: u64,
    weather: Weather,
    pp: u16,
    link_id: u16,
}

const GOLDENS: [GoldenSpec; 5] = [
    GoldenSpec {
        name: "leg_seed_01",
        world_seed: 1001,
        weather: Weather::Clear,
        pp: 5000,
        link_id: 1,
    },
    GoldenSpec {
        name: "leg_seed_02",
        world_seed: 1002,
        weather: Weather::Fog,
        pp: 4200,
        link_id: 2,
    },
    GoldenSpec {
        name: "leg_seed_03",
        world_seed: 1003,
        weather: Weather::Rains,
        pp: 3100,
        link_id: 3,
    },
    GoldenSpec {
        name: "leg_seed_04",
        world_seed: 1004,
        weather: Weather::Windy,
        pp: 6000,
        link_id: 4,
    },
    GoldenSpec {
        name: "leg_seed_05",
        world_seed: 1005,
        weather: Weather::Clear,
        pp: 2000,
        link_id: 5,
    },
];

#[test]
fn golden_records_are_stable() {
    for spec in GOLDENS.iter() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../repro/records");
        let json_path = base.join(format!("{}.json", spec.name));
        let hash_path = base.join(format!("{}.hash", spec.name));
        let record = Record::read_from_path(&json_path).expect("load golden");
        let hash = std::fs::read_to_string(&hash_path).expect("hash");
        assert_eq!(record.hash().unwrap(), hash.trim());

        let mut app = build_headless_app();
        {
            let mut inputs = app.world_mut().resource_mut::<DirectorInputs>();
            inputs.world_seed = spec.world_seed;
            inputs.pp = Pp(spec.pp);
            inputs.link_id = RouteId(spec.link_id);
        }
        {
            let mut state = app.world_mut().resource_mut::<DirectorState>();
            state.link_id = RouteId(spec.link_id);
            state.weather = spec.weather;
        }

        let max_tick = record.commands.iter().map(|c| c.t).max().unwrap_or(0);
        let commands = collect_commands(&mut app, max_tick + 5);
        assert_eq!(commands, record.commands);

        let rebuilt = build_record(record.meta.clone(), commands);
        assert_eq!(rebuilt.hash().unwrap(), hash.trim());
    }
}

fn build_record(meta: RecordMeta, commands: Vec<Command>) -> Record {
    let mut record = Record::new(meta);
    for cmd in commands {
        record.add_command(cmd);
    }
    record
}
