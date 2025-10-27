use repro::{Command, InputEvent, Record, RecordMeta};

fn sample_record() -> Record {
    let mut record = Record::new(RecordMeta {
        schema: 1,
        world_seed: "seed".into(),
        link_id: "L-01".into(),
        rulepack: "assets/rulepack.toml".into(),
        weather: "Fog".into(),
        rng_salt: "salt".into(),
    });
    record.push_command(5, Command::meter("danger_score", 42));
    record.push_command(3, Command::spawn("bandit", 100, -20, 0));
    record.push_input(
        2,
        InputEvent::Input {
            kind: "KeyDown".into(),
            key: "Q".into(),
        },
    );
    record
}

#[test]
fn canonical_format_orders_keys() {
    let record = sample_record();
    let text = record.canonical_json().unwrap();
    assert!(text.ends_with('\n'));
    assert!(text.contains("\"commands\""));
    assert!(text.contains("\"inputs\""));
    assert!(text.find("\"commands\"").unwrap() < text.find("\"inputs\"").unwrap());
    assert!(text.contains("\"Spawn\""));
    assert!(text.contains("\"Meter\""));
}
