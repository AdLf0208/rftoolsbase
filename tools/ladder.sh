#!/usr/bin/env bash
# The RFToolsBase ladder, in one build-slot lease:
#   red (todo!() bodies) -> green -> fmt -> clippy -> stage -> check -> test.
# Run through the registry, never bare:
#   ports/tools/build-slot.sh run --job rfb-ladder --port ports/rftoolsbase \
#       --step build -- env CARGO_INCREMENTAL=0 tools/ladder.sh
set -Eeuo pipefail
export LC_ALL=C
PORT=$(cd "$(dirname "$0")/.." && pwd)
cd "$PORT"
LOGS=$PORT/target/ladder
mkdir -p "$LOGS" "$PORT/target" "$PORT/../_shared/evidence/rftoolsbase"

RUNTIME=$PORT/mixins/rftoolsbase/runtime.rs
BACKUP=$(mktemp "$PORT/target/rftoolsbase-runtime.XXXXXX")
cp "$RUNTIME" "$BACKUP"
restore() { cp "$BACKUP" "$RUNTIME" 2>/dev/null || true; }
trap restore EXIT

echo "== red: the tests with todo!() bodies =="
STUB=$PORT/target/rftoolsbase-stub.rs
python3 - "$RUNTIME" <<'PY' > "$STUB"
import re, sys
source = open(sys.argv[1]).read()
def stub(signature, close):
    global source
    start = source.index(signature)
    end = source.index(close, start) + len(close)
    source = source[:start] + signature + " {\n    todo!()\n}\n" + source[end:]
stub("pub fn shard_cost(infused: i32) -> i32", "\n}\n")
stub("""pub fn tick(
        &mut self,
        has_shard: bool,
        target_infused: Option<i32>,
        block_infused: i32,
    ) -> Tick""", "\n    }\n")
stub("pub fn data_slots(&self) -> [i32; 3]", "\n    }\n")
sys.stdout.write(source)
PY
cp "$STUB" "$RUNTIME"
set +e
cargo test --locked --offline 2>&1 | tee "$LOGS/test-red.log"
RED=${PIPESTATUS[0]}
set -e
cp "$BACKUP" "$RUNTIME"
trap - EXIT
rm -f "$BACKUP"
if [ "$RED" -eq 0 ]; then
    echo "the stubbed runtime passed: the tests do not exercise it" >&2
    exit 1
fi
if ! grep -q "test result: FAILED" "$LOGS/test-red.log"; then
    echo "the red run did not fail at an assertion: see $LOGS/test-red.log" >&2
    exit 1
fi
echo "red: cargo_exit=$RED ($(grep -c '^test .* \.\.\. FAILED' "$LOGS/test-red.log" || true) failed)"

echo "== green: the real runtime =="
cargo test --locked --offline 2>&1 | tee "$LOGS/test-green.log"
echo "green: $(grep -m1 'test result' "$LOGS/test-green.log")"

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
echo "== embedded tests =="
python3 tools/horizon.py test 2>&1 | tee "$LOGS/horizon-test.log"
echo "test: $(grep -m1 'test result' "$LOGS/horizon-test.log")"
echo "== fast build =="
python3 tools/horizon.py build 2>&1 | tee "$LOGS/horizon-build.log"
echo "build: $(grep -o '"cargo_exit": [0-9]*' "$LOGS/horizon-build.log" | tail -1)"

cp "$LOGS"/*.log "$PORT/../_shared/evidence/rftoolsbase/"
echo "== ladder done =="
