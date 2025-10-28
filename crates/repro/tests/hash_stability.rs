use repro::{hash_record, Command, InputEvent, Record, RecordMeta};

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
            prior_danger_score: None,
        },
        commands: vec![Command::meter_at(0, "danger_score", 9001)],
        inputs: Vec::new(),
    };

    let hash_a = hash_record(&record).expect("hash");
    let hash_b = hash_record(&record).expect("hash");
    assert_eq!(hash_a, hash_b);
}

#[test]
fn hash_contract_fields_change_digest() {
    let base = Record {
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
            prior_danger_score: None,
        },
        commands: vec![Command::meter_at(0, "danger_score", 9001)],
        inputs: Vec::new(),
    };

    let hash_base = hash_record(&base).expect("hash");

    let mut changed_rulepack = base.clone();
    changed_rulepack.meta.rulepack = "assets/other_rulepack.toml".into();
    assert_ne!(hash_base, hash_record(&changed_rulepack).expect("hash"));

    let mut changed_day = base.clone();
    changed_day.meta.day = 6;
    assert_ne!(hash_base, hash_record(&changed_day).expect("hash"));

    let mut changed_pp = base.clone();
    changed_pp.meta.pp = 321;
    assert_ne!(hash_base, hash_record(&changed_pp).expect("hash"));

    let mut changed_danger = base.clone();
    changed_danger.meta.prior_danger_score = Some(5);
    assert_ne!(hash_base, hash_record(&changed_danger).expect("hash"));
}

#[test]
fn inputs_are_excluded_from_digest() {
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
            prior_danger_score: None,
        },
        commands: vec![Command::meter_at(0, "danger_score", 9001)],
        inputs: Vec::new(),
    };

    let hash_base = hash_record(&base).expect("hash");

    base.inputs.push(InputEvent {
        t: 12,
        input: "KeyDown(Q)".into(),
    });

    let hash_modified = hash_record(&base).expect("hash");
    assert_eq!(hash_base, hash_modified);
}
