#!/usr/bin/env bash
set -euo pipefail

pattern='thread_rng|rand::random|std::time::Instant'
search_dir='crates/game/src'

if rg -n -E "$pattern" "$search_dir" >/dev/null; then
  echo "banned nondeterministic APIs found in $search_dir" >&2
  rg -n -E "$pattern" "$search_dir"
  exit 1
fi
