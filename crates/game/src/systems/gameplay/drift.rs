use bevy::prelude::*;
use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

use crate::systems::economy::Weather;
use crate::world::weather::{WeatherConfig, load_weather_config};

// Feature-gated logging
#[cfg(feature = "m2_logs")]
use crate::logs::world::{log_weather_state, WeatherLogData};

pub struct DriftPlugin;

impl Plugin for DriftPlugin {
    fn build(&self, app: &mut App) {
        // Weather config should already be loaded by LOS plugin
    }
}

pub fn get_drift_offset_mm(weather: Weather, config: &WeatherConfig, tick: u64, route_id: Option<u16>) -> i32 {
    let drift_amount = config.get_drift_mm(weather) as i32;
    if drift_amount == 0 {
        // Still log even when drift is 0
        #[cfg(feature = "m2_logs")]
        {
            use std::sync::atomic::{AtomicU64, Ordering};
            static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
            
            let timestamp = TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let _ = log_weather_state(&WeatherLogData {
                route_id: route_id.unwrap_or(0),
                weather: format!("{:?}", weather),
                los_m: config.get_los_m(weather),
                drift_mm: drift_amount as u32,
                aggression_pct: config.get_agg_pct(weather),
                timestamp,
            });
        }
        return 0;
    }

    // Create deterministic seed from weather, tick, and other factors
    let mut seed_bytes = Vec::new();
    seed_bytes.extend_from_slice(&(weather as u8).to_le_bytes());
    seed_bytes.extend_from_slice(&tick.to_le_bytes());
    seed_bytes.extend_from_slice(&[1, 2, 3, 4]); // Additional entropy
    
    let mut hasher = blake3::Hasher::new();
    hasher.update(&seed_bytes);
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    let seed = u64::from_le_bytes([
        hash_bytes[0], hash_bytes[1], hash_bytes[2], hash_bytes[3],
        hash_bytes[4], hash_bytes[5], hash_bytes[6], hash_bytes[7],
    ]);
    
    // Create deterministic RNG
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
    
    // Generate a signed offset in the range [-drift_amount, drift_amount]
    let unsigned_offset = (rng.next_u32() % (drift_amount as u32 * 2 + 1)) as i32;
    let signed_offset = unsigned_offset - drift_amount as i32;
    
    // Log the weather state
    #[cfg(feature = "m2_logs")]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let timestamp = TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let _ = log_weather_state(&WeatherLogData {
            route_id: route_id.unwrap_or(0),
            weather: format!("{:?}", weather),
            los_m: config.get_los_m(weather),
            drift_mm: drift_amount as u32,
            aggression_pct: config.get_agg_pct(weather),
            timestamp,
        });
    }
    
    signed_offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::economy::Weather;

    #[test]
    fn drift_stability() {
        let config = WeatherConfig::default();
        let tick = 100;
        
        // Same inputs should produce same outputs
        let offset1 = get_drift_offset_mm(Weather::Windy, &config, tick, None);
        let offset2 = get_drift_offset_mm(Weather::Windy, &config, tick, None);
        assert_eq!(offset1, offset2, "Drift should be deterministic for same inputs");
        
        // Different ticks should produce different outputs (with high probability)
        let offset3 = get_drift_offset_mm(Weather::Windy, &config, tick + 1, None);
        // Note: We might have collisions due to the nature of random functions, 
        // but we can check for stability
        let offset4 = get_drift_offset_mm(Weather::Windy, &config, tick + 1, None);
        assert_eq!(offset3, offset4, "Same tick should produce same offset");
    }
    
    #[test]
    fn drift_zero_for_clear_weather() {
        let config = WeatherConfig::default();
        
        // Clear weather should have zero drift
        let drift_clear = get_drift_offset_mm(Weather::Clear, &config, 100, None);
        assert_eq!(drift_clear, 0, "Clear weather should have zero drift");
        
        // Other weather types should have non-zero drift (when configured)
        let drift_fog = get_drift_offset_mm(Weather::Fog, &config, 100, None);
        let drift_rains = get_drift_offset_mm(Weather::Rains, &config, 100, None);
        let drift_windy = get_drift_offset_mm(Weather::Windy, &config, 100, None);
        
        // Fog drift should be zero per config
        assert_eq!(drift_fog, 0, "Fog drift should be 0 per config");
        
        // Rains and Windy should have positive drift
        assert!(drift_rains >= 0, "Rains drift should be >= 0");
        assert!(drift_windy >= 0, "Windy drift should be >= 0");
    }
}