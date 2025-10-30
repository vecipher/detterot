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
- Trading price paths (`crates/game/src/systems/trading/**`) inherit the same "no floats" rule. Quote calculation, wallet math, fee accrual, and capacity enforcement must stay on `MoneyCents`/integer types so deterministic rounding holds end-to-end.
- Seed all economy simulations through `DetRng`; CI's `ci/grep_banned_random.sh` blocks `thread_rng`/`rand::random` in that tree.
- Breaking either rule fails the `Economy invariants` job in the main workflow alongside the determinism checks.

## Refreshing economy goldens
- Golden fixtures under `crates/econ_sim/tests/goldens/` and `crates/game/src/systems/economy/tests/state_step_golden.json` capture the deterministic outputs that CI enforces.
- When DI/basis/debt behavior changes intentionally, set `UPDATE_ECON_GOLDENS=1` and rerun the targeted tests so they rewrite their fixtures in place:
  ```
  UPDATE_ECON_GOLDENS=1 cargo test -p econ-sim micro_sim_generates_golden_csv
  UPDATE_ECON_GOLDENS=1 cargo test -p game state_step_matches_golden
  ```
- Inspect the diffs, re-run the tests without the env var to confirm the refreshed outputs, and include the updated golden files in your commit.

## Refreshing trading goldens
- Scripted trading fixtures live under `repro/trading/` and are validated by `cargo test -p game --features deterministic --test trading_replay`.
- To update them after intentional logic changes, set `DETTEROT_UPDATE_GOLDENS=1` so the test rewrites the canonical JSON snapshots and `.hash` digests:
  ```
  DETTEROT_UPDATE_GOLDENS=1 cargo test -p game --features deterministic --test trading_replay
  ```
- Inspect the changes, rerun the test without the environment variable to confirm the new outputs, and commit the refreshed goldens alongside the code change.

## Refreshing save migrations
- Save schema migrations live under `crates/game/src/systems/migrations/` and are exercised by the integration test suite.
- When a migration or save schema update lands, rerun the byte-stability guard and commit any fixture updates:
  ```
  cargo test -p game migrate_roundtrip
  ```
- Document the schema bump in `docs/plan_changelog.md` and update any deterministic config hashes affected by the change.

## Formatting
- Run `cargo fmt --all` locally before pushing to avoid CI failures on the formatting check.
