#!/usr/bin/env bash
# In-game check for RFToolsBase slice 1: the Machine Infuser spends dimensional
# shards and FE to raise a machine's infused level, and the level makes later
# infusions cheaper.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200 (away from the central island, its dragon egg
# and the ambient item entities of a generated level). Every machine stands at
# x=202 on the stone floor, spaced 4 apart in z.
#
# Default run:
#  A  a machine placed from its item is empty: 0/60,000 FE, no shards, no target
#  B  a shard and a machine with 599 FE infuse nothing (599 < the 600 cost)
#  C  exactly 600 FE infuses one shard: the item goes 0 -> 1 and the buffer to 0
#  D  at block level 255 the cost is 301 FE (the curve), so 600 -> 299 while the
#     item still goes 0 -> 1
#  E  at item level 256 a shard and 600 FE do nothing (the maximum)
#  F  a non-shard (dirt) in the shard slot with 600 FE does nothing
#  G  an item infused to 100 and placed as a block seeds the block's level; a
#     shard then costs 482 FE (600 -> 118) and a fresh item goes 0 -> 1
#
# EMBEDDED=1 launches the game with **only** the native bundle (no `--dev-mod`)
# and takes the content from the bundle's embedded copy (`mixins/<id>/data/…`,
# the AE2 pattern); it also checks the file the helper wrote and, with CONTROL=1,
# first launches the same binary with `.horizon-cache` unusable, where the ids
# must be unknown.
#
# SELFTEST=1 flips C's assertion (the buffer must still read 600), proving the
# scenario can fail.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${RFB_DEBUG:-}" ] || set -x

EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/rftoolsbase
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/rftoolsbase-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=RFToolsBaseE2E
CONTROL_WORLD=RFToolsBaseControl
X0=200
SX=$((X0 + 2))
EXPECT_CAPACITY=60000
EXPECT_MAX_INFUSE=256
BLOCK=rftoolsbase:machine_infuser
SHARD_ITEM=rftoolsbase:dimensionalshard

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/rftoolsbase-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 60 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --headless
                --wait --resources "$RES")
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    else
        args+=(--dev-mod "$PORT/content")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

