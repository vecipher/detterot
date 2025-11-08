use blake3::Hasher;
use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

use crate::systems::economy::{RouteId, Weather};
use crate::world::canon::canonical_json;
use crate::world::schema::{
    Board, BoardDims, Cover, CoverKind, Direction, Point, Rectangle, SpawnPoints, Wall, Zones,
};

pub fn generate_board(
    world_seed: u64,
    econ_version: u32,
    link_id: RouteId,
    style: &str,
    weather: Weather,
) -> Board {
    // Create deterministic seed from all inputs
    let mut seed_bytes = Vec::new();
    seed_bytes.extend_from_slice(&world_seed.to_le_bytes());
    seed_bytes.extend_from_slice(&econ_version.to_le_bytes());
    seed_bytes.extend_from_slice(&link_id.0.to_le_bytes());
    seed_bytes.extend_from_slice(&(style.len() as u64).to_le_bytes());
    seed_bytes.extend_from_slice(style.as_bytes());
    seed_bytes.extend_from_slice(&(weather as u8).to_le_bytes());

    let mut hasher = blake3::Hasher::new();
    hasher.update(&seed_bytes);
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    let seed = u64::from_le_bytes([
        hash_bytes[0],
        hash_bytes[1],
        hash_bytes[2],
        hash_bytes[3],
        hash_bytes[4],
        hash_bytes[5],
        hash_bytes[6],
        hash_bytes[7],
    ]);

    // Create a deterministic random number generator from the seed
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);

    // Create board dimensions (64x64 as specified in requirements)
    let dims = BoardDims { w: 64, h: 64 };

    // Cell size (500mm as specified in requirements)
    let cell_mm = 500;

    // Generate walls
    let walls = generate_walls(&mut rng, dims.w, dims.h);

    // Generate cover
    let cover = generate_cover(&mut rng, dims.w, dims.h);

    // Generate spawn points (1-2 player, enemy points based on RNG)
    let spawns = generate_spawn_points(&mut rng, dims.w, dims.h);

    // Generate zones
    let zones = generate_zones(&mut rng, dims.w, dims.h);

    Board {
        link_id,
        style: style.to_string(),
        weather,
        cell_mm,
        dims,
        walls,
        cover,
        spawns,
        zones,
    }
}

fn generate_walls(rng: &mut Xoshiro256PlusPlus, w: u32, h: u32) -> Vec<Wall> {
    let mut walls = Vec::new();

    // Generate 6-12 walls as specified
    let wall_count = (rng.next_u32() % 7) + 6; // 6..=12

    for _ in 0..wall_count {
        let is_horizontal = rng.next_u32().is_multiple_of(2);
        let x = (rng.next_u32() as i32) % w as i32;
        let y = (rng.next_u32() as i32) % h as i32;
        let len = (rng.next_u32() % 8) + 3; // 3..=10

        // Clamp coordinates and length to board bounds
        let x = x.max(1).min(w as i32 - 2);
        let y = y.max(1).min(h as i32 - 2);
        let len = len.min(
            if is_horizontal {
                w as i32 - x - 1
            } else {
                h as i32 - y - 1
            }
            .max(1) as u32,
        );

        let dir = if is_horizontal {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };

        walls.push(Wall { x, y, len, dir });
    }

    walls
}

fn generate_cover(rng: &mut Xoshiro256PlusPlus, w: u32, h: u32) -> Vec<Cover> {
    let mut cover = Vec::new();

    // Generate cover density ~8-12% of cells as specified
    let total_cells = w * h;
    // Using integer arithmetic: multiply by 100 first, then divide by 100
    let min_density = (total_cells * 8) / 100;  // 8%
    let max_density = (total_cells * 12) / 100; // 12%
    let cover_range = max_density.saturating_sub(min_density);
    let cover_count = if cover_range > 0 {
        min_density + (rng.next_u32() % (cover_range + 1))
    } else {
        min_density
    }
    .max(10);

    for _ in 0..cover_count {
        let x = 1 + ((rng.next_u32() as i32) % (w as i32 - 2)); // 1..w-1
        let y = 1 + ((rng.next_u32() as i32) % (h as i32 - 2)); // 1..h-1

        // Random cover kind
        let kind = match rng.next_u32() % 3 {
            0 => CoverKind::Rock,
            1 => CoverKind::Tree,
            _ => CoverKind::Bush,
        };

        cover.push(Cover { x, y, kind });
    }

    cover
}

fn generate_spawn_points(rng: &mut Xoshiro256PlusPlus, w: u32, h: u32) -> SpawnPoints {
    let mut player = Vec::new();
    let player_count = 1 + (rng.next_u32() % 2); // 1-2 player spawn points

    for _ in 0..player_count {
        // Keep spawn points away from edges
        let x = 2 + ((rng.next_u32() as i32) % (w as i32 - 4)); // 2..w-2
        let y = 2 + ((rng.next_u32() as i32) % (h as i32 - 4)); // 2..h-2

        player.push(Point { x, y });
    }

    let mut enemy = Vec::new();
    let enemy_count = 2 + (rng.next_u32() % 5); // 2-6 enemy spawn points

    for _ in 0..enemy_count {
        // Keep spawn points away from edges
        let x = 2 + ((rng.next_u32() as i32) % (w as i32 - 4)); // 2..w-2
        let y = 2 + ((rng.next_u32() as i32) % (h as i32 - 4)); // 2..h-2

        enemy.push(Point { x, y });
    }

    SpawnPoints { player, enemy }
}

