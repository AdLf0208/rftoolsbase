#!/usr/bin/env python3
"""Assemble the slice-1 `content/` folder from the pack's RFToolsBase jar.

The jar is the ATM10 file (`ports/_shared/licence-cache/rftoolsbase-1.21-6.0.11.jar`,
SHA-1 `9614bdc1…5aee`, the pack record's own hash), kept as the licence and
provenance evidence. Nothing is invented: the blockstate, models, textures, lang
lines, loot table, recipes, advancements and the `rftoolsbase:shards` tag come
out of the jar, re-spelled only where 26.3 differs (an ingredient `{"item": X}`
becomes `"X"` and `{"tag": T}` becomes `"#T"`; the loot table takes the singular
typed `condition`). The block/item/menu/tab declarations, the four 26.3 item
definitions and the `c:` tags the recipes name are written here, because the
carry has no oracle for a code-registered block and no convention tags without
one.

Rerun it to rebuild `content/` (it deletes its own generated folders first):

    python3 tools/assemble_content.py
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import zipfile

ROOT = Path(__file__).resolve().parents[1]
JAR = ROOT.parent / "_shared/licence-cache/rftoolsbase-1.21-6.0.11.jar"
CONTENT = ROOT / "content"
MOD = "rftoolsbase"
# The pack's Modrinth SHA-1 for rftoolsbase-1.21-6.0.11.jar.
JAR_SHA1 = "9614bdc12d6c633801b254a983d30dd4cbeb5aee"

# Carried byte-for-byte: the jar's own assets for this slice.
CARRIED_ASSETS = [
    "assets/rftoolsbase/blockstates/machine_infuser.json",
    "assets/rftoolsbase/models/block/machine_infuser.json",
    "assets/rftoolsbase/models/block/machine_frame.json",
    "assets/rftoolsbase/models/block/machine_base.json",
    "assets/rftoolsbase/models/block/rftools_logicblock.json",
    "assets/rftoolsbase/models/item/machine_infuser.json",
    "assets/rftoolsbase/models/item/machine_frame.json",
    "assets/rftoolsbase/models/item/machine_base.json",
    "assets/rftoolsbase/models/item/dimensionalshard.json",
    "assets/rftoolsbase/textures/block/infuser.png",
    "assets/rftoolsbase/textures/block/base/machinebottom.png",
    "assets/rftoolsbase/textures/block/base/machineside.png",
    "assets/rftoolsbase/textures/block/base/machinetop.png",
    "assets/rftoolsbase/textures/gui/infuser.png",
    "assets/rftoolsbase/textures/item/dimensionalsharditem.png",
    "data/rftoolsbase/advancement/recipes/misc/machine_infuser.json",
    "data/rftoolsbase/advancement/recipes/misc/machine_frame.json",
    "data/rftoolsbase/advancement/recipes/misc/machine_base.json",
    "data/rftoolsbase/advancement/recipes/misc/dimensionalshard.json",
    "data/rftoolsbase/loot_table/blocks/machine_infuser.json",
    "data/rftoolsbase/recipe/machine_infuser.json",
    "data/rftoolsbase/recipe/machine_frame.json",
    "data/rftoolsbase/recipe/machine_base.json",
    "data/rftoolsbase/recipe/dimensionalshard.json",
    "data/rftoolsbase/tags/item/shards.json",
]

# The machine is a `BaseBlock` (McJtyLib `BlockBuilder.STANDARD_IRON`: strength
# 2.0, correct tool for drops, metal sound, metal map colour) with a rotating
# `facing`. `minecraft:stonecutter` is vanilla's only template with a
# horizontal `facing` whose class opens a menu on use, which a mod container
# needs; its saw shape is overridden back to the full cube.
BLOCK = {
    "template": "minecraft:stonecutter",
    "properties": {
        "hardness": 2.0,
        "explosion_resistance": 2.0,
        "requires_tool": True,
        "sounds": "minecraft:iron_block",
        "occludes": True,
    },
    "shapes": {"default": [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]},
    "collision_shapes": {"default": [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]},
}

# `MachineInfuserModule.CONTAINER_MACHINE_INFUSER`: two slots; the port's menu
# adds the countdown and the block's own infused level to upstream's synced
# energy pair (energy/32, capacity/32).
MENU = {"slots": 2, "data_slots": 4}

# Upstream's own item group (`itemGroup.rftoolsbase`).
TAB = {
    "title": "itemGroup.rftoolsbase",
    "icon": f"{MOD}:machine_infuser",
    "items": [
        f"{MOD}:machine_infuser",
        f"{MOD}:machine_frame",
        f"{MOD}:machine_base",
        f"{MOD}:dimensionalshard",
    ],
}

# 26.3's item model definitions: the jar's own `models/item` file, named by it.
ITEM_MODELS = {
    "machine_infuser": f"{MOD}:item/machine_infuser",
    "machine_frame": f"{MOD}:item/machine_frame",
    "machine_base": f"{MOD}:item/machine_base",
    "dimensionalshard": f"{MOD}:item/dimensionalshard",
}

# The jar's recipes name `c:` convention tags the prepared 26.3 root does not
# ship (it ships `minecraft` tags only), and an unknown tag is an empty
# ingredient. These are the smallest honest memberships: NeoForge's `c:stones`
# is every stone, and the port lists the plain ones it can be sure of.
CONVENTION_TAGS = {
    "ingots/iron": ["minecraft:iron_ingot"],
    "dyes/blue": ["minecraft:blue_dye", "minecraft:lapis_lazuli"],
    "stones": [
        "minecraft:stone",
        "minecraft:cobblestone",
        "minecraft:granite",
        "minecraft:diorite",
        "minecraft:andesite",
        "minecraft:deepslate",
        "minecraft:tuff",
    ],
    "glass_blocks": ["minecraft:glass", "minecraft:tinted_glass"],
}

LANG = {
    "itemGroup.rftoolsbase": "RFTools Base",
    "block.rftoolsbase.machine_infuser": "Machine Infuser",
    "item.rftoolsbase.machine_frame": "Machine Frame",
    "item.rftoolsbase.machine_base": "Machine Base",
    "item.rftoolsbase.dimensionalshard": "Dimensional Shard",
    "message.rftoolsbase.machine_infuser.header": (
        "With this machine you can improve most other machines in RFTools in "
        "various ways. This needs dimensional shards"
    ),
    "message.rftoolsbase.machine_infuser.gold": (
        "Infusing bonus: reduced power consumption"
    ),
}

MANIFEST = f"""# Port-owned content mod for the RFToolsBase 1.21-6.0.11 slice: the Machine
# Infuser, the machine frame/base family and the dimensional shard. The models,
# textures, blockstate, loot table, recipes and advancements are upstream's own
# MIT assets, read from the exact ATM10 jar (see
# ../../_shared/carry-work/rftoolsbase/SOURCE.md).
id = "{MOD}"
name = "RFToolsBase"
version = "0.1.0"
api = "0.2"
side = "both"
capabilities = ["registries"]
"""


def upgrade_recipe(recipe: dict) -> dict:
    """The carry pipeline's 1.21.1 → 26.3 ingredient upgrade."""
    for key, value in list(recipe.get("key", {}).items()):
        if isinstance(value, dict) and "tag" in value:
            recipe["key"][key] = f"#{value['tag']}"
        elif isinstance(value, dict) and "item" in value:
            recipe["key"][key] = value["item"]
    return recipe


