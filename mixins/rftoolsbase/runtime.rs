// Pure RFToolsBase slice-1 arithmetic (standalone crate lib, injected as
// `crate::game::rftoolsbase`). No game access lives here: the Machine Infuser's
// shard cost, its five-tick infusion cycle, and the numbers its menu shows.
//
// Upstream (`MachineInfuserTileEntity.tickServer/startInfusing/finishInfusing`,
// read from RFToolsBase 1.21-6.0.11) holds a 60,000 FE buffer that only takes
// energy (600 FE per insert call), two slots — 0 takes dimensional shards, 1
// takes one infusable machine — and an `infusing` countdown. Starting an
// infusion spends one shard and `600 * (2 - infused/256) / 2` FE, where
// `infused` is the machine block's own level (0..=256); five ticks later the
// machine item in slot 1 gains one infused level, up to 256.

/// `MachineInfuserConfiguration.MAXENERGY`: the FE buffer's size.
pub const CAPACITY: i32 = 60_000;
/// `MachineInfuserConfiguration.RECEIVEPERTICK`: the most one insert moves.
pub const MAX_INSERT: i32 = 600;
/// `MachineInfuserConfiguration.RFPERTICK`: the base FE an infusion costs at
/// infused level 0.
pub const BASE_COST: i32 = 600;
/// `MachineInfuserConfiguration.MAX_INFUSE`: infused levels a machine can take
/// (and the divisor of `getInfusedFactor`).
pub const MAX_INFUSE: i32 = 256;
/// `startInfusing`: the `infusing` countdown one shard sets.
pub const INFUSING_TICKS: i32 = 5;

/// `startInfusing`'s cost: `(int) (RFPERTICK * (2.0f - infusedFactor) / 2.0f)`
/// with `infusedFactor = infused / MAX_INFUSE`, so 600 FE at level 0, 301 at
/// 255 and 300 at 256. Java's float arithmetic truncates toward zero; the same
/// expression in `f32` keeps every step identical.
#[must_use]
pub fn shard_cost(infused: i32) -> i32 {
    let factor = infused.clamp(0, MAX_INFUSE) as f32 / MAX_INFUSE as f32;
    (BASE_COST as f32 * (2.0 - factor) / 2.0) as i32
}

/// What one server tick of the machine did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tick {
    /// An infusion started: the FE and the shard were spent.
    pub started: bool,
    /// The tick took one shard out of slot 0 (`started` implies this).
    pub consumed_shard: bool,
    /// The countdown reached zero: the item in slot 1 gains one level.
    pub finished: bool,
    /// The FE `started` spent (0 otherwise).
    pub cost: i32,
}

/// The machine's saved state, under upstream's keys (`energy`, and the
/// attachment's `infusing`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Machine {
    /// Stored FE (upstream's `GenericEnergyStorage.energy`).
    pub energy: i32,
    /// Infusing ticks left; 0 is idle.
    pub infusing: i32,
}

impl Machine {
    /// An empty machine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `MachineInfuserTileEntity.tickServer`, in one tick. `has_shard` is slot
    /// 0 holding a `rftoolsbase:shards` item; `target_infused` is the level of
    /// the infusable machine in slot 1 (`None` when that slot holds nothing it
    /// can infuse, or holds one already at 256); `block_infused` is the
    /// machine block's own level, which sets the cost (`startInfusing` reads
    /// `this.infusable`, not the item's).
    ///
    /// The tick that starts an infusion only sets the countdown (upstream's
    /// `else` branch); the following `INFUSING_TICKS` ticks count it down, and
    /// the last one reports `finished`.
    #[must_use]
    pub fn tick(
        &mut self,
        has_shard: bool,
        target_infused: Option<i32>,
        block_infused: i32,
    ) -> Tick {
        let mut tick = Tick::default();
        if self.infusing > 0 {
            self.infusing -= 1;
            tick.finished = self.infusing == 0;
            return tick;
        }
        let Some(infused) = target_infused else {
            return tick;
        };
        if !has_shard || infused >= MAX_INFUSE {
            return tick;
        }
        let cost = shard_cost(block_infused);
        if self.energy < cost {
            return tick;
        }
        self.energy -= cost;
        self.infusing = INFUSING_TICKS;
        tick.started = true;
        tick.consumed_shard = true;
        tick.cost = cost;
        tick
    }

