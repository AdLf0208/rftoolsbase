// Game-side glue for RFToolsBase slice 1 (the Machine Infuser).
//
// Injected into `crate::game` as `rftoolsbase_state`; it needs the game, so it
// is not part of the standalone library (only `runtime.rs` is).
//
// The machine keeps upstream's state: the API 142 buffer's `energy` int, the
// `infusing` countdown, the block's own `infused` level and two slots — 0 takes
// `rftoolsbase:shards` items, 1 takes one infusable machine. Each server tick it
// spends one shard and `600 * (2 - infused/256) / 2` FE to start a five-tick
// infusion, and the last tick raises the machine item's `rftoolsbase:infused`
// component by one, up to 256.
//
// Read from RFToolsBase 1.21-6.0.11 (`MachineInfuserTileEntity`,
// `MachineInfuserConfiguration`, `MachineInfuserModule`, `GuiMachineInfuser`)
// and McJtyLib (`GenericEnergyStorage`, `DefaultInfusable`, `BaseBlock`). See
// `../../FINDINGS.md`.

use std::array;

use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::Game;
use crate::game::native_hooks::NativeScreen;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::rftoolsbase::{CAPACITY, INFUSING_TICKS, MAX_INFUSE, MAX_INSERT, Machine};

/// The machine block and its item (`MachineInfuserModule.MACHINE_INFUSER`).
pub const MACHINE_INFUSER: &str = "rftoolsbase:machine_infuser";
/// The item tag `MachineInfuserTileEntity.isShard` tests (`VariousModule.SHARDS`).
pub const SHARD_TAG: &str = "rftoolsbase:shards";
/// mcjtylib's `mcjtylib:infusable` component in the pin's own form.
pub const COMPONENT_INFUSED: &str = "rftoolsbase:infused";
/// The menu type the content mod declares (`data/rftoolsbase/horizon/menu/`).
pub const MENU: &str = "rftoolsbase:machine_infuser_menu";
/// Upstream's slot order (`MachineInfuserTileEntity.SLOT_SHARDINPUT/…OUTPUT`).
const SHARD_SLOT: usize = 0;
const TARGET_SLOT: usize = 1;
const SLOT_COUNT: usize = 2;
/// Upstream's saved keys.
const KEY_ENERGY: &str = "energy";
const KEY_INFUSING: &str = "infusing";
const KEY_INFUSED: &str = "infused";
/// The items whose block an infusion may raise. Upstream tests `BaseBlock
/// .isInfusable()`, and the Machine Infuser is RFToolsBase's only infusable
/// block; other RFTools mods' machines are not in this port.
const INFUSABLE_ITEMS: &[&str] = &[MACHINE_INFUSER];

// ---------------------------------------------------------------------------
// Declaration hooks
// ---------------------------------------------------------------------------

/// Registers the item component that carries the infused level (`component_types`, API 65).
pub fn on_component_types(types: &mut Vec<String>) {
    types.push(COMPONENT_INFUSED.to_owned());
}

/// The machine has a block entity (`native_block_entity`, API 15).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == MACHINE_INFUSER {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// ... and ticks on the server (`native_block_entity_ticks`, API 15).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == MACHINE_INFUSER {
        *ticks = true;
    }
}

/// Two slots and the machine menu (`native_block_entity_container`, API 16/17).
/// Upstream's handler takes shards in slot 0 and infusable machines in slot 1,
/// and automation may take the finished machine out of slot 1.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_container(block: &str, container: &mut Option<ModContainer>) {
    if block != MACHINE_INFUSER {
        return;
    }
    *container = Some(ModContainer {
        slots: SLOT_COUNT,
        faces: array::from_fn(|_| vec![SHARD_SLOT, TARGET_SLOT]),
        inputs: vec![SHARD_SLOT, TARGET_SLOT],
        outputs: vec![TARGET_SLOT],
        menu: Some(MENU.to_owned()),
    });
}

