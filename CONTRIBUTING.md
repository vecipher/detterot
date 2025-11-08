## Etiquette
- Feature branches; merge via PR with green CI.
- No TODOs without a linked issue.
- Changes to economy/saves/rulepacks must update tests and changelog.

## Performance budgets
- Gameplay ≤ 6 ms CPU; GPU ≤ 8/16/33 ms for 120/60/30 FPS tiers.

## Determinism
- Fixed timestep; DetRng only; stable system sets/order.

## Economy invariants
- `crates/game/src/systems/economy/**` must stay free of `f32`/`f64` usage; CI enforces this via the Clippy lint configuration (`crates/game/Cargo.toml` and `ci/.clippy.toml`).
- Trading code (`crates/game/src/systems/trading/**` and dependants) shares the same "no floats in the price path" rule—always use the fixed-point economy types when handling prices or basis values.
- The `game` crate denies `clippy::float_arithmetic` globally. When non-economy code truly needs floats (UI readouts, perf tooling, etc.), add a module-level `#![allow(clippy::float_arithmetic)]` plus a short comment that explains the exception before landing the change.
- Seed all economy simulations through `DetRng`; CI's `ci/grep_banned_random.sh` blocks `thread_rng`/`rand::random` in that tree.
- Breaking either rule fails the `Economy invariants` job in the main workflow alongside the determinism checks.

## Save format
- The runtime save schema is v1.1. Any change to save data must keep the migration tests up to date and refresh the assets changelog.

## Refreshing economy goldens
- Golden fixtures under `crates/econ_sim/tests/goldens/` and `crates/game/src/systems/economy/tests/state_step_golden.json` capture the deterministic outputs that CI enforces.
- When DI/basis/debt behavior changes intentionally, set `UPDATE_ECON_GOLDENS=1` and rerun the targeted tests so they rewrite their fixtures in place:
  ```
  UPDATE_ECON_GOLDENS=1 cargo test -p econ-sim micro_sim_generates_golden_csv
  UPDATE_ECON_GOLDENS=1 cargo test -p game state_step_matches_golden
  ```
- Inspect the diffs, re-run the tests without the env var to confirm the refreshed outputs, and include the updated golden files in your commit.

## Formatting
- Run `cargo fmt --all` locally before pushing to avoid CI failures on the formatting check.