    /// The menu's data slots: the buffer and its size in 32-FE units (so both
    /// fit a 16-bit value) and the countdown. Upstream draws an energy bar from
    /// the first two; the third is this port's readout.
    #[must_use]
    pub fn data_slots(&self) -> [i32; 3] {
        [self.energy / 32, CAPACITY / 32, self.infusing]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_uninfused_machine_pays_the_base_600_fe() {
        assert_eq!(shard_cost(0), 600);
    }

    #[test]
    fn the_cost_falls_by_about_one_fe_per_two_levels() {
        // (int)(600 * (2 - 1/256) / 2) = (int)598.828 = 598
        assert_eq!(shard_cost(1), 598);
        assert_eq!(shard_cost(100), 482);
        assert_eq!(shard_cost(255), 301);
        assert_eq!(shard_cost(256), 300);
    }

    #[test]
    fn a_level_outside_the_range_is_clamped() {
        assert_eq!(shard_cost(-5), 600);
        assert_eq!(shard_cost(9999), 300);
    }

    #[test]
    fn a_tick_without_a_shard_does_nothing() {
        let mut machine = Machine {
            energy: 60_000,
            infusing: 0,
        };
        assert_eq!(machine.tick(false, Some(0), 0), Tick::default());
        assert_eq!(machine.energy, 60_000);
        assert_eq!(machine.infusing, 0);
    }

    #[test]
    fn a_tick_without_an_infusable_target_does_nothing() {
        let mut machine = Machine {
            energy: 60_000,
            infusing: 0,
        };
        assert_eq!(machine.tick(true, None, 0), Tick::default());
        assert_eq!(machine.energy, 60_000);
    }

    #[test]
    fn less_fe_than_the_cost_does_not_start() {
        let mut machine = Machine {
            energy: 599,
            infusing: 0,
        };
        assert_eq!(machine.tick(true, Some(0), 0), Tick::default());
        assert_eq!(machine.energy, 599);
        assert_eq!(machine.infusing, 0);
    }

    #[test]
    fn exactly_the_cost_starts_and_spends_one_shard() {
        let mut machine = Machine {
            energy: 600,
            infusing: 0,
        };
        let tick = machine.tick(true, Some(0), 0);
        assert!(tick.started);
        assert!(tick.consumed_shard);
        assert!(!tick.finished);
        assert_eq!(tick.cost, 600);
        assert_eq!(machine.energy, 0);
        assert_eq!(machine.infusing, INFUSING_TICKS);
    }

    #[test]
    fn a_started_infusion_finishes_five_ticks_later() {
        let mut machine = Machine {
            energy: 600,
            infusing: 0,
        };
        let _ = machine.tick(true, Some(0), 0);
        for remaining in (1..=4).rev() {
            let tick = machine.tick(true, Some(0), 0);
            assert!(!tick.finished);
            assert_eq!(machine.infusing, remaining);
        }
        let tick = machine.tick(true, Some(0), 0);
        assert!(tick.finished);
        assert_eq!(machine.infusing, 0);
        assert_eq!(machine.energy, 0, "a countdown spends no more FE");
    }

    #[test]
    fn a_countdown_does_not_start_a_second_infusion() {
        let mut machine = Machine {
            energy: 60_000,
            infusing: 0,
        };
        let _ = machine.tick(true, Some(0), 0);
        let tick = machine.tick(true, Some(0), 0);
        assert!(!tick.started);
        assert!(!tick.consumed_shard);
        assert_eq!(machine.energy, 59_400);
    }

    #[test]
    fn a_machine_at_the_maximum_takes_no_more_shards() {
        let mut machine = Machine {
            energy: 60_000,
            infusing: 0,
        };
        assert_eq!(machine.tick(true, Some(MAX_INFUSE), 0), Tick::default());
        assert_eq!(machine.energy, 60_000);
    }

    #[test]
    fn a_level_255_block_costs_301_fe() {
        let mut machine = Machine {
            energy: 600,
            infusing: 0,
        };
        let tick = machine.tick(true, Some(0), 255);
        assert!(tick.started);
        assert_eq!(tick.cost, 301);
        assert_eq!(machine.energy, 299);
    }

    #[test]
    fn the_cost_comes_from_the_block_not_the_item() {
        // `startInfusing` reads the block's own `infusable`; the item's level
        // only decides whether it can still be raised.
        let mut machine = Machine {
            energy: 600,
            infusing: 0,
        };
        let tick = machine.tick(true, Some(255), 0);
        assert!(tick.started);
        assert_eq!(tick.cost, 600);
        assert_eq!(machine.energy, 0);
    }

    #[test]
    fn the_menu_slots_are_thirty_two_fe_units_and_the_countdown() {
        let machine = Machine {
            energy: 60_000,
            infusing: 5,
        };
        assert_eq!(machine.data_slots(), [1875, 1875, 5]);
        assert_eq!(Machine::new().data_slots(), [0, 1875, 0]);
    }

    #[test]
    fn a_finished_tick_leaves_the_level_to_the_caller() {
        // The runtime never changes the item; it only says the level is due, so
        // a removed item or one at 256 cannot be raised here.
        let mut machine = Machine {
            energy: 0,
            infusing: 1,
        };
        assert!(machine.tick(false, None, 0).finished);
        assert_eq!(machine.energy, 0);
    }
}
