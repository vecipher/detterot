use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn micro_sim_generates_golden_csv() {
    let dir = tempdir().expect("temp dir");
    let out_path = dir.path().join("econ_curves.csv");
    let status = Command::new(env!("CARGO_BIN_EXE_econ-sim"))
        .args([
            "--world-seed",
            "42",
            "--days",
            "15",
            "--hubs",
            "3",
            "--pp",
            "1500,5000,9000",
            "--debt",
            "0,500_000_00,5_000_000_00",
            "--out",
            out_path.to_str().expect("utf8 path"),
        ])
        .status()
        .expect("run econ-sim");
    assert!(status.success(), "econ-sim exited with {status:?}");

    let actual = fs::read_to_string(&out_path).expect("read csv");
    let golden = include_str!("goldens/econ_curves_seed42.csv");
    verify_single_global_advance(&actual);
    assert_eq!(actual, golden);
}

fn verify_single_global_advance(actual: &str) {
    use std::collections::BTreeMap;

    const EXPECTED_HUBS: usize = 3;

    let mut per_day: BTreeMap<u32, Vec<(u16, i64, i64)>> = BTreeMap::new();
    for line in actual.lines().skip(1) {
        let mut parts = line.split(',');
        let day: u32 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .expect("day value");
        let hub: u16 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .expect("hub value");
        let commodity: u32 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .expect("commodity value");
        if commodity != 1 {
            continue;
        }
        // Skip di_bp, basis_bp, price
        parts.next();
        parts.next();
        parts.next();
        let debt: i64 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .expect("debt value");
        let interest: i64 = parts
            .next()
            .and_then(|value| value.parse().ok())
            .expect("interest value");
        per_day.entry(day).or_default().push((hub, debt, interest));
    }

    for (day, entries) in per_day {
        assert_eq!(
            entries.len(),
            EXPECTED_HUBS,
            "expected {EXPECTED_HUBS} hub snapshots for day {day}",
        );
        let mut sorted = entries;
        sorted.sort_by_key(|(hub, _, _)| *hub);
        let (_, reference_debt, _) = sorted[0];
        for (hub, debt, interest) in sorted.iter().copied().skip(1) {
            assert_eq!(
                debt, reference_debt,
                "global debt diverged for hub {hub} on day {day}",
            );
            assert_eq!(
                interest, 0,
                "interest accrued multiple times on day {day} (hub {hub})",
            );
        }
    }
}