/// The FE buffer (`native_block_entity_energy`, API 142): 60,000 FE, 600 a
/// call, inserts only (`GenericEnergyStorage(this, true, 60000, 600)` has
/// `canExtract` false).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    if block == MACHINE_INFUSER {
        *energy = Some(ModEnergyBuffer {
            capacity: CAPACITY,
            max_insert: MAX_INSERT,
            max_extract: 0,
            insert_faces: [true; 6],
            extract_faces: [false; 6],
        });
    }
}

// ---------------------------------------------------------------------------
// The machine tick
// ---------------------------------------------------------------------------

/// One server tick (`native_block_entity_tick`, API 17),
/// `MachineInfuserTileEntity.tickServer`.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_tick(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id != MACHINE_INFUSER {
        return;
    }
    if entity.items.len() < SLOT_COUNT {
        entity.items.resize(SLOT_COUNT, None);
    }
    let own_infused = read_int(&entity.nbt, KEY_INFUSED).clamp(0, MAX_INFUSE);
    let mut machine = Machine::new();
    machine.energy = read_int(&entity.nbt, KEY_ENERGY).clamp(0, CAPACITY);
    machine.infusing = read_int(&entity.nbt, KEY_INFUSING).clamp(0, INFUSING_TICKS);
    let has_shard = entity.items[SHARD_SLOT]
        .as_ref()
        .is_some_and(|stack| stack.count > 0 && is_shard(game, stack));
    let target = target_level(game, entity);
    let tick = machine.tick(has_shard, target, own_infused);
    if tick.consumed_shard {
        take_shard(entity);
    }
    if tick.finished {
        raise_target(game, entity);
    }
    write_int(&mut entity.nbt, KEY_ENERGY, machine.energy);
    write_int(&mut entity.nbt, KEY_INFUSING, machine.infusing);
    let mut data = machine.data_slots().to_vec();
    data.push(own_infused);
    if entity.data != data {
        entity.data = data;
    }
    // `setChanged`: the chunk is saved with what this tick did.
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}

/// Is the item in slot 0 a `rftoolsbase:shards` entry (`TagTools.hasTag`)?
fn is_shard(game: &Game<'_>, stack: &ItemStack) -> bool {
    game.native_server_item_in_tag(stack, SHARD_TAG)
        .unwrap_or(false)
}

/// The infused level of the infusable machine in slot 1, or `None` when the
/// slot holds nothing this port can infuse.
fn target_level(game: &Game<'_>, entity: &ModBlockEntity) -> Option<i32> {
    let stack = entity.items.get(TARGET_SLOT)?.as_ref()?;
    is_infusable(stack).then(|| infused_level(game, stack))
}

/// `getStackIfInfusable`: an infusable machine block item, at any level (the
/// runtime refuses 256 itself).
fn is_infusable(stack: &ItemStack) -> bool {
    stack.count > 0 && INFUSABLE_ITEMS.contains(&&*stack.item)
}

/// The `rftoolsbase:infused` component, 0 when absent.
fn infused_level(game: &Game<'_>, stack: &ItemStack) -> i32 {
    match game.native_component(stack, COMPONENT_INFUSED) {
        Ok(Some(Tag::Int(level))) => (*level).clamp(0, MAX_INFUSE),
        _ => 0,
    }
}

/// `items.getStackInSlot(0).split(1)`.
fn take_shard(entity: &mut ModBlockEntity) {
    let count = match entity.items.get(SHARD_SLOT) {
        Some(Some(stack)) => stack.count,
        _ => return,
    };
    if count > 1 {
        if let Some(Some(stack)) = entity.items.get_mut(SHARD_SLOT) {
            stack.count -= 1;
        }
    } else {
        entity.items[SHARD_SLOT] = None;
    }
}

/// `finishInfusing`: `BaseBlock.setInfused(stack, getInfused(stack) + 1)`, for
/// an infusable machine still in slot 1 and not yet at the maximum (an item
/// removed during the five ticks is simply not raised).
fn raise_target(game: &Game<'_>, entity: &mut ModBlockEntity) {
    let Some(Some(stack)) = entity.items.get_mut(TARGET_SLOT) else {
        return;
    };
    if !is_infusable(stack) {
        return;
    }
    let level = infused_level(game, stack);
    if level >= MAX_INFUSE {
        return;
    }
    let _ = game.native_set_component(stack, COMPONENT_INFUSED, Tag::Int(level + 1));
}

