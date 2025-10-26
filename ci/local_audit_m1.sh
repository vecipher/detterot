#!/usr/bin/env bash
set -euo pipefail

cargo clippy -D warnings
cargo test --workspace

rg -n --glob 'crates/game/src/systems/economy/**' '\bf(32|64)\b' && { echo "FLOAT FOUND"; exit 1; } || true
rg -n 'thread_rng|rand::random' crates/game/src/systems/economy && { echo "BANNED RNG FOUND"; exit 1; } || true

mapfile -t gameplay_paths < <(compgen -G 'crates/game/src/**/*gameplay*' || true)
if (( ${#gameplay_paths[@]} > 0 )); then
  rg -n 'thread_rng|rand::random' "${gameplay_paths[@]}" && { echo "BANNED RNG FOUND"; exit 1; } || true
fi

cargo run -p econ-sim -- --world-seed 42 --days 15 --hubs 3 \
  --pp 1500,5000,9000 --debt 0,500_000_00,5_000_000_00 \
  --out target/econ_curves.csv

diff -u crates/econ_sim/tests/goldens/econ_curves_seed42.csv target/econ_curves.csv