arena() {
    ctl command "execute in minecraft:the_end run tp @s $X0.5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 100 -14 $((X0 + 4)) 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 101 -14 $((X0 + 4)) 108 14 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $X0.5 101 0.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# `/rftoolsbase info X Y Z`: the line, then the parsed fields.
NONCE=0
info() { # info X Y Z
    NONCE=$((NONCE + 1))
    say "rftoolsbase info $1 $2 $3 $NONCE"
    ctl wait 12 >/dev/null
    INFO_LINE=$(ctl messages 20 | jq -r '.chat[]' | grep -m1 "nonce=$NONCE" || true)
    [ -n "$INFO_LINE" ] || fail "no fresh /rftoolsbase info reply for nonce $NONCE: $(ctl messages 8 | jq -c '.chat')"
    eval "$(python3 "$PORT/tools/parse_info.py" "$INFO_LINE")"
    [ "${ok:-0}" = 1 ] || fail "the info line did not parse: $INFO_LINE"
    echo "  $INFO_LINE" >&2
    echo "  parsed: energy=$ENERGY shards=$SHARD_COUNT item=$TARGET_LEVEL block=$BLOCK_INFUSED t=$T" >&2
}

expect_energy() { [ "$ENERGY" = "$1" ] || fail "the machine holds $ENERGY FE, expected $1"; }
expect_level() { [ "$TARGET_LEVEL" = "$1" ] || fail "the machine in slot 1 is infused $TARGET_LEVEL, expected $1"; }
expect_shards() { [ "$SHARD_COUNT" = "$1" ] || fail "the shard slot holds $SHARD_COUNT, expected $1"; }

# Hold one machine item in the first hotbar slot.
hold_machine() {
    ctl command "item replace entity @s hotbar.0 with $BLOCK 1" >/dev/null
    ctl wait 4 >/dev/null
    ctl select 1 >/dev/null
    ctl wait 2 >/dev/null
}

place_machine() { # place_machine X Y Z
    local x=$1 y=$2 z=$3
    ctl command "execute in minecraft:the_end run tp @s $((x - 2)).5 $y $z.5" >/dev/null
    ctl wait 6 >/dev/null
    hold_machine
    local aim reply
    aim=$(ctl look-at-block "$x" $((y - 1)) "$z" up 2>&1) \
        || fail "cannot aim at the floor under $x,$y,$z: $aim"
    reply=$(ctl place "$x" "$y" "$z" --item "$BLOCK" 2>&1) \
        || fail "ctl place at $x,$y,$z failed: $(tr -d '\n' <<<"$reply" | cut -c1-300)"
    ctl wait 6 >/dev/null
    local name
    name=$(block_name "$x" "$y" "$z")
    [ "$name" = "$BLOCK" ] || fail "the machine did not place at $x,$y,$z (got '$name')"
}

# `ctl container` fills the first empty slot, so the shard goes in before the
# machine: slot 0 first, then slot 1.
give_shard() { ctl container "$1" "$2" "$3" "$SHARD_ITEM" 1 >/dev/null; ctl wait 6 >/dev/null; }
give_machine() { ctl container "$1" "$2" "$3" "$BLOCK" 1 >/dev/null; ctl wait 6 >/dev/null; }
give_dirt() { ctl container "$1" "$2" "$3" minecraft:dirt 1 >/dev/null; ctl wait 6 >/dev/null; }
set_energy() { say "rftoolsbase energy $1 $2 $3 $4"; ctl wait 12 >/dev/null; }
set_level() { say "rftoolsbase infused $1 $2 $3 $4"; ctl wait 12 >/dev/null; }
set_block_level() { say "rftoolsbase level $1 $2 $3 $4"; ctl wait 12 >/dev/null; }

control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch "$CONTROL_WORLD"
    arena
    ctl command "item replace entity @s hotbar.0 with $BLOCK 1" >/dev/null || true
    ctl wait 4 >/dev/null
    ctl inventory | jq -e "[.hotbar[], .main[] | select(.item == \"$BLOCK\")] | length == 0" >/dev/null \
        || fail "the control game has the machine item, so the content did not come from the embedded file"
    ctl command "execute in minecraft:the_end run setblock $SX 101 0 $BLOCK" >/dev/null || true
    ctl wait 4 >/dev/null
    local name
    name=$(block_name "$SX" 101 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no machine item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$SCRATCH/.horizon-cache"
}

embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/rftoolsbase-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/rftoolsbase.hmod" \
        || fail "the embedded content differs from content/rftoolsbase.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/rftoolsbase.hmod"
}

echo "== launch (The End arena, content + native bundle) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
arena

ctl command "item replace entity @s hotbar.0 with $BLOCK 1" >/dev/null
ctl wait 4 >/dev/null
ctl inventory | jq -e "[.hotbar[], .main[] | select(.item == \"$BLOCK\")] | length >= 1" >/dev/null \
    || fail "the machine item did not register: $(ctl inventory | jq -c '{hotbar,main}')"
ctl give "$SHARD_ITEM" 1 >/dev/null
ctl wait 4 >/dev/null
echo "  items: $(ctl inventory | jq -c '[.hotbar[], .main[] | select(.item != null) | .item | select(startswith("rftoolsbase:"))] | unique')"

echo "== A: a machine placed from its item is empty =="
place_machine "$SX" 101 0
info "$SX" 101 0
[ "$CAPACITY" = "$EXPECT_CAPACITY" ] || fail "the machine's buffer reads $CAPACITY, expected $EXPECT_CAPACITY"
[ "$MAX_INFUSE" = "$EXPECT_MAX_INFUSE" ] || fail "the maximum infused level reads $MAX_INFUSE, expected $EXPECT_MAX_INFUSE"
[ "$ENERGY" = 0 ] || fail "a fresh machine holds $ENERGY FE"
[ "$INFUSING" = 0 ] || fail "a fresh machine is infusing $INFUSING"
[ "$BLOCK_INFUSED" = 0 ] || fail "a fresh machine's block is infused $BLOCK_INFUSED"
[ "$SHARD_COUNT" = 0 ] || fail "a fresh machine has $SHARD_COUNT shards"
[ "$TARGET_COUNT" = 0 ] || fail "a fresh machine has a target"
echo "  block $(block_name "$SX" 101 0), $ENERGY/$CAPACITY FE, block infused $BLOCK_INFUSED/$MAX_INFUSE"

