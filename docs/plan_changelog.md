# Planning changelog

## v1.2.0 (M3 implemented)
- Deterministic builds now emit the BLAKE3 hash of the director configuration at startup so replay traces can pin the exact mission config set.【F:crates/game/src/lib.rs†L322-L369】
- Save loading funnels every legacy payload through the migrator, which dispatches on the embedded schema version and upgrades 1.0 saves into the 1.1 format automatically.【F:crates/game/src/systems/migrations/mod.rs†L1-L42】

## v1.1.0 (Save schema refresh)
- Bumped the save-game schema to v1.1 with explicit metadata headers.
- Captured the player's last visited hub and cargo manifest for trading persistence.

## v1.0.2 (M1 economy milestones)
- Locked in macro/micro money supply goals for milestone 1.
- Landed deterministic interest/basis adjustments in the game economy systems.
- Synced economy save/load schemas with the simulation harness for reproducible runs.
