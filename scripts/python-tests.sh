#!/usr/bin/env bash
# Builds the `hpr` Python package's wheel with maturin, then runs its pytest suite against the
# installed wheel on each Python named (default: 3.10 and 3.13). The wheel is abi3, one per
# operating system for CPython 3.10 and later, so the suite runs on the oldest Python it serves and
# a new one. CI's `python` job runs this on three operating systems, and `scripts/gate.sh python`
# runs it here.
#
# Needs uv (https://docs.astral.sh/uv/), which fetches the pinned maturin and pytest, NumPy and
# any Python not already installed. The wheel lands in target/wheels/, and nothing is published.
#
# Usage:
#   scripts/python-tests.sh              # 3.10 and 3.13
#   scripts/python-tests.sh 3.12         # just these Pythons

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

MATURIN="1.15.0"
PYTEST="9.1.1"
PYTHONS="${*:-3.10 3.13}"
OUT="target/wheels"

rm -rf "$OUT"
uvx --from "maturin==$MATURIN" maturin build --release --locked \
  --manifest-path crates/hpr-py/Cargo.toml --out "$OUT"
shopt -s nullglob
wheels=("$OUT"/*.whl)
if [ "${#wheels[@]}" -ne 1 ]; then
  echo "expected one wheel in $OUT, found ${#wheels[@]}" >&2
  exit 1
fi
for python in $PYTHONS; do
  echo "pytest on Python $python"
  uv run --isolated --no-project --python "$python" --with "${wheels[0]}" \
    --with "pytest==$PYTEST" -- python -m pytest -q crates/hpr-py/tests
done
