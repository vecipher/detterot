#!/usr/bin/env bash
set -euo pipefail

targets=("crates/game/src")
pattern='thread_rng|rand::random|std::time::Instant'

if rg -n -S "$pattern" "${targets[@]}" >/dev/null; then
  echo "banned nondeterministic APIs found in gameplay code" >&2
  rg -n -S "$pattern" "${targets[@]}"
  exit 1
fi
