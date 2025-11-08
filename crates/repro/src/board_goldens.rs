use anyhow::Result;
use std::collections::HashMap;

use game::systems::economy::{RouteId, Weather};
use game::world::boardgen::{board_hash, generate_board};
use game::world::schema::Board;

#[derive(Debug, serde::Deserialize)]
struct SeedSpec {
    world_seed: u64,
    link: String,  // This will be converted to a RouteId
    weather: String,
}

fn parse_link_id(link_str: &str) -> RouteId {
    // Parse format like "L01" to get numeric part and convert to RouteId
    if link_str.starts_with('L') {
        let num_part = &link_str[1..]; // Remove 'L'
        if let Ok(num) = num_part.parse::<u16>() {
            return RouteId(num);
        }
    }
    // Default to RouteId(1) if parsing fails
    RouteId(1)
}

fn weather_from_str(weather_str: &str) -> Weather {
    match weather_str.as_str() {
        "Clear" => Weather::Clear,
        "Rains" => Weather::Rains,
        "Fog" => Weather::Fog,
        "Windy" => Weather::Windy,
        _ => Weather::Clear, // Default
    }
}

#[test]
fn boards_golden_verify() -> Result<()> {
    // Load seeds config
    let seeds_content = std::fs::read_to_string("repro/seeds.toml")?;
    let seeds: HashMap<String, SeedSpec> = toml::from_str(&seeds_content)?;

    // Test each seed
    for (seed_name, seed_spec) in seeds {
        println!("Testing board for seed: {}", seed_name);
        
        let link_id = parse_link_id(&seed_spec.link);
        let weather = weather_from_str(&seed_spec.weather);
        
        // Generate the board
        let board = generate_board(
            seed_spec.world_seed,
            1, // econ_version
            link_id,
            "coast", // default style
            weather,
        );
        
        let computed_hash = board_hash(&board);
        
        // Load the expected hash
        let hash_path = format!("repro/boards/board_{}.hash", seed_name);
        if std::path::Path::new(&hash_path).exists() {
            let expected_hash_hex = std::fs::read_to_string(&hash_path)?.trim().to_string();
            let expected_hash = u64::from_str_radix(&expected_hash_hex, 16)?;
            
            assert_eq!(computed_hash, expected_hash, "Hash mismatch for seed {}", seed_name);
            println!("  ✓ Hash matches for seed {}: {:016x}", seed_name, computed_hash);
        } else {
            // If the golden file doesn't exist, just print the computed hash for reference
            eprintln!("Golden hash file not found for seed {}: {:016x}", seed_name, computed_hash);
        }
    }
    
    Ok(())
}

/// Generate golden boards for the seeds
#[test]
#[ignore = "Run manually to generate golden files"]
fn generate_golden_boards() -> Result<()> {
    // Load seeds config
    let seeds_content = std::fs::read_to_string("repro/seeds.toml")?;
    let seeds: HashMap<String, SeedSpec> = toml::from_str(&seeds_content)?;

    for (seed_name, seed_spec) in seeds {
        println!("Generating board for seed: {}", seed_name);
        
        let link_id = parse_link_id(&seed_spec.link);
        let weather = weather_from_str(&seed_spec.weather);
        
        // Generate the board
        let board = generate_board(
            seed_spec.world_seed,
            1, // econ_version
            link_id,
            "coast", // default style
            weather,
        );
        
        let hash = board_hash(&board);
        
        // Save the board as JSON
        let board_json = serde_json::to_string_pretty(&board)?;
        let board_path = format!("repro/boards/board_{}.json", seed_name);
        std::fs::write(&board_path, board_json)?;
        
        // Save the hash
        let hash_path = format!("repro/boards/board_{}.hash", seed_name);
        std::fs::write(&hash_path, format!("{:016x}", hash))?;
        
        println!("  Board saved to: {}", board_path);
        println!("  Hash: {:016x}", hash);
    }
    
    Ok(())
}