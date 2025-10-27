use repro::{hash_record, Command, Record, RecordMeta};

#[test]
fn identical_records_hash_the_same() {
    let record = Record {
        meta: RecordMeta {
            schema: 1,
            world_seed: "omega".into(),
            link_id: "leg_01".into(),
            rulepack: "assets/rulepack.toml".into(),
            weather: "Clear".into(),
            rng_salt: "salt".into(),
            day: 5,
            pp: 320,
            density_per_10k: 9,
            cadence_per_min: 6,
            mission_minutes: 14,
            player_rating: 58,
        },
        commands: vec![Command::meter_at(0, "danger_score", 9001)],
        inputs: Vec::new(),
    };

    let hash_a = hash_record(&record).expect("hash");
    let hash_b = hash_record(&record).expect("hash");
    assert_eq!(hash_a, hash_b);
}

#[test]
fn non_hash_meta_fields_do_not_change_digest() {
    let mut base = Record {
        meta: RecordMeta {
            schema: 1,
            world_seed: "omega".into(),
            link_id: "leg_01".into(),
            rulepack: "assets/rulepack.toml".into(),
            weather: "Clear".into(),
            rng_salt: "salt".into(),
            day: 5,
            pp: 320,
            density_per_10k: 9,
            cadence_per_min: 6,
            mission_minutes: 14,
            player_rating: 58,
        },
        commands: vec![Command::meter_at(0, "danger_score", 9001)],
        inputs: Vec::new(),
    };

    let hash_base = hash_record(&base).expect("hash");

    base.meta.day = 42;
    base.meta.pp = 999;
    base.meta.density_per_10k = 123;
    base.meta.cadence_per_min = 77;
    base.meta.mission_minutes = 3;
    base.meta.player_rating = 12;

    let hash_modified = hash_record(&base).expect("hash");
    assert_eq!(hash_base, hash_modified);
}
