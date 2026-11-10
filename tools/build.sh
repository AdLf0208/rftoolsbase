#!/usr/bin/env bash
# The compile tail of the ladder, in one build-slot lease: fmt -> clippy ->
# stage -> check -> test -> build. Run after a change that does not touch the
# pure runtime (whose red/green run is `tools/ladder.sh`):
#   BUILD_LOCAL=1 ports/tools/build-slot.sh run --job rfb-build --port ports/rftoolsbase \
#       --step build -- bash ports/rftoolsbase/tools/build.sh
set -Eeuo pipefail
export LC_ALL=C
PORT=$(cd "$(dirname "$0")/.." && pwd)
cd "$PORT"
LOGS=$PORT/target/ladder
mkdir -p "$LOGS" "$PORT/../_shared/evidence/rftoolsbase"

echo "== fmt (write, then check) =="
cargo fmt --all
rustfmt --edition 2024 mixins/rftoolsbase/*.rs
cargo fmt --all -- --check 2>&1 | tee "$LOGS/fmt.log"
rustfmt --edition 2024 --check mixins/rftoolsbase/*.rs 2>&1 | tee "$LOGS/fmt-helper.log"
echo "fmt: exit 0"

echo "== clippy =="
cargo clippy --all-targets --locked --offline -- -D warnings 2>&1 | tee "$LOGS/clippy.log"
echo "clippy: exit 0"

echo "== stage =="
python3 tools/horizon.py stage 2>&1 | tee "$LOGS/stage.log"
echo "== check =="
python3 tools/horizon.py check 2>&1 | tee "$LOGS/horizon-check.log"
echo "check: $(grep -o '"cargo_exit": [0-9]*' "$LOGS/horizon-check.log" | tail -1) ($(grep -c '^warning' "$LOGS/horizon-check.log") warnings)"
echo "== embedded tests =="
python3 tools/horizon.py test 2>&1 | tee "$LOGS/horizon-test.log"
echo "test: $(grep 'test result' "$PORT/target/horizon-test.log" | head -1)"
echo "== fast build =="
python3 tools/horizon.py build 2>&1 | tee "$LOGS/horizon-build.log"
echo "build: $(grep -o '"cargo_exit": [0-9]*' "$LOGS/horizon-build.log" | tail -1) ($(grep -c '^warning' "$LOGS/horizon-build.log") warnings)"

cp "$LOGS"/*.log "$PORT/../_shared/evidence/rftoolsbase/"
echo "== build tail done =="
