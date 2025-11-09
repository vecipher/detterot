use bevy::prelude::*;

use crate::systems::economy::Weather;
use crate::world::weather::{WeatherConfig, load_weather_config};

// Feature-gated logging
#[cfg(feature = "m2_logs")]
use crate::logs::world::{log_weather_state, WeatherLogData};

pub struct LOSPlugin;

impl Plugin for LOSPlugin {
    fn build(&self, app: &mut App) {
        // Load weather config resource at startup
        let config_path = std::path::Path::new("assets/world/weather.toml");
        if config_path.exists() {
            if let Ok(config) = load_weather_config(config_path) {
                app.insert_resource(config);
            } else {
                app.insert_resource(WeatherConfig::default());
            }
        } else {
            app.insert_resource(WeatherConfig::default());
        }
    }
}

pub fn get_los_distance_m(weather: Weather, config: &WeatherConfig, route_id: Option<u16>) -> u32 {
    let distance = config.get_los_m(weather);
    
    #[cfg(feature = "m2_logs")]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let timestamp = TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let _ = log_weather_state(&WeatherLogData {
            route_id: route_id.unwrap_or(0),
            weather: format!("{:?}", weather),
            los_m: distance,
            drift_mm: config.get_drift_mm(weather),
            aggression_pct: config.get_agg_pct(weather),
            timestamp,
        });
    }
    
    distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::economy::Weather;

    #[test]
    fn los_bounds() {
        let config = WeatherConfig::default();
        
        // Verify weather effects are monotonic (more fog should never increase LOS)
        let clear_los = get_los_distance_m(Weather::Clear, &config, None);
        let fog_los = get_los_distance_m(Weather::Fog, &config, None);
        
        // Fog should have less LOS than Clear
        assert!(fog_los <= clear_los, "Fog should not increase LOS distance");
        
        // Validate all weather values are reasonable
        assert!(get_los_distance_m(Weather::Clear, &config, None) > 0);
        assert!(get_los_distance_m(Weather::Fog, &config, None) <= get_los_distance_m(Weather::Clear, &config, None));
        assert!(get_los_distance_m(Weather::Rains, &config, None) <= get_los_distance_m(Weather::Clear, &config, None));
        assert!(get_los_distance_m(Weather::Windy, &config, None) <= get_los_distance_m(Weather::Clear, &config, None));
    }
}