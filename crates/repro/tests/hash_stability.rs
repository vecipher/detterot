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
