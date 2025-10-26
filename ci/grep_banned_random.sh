#!/usr/bin/env bash
set -euo pipefail

target_dir="src/systems/economy"
if [ ! -d "$target_dir" ]; then
  echo "missing $target_dir" >&2
  exit 1
fi

if grep -R -n -E "thread_rng|rand::random" "$target_dir" >/dev/null; then
  echo "banned RNG APIs found in $target_dir" >&2
  grep -R -n -E "thread_rng|rand::random" "$target_dir"
  exit 1
fi
