# Detterot

![CI](https://github.com/Vecipher/detterot/actions/workflows/ci.yml/badge.svg)

Foundation for the Detterot prototype. Launch the Bevy game from VS Code (F5) to see a neon cube lit in a simple scene with diagnostics overlay, Kira audio, and automated camera paths. Enable the optional `avian_physics` feature to swap the default grid/no-op physics loop for Avian 3D's schedule. Continuous integration keeps formatting, linting, tests, and determinism checks green on both macOS and Ubuntu.

## Getting started
1. Install the recommended VS Code extensions (Rust Analyzer, CodeLLDB, Even Better TOML).
2. Open the workspace and press <kbd>F5</kbd> ("Run game (debug)") to build and launch the window.
3. Explore `repro/perf_scenes.toml` and `repro/paths/` to tweak autoplay camera paths.
4. Run the debug config (F5) to build with `--features dev` (which enables `avian_physics`) and access Avian's collider debug overlay; release builds omit the extra debug plugin and stick to the deterministic grid physics loop.

## Tooling
- `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` must pass before merging.
- `tools/repro_harness` replays golden records and validates hashes for determinism checks.

See [CONTRIBUTING.md](CONTRIBUTING.md) for etiquette, performance expectations, and the economy invariants CI enforces.

## Planning docs
- Track milestone changes in [docs/plan_changelog.md](docs/plan_changelog.md); the latest entry covers the v1.0.2 M1 economy deliverables.

## Economy goldens
Economy determinism is guarded by CSV/JSON fixtures under `crates/econ_sim/tests/goldens/` and `crates/game/src/systems/economy/tests/`. Any intentional change to DI, basis, or interest math will move those fixtures. Regenerate them with the helper env var so the updated values are written back in-place:

```
UPDATE_ECON_GOLDENS=1 cargo test -p econ-sim micro_sim_generates_golden_csv
UPDATE_ECON_GOLDENS=1 cargo test -p game state_step_matches_golden
```

Both tests now read the golden data at runtime, so re-running them without the env var immediately verifies the refreshed outputs. For implementation details, review the economy bullets in [CONTRIBUTING.md](CONTRIBUTING.md) and the CI helpers under `ci/.clippy.toml` and `ci/grep_banned_random.sh`.
