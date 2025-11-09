# World System Documentation

## Overview

The world system manages the deterministic world graph structure, board generation, and weather effects for gameplay. It consists of:

- World topology (hubs and links)
- Weather configuration and scheduling 
- Deterministic board generation
- Board hashing and canonical serialization
- Route closures functionality

## World Graph Schema

The world graph is defined in `assets/world/graph_v1.toml` with the following structure:

```toml
[hubs]
# Stable integer ids; display names are cosmetic
H01 = { name="Aster", x_mm=0,     y_mm=0   }
H02 = { name="Brine", x_mm=20000, y_mm=-5000 }
H03 = { name="Cinder",x_mm=-16000,y_mm=12000 }

[links]
L01 = { from="H01", to="H02", style="coast",   base_minutes=9 }
L02 = { from="H02", to="H03", style="ridge",   base_minutes=11 }
L03 = { from="H01", to="H03", style="wetland", base_minutes=14 }
```

### Schema Validation
- All unknown keys are rejected via `#[serde(deny_unknown_fields)]`
- Hub and link IDs must be unique
- Links connect existing hubs

## Weather Configuration

Weather is configured in `assets/world/weather.toml`:

```toml
[defaults]     # mapping Weather per style
coast = "Rains"
ridge = "Windy"
wetland = "Fog"

[overrides]    # fixed (link_id -> Weather) for goldens if desired
"L01" = "Fog"

[effects]      # weather effects on gameplay
clear_los_m = 1000
rains_los_m = 800
fog_los_m = 600
windy_los_m = 900

clear_drift_mm = 0
rains_drift_mm = 20
fog_drift_mm = 0
windy_drift_mm = 35

clear_agg_pct = 0
rains_agg_pct = 5
fog_agg_pct = 8
windy_agg_pct = 3
```

### Weather Effects on Gameplay

Weather modifies three gameplay aspects:

1. **Line of Sight (LOS)**: Affects detection checks and targeting cones
2. **Drift**: Adds signed integer offset to movement/projectiles per fixed tick
3. **Aggression**: Adds percentage to director spawn budget (additive to economy system)

## Board Generation

Board generation is fully deterministic and uses the following parameters:

- `world_seed`: World seed for reproducibility  
- `econ_version`: Version for generation stability
- `link_id`: Link identifier
- `style`: Link style (affects board generation characteristics)
- `weather`: Current weather affecting gameplay

### Board Structure

```rust
Board = {
  link_id, style, weather,
  cell_mm, dims:{w,h},           // 500mm cells, 64x64 dimensions
  walls:[{x,y,len,dir}],         // Axis-aligned segments on integer grid
  cover:[{x,y,kind}],            // Small obstacles (Rock, Tree, Bush)
  spawns:{enemy:[{x,y}], player:[{x,y}]},  // Spawn positions
  zones:{hold:[...], evac:[...]} // Rectangles on grid for mission areas
}
```

### Board Hashing

Board hashing ensures reproducibility using:
1. Canonical JSON serialization with sorted keys
2. Blake3 hashing algorithm
3. First 8 bytes converted to u64 for deterministic 64-bit hash

## Integration Points

### Director System
- BoardCache stores generated boards for reuse
- Director reads board spawns, zones, and dimensions
- Meters include `last_board_hash` for crash forensics

### Save System
- SaveV12 includes `last_board_hash: u64` and `visited_links: Vec<RouteId>`
- Migration from v1.1 sets defaults (hash=0, visited=[])

### Trading System  
- Route planner uses real weather from world index
- Weather affects trading calculations through gameplay systems

## Determinism Guarantees

- Same seed inputs always produce identical boards and hashes
- Cross-platform consistency (macOS/arm64, Ubuntu/x86_64)
- All randomness uses `DetRng` with deterministic seeds
- Integer-only calculations to avoid floating-point nondeterminism