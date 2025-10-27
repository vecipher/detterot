use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{builder::BoolishValueParser, ArgAction, Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RunMode {
    Play,
    Record,
    Replay,
}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub mode: RunMode,
    pub io: Option<PathBuf>,
    pub fixed_dt: Option<f64>,
    pub headless: bool,
    pub continue_after_mismatch: bool,
    pub debug_logs: bool,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Detterot game runtime", long_about = None)]
struct RawCli {
    #[arg(long, value_enum, default_value = "play")]
    mode: RunMode,
    #[arg(long)]
    io: Option<PathBuf>,
    #[arg(long)]
    fixed_dt: Option<f64>,
    #[arg(long, default_value_t = false)]
    headless: bool,
    #[arg(
        long,
        default_value_t = true,
        value_parser = BoolishValueParser::new(),
        action = ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    continue_after_mismatch: bool,
    #[arg(long, default_value_t = false)]
    debug_logs: bool,
}

impl CliOptions {
    pub fn parse() -> Result<Self> {
        Self::from_raw(RawCli::parse())
    }

    fn from_raw(raw: RawCli) -> Result<Self> {
        let options = Self {
            mode: raw.mode,
            io: raw.io,
            fixed_dt: raw.fixed_dt,
            headless: raw.headless,
            continue_after_mismatch: raw.continue_after_mismatch,
            debug_logs: raw.debug_logs,
        };
        options.validate()?;
        Ok(options)
    }

    fn validate(&self) -> Result<()> {
        match self.mode {
            RunMode::Play => Ok(()),
            RunMode::Record | RunMode::Replay => {
                if self.io.is_none() {
                    bail!("--io is required in record/replay mode");
                }
                if self.fixed_dt.is_none() {
                    bail!("--fixed-dt is required in record/replay mode");
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn validates_io_requirement() {
        let raw = RawCli {
            mode: RunMode::Record,
            io: None,
            fixed_dt: Some(0.033),
            headless: true,
            continue_after_mismatch: false,
            debug_logs: false,
        };
        assert!(CliOptions::from_raw(raw).is_err());
    }

    #[test]
    fn play_mode_allows_missing_io() {
        let raw = RawCli {
            mode: RunMode::Play,
            io: None,
            fixed_dt: None,
            headless: false,
            continue_after_mismatch: false,
            debug_logs: false,
        };
        let parsed = CliOptions::from_raw(raw).unwrap();
        assert_eq!(parsed.mode, RunMode::Play);
    }

    #[test]
    fn replay_mode_accepts_false_continue_after_mismatch() {
        let raw = RawCli::try_parse_from([
            "detterot",
            "--mode",
            "replay",
            "--io",
            "record.json",
            "--fixed-dt",
            "0.0333333333",
            "--continue-after-mismatch",
            "false",
        ])
        .unwrap();
        assert!(!raw.continue_after_mismatch);
    }
}
