use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use game::systems::economy::{RouteId, Weather};
use game::world::boardgen::{board_hash, generate_board};
use game::world::schema::Board;

#[derive(Debug, Deserialize)]
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
    match weather_str {
        "Clear" => Weather::Clear,
        "Rains" => Weather::Rains,
        "Fog" => Weather::Fog,
        "Windy" => Weather::Windy,
        _ => Weather::Clear, // Default
    }
}

fn main() -> Result<()> {
    // Load seeds config
    let seeds_content = std::fs::read_to_string("repro/seeds.toml")?;
    let seeds: HashMap<String, SeedSpec> = toml::from_str(&seeds_content)?;

    for (seed_name, seed_spec) in seeds {
        println!("Generating board for seed: {}", seed_name);
        
        let link_id = parse_link_id(&seed_spec.link);
        let weather = weather_from_str(&seed_spec.weather);
        
        // Generate the board - for now use default style since we don't have mapping
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
        let mut file = File::create(&board_path)?;
        file.write_all(board_json.as_bytes())?;
        
        // Save the hash
        let hash_path = format!("repro/boards/board_{}.hash", seed_name);
        std::fs::write(hash_path, format!("{:016x}", hash))?;
        
        println!("  Board saved to: {}", board_path);
        println!("  Hash: {:016x}", hash);
    }
    
    Ok(())
}