def upgrade_loot(loot: dict) -> dict:
    """The 26.3 loot table: one singular typed `condition` per pool, and the
    mod's own component copy (mcjtylib's `item_energy`/`item_inventory`/
    `infusable`) replaced by the plain block item — the port keeps the buffer and
    slots in the block entity, not on the item (recorded gap)."""
    for pool in loot.get("pools", []):
        conditions = pool.pop("conditions", None)
        # 26.3 takes one singular, typed condition (the 1.21.1 plural list is
        # silently false and every block would drop nothing).
        pool["condition"] = conditions[0] if conditions else {
            "type": "minecraft:survives_explosion"
        }
        for entry in pool.get("entries", []):
            entry.pop("functions", None)
    return loot


def patch_blockstate(state: dict) -> dict:
    """The port's `facing` is horizontal (the menued template has four values):
    keep north/south/east/west and drop the two vertical variants."""
    state["variants"] = {
        key: value
        for key, value in state["variants"].items()
        if key in ("facing=north", "facing=south", "facing=east", "facing=west")
    }
    return state


def dump(path: Path, value: dict | list) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main() -> None:
    data = JAR.read_bytes()
    assert hashlib.sha1(data).hexdigest() == JAR_SHA1, "the jar is not the pack's file"

    assets = CONTENT / "assets" / MOD
    data_dir = CONTENT / "data" / MOD
    for folder in (assets, data_dir, CONTENT / "data" / "c", CONTENT / "data" / "minecraft"):
        shutil.rmtree(folder, ignore_errors=True)

    with zipfile.ZipFile(JAR) as jar:
        for name in CARRIED_ASSETS:
            target = CONTENT / name
            target.parent.mkdir(parents=True, exist_ok=True)
            raw = jar.read(name)
            if name.endswith("blockstates/machine_infuser.json"):
                raw = (json.dumps(patch_blockstate(json.loads(raw)), indent=2) + "\n").encode()
            elif name.endswith("/recipe/machine_infuser.json") or name.endswith(
                "/recipe/machine_frame.json"
            ) or name.endswith("/recipe/machine_base.json") or name.endswith(
                "/recipe/dimensionalshard.json"
            ):
                raw = (json.dumps(upgrade_recipe(json.loads(raw)), indent=2) + "\n").encode()
            elif name.endswith("loot_table/blocks/machine_infuser.json"):
                raw = (json.dumps(upgrade_loot(json.loads(raw)), indent=2) + "\n").encode()
            target.write_bytes(raw)

    for item, model in ITEM_MODELS.items():
        dump(assets / "items" / f"{item}.json", {"model": {"type": "minecraft:model", "model": model}})

    dump(CONTENT / "data" / MOD / "horizon" / "block" / "machine_infuser.json", BLOCK)
    for item in ("machine_frame", "machine_base", "dimensionalshard"):
        dump(CONTENT / "data" / MOD / "horizon" / "item" / f"{item}.json", {"template": "minecraft:stick"})
    dump(CONTENT / "data" / MOD / "horizon" / "menu" / "machine_infuser_menu.json", MENU)
    dump(CONTENT / "data" / MOD / "horizon" / "creative_tab" / "rftoolsbase.json", TAB)

    for tag, values in CONVENTION_TAGS.items():
        dump(CONTENT / "data" / "c" / "tags" / "item" / f"{tag}.json", {"values": values})

    dump(
        CONTENT / "data" / "minecraft" / "tags" / "block" / "mineable" / "pickaxe.json",
        {"values": [f"{MOD}:machine_infuser"]},
    )

    dump(assets / "lang" / "en_us.json", LANG)
    (CONTENT / "horizon-mod.toml").write_text(MANIFEST)

    print(f"assembled {CONTENT} from {JAR.name}")


if __name__ == "__main__":
    main()
