#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
CC="${CC:-cc}"
OUT_DIR="${OUT_DIR:-$(mktemp -d)}"
KEEP_OUT="${KEEP_OUT:-0}"

if [ "$KEEP_OUT" != "1" ]; then
    trap 'rm -rf "$OUT_DIR"' EXIT HUP INT TERM
else
    mkdir -p "$OUT_DIR"
fi

"$CC" -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/power" \
    "$ROOT/platform/h700/power/h700_cpu_performance_policy.c" \
    "$ROOT/platform/h700/power/h700_cpu_performance_service.c" \
    "$ROOT/platform/h700/power/tests/h700_cpu_performance_service_test.c" \
    -o "$OUT_DIR/h700_cpu_performance_service_test"

"$OUT_DIR/h700_cpu_performance_service_test"

"$CC" -std=c11 -O2 -Wall -Wextra -Werror \
    -I"$ROOT/platform/h700/power" \
    "$ROOT/platform/h700/power/h700_cpu_performance_policy.c" \
    "$ROOT/platform/h700/power/h700_cpu_performance_service.c" \
    "$ROOT/platform/h700/power/h700_cpu_performance_runtime.c" \
    "$ROOT/platform/h700/power/tests/h700_cpu_performance_runtime_test.c" \
    -o "$OUT_DIR/h700_cpu_performance_runtime_test"

"$OUT_DIR/h700_cpu_performance_runtime_test"

# Keep the policy-only contract covered independently from the service facade.
sh "$ROOT/scripts/test-h700-cpu-performance-policy.sh"
