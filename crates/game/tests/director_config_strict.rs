use game::systems::director::config::load_director_cfg;
use std::fs;

#[test]
fn director_config_rejects_unknown_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    fs::write(
        &path,
        r#"
[spawn]
base = 1
alpha_pp_per_100 = 0
growth_cap_per_leg = 1
clamp_min = 0
clamp_max = 10
unexpected = 7

[spawn.beta_weather]
Clear = 0

[missions.alpha]
pp_success = 0
pp_fail = 0
basis_bp_success = 0
basis_bp_fail = 0
"#,
    )
    .expect("write config");

    let result = load_director_cfg(path.to_str().expect("path"));
    assert!(result.is_err(), "unknown fields should error");
}
