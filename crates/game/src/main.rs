fn main() {
    if let Err(err) = game::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
