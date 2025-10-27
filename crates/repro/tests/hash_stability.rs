use repro::{Command, Record, RecordMeta};

#[test]
fn hash_is_stable_for_same_inputs() {
    let mut record_a = Record::new(RecordMeta {
        schema: 1,
        world_seed: "seed".into(),
        link_id: "L-01".into(),
        rulepack: "assets/rulepack.toml".into(),
        weather: "Fog".into(),
        rng_salt: "salt".into(),
    });
    record_a.push_command(3, Command::spawn("bandit", 10, 20, 0));
    record_a.push_command(5, Command::meter("danger_score", 123));

    let record_b = record_a.clone();

    assert_eq!(record_a.hash_hex().unwrap(), record_b.hash_hex().unwrap());
}
