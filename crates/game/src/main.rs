use game::cli::{parse_args, run, CliError};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), CliError> {
    let options = parse_args(std::env::args())?;
    run(options)
}