fn generate_zones(rng: &mut Xoshiro256PlusPlus, w: u32, h: u32) -> Zones {
    // Generate hold and evac zones as rectangles
    let mut hold = Vec::new();
    let mut evac = Vec::new();

    // Generate 1-3 hold zones
    let hold_count = 1 + (rng.next_u32() % 3); // 1-3
    for _ in 0..hold_count {
        let w_size = 3 + (rng.next_u32() % 6); // 3-8
        let h_size = 3 + (rng.next_u32() % 6); // 3-8
        let max_x = (w as i32 - w_size as i32 - 1).max(1);
        let max_y = (h as i32 - h_size as i32 - 1).max(1);
        let x = 1 + ((rng.next_u32() as i32) % max_x);
        let y = 1 + ((rng.next_u32() as i32) % max_y);

        hold.push(Rectangle {
            x,
            y,
            w: w_size,
            h: h_size,
        });
    }

    // Generate 1-2 evac zones
    let evac_count = 1 + (rng.next_u32() % 2); // 1-2
    for _ in 0..evac_count {
        let w_size = 4 + (rng.next_u32() % 7); // 4-10
        let h_size = 4 + (rng.next_u32() % 7); // 4-10
        let max_x = (w as i32 - w_size as i32 - 1).max(1);
        let max_y = (h as i32 - h_size as i32 - 1).max(1);
        let x = 1 + ((rng.next_u32() as i32) % max_x);
        let y = 1 + ((rng.next_u32() as i32) % max_y);

        evac.push(Rectangle {
            x,
            y,
            w: w_size,
            h: h_size,
        });
    }

    Zones { hold, evac }
}

pub fn board_hash(board: &Board) -> u64 {
    let json = canonical_json(board).expect("Failed to serialize board to canonical JSON");
    let mut hasher = Hasher::new();
    hasher.update(json.as_bytes());
    let hash = hasher.finalize();

    // Convert the first 8 bytes to a u64
    let hash_bytes = hash.as_bytes();
    u64::from_le_bytes([
        hash_bytes[0],
        hash_bytes[1],
        hash_bytes[2],
        hash_bytes[3],
        hash_bytes[4],
        hash_bytes[5],
        hash_bytes[6],
        hash_bytes[7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::economy::Weather;

    #[test]
    fn boardgen_determinism() {
        // Test that the same inputs always produce the same board_hash
        let world_seed = 12345;
        let econ_version = 1;
        let link_id = RouteId(1);
        let style = "coast";
        let weather = Weather::Fog;

        // Generate the board multiple times with the same inputs
        let board1 = generate_board(world_seed, econ_version, link_id, style, weather);
        let board2 = generate_board(world_seed, econ_version, link_id, style, weather);

        // They should be identical
        assert_eq!(
            canonical_json(&board1).unwrap(),
            canonical_json(&board2).unwrap()
        );

        // Their hashes should be the same
        let hash1 = board_hash(&board1);
        let hash2 = board_hash(&board2);
        assert_eq!(hash1, hash2);

        // Test across different styles/weather
        let board3 = generate_board(world_seed, econ_version, link_id, "ridge", weather);
        let board4 = generate_board(world_seed, econ_version, link_id, style, Weather::Clear);

        // Different inputs should generally produce different boards (though collisions are possible with hash functions)
        let hash3 = board_hash(&board3);
        let hash4 = board_hash(&board4);

        // Verify they are all different (or the same in rare hash collision cases)
        // At least some should be different with high probability
        assert!(
            hash1 != hash3 || hash1 != hash4 || hash3 != hash4,
            "Expected different inputs to produce different hashes with high probability"
        );
    }

    #[test]
    fn board_hash_consistency() {
        // Test that the same board always produces the same hash
        let world_seed = 98765;
        let link_id = RouteId(5);
        let board = generate_board(world_seed, 1, link_id, "wetland", Weather::Rains);

        // Generate hashes multiple times
        let hash1 = board_hash(&board);
        let hash2 = board_hash(&board);
        let hash3 = board_hash(&board);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn different_seeds_produce_different_boards() {
        let link_id = RouteId(1);
        let style = "coast";
        let weather = Weather::Clear;

        let board1 = generate_board(100, 1, link_id, style, weather);
        let board2 = generate_board(200, 1, link_id, style, weather);

        // Different seeds should produce different boards
        let hash1 = board_hash(&board1);
        let hash2 = board_hash(&board2);

        assert_ne!(
            hash1, hash2,
            "Different seeds should produce different boards"
        );
    }
}
