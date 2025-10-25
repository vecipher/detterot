# Detterot

![CI](https://github.com/Vecipher/detterot/actions/workflows/ci.yml/badge.svg)

Foundation for the Detterot prototype. Launch the Bevy game from VS Code (F5) to see a neon cube lit in a simple scene with diagnostics overlay, Avian 3D physics, Kira audio, and automated camera paths. Continuous integration keeps formatting, linting, tests, and determinism checks green on both macOS and Ubuntu.

## Getting started
1. Install the recommended VS Code extensions (Rust Analyzer, CodeLLDB, Even Better TOML).
2. Open the workspace and press <kbd>F5</kbd> ("Run game (debug)") to build and launch the window.
3. Explore `repro/perf_scenes.toml` and `repro/paths/` to tweak autoplay camera paths.
4. Run the debug config (F5) to build with `--features dev` and access Avian's collider debug overlay; release builds omit the extra debug plugin.

## Tooling
- `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` must pass before merging.
- `tools/repro_harness` replays golden records and validates hashes for determinism checks.

See [CONTRIBUTING.md](CONTRIBUTING.md) for etiquette and performance expectations.