// ---------------------------------------------------------------------------
// Client side: the screen
// ---------------------------------------------------------------------------

/// The machine's two slots (`native_screen`, API 19), at upstream's
/// `CONTAINER_FACTORY` pixels in the 180 x 152 panel of `GuiMachineInfuser`.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_screen(block: &str, screen: &mut Option<NativeScreen>) {
    if block != MACHINE_INFUSER {
        return;
    }
    *screen = Some(NativeScreen {
        width: 180.0,
        height: 152.0,
        slots: vec![Some([64.0, 24.0]), Some([118.0, 24.0])],
        inventory_y: 70.0,
        inventory_x: 10.0,
        own_labels: false,
        own_background: false,
    });
}

/// The machine's readouts (`native_screen_draw`, API 19): the buffer, the
/// block's infused level and the countdown, from the menu's data slots
/// `[energy/32, capacity/32, infusing, infused]`.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_screen_draw(
    game: &Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut HudBatch,
    origin: [f32; 2],
) {
    if block != MACHINE_INFUSER {
        return;
    }
    let energy = data.first().copied().unwrap_or(0) * 32;
    let capacity = data.get(1).copied().unwrap_or(0) * 32;
    let infusing = data.get(2).copied().unwrap_or(0);
    let infused = data.get(3).copied().unwrap_or(0);
    let energy_text = format!("{energy}/{capacity} FE");
    let infused_text = format!("infused {infused}/{MAX_INFUSE}");
    game.native_text(
        batch,
        &energy_text,
        [origin[0] + 8.0, origin[1] + 6.0],
        0x404040,
        false,
    );
    game.native_text(
        batch,
        &infused_text,
        [origin[0] + 8.0, origin[1] + 16.0],
        0x404040,
        false,
    );
    if infusing > 0 {
        game.native_text(
            batch,
            &format!("infusing {infusing}"),
            [origin[0] + 8.0, origin[1] + 26.0],
            0x404040,
            false,
        );
    }
}

// ---------------------------------------------------------------------------
// Placement and tooltip
// ---------------------------------------------------------------------------

/// An infused machine item seeds the block it is placed as
/// (`applyImplicitComponents` → `DefaultInfusable.load`), `native_place_block`
/// (API 14): the new machine infuses more cheaply.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_place_block(game: &mut Game<'_>, stack: &ItemStack, pos: [i32; 3]) {
    if &*stack.item != MACHINE_INFUSER || game.native_block(pos) != MACHINE_INFUSER {
        return;
    }
    let level = infused_level(game, stack);
    if level == 0 {
        return;
    }
    let _ = game.native_with_mod_block_entity(pos, |entity| {
        if entity.id == MACHINE_INFUSER {
            write_int(&mut entity.nbt, KEY_INFUSED, level);
        }
    });
}

/// The item's tooltip (`native_tooltip`, API 12): upstream's
/// `BaseBlock.intAddInformation` "Infused: N%" line, drawn once the item has a
/// level.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_tooltip(game: &Game<'_>, stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    if &*stack.item != MACHINE_INFUSER {
        return;
    }
    let level = infused_level(game, stack);
    if level == 0 {
        return;
    }
    let percent = level * 100 / MAX_INFUSE;
    lines.push((Text::literal(format!("Infused: {percent}%")), 0xFF_FF_55));
}

// ---------------------------------------------------------------------------
// The `/rftoolsbase` harness controls
// ---------------------------------------------------------------------------

