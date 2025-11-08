use std::path::Path;

use anyhow::Context;

use super::schema::WorldGraph;

pub fn load_world_graph(path: &Path) -> anyhow::Result<WorldGraph> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    
    use super::*;

    #[test]
    fn world_loader_strict() {
        // Test that unknown keys are rejected
        let bad_toml = r#"
            [hubs]
            H01 = { name = "Aster", x_mm = 0, y_mm = 0, unknown_field = 42 }
            
            [links]
            L01 = { from = "H01", to = "H02", style = "coast", base_minutes = 9 }
        "#;
        
        let result = toml::from_str::<WorldGraph>(bad_toml);
        assert!(result.is_err(), "Expected unknown field to cause error");
        
        // Test good toml with proper structure
        let good_toml = r#"
            [hubs]
            H01 = { name = "Aster", x_mm = 0, y_mm = 0 }
            H02 = { name = "Brine", x_mm = 20000, y_mm = -5000 }
            
            [links]
            L01 = { from = "H01", to = "H02", style = "coast", base_minutes = 9 }
        "#;
        
        let result = toml::from_str::<WorldGraph>(good_toml);
        assert!(result.is_ok(), "Expected valid TOML to parse successfully");
        
        let graph = result.unwrap();
        assert_eq!(graph.hubs.len(), 2);
        assert_eq!(graph.links.len(), 1);
        assert!(graph.hubs.contains_key("H01"));
        assert!(graph.hubs.contains_key("H02"));
        assert!(graph.links.contains_key("L01"));
    }
}