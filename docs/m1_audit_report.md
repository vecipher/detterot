# M1 Economy Audit Status

This document captures the current alignment of the economy code with the
Milestone 1 (M1) dossier checklist. Each section references the
implementation or tests that enforce the requirement.

## A. Repo & Hygiene
- `ci/.clippy.toml` denies `clippy::float_arithmetic` when the economy audit
  runs, ensuring float bans hold in CI. 【F:ci/.clippy.toml†L1-L4】
- `ci/grep_banned_random.sh` guards against `thread_rng` / `rand::random`
  within `crates/game/src/systems/economy/**`. 【F:ci/grep_banned_random.sh†L1-L13】
- `ci/local_audit_m1.sh` encapsulates the clippy, test, RNG, float, and
  golden CSV diff steps described in the dossier. 【F:ci/local_audit_m1.sh†L1-L19】
- The GitHub Actions workflow runs the clippy/test suite on macOS and Ubuntu
  and wires the dedicated `econ_goldens` job plus the RNG/float checks.
  【F:.github/workflows/ci.yml†L1-L55】【F:.github/workflows/ci.yml†L56-L74】

## B. Types & Rounding
- Fixed-point economy types live in `money.rs` and `types.rs`.
  【F:crates/game/src/systems/economy/money.rs†L1-L41】【F:crates/game/src/systems/economy/types.rs†L1-L34】
- Bankers rounding and floor-to-cents are implemented in `rounding.rs` with
  unit tests covering the edge cases. 【F:crates/game/src/systems/economy/rounding.rs†L1-L21】【F:crates/game/src/systems/economy/tests/pricing_rounding_golden.rs†L1-L62】

## C. Deterministic RNG
- `rng.rs` defines the `DetRng` wrapper with deterministic seeding, cursor
  tracking, and the bounded normal helper. 【F:crates/game/src/systems/economy/rng.rs†L1-L79】
- `rng_discipline.rs` exercises seed parity and clamp behaviour.
  【F:crates/game/src/systems/economy/tests/rng_discipline.rs†L1-L30】

## D. Pricing Function
- `compute_price` works entirely in fixed point, clamps multipliers, and
  applies the dossier’s rounding contract. 【F:crates/game/src/systems/economy/pricing.rs†L1-L38】
- The golden tests assert half-cent handling, monotonicity, saturation, and
  pricing bounds. 【F:crates/game/src/systems/economy/tests/pricing_rounding_golden.rs†L63-L120】

## E. Rulepack Loader (TOML)
- `rulepack.rs` models the schema, enables `deny_unknown_fields`, and logs a
  schema hash on load. 【F:crates/game/src/systems/economy/rulepack.rs†L1-L106】【F:crates/game/src/systems/economy/rulepack.rs†L146-L167】
- Tests cover happy-path loads, unknown key rejection, and missing sections.
  【F:crates/game/src/systems/economy/tests/rulepack_load.rs†L1-L55】
- The sample rulepack is documented inline. 【F:assets/rulepacks/day_001.toml†L1-L70】

## F. DI AR(1) + Overlay
- `di.rs` advances per-commodity DI with retention, noise, overlay decay, and
  clamps. 【F:crates/game/src/systems/economy/di.rs†L1-L71】
- `di_golden.rs` locks the 30-day deterministic sequence. 【F:crates/game/src/systems/economy/tests/di_golden.rs†L1-L55】

## G. Basis Drivers & Clamps
- `basis.rs` applies PP, weather, routes, stock drivers, and per-day/absolute
  clamps. 【F:crates/game/src/systems/economy/basis.rs†L1-L57】
- Goldens verify monotonicity and clamp enforcement.
  【F:crates/game/src/systems/economy/tests/basis_dynamics_golden.rs†L1-L75】

## H. Interest (Piecewise-Convex) & Caps
- `interest.rs` implements base, linear, and convex components with Q16 fixed
  point, plus the per-leg cap. 【F:crates/game/src/systems/economy/interest.rs†L1-L84】
