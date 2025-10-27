use std::fs;
use std::path::Path;

use repro::{Command, Record, RecordMeta};

fn main() {
    let out_dir = Path::new("repro/records");
    fs::create_dir_all(out_dir).unwrap();
    for (idx, meta) in samples().into_iter().enumerate() {
        let mut record = Record::new(meta);
        record.push_command(3, Command::spawn("bandit", 1200, -300, 0));
        record.push_command(4, Command::meter("danger_score", 1234 + idx as i32));
        record.push_command(
            5,
            Command::meter("danger_diff", if idx % 2 == 0 { 1 } else { -1 }),
        );
        let path = out_dir.join(format!("leg_seed_{:02}.json", idx + 1));
        fs::write(&path, record.canonical_json().unwrap()).unwrap();
        let hash_path = out_dir.join(format!("leg_seed_{:02}.hash", idx + 1));
        fs::write(hash_path, record.hash_hex().unwrap()).unwrap();
    }
}

fn samples() -> Vec<RecordMeta> {
    vec![
        RecordMeta {
            schema: 1,
            world_seed: "world_seed_01".into(),
            link_id: "route_a".into(),
            rulepack: "assets/rulepacks/day_001.toml".into(),
            weather: "Fog".into(),
            rng_salt: "salt_a".into(),
        },
        RecordMeta {
            schema: 1,
            world_seed: "world_seed_02".into(),
            link_id: "route_b".into(),
            rulepack: "assets/rulepacks/day_001.toml".into(),
            weather: "Rains".into(),
            rng_salt: "salt_b".into(),
        },
        RecordMeta {
            schema: 1,
            world_seed: "world_seed_03".into(),
            link_id: "route_c".into(),
            rulepack: "assets/rulepacks/day_001.toml".into(),
            weather: "Clear".into(),
            rng_salt: "salt_c".into(),
        },
        RecordMeta {
            schema: 1,
            world_seed: "world_seed_04".into(),
            link_id: "route_d".into(),
            rulepack: "assets/rulepacks/day_001.toml".into(),
            weather: "Windy".into(),
            rng_salt: "salt_d".into(),
        },
        RecordMeta {
            schema: 1,
            world_seed: "world_seed_05".into(),
            link_id: "route_e".into(),
            rulepack: "assets/rulepacks/day_001.toml".into(),
            weather: "Fog".into(),
            rng_salt: "salt_e".into(),
        },
    ]
}