/// The commands (`native_register_commands`, API 8). Upstream has none: they
/// exist so the harness can read and set a machine without a GUI (precedent:
/// `/charginggadgets info|energy`, `/powah info|set`).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("rftoolsbase info <pos:block_pos>"));
    commands.push(NativeCommand::op(
        "rftoolsbase energy <pos:block_pos> <amount:integer>",
        2,
    ));
    commands.push(NativeCommand::op(
        "rftoolsbase infused <pos:block_pos> <amount:integer>",
        2,
    ));
    commands.push(NativeCommand::op(
        "rftoolsbase level <pos:block_pos> <amount:integer>",
        2,
    ));
    commands.push(NativeCommand::op("rftoolsbase held <amount:integer>", 2));
}

/// `/rftoolsbase info|energy|infused|held` (`native_perform_command`, API 8).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_perform_command(
    game: &mut Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.first() != Some(&"rftoolsbase") {
        return;
    }
    *ran = true;
    match words.get(1).copied() {
        Some("info") => {
            let Some(pos) = parse_pos(words.get(2..).unwrap_or_default()) else {
                game.native_feedback(Text::literal(
                    "usage: /rftoolsbase info <x> <y> <z> [nonce]",
                ));
                return;
            };
            // The harness passes a nonce so it can tell its own reply from an
            // older one still in the client's chat.
            let nonce = words.get(5).copied();
            game.native_feedback(Text::literal(readout(game, pos, nonce)));
        }
        Some("energy") => {
            let Some((pos, amount)) = parse_pos_amount(words.get(2..).unwrap_or_default()) else {
                game.native_feedback(Text::literal(
                    "usage: /rftoolsbase energy <x> <y> <z> <amount>",
                ));
                return;
            };
            let amount = amount.clamp(0, CAPACITY);
            let changed = game
                .native_with_mod_block_entity(pos, |entity| {
                    if entity.id != MACHINE_INFUSER {
                        return false;
                    }
                    write_int(&mut entity.nbt, KEY_ENERGY, amount);
                    true
                })
                .unwrap_or(false);
            game.native_feedback(Text::literal(if changed {
                format!("Machine Infuser at {pos:?}: {amount}/{CAPACITY} FE.")
            } else {
                format!("{pos:?} is not a machine infuser.")
            }));
        }
        Some("infused") => {
            let Some((pos, amount)) = parse_pos_amount(words.get(2..).unwrap_or_default()) else {
                game.native_feedback(Text::literal(
                    "usage: /rftoolsbase infused <x> <y> <z> <amount>",
                ));
                return;
            };
            match set_target_level(game, pos, amount.clamp(0, MAX_INFUSE)) {
                Some(level) => game.native_feedback(Text::literal(format!(
                    "Machine Infuser at {pos:?}: the machine in slot 1 is infused {level}/{MAX_INFUSE}."
                ))),
                None => game.native_feedback(Text::literal(format!(
                    "{pos:?} has no infusable machine in slot 1."
                ))),
            }
        }
        Some("level") => {
            let Some((pos, amount)) = parse_pos_amount(words.get(2..).unwrap_or_default()) else {
                game.native_feedback(Text::literal(
                    "usage: /rftoolsbase level <x> <y> <z> <amount>",
                ));
                return;
            };
            let level = amount.clamp(0, MAX_INFUSE);
            let changed = game
                .native_with_mod_block_entity(pos, |entity| {
                    if entity.id != MACHINE_INFUSER {
                        return false;
                    }
                    write_int(&mut entity.nbt, KEY_INFUSED, level);
                    true
                })
                .unwrap_or(false);
            game.native_feedback(Text::literal(if changed {
                format!("Machine Infuser at {pos:?}: block infused {level}/{MAX_INFUSE}.")
            } else {
                format!("{pos:?} is not a machine infuser.")
            }));
        }
        Some("held") => {
            let Some(amount) = words.get(2).and_then(|word| word.parse::<i32>().ok()) else {
                game.native_feedback(Text::literal("usage: /rftoolsbase held <amount>"));
                return;
            };
            match set_held_level(game, amount.clamp(0, MAX_INFUSE)) {
                Some(level) => game.native_feedback(Text::literal(format!(
                    "Machine Infuser held item: infused {level}/{MAX_INFUSE}."
                ))),
                None => game.native_feedback(Text::literal("Hold a Machine Infuser item first.")),
            }
        }
        _ => game.native_feedback(Text::literal(
            "usage: /rftoolsbase info|energy|infused|level|held ...",
        )),
    }
}

