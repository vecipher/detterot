use bevy::prelude::*;
use std::collections::HashMap;

use crate::systems::economy::{RouteId, Weather};
use crate::world::boardgen::{board_hash, generate_board};
use crate::world::schema::Board;
use crate::world::weather::WeatherConfig;

#[derive(Hash, PartialEq, Eq, Clone)]
struct BoardCacheKey {
    link_id: RouteId,
    world_seed: u64,
    econ_version: u32,
    style: String, // Using String instead of &str to make it owned
    weather: Weather,
}

#[derive(Resource, Default)]
pub struct BoardCache {
    boards: HashMap<BoardCacheKey, CachedBoard>,
}

#[derive(Clone)]
struct CachedBoard {
    board: Board,
    hash: u64,
    #[allow(dead_code)]
    timestamp: u64,
}

impl BoardCache {
    pub fn get_or_generate(
        &mut self,
        world_seed: u64,
        econ_version: u32,
        link_id: RouteId,
        style: &str,
        weather: crate::systems::economy::Weather,
        _weather_config: &WeatherConfig,
    ) -> (u64, &Board) {
        let key = BoardCacheKey {
            link_id,
            world_seed,
            econ_version,
            style: style.to_string(),
            weather,
        };

        // If board is already cached, return it
        if self.boards.contains_key(&key) {
            let cached = self.boards.get(&key).unwrap();
            return (cached.hash, &cached.board);
        }

        // Generate the board first
        let board = generate_board(world_seed, econ_version, link_id, style, weather);
        let hash = board_hash(&board);
        let timestamp = self.boards.len() as u64;

        // Insert the new board (key is cloned to preserve the original for lookup)
        self.boards.insert(
            key.clone(),
            CachedBoard {
                board,
                hash,
                timestamp,
            },
        );

        // Return a reference to the newly inserted board
        let cached = self.boards.get(&key).unwrap();
        (cached.hash, &cached.board)
    }

    /// Get any board for the given link ID - useful for testing/debugging but not recommended for production
    /// since there may be multiple cached boards for the same link ID with different parameters
    pub fn get_board(&self, link_id: RouteId) -> Option<&Board> {
        // Find the first board with matching link ID
        self.boards
            .iter()
            .find(|(key, _)| key.link_id == link_id)
            .map(|(_, cached)| &cached.board)
    }

    /// Get any hash for the given link ID - useful for testing/debugging but not recommended for production
    /// since there may be multiple cached hashes for the same link ID with different parameters
    pub fn get_hash(&self, link_id: RouteId) -> Option<u64> {
        // Find the first hash with matching link ID
        self.boards
            .iter()
            .find(|(key, _)| key.link_id == link_id)
            .map(|(_, cached)| cached.hash)
    }

    /// Get board with exact parameters (if previously cached with those parameters)
    pub fn get_board_exact(
        &self,
        link_id: RouteId,
        world_seed: u64,
        econ_version: u32,
        style: &str,
        weather: Weather,
    ) -> Option<&Board> {
        let key = BoardCacheKey {
            link_id,
            world_seed,
            econ_version,
            style: style.to_string(),
            weather,
        };
        self.boards.get(&key).map(|cached| &cached.board)
    }

    /// Get hash with exact parameters (if previously cached with those parameters)
    pub fn get_hash_exact(
        &self,
        link_id: RouteId,
        world_seed: u64,
        econ_version: u32,
        style: &str,
        weather: Weather,
    ) -> Option<u64> {
        let key = BoardCacheKey {
            link_id,
            world_seed,
            econ_version,
            style: style.to_string(),
            weather,
        };
        self.boards.get(&key).map(|cached| cached.hash)
    }
}

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BoardCache>();
    }
}

// Function to be called when entering a link to generate/cache the board
pub fn prepare_board_for_link(
    mut board_cache: ResMut<BoardCache>,
    weather_config: Option<Res<WeatherConfig>>,
    world_seed: u64,
    econ_version: u32,
    link_id: RouteId,
    style: &str,
    weather: crate::systems::economy::Weather,
) -> u64 {
    // Returns the board hash
    let default_config = WeatherConfig::default();
    let config = weather_config
        .as_ref()
        .map(|w| w.as_ref())
        .unwrap_or(&default_config);
    let (hash, _board) =
        board_cache.get_or_generate(world_seed, econ_version, link_id, style, weather, config);

    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::economy::{RouteId, Weather};

    #[test]
    fn director_uses_board() {
        let mut cache = BoardCache::default();
        let config = WeatherConfig::default();

        // Test that boards are properly cached and retrieved
        let link_id = RouteId(1);
        let (hash1, board1) =
            cache.get_or_generate(12345, 1, link_id, "coast", Weather::Fog, &config);
        // Clone the board data to avoid borrowing conflicts
        let board1_clone = board1.clone();

        let (hash2, board2) =
            cache.get_or_generate(12345, 1, link_id, "coast", Weather::Fog, &config);

        // Same inputs should return the same cached board
        assert_eq!(hash1, hash2);
        assert_eq!(board1_clone.link_id, board2.link_id);
        assert_eq!(board1_clone.style, board2.style);
        assert_eq!(board1_clone.weather, board2.weather);

        // Different inputs should return different boards (with high probability)
        let (hash3, _) =
            cache.get_or_generate(12346, 1, RouteId(2), "coast", Weather::Fog, &config); // Different seed and different link_id
        assert_ne!(
            hash1, hash3,
            "Different seeds should produce different boards"
        );

        // Test that the fix works: same link_id but different parameters should generate different boards
        let (hash4, _) = cache.get_or_generate(12345, 1, link_id, "coast", Weather::Clear, &config);
        let (hash5, _) = cache.get_or_generate(12345, 1, link_id, "coast", Weather::Fog, &config);

        // Different weather should produce different boards
        assert_ne!(
            hash4, hash5,
            "Different weather should produce different boards"
        );

        // Test with different style
        let (hash6, _) = cache.get_or_generate(12345, 1, link_id, "ridge", Weather::Clear, &config);
        assert_ne!(
            hash4, hash6,
            "Different styles should produce different boards"
        );

        // Verify the boards have proper structure (spawns, dims, etc.)
        let board = cache.get_board(link_id).unwrap();
        assert_eq!(board.dims.w, 64); // Standard board size
        assert_eq!(board.dims.h, 64);
        assert!(!board.spawns.player.is_empty() || !board.spawns.enemy.is_empty());
        // Should have some spawns
    }
}
