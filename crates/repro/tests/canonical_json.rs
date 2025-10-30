use repro::{canonical_json_bytes, Command, Record, RecordMeta};

#[test]
fn canonical_json_is_sorted() {
    let record = Record {
        meta: RecordMeta {
            schema: 1,
            world_seed: "zeta".into(),
            link_id: "alpha".into(),
            rulepack: "assets/example.toml".into(),
            weather: "Fog".into(),
            rng_salt: "salt".into(),
            day: 4,
            pp: 200,
            density_per_10k: 7,
            cadence_per_min: 5,
            mission_minutes: 12,
            player_rating: 62,
            prior_danger_score: None,
        },
        commands: vec![
            Command::meter_at(0, "danger", 1),
            Command::meter_at(0, "ui_click_buy", 1),
            Command::meter_at(0, "wallet_delta_buy", -200),
        ],
        inputs: Vec::new(),
    };

    let bytes = canonical_json_bytes(&record).expect("canonical bytes");
    let json = String::from_utf8(bytes).expect("utf8");
    let expected = "{\"commands\":[{\"Meter\":{\"key\":\"danger\",\"value\":1},\"t\":0},{\"Meter\":{\"key\":\"ui_click_buy\",\"value\":1},\"t\":0},{\"Meter\":{\"key\":\"wallet_delta_buy\",\"value\":-200},\"t\":0}],\"inputs\":[],\"meta\":{\"cadence_per_min\":5,\"day\":4,\"density_per_10k\":7,\"link_id\":\"alpha\",\"mission_minutes\":12,\"player_rating\":62,\"pp\":200,\"rng_salt\":\"salt\",\"rulepack\":\"assets/example.toml\",\"schema\":1,\"weather\":\"Fog\",\"world_seed\":\"zeta\"}}\n";
    assert_eq!(json, expected);
}
