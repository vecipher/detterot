#!/usr/bin/env bash
set -euo pipefail

target_dir="crates/game/src"

if [ ! -d "$target_dir" ]; then
  echo "missing $target_dir" >&2
  exit 1
fi

pattern="thread_rng|rand::random|std::time::Instant::now|Instant::now"

if grep -R -n -E "$pattern" "$target_dir" >/dev/null; then
  echo "banned nondeterministic APIs found in $target_dir" >&2
  grep -R -n -E "$pattern" "$target_dir"
  exit 1
fi
