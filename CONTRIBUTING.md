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
- The `game` crate denies `clippy::float_arithmetic` globally. When non-economy code truly needs floats (UI readouts, perf tooling, etc.), add a module-level `#![allow(clippy::float_arithmetic)]` plus a short comment that explains the exception before landing the change.
- Seed all economy simulations through `DetRng`; CI's `ci/grep_banned_random.sh` blocks `thread_rng`/`rand::random` in that tree.
- Breaking either rule fails the `Economy invariants` job in the main workflow alongside the determinism checks.

## Formatting
- Run `cargo fmt --all` locally before pushing to avoid CI failures on the formatting check.
