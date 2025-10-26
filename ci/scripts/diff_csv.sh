#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <expected> <actual>" >&2
  exit 1
fi

expected="$1"
actual="$2"

if cmp -s "$expected" "$actual"; then
  exit 0
fi

echo "CSV mismatch between $expected and $actual" >&2
if command -v diff >/dev/null 2>&1; then
  diff -u "$expected" "$actual" || true
fi
exit 1