- Tests assert golden deltas, cap behaviour, and rounding boundaries.
  【F:crates/game/src/systems/economy/tests/interest_piecewise_golden.rs†L1-L53】

## I. Rot Rails & Conversion
- `rot.rs` performs decay, chunking, and debt conversion under the configured
  rails. 【F:crates/game/src/systems/economy/rot.rs†L1-L20】
- Tests cover floor enforcement, idempotency, and decay precedence.
  【F:crates/game/src/systems/economy/tests/rot_convert.rs†L1-L45】

## J. Planting Pull (Economy Side)
- `planting.rs` schedules plantings, advances ages, and applies decay-driven
  PP pulls with clamps. 【F:crates/game/src/systems/economy/planting.rs†L1-L49】
- Tests exercise decay curves, neutrality, pull math, and cleanup.
  【F:crates/game/src/systems/economy/tests/planting_pull.rs†L1-L83】

## K. EconomyDay Orchestrator
- `state.rs` ties DI, basis, planting, rot→debt, and interest together with
  deterministic RNG cursors and logging hook. 【F:crates/game/src/systems/economy/state.rs†L1-L153】【F:crates/game/src/systems/economy/state.rs†L154-L231】
- Golden JSON asserts the 7-day integrated run and hub-only behaviour.
  【F:crates/game/src/systems/economy/tests/state_step.rs†L1-L92】【F:crates/game/src/systems/economy/tests/state_step_golden.json†L1-L70】

## L. Save v1 + Migrations
- `save.rs` defines `SaveV1`, normalises ordering, and enforces strict
  deserialization. 【F:crates/game/src/systems/save.rs†L1-L84】
- Migration stubs and errors live in `migrations/mod.rs` and `v1.rs`.
  【F:crates/game/src/systems/migrations/mod.rs†L1-L16】【F:crates/game/src/systems/migrations/v1.rs†L1-L9】
- Round-trip and unknown-key tests keep the schema stable.
  【F:crates/game/tests/serde_roundtrip.rs†L1-L71】

## M. Micro-Sim + Goldens
- The `econ-sim` CLI seeds state, advances hubs deterministically, and emits
  the CSV columns required by the dossier. 【F:crates/econ_sim/src/main.rs†L1-L135】
- Integration tests run the binary and compare against the golden CSV with
  structural checks. 【F:crates/econ_sim/tests/micro_sim_runs.rs†L1-L69】
- The golden file captures the reference curves. 【F:crates/econ_sim/tests/goldens/econ_curves_seed42.csv†L1-L37】

## N. CI Enforcement
- CI runs the econ sim diff, RNG/float guards, and clippy denial on both OSes.
  【F:.github/workflows/ci.yml†L33-L74】

## O. Determinism Harness Touchpoint
- The determinism workflow replays the existing M0 harness to ensure hash
  parity. 【F:.github/workflows/ci.yml†L22-L32】

## P. Logging (Feature-Gated)
- `log.rs` emits JSONL records only under the `econ_logs` feature and includes
  a smoke test gated by that feature. 【F:crates/game/src/systems/economy/log.rs†L1-L94】

## Q. Performance & Footprint
- No automated perf harness is checked in; the dossier only called for spot
  checks. Existing code paths operate purely in fixed point without dynamic
  allocations in the hot loops highlighted above.

## R. Docs & Versioning
- The rulepack file is fully commented. 【F:assets/rulepacks/day_001.toml†L1-L70】
- `CONTRIBUTING.md` reiterates the float ban and RNG discipline.
  【F:CONTRIBUTING.md†L4-L15】
- The plan changelog contains the v1.0.2 entry referencing the economy stack.
  【F:docs/plan_changelog.md†L1-L6】

## Outstanding Environment Limitation
Running `cargo test --workspace` locally currently fails because the container
image lacks the ALSA development libraries required by Bevy’s audio backend.
(See chunk `e86dce†L1-L43`.) CI installs `libasound2-dev`, so the workflow
remains green on supported platforms.
