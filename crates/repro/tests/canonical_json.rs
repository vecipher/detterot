use repro::{Command, CommandKind, Record, RecordMeta};

#[test]
fn canonical_serialization_orders_keys_and_arrays() {
    let mut record = Record::new(RecordMeta {
        schema: 1,
        world_seed: "seed".into(),
        link_id: "route-01".into(),
        rulepack: "assets/rulepacks/day_001.toml".into(),
        weather: "Clear".into(),
        rng_salt: "salt".into(),
        pp: 8,
        mission_minutes: 12,
        density_per_10k: 5,
        cadence_per_min: 3,
        player_rating: 50,
        day: 1,
    });
    record.add_command(Command {
        t: 124,
        kind: CommandKind::Meter {
            key: "danger_score".into(),
            value: 12345,
        },
    });
    record.add_command(Command {
        t: 123,
        kind: CommandKind::Spawn {
            kind: "bandit".into(),
            x_mm: 1200,
            y_mm: -300,
            z_mm: 0,
        },
    });
    let mut bytes = Vec::new();
    record.to_writer(&mut bytes).unwrap();
    let actual = String::from_utf8(bytes).unwrap();
    let expected = concat!(
        "{",
        "\"commands\":[",
        "{\"Meter\":{\"key\":\"danger_score\",\"value\":12345},\"t\":124}",
        ",",
        "{\"Spawn\":{\"kind\":\"bandit\",\"x_mm\":1200,\"y_mm\":-300,\"z_mm\":0},\"t\":123}",
        "],",
        "\"inputs\":[],",
        "\"meta\":{",
        "\"cadence_per_min\":3,",
        "\"day\":1,",
        "\"density_per_10k\":5,",
        "\"link_id\":\"route-01\",",
        "\"mission_minutes\":12,",
        "\"player_rating\":50,",
        "\"pp\":8,",
        "\"rng_salt\":\"salt\",",
        "\"rulepack\":\"assets/rulepacks/day_001.toml\",",
        "\"schema\":1,",
        "\"weather\":\"Clear\",",
        "\"world_seed\":\"seed\"",
        "}}\n",
    );
    assert_eq!(expected, actual);
}
