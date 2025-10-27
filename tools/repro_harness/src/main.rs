use bevy::prelude::*;
use clap::Parser;
use repro::Record;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    replay: String,
    #[arg(long)]
    assert_hash: Option<String>,
}

fn main() {
    let args = Args::parse();
    let rec = Record::read_from_path(&args.replay).expect("record file");
    let got = rec.hash().expect("hash record");
    if let Some(expected_path) = args.assert_hash {
        let expected = std::fs::read_to_string(expected_path)
            .expect("hash file")
            .trim()
            .to_string();
        if got != expected {
            eprintln!("hash mismatch:\n got: {got}\n exp: {expected}");
            std::process::exit(1);
        }
    }
    // Prepare a tiny app to prove headless plugin init works (no renderer).
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // later: add your gameplay schedules to exercise replay
    // run one tick to ensure bevy startup executes without errors
    app.update();
}
