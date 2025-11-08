use anyhow::Result;
use game::systems::economy::{RouteId, Weather};
use game::world::boardgen::{board_hash, generate_board};
use game::world::loader::load_world_graph;
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
struct SeedSpec {
    world_seed: u64,
    link: String, // This will be converted to a RouteId
    weather: String,
}

// Structure for the top-level TOML file
#[derive(Debug, serde::Deserialize)]
struct SeedsFile {
    seeds: HashMap<String, SeedSpec>,
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

#[test]
fn boards_golden_verify() -> Result<()> {
    // Get the project root directory from the manifest
    // When running from game crate tests, CARGO_MANIFEST_DIR is the game crate directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| "/Users/vkfyka/Desktop/detterot/crates/game".to_string());
    let project_root = std::path::Path::new(&manifest_dir)
        .parent() // go from crates/game to crates/
        .unwrap()
        .parent() // go from crates/ to project root
        .unwrap();

    let seeds_path = project_root.join("repro/seeds.toml");
    let graph_path = project_root.join("assets/world/graph_v1.toml");

    // Load world graph
    let world_graph = load_world_graph(&graph_path)
        .map_err(|e| anyhow::anyhow!("Failed to load world graph: {}", e))?;

    // Load seeds config
    let seeds_content = std::fs::read_to_string(seeds_path)?;
    let seeds_file: SeedsFile = toml::from_str(&seeds_content)?;

    // Test each seed
    for (seed_name, seed_spec) in seeds_file.seeds {
        println!("Testing board for seed: {}", seed_name);

        let link_id = parse_link_id(&seed_spec.link);
        let weather = weather_from_str(&seed_spec.weather);

        // Get the style from the world graph
        let style = if let Some(link_spec) = world_graph.links.get(&seed_spec.link) {
            link_spec.style.clone()
        } else {
            "coast".to_string() // default
        };

        // Generate the board
        let board = generate_board(
            seed_spec.world_seed,
            1, // econ_version
            link_id,
            &style,
            weather,
        );

        let computed_hash = board_hash(&board);

        // Load the expected hash
        let hash_path = project_root.join(format!("repro/boards/board_{}.hash", seed_name));
        if hash_path.exists() {
            let expected_hash_hex = std::fs::read_to_string(&hash_path)?.trim().to_string();
            let expected_hash = u64::from_str_radix(&expected_hash_hex, 16)?;

            assert_eq!(
                computed_hash, expected_hash,
                "Hash mismatch for seed {}",
                seed_name
            );
            println!(
                "  ✓ Hash matches for seed {}: {:016x}",
                seed_name, computed_hash
            );
        } else {
            // If the golden file doesn't exist, just print the computed hash for reference
            eprintln!(
                "Golden hash file not found for seed {}: {:016x}",
                seed_name, computed_hash
            );
        }
    }

    Ok(())
}

/// Generate golden boards for the seeds
#[test]
#[ignore = "Run manually to generate golden files"]
fn generate_golden_boards() -> Result<()> {
    // Get the project root directory from the manifest
    // When running from game crate tests, CARGO_MANIFEST_DIR is the game crate directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| "/Users/vkfyka/Desktop/detterot/crates/game".to_string());
    let project_root = std::path::Path::new(&manifest_dir)
        .parent() // go from crates/game to crates/
        .unwrap()
        .parent() // go from crates/ to project root
        .unwrap();

    let seeds_path = project_root.join("repro/seeds.toml");
    let graph_path = project_root.join("assets/world/graph_v1.toml");

    // Load world graph
    let world_graph = load_world_graph(&graph_path)
        .map_err(|e| anyhow::anyhow!("Failed to load world graph: {}", e))?;

    // Load seeds config
    let seeds_content = std::fs::read_to_string(seeds_path)?;
    let seeds_file: SeedsFile = toml::from_str(&seeds_content)?;

    for (seed_name, seed_spec) in seeds_file.seeds {
        println!("Generating board for seed: {}", seed_name);

        let link_id = parse_link_id(&seed_spec.link);
        let weather = weather_from_str(&seed_spec.weather);

        // Get the style from the world graph
        let style = if let Some(link_spec) = world_graph.links.get(&seed_spec.link) {
            link_spec.style.clone()
        } else {
            "coast".to_string() // default
        };

        // Generate the board
        let board = generate_board(
            seed_spec.world_seed,
            1, // econ_version
            link_id,
            &style,
            weather,
        );

        let hash = board_hash(&board);

        // Save the board as JSON
        let board_json = serde_json::to_string_pretty(&board)?;
        let board_path = project_root.join(format!("repro/boards/board_{}.json", seed_name));
        std::fs::write(&board_path, board_json)?;

        // Save the hash
        let hash_path = project_root.join(format!("repro/boards/board_{}.hash", seed_name));
        std::fs::write(&hash_path, format!("{:016x}", hash))?;

        println!("  Board saved to: {}", board_path.display());
        println!("  Hash: {:016x}", hash);
    }

    Ok(())
}
