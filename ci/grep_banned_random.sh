#!/usr/bin/env bash
set -euo pipefail

pattern='thread_rng|rand::random|std::time::Instant'
primary_dir='crates/game'
systems_glob='*/systems/**'

fail=false

if rg -n -S -E "$pattern" "$primary_dir" >/dev/null; then
  fail=true
fi

if rg -n -S -E "$pattern" --glob "$systems_glob" crates >/dev/null; then
  fail=true
fi

if [ "$fail" = true ]; then
  echo "banned nondeterministic APIs found" >&2
  rg -n -S -E "$pattern" "$primary_dir" || true
  rg -n -S -E "$pattern" --glob "$systems_glob" crates || true
  exit 1
fi
