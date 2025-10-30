use std::io::Write;

use tempfile::NamedTempFile;

use crate::systems::trading::load_commodities;

#[test]
fn rejects_unknown_fields() {
    let mut tmp = NamedTempFile::new().expect("tmp file");
    write!(
        tmp,
        "[[commodity]]\nid = 1\nslug = \"grain\"\ndisplay_name = \"Grain\"\nbase_price_cents = 250\nmass_per_unit_kg = 2\nvolume_per_unit_l = 4\nunknown_key = \"nope\"\n"
    )
    .expect("write tmp");

    let err = load_commodities(tmp.path().to_str().unwrap()).expect_err("should fail");
    let msg = err.to_string();
    assert!(msg.contains("unknown"), "unexpected error: {}", msg);
}