/// The machine's state, as `/rftoolsbase info` prints it.
fn readout(game: &Game<'_>, pos: [i32; 3], nonce: Option<&str>) -> String {
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2]))
    else {
        return format!("{pos:?} is not a mod block entity.");
    };
    if entity.id != MACHINE_INFUSER {
        return format!("{pos:?} is a {} not a machine infuser.", entity.id);
    }
    let energy = read_int(&entity.nbt, KEY_ENERGY).clamp(0, CAPACITY);
    let infusing = read_int(&entity.nbt, KEY_INFUSING).clamp(0, INFUSING_TICKS);
    let infused = read_int(&entity.nbt, KEY_INFUSED).clamp(0, MAX_INFUSE);
    let shard = slot_text(entity.items.first().and_then(Option::as_ref));
    let target = match entity.items.get(TARGET_SLOT).and_then(Option::as_ref) {
        Some(stack) => format!(
            "{} x{} infused {}",
            stack.item,
            stack.count,
            infused_level(game, stack)
        ),
        None => "empty".to_owned(),
    };
    let suffix = nonce.map_or(String::new(), |nonce| format!(" nonce={nonce}"));
    format!(
        "Machine Infuser at {pos:?}: {energy}/{CAPACITY} FE, infusing {infusing}/{INFUSING_TICKS}, block infused {infused}/{MAX_INFUSE}, shard {shard}, target {target}, t={}{suffix}",
        game.world.game_time
    )
}

fn slot_text(stack: Option<&ItemStack>) -> String {
    match stack {
        Some(stack) => format!("{} x{}", stack.item, stack.count),
        None => "empty".to_owned(),
    }
}

/// Set the machine in the machine's slot 1 to `target` infused levels.
fn set_target_level(game: &mut Game<'_>, pos: [i32; 3], target: i32) -> Option<i32> {
    let key = (pos[0], pos[1], pos[2]);
    let mut stack = match game.world.block_entities.get(&key) {
        Some(BlockEntity::Mod(entity)) if entity.id == MACHINE_INFUSER => {
            entity.items.get(TARGET_SLOT).and_then(Option::clone)?
        }
        _ => return None,
    };
    if !is_infusable(&stack) {
        return None;
    }
    game.native_set_component(&mut stack, COMPONENT_INFUSED, Tag::Int(target))
        .ok()?;
    game.native_with_mod_block_entity(pos, |entity| {
        if entity.id == MACHINE_INFUSER {
            entity.items[TARGET_SLOT] = Some(stack);
        }
    })?;
    Some(target)
}

/// Set the machine item the acting player holds to `target` infused levels (the
/// harness's way to make an infused item).
fn set_held_level(game: &mut Game<'_>, target: i32) -> Option<i32> {
    let slot = game.held_slot();
    let mut stack = game.stack_in_mut(slot).take()?;
    if !is_infusable(&stack) {
        *game.stack_in_mut(slot) = Some(stack);
        return None;
    }
    let changed = game
        .native_set_component(&mut stack, COMPONENT_INFUSED, Tag::Int(target))
        .is_ok();
    *game.stack_in_mut(slot) = Some(stack);
    if changed {
        game.hand_stack_replaced(slot);
        return Some(target);
    }
    None
}

fn parse_pos(words: &[&str]) -> Option<[i32; 3]> {
    Some([
        words.first()?.parse().ok()?,
        words.get(1)?.parse().ok()?,
        words.get(2)?.parse().ok()?,
    ])
}

fn parse_pos_amount(words: &[&str]) -> Option<([i32; 3], i32)> {
    let pos = parse_pos(words)?;
    let amount = words.get(3)?.parse().ok()?;
    Some((pos, amount))
}

// ---------------------------------------------------------------------------
// Saved-compound reads
// ---------------------------------------------------------------------------

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&JavaString::from(key)) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(JavaString::from(key), Tag::Int(value));
}
