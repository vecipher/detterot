use std::env;
use std::fs;

fn main() {
    let path = env::args().nth(1).expect("path to record");
    let data = fs::read_to_string(&path).expect("read record");
    let record: repro::Record = serde_json::from_str(&data).expect("parse record");
    let hash = repro::hash_record(&record).expect("hash record");
    println!("{hash}");
}