echo "== B: 599 FE is one short of the 600 a shard costs =="
give_shard "$SX" 101 0
give_machine "$SX" 101 0
set_energy "$SX" 101 0 599
info "$SX" 101 0
case "$SHARD" in
    *rftoolsbase:dimensionalshard*) ;;
    *) fail "the shard slot holds $SHARD, not the shard" ;;
esac
[ "$SHARD_COUNT" = 1 ] || fail "the shard was consumed with only 599 FE"
[ "$TARGET_LEVEL" = 0 ] || fail "the machine was infused with only 599 FE ($TARGET_LEVEL)"
ctl wait 40 >/dev/null
info "$SX" 101 0
expect_energy 599
expect_shards 1
expect_level 0
echo "  599 FE for 40 ticks: still $ENERGY FE, $SHARD_COUNT shard, item at $TARGET_LEVEL"

echo "== C: exactly 600 FE infuses one shard, so the item goes 0 -> 1 =="
set_energy "$SX" 101 0 600
ctl wait 40 >/dev/null
info "$SX" 101 0
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting the machine still holds 600)"
    expect_energy 600
else
    expect_energy 0
fi
expect_shards 0
expect_level 1
echo "  one shard: $ENERGY FE left, $SHARD_COUNT shards, item infused $TARGET_LEVEL"

echo "== D: at block level 255 a shard costs 301 FE =="
give_shard "$SX" 101 0
set_level "$SX" 101 0 0
set_block_level "$SX" 101 0 255
set_energy "$SX" 101 0 600
ctl wait 40 >/dev/null
info "$SX" 101 0
expect_energy 299
expect_shards 0
expect_level 1
echo "  600 - 301 = $ENERGY FE, item $TARGET_LEVEL"

echo "== E: at item level 256 a shard and 600 FE do nothing =="
give_shard "$SX" 101 0
set_level "$SX" 101 0 256
set_block_level "$SX" 101 0 0
set_energy "$SX" 101 0 600
ctl wait 40 >/dev/null
info "$SX" 101 0
expect_energy 600
expect_shards 1
expect_level 256
echo "  capped: still $ENERGY FE and $SHARD_COUNT shard, item $TARGET_LEVEL"

echo "== F: a non-shard in the shard slot does nothing =="
place_machine "$SX" 101 -4
give_dirt "$SX" 101 -4
give_machine "$SX" 101 -4
set_energy "$SX" 101 -4 600
ctl wait 40 >/dev/null
info "$SX" 101 -4
expect_energy 600
expect_level 0
[ "$SHARD_COUNT" = 1 ] || fail "the dirt stack vanished"
echo "  dirt: still $ENERGY FE, item $TARGET_LEVEL"

echo "== G: an item infused to 100 seeds the block; a shard then costs 482 FE =="
hold_machine
say "rftoolsbase held 100"
ctl wait 12 >/dev/null
HELD=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'held item' || true)
echo "  $HELD"
case "$HELD" in
    *"infused 100/256"*) ;;
    *) fail "the held machine was not infused: $HELD" ;;
esac
ctl command "execute in minecraft:the_end run tp @s $X0.5 101 8.5" >/dev/null
ctl wait 6 >/dev/null
ctl look-at-block "$SX" 100 8 up >/dev/null
ctl wait 2 >/dev/null
ctl place "$SX" 101 8 --item "$BLOCK" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$SX" 101 8)" = "$BLOCK" ] || fail "the infused machine item did not place"
info "$SX" 101 8
[ "$BLOCK_INFUSED" = 100 ] || fail "the placed block is infused $BLOCK_INFUSED, expected the item's 100"
give_shard "$SX" 101 8
give_machine "$SX" 101 8
set_energy "$SX" 101 8 600
ctl wait 40 >/dev/null
info "$SX" 101 8
expect_energy 118
expect_shards 0
expect_level 1
echo "  block infused $BLOCK_INFUSED: 600 - 482 = $ENERGY FE, item $TARGET_LEVEL"

echo "PASS: a fresh machine is empty; 599 FE is one short of a shard's 600; exactly 600 infuses one shard and raises the item 0->1; at block level 255 a shard costs 301 FE (600->299) while the item still goes 0->1; at item level 256 a shard is refused; a non-shard is refused; and an item infused to 100 placed as a block makes a shard cost 482 FE (600->118, 0->1)"
