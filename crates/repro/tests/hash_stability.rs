use repro::{Command, CommandKind, Record, RecordMeta};

fn sample_record() -> Record {
    let mut record = Record::new(RecordMeta {
        schema: 1,
        world_seed: "seed".into(),
        link_id: "link".into(),
        rulepack: "assets/rulepacks/day_001.toml".into(),
        weather: "Fog".into(),
        rng_salt: "salt".into(),
        pp: 12,
        mission_minutes: 8,
        density_per_10k: 4,
        cadence_per_min: 2,
        player_rating: 60,
        day: 3,
    });
    record.add_command(Command {
        t: 0,
        kind: CommandKind::Meter {
            key: "danger_score".into(),
            value: 42,
        },
    });
    record.add_command(Command {
        t: 1,
        kind: CommandKind::Spawn {
            kind: "bandit".into(),
            x_mm: 10,
            y_mm: 20,
            z_mm: 30,
        },
    });
    record
}

#[test]
fn hash_is_stable_across_invocations() {
    let record = sample_record();
    let hash1 = record.hash().unwrap();
    let hash2 = record.hash().unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn roundtrip_preserves_hash() {
    let record = sample_record();
    let mut bytes = Vec::new();
    record.to_writer(&mut bytes).unwrap();
    let parsed = Record::from_reader(bytes.as_slice()).unwrap();
    assert_eq!(record.hash().unwrap(), parsed.hash().unwrap());
}
