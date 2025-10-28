use clap::{ArgAction, Parser, ValueEnum};

use crate::systems::economy::Weather;

const DEFAULT_WORLD_SEED: u64 = 0xD7E7_2024_0001_0001;
const DEFAULT_LINK_ID: u16 = 11;
const DEFAULT_DAY: u32 = 3;
const DEFAULT_PP: u16 = 120;
const DEFAULT_DENSITY_PER_10K: u32 = 5;
const DEFAULT_CADENCE_PER_MIN: u32 = 3;
const DEFAULT_MISSION_MINUTES: u32 = 8;
const DEFAULT_PLAYER_RATING: u8 = 50;

fn parse_u64(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|err| err.to_string())
    } else {
        trimmed.parse::<u64>().map_err(|err| err.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WeatherArg {
    Clear,
    Rains,
    Fog,
    Windy,
}

impl From<WeatherArg> for Weather {
    fn from(value: WeatherArg) -> Self {
        match value {
            WeatherArg::Clear => Weather::Clear,
            WeatherArg::Rains => Weather::Rains,
            WeatherArg::Fog => Weather::Fog,
            WeatherArg::Windy => Weather::Windy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    Play,
    Record,
    Replay,
}

#[derive(Debug, Parser, Clone)]
#[command(
    name = "game",
    version,
    about = "Detterot CLI",
    disable_help_subcommand = true
)]
pub struct CliOptions {
    #[arg(long, value_enum, default_value_t = Mode::Play)]
    mode: Mode,
    #[arg(long)]
    pub io: Option<String>,
    #[arg(long = "fixed-dt")]
    pub fixed_dt: Option<f64>,
    #[arg(long)]
    pub headless: bool,
    #[arg(long = "continue-after-mismatch", action = ArgAction::SetTrue)]
    pub continue_after_mismatch: bool,
    #[arg(long = "debug-logs")]
    pub debug_logs: bool,
    #[arg(long = "world-seed", value_parser = parse_u64, default_value = "0xD7E7202400010001")]
    world_seed: u64,
    #[arg(long = "link-id", default_value_t = DEFAULT_LINK_ID)]
    link_id: u16,
    #[arg(long, default_value_t = DEFAULT_DAY)]
    day: u32,
    #[arg(long, value_enum, default_value_t = WeatherArg::Clear)]
    weather: WeatherArg,
    #[arg(long = "pp", default_value_t = DEFAULT_PP)]
    pp: u16,
    #[arg(long = "density-per-10k", default_value_t = DEFAULT_DENSITY_PER_10K)]
    density_per_10k: u32,
    #[arg(long = "cadence-per-min", default_value_t = DEFAULT_CADENCE_PER_MIN)]
    cadence_per_min: u32,
    #[arg(long = "mission-minutes", default_value_t = DEFAULT_MISSION_MINUTES)]
    mission_minutes: u32,
    #[arg(long = "player-rating", default_value_t = DEFAULT_PLAYER_RATING)]
    player_rating: u8,
}

impl CliOptions {
    const DEFAULT_FIXED_DT: f64 = 0.033_333_333_333_333_33;

    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn effective_fixed_dt(&self) -> f64 {
        self.fixed_dt.unwrap_or(Self::DEFAULT_FIXED_DT)
    }

    pub fn for_mode(mode: Mode) -> Self {
        Self {
            mode,
            io: None,
            fixed_dt: None,
            headless: false,
            continue_after_mismatch: false,
            debug_logs: false,
            world_seed: DEFAULT_WORLD_SEED,
            link_id: DEFAULT_LINK_ID,
            day: DEFAULT_DAY,
            weather: WeatherArg::Clear,
            pp: DEFAULT_PP,
            density_per_10k: DEFAULT_DENSITY_PER_10K,
            cadence_per_min: DEFAULT_CADENCE_PER_MIN,
            mission_minutes: DEFAULT_MISSION_MINUTES,
            player_rating: DEFAULT_PLAYER_RATING,
        }
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn link_id(&self) -> u16 {
        self.link_id
    }

    pub fn day(&self) -> u32 {
        self.day
    }

    pub fn weather(&self) -> Weather {
        self.weather.into()
    }

    pub fn pp(&self) -> u16 {
        self.pp
    }

    pub fn density_per_10k(&self) -> u32 {
        self.density_per_10k
    }

    pub fn cadence_per_min(&self) -> u32 {
        self.cadence_per_min
    }

    pub fn mission_minutes(&self) -> u32 {
        self.mission_minutes
    }

    pub fn player_rating(&self) -> u8 {
        self.player_rating
    }
}
