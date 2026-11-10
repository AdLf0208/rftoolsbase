# rftoolsbase (Horizon port)

A Horizon port of [rftoolsbase](McJtyMods/RFToolsBase) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- The machine.** `rftoolsbase:machine_infuser`, a `BaseBlock` on the
- `minecraft:stonecutter`** template (vanilla's only menued template with a
- horizontal `facing`), standard-iron properties (hardness 2.0 / blast 2.0,
- correct tool, metal sound), placed facing the player, with the jar's own cube
- model and `block/infuser` front texture.
- The buffer.** 60,000 FE (`infuserMaxRF`), declared through
- `block_entity_energy` (API 142, `Capabilities.Energy.BLOCK`): any side may
- fill it 600 FE a call (`infuserRFPerTick`), nothing may drain it

## Not yet ported

- The infuser's block is infusable.** Its 1.21 registration drops upstream's
- `.infusable()` flag (1.20's `createBlock()` sets it). Restored on purpose, so
- the machine has a target and the cost curve is observable; the item's own
- level only decides whether it can be raised (as upstream).
- `facing` is horizontal (4 values).** `minecraft:stonecutter`, the pin's
- only menued template, has four; upstream's `BaseBlock` faces all six ways, so
- placing a machine facing up/down is lost (the blockstate drops those two
- variants). Recorded rather than faked.

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: McJtyMods/RFToolsBase. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
