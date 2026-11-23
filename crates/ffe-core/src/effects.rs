//! Hit particles, ship explosions and their debris (F939, F942, F926).
//!
//! Source: JJFFE 2.8a7 `ffe.asm` (the gameplay reference), with the object
//! fields and tables in `ffedat.asm`. The routines:
//!
//! - **F939** (`ffe.asm:70291`) spawns the laser hit effect. F931 calls it once
//!   per hit as `F939(player, hit_position, 1)` (`ffe.asm:69804-69808`). It
//!   creates one object with **model 0x9e (158)**, control byte `9 | 0x20`
//!   (`+0x29`; the 0x20 bit is F928's "counted explosion" flag), AI state
//!   `+0xff = 0x1a`, AI class `+0x118 = 0xf8`, and clones the source object
//!   (`F1533`): its position, velocity, matrix rows (`+0x3e..+0x55`) and model
//!   statistics. It then
//!   - builds a random YZT matrix (`F1675`), and
//!   - keeps the cloned velocity only when the *player's* speed
//!     (`DATA_008867`) is at least `0x9c40` or the source's `+0x14c` has bit
//!     0x10; otherwise it zeroes `+0x8c/+0x90/+0x94`.
//!
//!   The flags in the third argument change the object's distance shift
//!   (`+0x7e`): bit 1 adds 2 and bit 2 subtracts 1 (`ffe.asm:70345-70351`).
//!   The laser path passes 1. The port does not model the per-object distance
//!   shift, so the particle draws at its model's own scale.
//!
//! - **F942** (`ffe.asm:70650`) is the destruction effect F953 calls when the
//!   hull is used up (`ffe.asm:71281-71287`). It creates one object with
//!   **model 0x9b (155)**, control byte `0xb`, AI state `+0xff = 0x1c`, a
//!   random YZT matrix and the destroyed ship's distance word (`+0x7e`)
//!   decremented by one. Two adjustments follow (`ffe.asm:70747-70786`):
//!   - with statistics, an object count at `+0x6` of 10 or more plays sample
//!     0x1f and re-increments the distance word; at 0x190 or more the distance
//!     word becomes `(ship_scale + ship_collision_radius) - (explosion_scale +
//!     explosion_collision_radius) + 4`, which the port does not model (it has
//!     no separate descriptor radii here);
//!   - for ship models 0x0f..0x3f it also spawns debris with **F926**
//!     (`ffe.asm:69099`): `F922(6) - 2` pods (0..4) with string `0x8e0f`, and
//!     `min(4, cargo_space >> 1)` more with a random cargo string
//!     (`0x8e00 + DATA_007212[rand & 0x1f]`). F926 makes a **model 0x99 (153)**
//!     object, control 0xb, AI state 0x15, and adds `DATA_008818 >> 22` (a
//!     signed -512..=511) to each velocity word (`+0x8c..+0x94`).
//!
//! - **F952** (`ffe.asm:71154`) is not a visual effect: on a hull hit that
//!   does not destroy the ship it rolls for subsystem damage (`F945`/`F947`/
//!   `F948`/`F949` messages and sounds). The port leaves it out with the rest
//!   of the equipment damage model.
//!
//! Lifetimes are the AI table's (`DATA_006255..58`, four slots per `+0xff`
//! state; the movement slot runs every update with the frame's game tics
//! `DATA_009155`, 49,700 a second at 1x):
//!
//! - **F708** (state `0x1c`, the explosion): object global word 4 (`+0xa4`)
//!   starts at `0xffff` and loses `tics >> 2` an update; the object goes when
//!   that would pass zero, about 5.3 s at 1x. While the word is in
//!   `0xbc00..0xc800` (about 1.15 to 1.4 s in) each update also throws an F939
//!   spark from the explosion's centre.
//! - **F709** (state `0x1a`, the hit particle): the word starts at
//!   `F1530 | 0x8000` and loses `tics >> 1`, so a particle lives 1.3 to 2.6 s.
//! - **F712** (state `0x15`, a debris pod) throws an F939 spark every update
//!   and F768, on each event-6 tick, counts `+0x14e` down from 6; below zero
//!   the pod becomes state `0x16`, where F713 only moves it and F767 removes
//!   it once its distance class (`+0x88`) reaches `0xd`, about 67 km away.
//!
//! F764 (state `0x1d`) is not the explosion's: its `and` of the two clock
//! words (the original executable has `23 c2` there too) always writes
//! `0xffffffff`.
//!
//! Movement is F1045's call to **F1512** (`ffe.asm:90407`), which passes the
//! velocity words `+0x8c..+0x94` and the frame's 64-bit game time to F1511
//! (`ffe.asm:90203`). The port keeps a speed in millimetres per second
//! ([`Effect::velocity`]) because its renderer and saves use that unit; one
//! source word is `49700 / 512` mm/s and [`Effect::velocity_words`] recovers
//! the word, so [`Effect::tick`] advances the position by F1511's integer
//! arithmetic over the frame's tics, not by a floating-point step. F926 adds
//! its `DATA_008818 >> 22` drift to those words, not to millimetres a second.
//!
//! F939 and F942 allocate their objects with **F928** (`ffe.asm:69259`), while
//! F926's pods use **F927** (`69170`): both take the topmost free flag byte
//! from `0x72` down (F927 keeps slots below `0x14` back for ships), and when
//! none is free they replace the topmost object flagged `0x20` (a counted
//! explosion), decrementing `DATA_008834` before the new flag is written.
//! **F924** (`69030`) frees the slot: it clears the flag byte and decrements
//! the counted word for a `0x20` object. [`EffectTable`] models that table so
//! a freed effect slot is handed to the next spawn exactly as the source does;
//! [`Effects`] is the flight side's list over it (`Navigator::effects`), so the
//! running game allocates and recycles the same slots.
//!
//! The explosion model's own command stream gates its twelve fragment calls on
//! object global word 4 (`+0xa4`): the arithmetic and bit branches in model 155
//! select no fragment for the `0xffffffff` F942 writes there, but select all
//! twelve for FFP 1.0 (`0x4000`). The renderer passes the object's live timer
//! word as [`EXPLOSION_MASS_GLOBAL`], as the original reads `+0xa4`, so the
//! fragments follow F708's countdown; the gate is asserted in the cached-frame
//! test (`crates/ffe-rs/tests/effects_view.rs`). Global word 1 (`+0x9e`) is
//! `0xffff` on the explosion, the cargo code on a pod, and cloned into each
//! spark from its thrower. The executed death probe (`jjffe-smoke
//! --death-probe`) matches these words and the timers' rates: the explosion's
//! word falls by `tics >> 2` and a particle's by `tics >> 1` each update.

use crate::npc::{Random, MM_PER_VELOCITY_UNIT};

/// The three velocity words `+0x8c..+0x94` a port speed in millimetres per
/// second stands for, as F926 adds its drift to them.
pub fn to_velocity_words(velocity: [f64; 3]) -> [i32; 3] {
    velocity.map(|v| (v / MM_PER_VELOCITY_UNIT).round() as i32)
}

/// The millimetres per second a source velocity word stands for; the effects
/// this module builds store exactly this value in [`Effect::velocity`].
pub fn word_speed(word: i32) -> f64 {
    f64::from(word) * MM_PER_VELOCITY_UNIT
}

fn words_to_speed(words: [i32; 3]) -> [f64; 3] {
    words.map(word_speed)
}

/// F939's hit-particle model, `0x9e` (158).
pub const HIT_MODEL: u32 = 0x9e;
/// F942's destruction model, `0x9b` (155).
pub const EXPLOSION_MODEL: u32 = 0x9b;
/// F926's debris model, `0x99` (153).
pub const DEBRIS_MODEL: u32 = 0x99;

/// Game tics a second at 1x (`DATA_009155` per 20 ms is 994).
pub const TICS_PER_SECOND: f64 = 49_700.0;
/// F942's starting timer word (`+0xa4`).
pub const EXPLOSION_TIMER: u16 = 0xffff;
/// F708's spark window on the explosion's timer word.
pub const SPARK_WINDOW: std::ops::Range<u16> = 0xbc00..0xc800;
/// F926's `+0x14e`: the event-6 ticks a pod sparks for (it changes state once
/// the count goes below zero).
pub const DEBRIS_SPARK_TICKS: i8 = 6;
/// F152's whole-day tick, which drives event 6 (F706 → F768): every `0xc22e`
/// game tics, one second at 1x.
pub const THINK_TICS: u32 = 0xc22e;
/// F767's removal distance: distance class `0xd`, `2^(13 + 13)` mm.
pub const DEBRIS_RANGE: i64 = 1 << 26;
/// Seconds an explosion lives at 1x: F708's `0xffff` at `tics >> 2`.
pub const EXPLOSION_SECONDS: f64 = 0x1_0000 as f64 * 4.0 / TICS_PER_SECOND;

/// F939's speed cutoff (`DATA_008867` vs `0x9c40`): below it the particle does
/// not inherit the ship's velocity.
pub const PARTICLE_SPEED: f64 = 0x9c40 as f64;

/// Object global word 4 (`+0xa4`): the explosion's and particle's timer,
/// which model 155's fragment branches read.
pub const EXPLOSION_MASS_GLOBAL: usize = 4;

/// F942's `+0x9e` (object global word 1) on the explosion, `0xffff`
/// (`ffe.asm:70730`). Model 155's first instruction is `local5 = global1 >> 7`,
/// the fragments' size.
pub const EXPLOSION_SIZE: u16 = 0xffff;

/// Which spawn created an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    /// F939's one model-0x9e object at the hit point.
    Hit,
    /// F942's one model-0x9b destruction object.
    Explosion,
    /// F926's model-0x99 pod.
    Debris,
    /// A hyperspace cloud (model 0x9a): F738's departure cloud (AI state
    /// 0x1d, F764) or F740/F1094's exit cloud (state 0x1e, F763).
    Cloud,
    /// F1098: the ship the player left for an escape craft, in AI state 0x16
    /// with its thrust zeroed: F713 only moves it and F767 removes it at
    /// distance class 0xd ([`DEBRIS_RANGE`]).
    Derelict,
}

/// One short-lived visual object in the system frame.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Effect {
    pub kind: Kind,
    pub model: u32,
    /// Position in the system frame, original units (millimetres).
    pub position: [i64; 3],
    /// The velocity as its source words (`+0x8c..+0x94`) in millimetres per
    /// second: one word is `49700 / 512` mm/s. F939 inherits the ship's words,
    /// F926 adds `DATA_008818 >> 22` (-512..=511) to them.
    pub velocity: [f64; 3],
    /// The object's forward and up axes (right is `up x forward`).
    pub forward: [f64; 3],
    pub up: [f64; 3],
    /// Global word 4 (`+0xa4`): F708/F709's countdown. Unused by debris.
    pub timer: u16,
    /// F926's `+0x14e`: a pod's remaining event-6 spark ticks.
    pub countdown: i8,
    /// Game tics towards the next event-6 tick.
    pub think: u32,
    pub alive: bool,
    /// The distance word `+0x7e`, a shift on the model's own scale: F1533
    /// copies the source ship's (0 for the player in the executed death
    /// probe), F942 and F939 change it.
    pub shift: i8,
    /// A canister's commodity (`+0x9e`): what F714 scoops into the hold.
    pub cargo: Option<u16>,
    /// A cloud's global word 1 (`+0x9e`), the model's size ramp.
    pub size: u16,
    /// A cloud's game tics left before F763/F764 remove it (`+0xa8/+0xac`).
    pub life: u64,
    /// An exit cloud (state 0x1e) rather than a departure cloud (0x1d).
    pub exit: bool,
    /// The object id a mission script knows this object by (statements 52
    /// and 58 create canisters and hand their ids to the scripts).
    pub tag: Option<i32>,
    /// The body F603 pulls a drifting object toward (the parent the ship it
    /// came from was in the frame of); `None` is free space.
    #[serde(default)]
    pub parent: Option<Parent>,
    /// F603's sub-word velocity accumulators (`+0xec..+0xf0`).
    #[serde(default)]
    pub fraction: [u16; 3],
    /// Born during this frame's sweep: its F1512 move still has a zero time
    /// step (the clone carries the thrower's time stamp, `+0xa8`), though its
    /// timer already loses the frame's tics. Cleared by the first update.
    #[serde(skip)]
    pub fresh: bool,
}

/// An object's parent for F603: the body's centre in the same frame as the
/// effect's position and its `+0xa4` ffp mass word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Parent {
    pub centre: [i64; 3],
    pub mass: u32,
}

/// F926's first pods (`0x8e0f + 0x7200`): commodity 15.
pub const WRECK_CARGO: u16 = 0x0f;

/// `DATA_007212`: the cargo of a destroyed ship's pods, by `F1530 & 0x1f`.
pub const WRECK_CARGO_TABLE: [u8; 32] = [
    0x00, 0x1e, 0x1e, 0x1e, 0x04, 0x05, 0x1e, 0x07, 0x08, 0x09, 0x0a, 0x04, 0x09, 0x1e, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x1e, 0x1d, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

/// `DATA_005112`: what a jettisoned commodity becomes in its canister (F291).
pub const JETTISON_CARGO: [u8; 33] = [
    0x00, 0x01, 0x1e, 0x1e, 0x04, 0x05, 0x1e, 0x07, 0x08, 0x09, 0x0a, 0x04, 0x09, 0x1e, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    0x20,
];

/// F714's Tractor Beam reach: `Vec64Truncate(.., 13)` below 2, under
/// `2^15` mm on every axis.
pub const TRACTOR_RANGE: i64 = 1 << 15;
/// F714's Tractor Beam cone: the canister's direction against the ship's
/// forward row at least `0x3fffffff` (cos 60°).
pub const TRACTOR_COS: f64 = 0.5;

/// What F714 needs of the player's ship to scoop a canister.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scoop {
    pub position: [i64; 3],
    pub forward: [f64; 3],
    /// The ship's collision radius (mm), for the contact route.
    pub radius: f64,
    /// General bit 10, the Tractor Beam Cargo Scoop.
    pub tractor: bool,
    /// General bit 29, the Cargo Scoop Conversion.
    pub scoop: bool,
}

/// The canister model's collision radius (model 0x99, mm).
pub const CANISTER_RADIUS: f64 = 1664.0;

impl Effect {
    /// F714: the player takes this canister. With the Tractor Beam it must be
    /// within [`TRACTOR_RANGE`] and ahead within 60°; otherwise the ship's
    /// scoop must touch it. The original's touch is the scoop region of the
    /// ship's collision stream (F582's flag 0x10); the port's is the two
    /// collision spheres meeting ahead of the ship, with the Cargo Scoop
    /// Conversion fitted.
    pub fn scooped_by(&self, ship: &Scoop) -> bool {
        if self.cargo.is_none() || !self.alive {
            return false;
        }
        let offset: [f64; 3] =
            std::array::from_fn(|k| (self.position[k] - ship.position[k]) as f64);
        if ship.tractor {
            if offset.iter().any(|v| v.abs() >= TRACTOR_RANGE as f64) {
                return false;
            }
            let length = offset.iter().map(|v| v * v).sum::<f64>().sqrt();
            if length < 1.0 {
                return true;
            }
            let cos = (0..3).map(|k| offset[k] * ship.forward[k]).sum::<f64>() / length;
            return cos >= TRACTOR_COS;
        }
        // The scoop's region of the collision volume is at the nose: a
        // canister behind the ship (a jettison leaves one there) is not in it.
        let length = offset.iter().map(|v| v * v).sum::<f64>().sqrt();
        let ahead = (0..3).map(|k| offset[k] * ship.forward[k]).sum::<f64>() > 0.0;
        ship.scoop && ahead && length <= ship.radius + CANISTER_RADIUS
    }
}

/// What an update asks the caller for besides the object itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update {
    Nothing,
    /// F939 from the object's centre (F708's window, F712's every update).
    Spark,
}

impl Effect {
    /// The right/up/forward axes, as `screen` and the camera take them.
    pub fn axes(&self) -> [[f64; 3]; 3] {
        [
            normalise(cross(self.up, self.forward)),
            self.up,
            self.forward,
        ]
    }

    /// The source velocity words `+0x8c..+0x94` this effect moves with.
    pub fn velocity_words(&self) -> [i32; 3] {
        to_velocity_words(self.velocity)
    }

    /// One movement-slot update of `tics` game tics: F1512/F1511 carry the
    /// object on its integer velocity words, then F708 (explosion), F709
    /// (particle) or F712/F768 (debris) run their timers. `seconds` is the
    /// frame's presented time, kept for callers that only have that; F1512
    /// uses the game tics.
    pub fn tick(&mut self, _seconds: f64, tics: u32) -> Update {
        self.step(tics, None)
    }

    /// [`Self::tick`] with the number of F152 second boundaries (`0xc22e`-tic
    /// multiples of the game clock) the frame crossed. F768 counts a pod's
    /// `+0x14e` down on those global events, not from the pod's birth; with
    /// `None` the pod counts its own tics from zero.
    pub fn step(&mut self, tics: u32, crossings: Option<u32>) -> Update {
        if tics == 0 || !self.alive {
            return Update::Nothing;
        }
        let words = self.velocity_words();
        if std::mem::take(&mut self.fresh) {
            // No time has passed for the object itself yet.
        } else {
            crate::physics::advance_position(&mut self.position, words, i64::from(tics));
        }
        let step = |shift: u32| (tics.wrapping_shl(16 - shift) >> 16) as u16;
        match self.kind {
            Kind::Explosion => {
                let step = step(2);
                if step > self.timer {
                    self.alive = false;
                    return Update::Nothing;
                }
                self.timer -= step;
                if SPARK_WINDOW.contains(&self.timer) {
                    return Update::Spark;
                }
                Update::Nothing
            }
            Kind::Hit => {
                let step = step(1);
                if step > self.timer {
                    self.alive = false;
                } else {
                    self.timer -= step;
                }
                Update::Nothing
            }
            Kind::Cloud => {
                // F735's `+0x9e` ramp, `tics >> 1` a frame: a departure cloud
                // (below state 0x1e) shrinks it to 0, an exit cloud grows it
                // to 0xffff.
                let step = step(1);
                self.size = if self.exit {
                    self.size.saturating_add(step)
                } else {
                    self.size.saturating_sub(step)
                };
                // F763/F764 remove the cloud once the game time passes
                // `+0xa8/+0xac`.
                if u64::from(tics) >= self.life {
                    self.alive = false;
                } else {
                    self.life -= u64::from(tics);
                }
                Update::Nothing
            }
            // F713: movement, then F603's pull of the parent.
            Kind::Derelict => {
                self.fall(tics);
                Update::Nothing
            }
            Kind::Debris => {
                self.fall(tics);
                match crossings {
                    Some(count) => {
                        for _ in 0..count {
                            if self.countdown < 0 {
                                break;
                            }
                            self.countdown -= 1;
                        }
                    }
                    None => {
                        self.think = self.think.saturating_add(tics);
                        while self.think >= THINK_TICS && self.countdown >= 0 {
                            self.think -= THINK_TICS;
                            self.countdown -= 1;
                        }
                    }
                }
                if self.countdown >= 0 {
                    Update::Spark
                } else {
                    Update::Nothing
                }
            }
        }
    }

    /// F942's distance word for the explosion of a ship of `mass` t: the
    /// ship's less one, given back for 10 t or more (the executed probe's
    /// 28 t Saker leaves 0). F942's own formula for 400 t or more is not
    /// modelled; those keep the ship's word.
    pub fn set_mass(&mut self, mass: u16) {
        if self.kind == Kind::Explosion && mass < 10 {
            self.shift = -1;
        }
    }

    /// The F939 spark this object throws: F708 passes flag 2 (distance word
    /// `+2`, four times the size), F712 flag 4 (`-1`).
    ///
    /// F939 clones the thrower (F1533), so the spark keeps its global word 1
    /// (`+0x9e`): `0xffff` from the explosion, the pod's cargo code from a
    /// debris pod (the executed death probe's `g1` 65535 and 15).
    pub fn spark(&self, rng: &mut Random) -> Effect {
        let mut spark = hit(self.position, [0.0; 3], self.axes(), 0.0, rng);
        spark.shift = self.shift + if self.kind == Kind::Debris { -1 } else { 2 };
        spark.size = match self.kind {
            Kind::Explosion => EXPLOSION_SIZE,
            _ => self.size,
        };
        spark
    }

    /// F713's F603: the parent body pulls the drifting object. The position
    /// relative to the parent, the frame's tics and the parent's mass give a
    /// velocity change through F603's own arithmetic ([`crate::physics::Motion::gravity`])
    /// with the fraction words carried between frames.
    fn fall(&mut self, tics: u32) {
        let Some(parent) = self.parent else {
            return;
        };
        let mut motion = crate::physics::Motion {
            parent: 1,
            position: std::array::from_fn(|k| self.position[k] - parent.centre[k]),
            velocity: self.velocity_words(),
            fraction: self.fraction,
            ..Default::default()
        };
        if motion.gravity(parent.mass, tics as i32).is_some() {
            self.velocity = words_to_speed(motion.velocity);
            self.fraction = motion.fraction;
        }
    }

    /// F708/F712's F939 call (flag 2 or 4, bit 0 clear): one F1530 draw gates
    /// it, a word with bit 15 set throws nothing (about half the frames), then
    /// the spark is [`Self::spark`]'s, born at `from` (the thrower's position
    /// before F1512 moved it this update). The spark keeps the thrower's
    /// velocity words only when the player's own speed (`DATA_008867`) is at
    /// least `0x9c40`, as [`hit`] takes `player_speed`.
    pub fn thrown_spark(
        &self,
        from: [i64; 3],
        velocity: [f64; 3],
        player_speed: f64,
        rng: &mut Random,
    ) -> Option<Effect> {
        if draw16(rng) & 0x8000 != 0 {
            return None;
        }
        let mut spark = hit(from, velocity, self.axes(), player_speed, rng);
        spark.shift = self.shift + if self.kind == Kind::Debris { -1 } else { 2 };
        spark.size = match self.kind {
            Kind::Explosion => EXPLOSION_SIZE,
            _ => self.size,
        };
        spark.fresh = true;
        Some(spark)
    }

    /// F767: a drifting pod goes once it is `DEBRIS_RANGE` from `player`.
    pub fn out_of_range(&self, player: [i64; 3]) -> bool {
        // A mission script's canister stays until it is scooped or the
        // script removes it.
        self.tag.is_none()
            && matches!(self.kind, Kind::Debris | Kind::Derelict)
            && (0..3).any(|axis| (self.position[axis] - player[axis]).abs() >= DEBRIS_RANGE)
    }

    pub fn alive(&self) -> bool {
        self.alive
    }
}

/// Object slots in the source's physics list (flag bytes 0 to `0x72`).
pub const OBJECT_SLOTS: usize = 0x73;
/// F927's floor: slots below `0x14` are kept back for ships.
pub const SHIP_SLOT_FLOOR: usize = 0x14;

/// The table flag byte `DATA_007758[slot]` an effect's birth writes: F939's
/// `0x9 | 0x20` (a counted explosion), F942's `0x9` or F926/F291's `0xf`.
fn slot_flag(effect: &Effect) -> u8 {
    match effect.kind {
        Kind::Hit => 0x29,
        Kind::Debris => 0x0f,
        _ => 0x09,
    }
}

/// F939/F942/F926's object slots, with F928/F927's allocation and F924's
/// removal. A slot is a flag byte plus an object; freeing clears the flag so
/// the next spawn takes the topmost free slot, and a table with no free slot
/// replaces the topmost object flagged `0x20` (F928/F927's second scan).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EffectTable {
    slots: Vec<Option<Effect>>,
    /// `DATA_008834`: the objects flagged `0x20` (counted explosions).
    counted: u32,
}

impl Default for EffectTable {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectTable {
    pub fn new() -> Self {
        Self {
            slots: (0..OBJECT_SLOTS).map(|_| None).collect(),
            counted: 0,
        }
    }

    /// `DATA_008834`: the counted-explosion total F928/F927 maintain.
    pub fn counted(&self) -> u32 {
        self.counted
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, slot: usize) -> Option<&Effect> {
        self.slots.get(slot).and_then(Option::as_ref)
    }

    pub fn get_mut(&mut self, slot: usize) -> Option<&mut Effect> {
        self.slots.get_mut(slot).and_then(Option::as_mut)
    }

    /// The live objects in slot order, as the source walks its object list.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &Effect)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, effect)| effect.as_ref().map(|effect| (slot, effect)))
    }

    /// The live objects' slots, for a caller that edits them in place.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Effect> {
        self.slots.iter_mut().filter_map(Option::as_mut)
    }

    /// F928: F939, F942 and F291's canister take the topmost free slot (index
    /// one and above); a full table replaces the topmost `0x20` object.
    pub fn spawn(&mut self, effect: Effect) -> Option<usize> {
        self.allocate(effect, 1)
    }

    /// F927: F926's debris pods keep slots below [`SHIP_SLOT_FLOOR`] back, so
    /// only `0x14..=0x72` is free and only a `0x20` object there is replaced.
    pub fn spawn_pod(&mut self, effect: Effect) -> Option<usize> {
        self.allocate(effect, SHIP_SLOT_FLOOR)
    }

    fn allocate(&mut self, effect: Effect, floor: usize) -> Option<usize> {
        self.slots.resize(OBJECT_SLOTS, None);
        let free = (floor..OBJECT_SLOTS)
            .rev()
            .find(|slot| self.slots[*slot].is_none());
        let slot = match free {
            Some(slot) => slot,
            None => {
                let counted = (floor..OBJECT_SLOTS).rev().find(|slot| {
                    self.slots[*slot]
                        .as_ref()
                        .is_some_and(|effect| slot_flag(effect) & 0x20 != 0)
                })?;
                self.counted = self.counted.saturating_sub(1);
                counted
            }
        };
        if slot_flag(&effect) & 0x20 != 0 {
            self.counted += 1;
        }
        self.slots[slot] = Some(effect);
        Some(slot)
    }

    /// F924: free the slot, decrementing `DATA_008834` when the object was a
    /// counted explosion.
    pub fn free(&mut self, slot: usize) -> Option<Effect> {
        let effect = self.slots.get_mut(slot)?.take()?;
        if slot_flag(&effect) & 0x20 != 0 {
            self.counted = self.counted.saturating_sub(1);
        }
        Some(effect)
    }

    /// F924 for every object whose state routine ended it.
    pub fn free_dead(&mut self) {
        self.slots.resize(OBJECT_SLOTS, None);
        for slot in 0..OBJECT_SLOTS {
            let dead = self.slots[slot]
                .as_ref()
                .is_some_and(|effect| !effect.alive());
            if dead {
                self.free(slot);
            }
        }
    }

    /// F767: free the drifting objects out of the player's range.
    pub fn free_out_of_range(&mut self, player: [i64; 3]) {
        self.slots.resize(OBJECT_SLOTS, None);
        for slot in 0..OBJECT_SLOTS {
            let gone = self.slots[slot]
                .as_ref()
                .is_some_and(|effect| effect.out_of_range(player));
            if gone {
                self.free(slot);
            }
        }
    }

    /// One movement-slot pass over the objects the table holds now, from slot
    /// `0x72` down to 1 as the executed original does (its sparks come out in
    /// that order, 107, 99, 98 for pods 102, 101, 100): every object runs its
    /// state routine, throws the sparks it asks for, and is freed at once
    /// when it ends (F940) or is out of range (F767). A spark takes F928's
    /// topmost free slot immediately, so one landing below the object that
    /// threw it is updated again later in the same pass and one above it is
    /// not. Returns the spark slots.
    pub fn tick(
        &mut self,
        seconds: f64,
        tics: u32,
        player: [i64; 3],
        rng: &mut Random,
    ) -> Vec<usize> {
        let _ = seconds;
        self.sweep(tics, None, 0.0, player, rng)
    }

    /// [`Self::tick`] with the game clock's low word before the frame (so pods
    /// count F152's global second boundaries) and the player's own speed
    /// (`DATA_008867`, which decides whether sparks keep the thrower's
    /// velocity).
    pub fn sweep(
        &mut self,
        tics: u32,
        clock: Option<u32>,
        player_speed: f64,
        player: [i64; 3],
        rng: &mut Random,
    ) -> Vec<usize> {
        self.slots.resize(OBJECT_SLOTS, None);
        let crossings = clock.map(|before| {
            let after = u64::from(before) + u64::from(tics);
            let second = u64::from(THINK_TICS);
            if after >> 32 > 0 {
                // The day rolled over: the second counter restarts at zero.
                1
            } else {
                (after / second - u64::from(before) / second) as u32
            }
        });
        let mut born = Vec::new();
        for slot in (1..OBJECT_SLOTS).rev() {
            let Some(effect) = self.slots[slot].as_mut() else {
                continue;
            };
            // F939 clones the thrower as F708/F712 find it, before F1512's move
            // and F603's pull of this update.
            let (before, before_velocity) = (effect.position, effect.velocity);
            let update = effect.step(tics, crossings);
            let ended = !effect.alive() || effect.out_of_range(player);
            let spark = (update == Update::Spark)
                .then(|| effect.thrown_spark(before, before_velocity, player_speed, rng))
                .flatten();
            if ended {
                self.free(slot);
            }
            if let Some(spark) = spark {
                born.extend(self.spawn(spark));
            }
        }
        // A spark born above the slot that threw it was not visited: its next
        // update is a whole frame's.
        for effect in self.iter_mut() {
            effect.fresh = false;
        }
        born
    }
}

/// The system's live effect objects, as `Navigator::effects`: [`EffectTable`]
/// with the list-style access the flight, scripts and renderer use.
///
/// Every spawn takes the topmost free source slot, so F924's freed slot goes to
/// the next object exactly as in the source; iteration is in slot order, the
/// order the original walks its object list. The facade keeps the existing
/// callers ([`Self::push`], [`Self::iter`], [`Self::get`], ...) working while
/// the slot identity lives in the table.
#[derive(Debug, Clone)]
pub struct Effects {
    table: EffectTable,
    /// `DATA_008867`: the player's own speed (mm/s), which lets F939's sparks
    /// keep their thrower's velocity at or above [`PARTICLE_SPEED`].
    player_speed: f64,
    /// The global clock's low word (`DATA_008804`'s day fraction) at the last
    /// sweep. F152's event 6 lands on its [`THINK_TICS`] multiples and F768
    /// counts a pod's `+0x14e` down there, so the sweep advances this by the
    /// same game tics it moves the objects by. It is the effects' own clock so
    /// that a caller which ticks the effects without advancing the world clock
    /// (a paused clock, a test) still gets F768's count.
    clock: u32,
}

impl Default for Effects {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Effects {
    /// Saves compare the live objects in slot order; the slot numbers a load
    /// assigns are deterministic, so equal content is the resume contract.
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Effects {
    pub fn new() -> Self {
        Self {
            table: EffectTable::new(),
            player_speed: 0.0,
            clock: 0,
        }
    }

    /// The player's speed F1011/F1012 last left in `DATA_008867`.
    pub fn player_speed(&self) -> f64 {
        self.player_speed
    }

    pub fn set_player_speed(&mut self, speed: f64) {
        self.player_speed = speed;
    }

    /// F928: the topmost free slot. F939's hit particles, F942's explosion,
    /// F291's canisters, clouds and F1098's derelicts are ordinary F928
    /// objects; F942's F926 pods use [`Self::push_pod`]. A full table replaces
    /// the topmost counted (`+0x20`) explosion or, when there is none, drops
    /// the object as F928 does.
    pub fn push(&mut self, effect: Effect) {
        self.table.spawn(effect);
    }

    /// F927: F926's debris pods, which keep slots below [`SHIP_SLOT_FLOOR`]
    /// back for ships.
    pub fn push_pod(&mut self, effect: Effect) {
        self.table.spawn_pod(effect);
    }

    /// F942's destruction burst: the model-0x9b explosion takes F928's topmost
    /// slot and every F926 pod after it takes F927's. Returns the explosion's
    /// slot, which the death sequence follows (F1104).
    pub fn push_burst(&mut self, burst: Vec<Effect>) -> Option<usize> {
        let mut first = None;
        for (index, effect) in burst.into_iter().enumerate() {
            if index == 0 {
                first = self.table.spawn(effect);
            } else {
                self.push_pod(effect);
            }
        }
        first
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// The live objects in slot order.
    pub fn iter(&self) -> impl Iterator<Item = &Effect> {
        self.table.iter().map(|(_, effect)| effect)
    }

    /// The live objects' slots in order, as a test or debug caller sees them.
    pub fn slots(&self) -> impl Iterator<Item = (usize, &Effect)> {
        self.table.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Effect> {
        self.table.iter_mut()
    }

    /// The `position`-th live object in slot order (the old `Vec` index).
    pub fn get(&self, position: usize) -> Option<&Effect> {
        self.iter().nth(position)
    }

    /// The object in its source slot, the identity F924/F928 give it.
    pub fn get_slot(&self, slot: usize) -> Option<&Effect> {
        self.table.get(slot)
    }

    /// `DATA_008834`: the counted explosions the table holds.
    pub fn counted(&self) -> u32 {
        self.table.counted()
    }

    /// F924 for every object whose state routine ended it.
    pub fn free_dead(&mut self) {
        self.table.free_dead();
    }

    /// Drop every object (a system change or a load).
    pub fn clear(&mut self) {
        self.table = EffectTable::new();
    }

    /// The saved objects, in slot order.
    pub fn to_vec(&self) -> Vec<Effect> {
        self.iter().cloned().collect()
    }

    /// Rebuild the table from a saved slot-ordered list. F928's slots run
    /// downwards, so spawning the list in reverse hands every object the slot
    /// it had when the save was written.
    pub fn from_saved(saved: &[Effect]) -> Self {
        let mut effects = Self::new();
        for effect in saved.iter().rev() {
            effects.table.spawn(effect.clone());
        }
        effects
    }

    /// One movement-slot pass (F1512/F1511, then the state routines), with
    /// F928 allocating each spark and F924/F767 freeing the dead and out-of-
    /// range objects. The frame's game tics also carry the F152 clock the pods
    /// read for their F768 count, so their lives do not depend on a caller-side
    /// wall clock advancing in step with the effects.
    pub fn tick(
        &mut self,
        seconds: f64,
        tics: u32,
        player: [i64; 3],
        rng: &mut Random,
    ) -> Vec<usize> {
        let _ = seconds;
        let before = self.clock;
        self.clock = self.clock.wrapping_add(tics);
        self.table
            .sweep(tics, Some(before), self.player_speed, player, rng)
    }
}

/// F939: one hit particle (model 0x9e) at `position`, with the source's axes
/// and velocity. `speed` is the source's speed used for the `0x9c40` cutoff.
pub fn hit(
    position: [i64; 3],
    velocity: [f64; 3],
    axes: [[f64; 3]; 3],
    speed: f64,
    rng: &mut Random,
) -> Effect {
    // F939: two F1530 draws; the first is also the timer, `+0xa4 = F1530 |
    // 0x8000`, and both are F1675's angles.
    let (first, second) = (draw16(rng), draw16(rng));
    let (forward, up) = yzt_axes(first, second);
    let timer = (first | 0x8000) as u16;
    let _ = axes;
    Effect {
        kind: Kind::Hit,
        model: HIT_MODEL,
        position,
        velocity: if speed >= PARTICLE_SPEED {
            words_to_speed(to_velocity_words(velocity))
        } else {
            [0.0; 3]
        },
        forward,
        up,
        timer,
        countdown: 0,
        think: 0,
        alive: true,
        shift: 0,
        cargo: None,
        size: 0,
        life: 0,
        exit: false,
        tag: None,
        parent: None,
        fraction: [0; 3],
        fresh: false,
    }
}

/// F719: the missile's own explosion (model 0x9b) after ECM or F721. F719
/// writes `+0xa4 = 0xffffffff` and `+0x9e = 0xffff` and calls F925 to remove
/// the missile; it does not spawn F926's debris pods the way F942 does, and
/// its velocity word is left zero.
pub fn missile_explosion(position: [i64; 3], axes: [[f64; 3]; 3], rng: &mut Random) -> Effect {
    // F719: one F1530 draw; its low word is the first angle and `eax >> 16`,
    // which F1530's 16-bit result never has, the second.
    let _ = axes;
    let (forward, up) = yzt_axes(draw16(rng), 0);
    Effect {
        kind: Kind::Explosion,
        model: EXPLOSION_MODEL,
        position,
        velocity: [0.0; 3],
        forward,
        up,
        timer: EXPLOSION_TIMER,
        countdown: 0,
        think: 0,
        alive: true,
        shift: 0,
        cargo: None,
        size: EXPLOSION_SIZE,
        life: 0,
        exit: false,
        tag: None,
        parent: None,
        fraction: [0; 3],
        fresh: false,
    }
}

/// F942 and F926: the destruction object plus its debris pods.
///
/// `cargo` is the ship's cargo space (`+0x116`, F942's second debris count).
/// The result always starts with the model-0x9b object.
pub fn explosion(
    position: [i64; 3],
    velocity: [f64; 3],
    cargo: u16,
    axes: [[f64; 3]; 3],
    rng: &mut Random,
) -> Vec<Effect> {
    // F942: two F1530 draws, F1675's angles for the explosion's own matrix.
    let (forward, up) = yzt_axes(draw16(rng), draw16(rng));
    let ship_words = to_velocity_words(velocity);
    let mut effects = vec![Effect {
        kind: Kind::Explosion,
        model: EXPLOSION_MODEL,
        position,
        velocity: words_to_speed(ship_words),
        forward,
        up,
        timer: EXPLOSION_TIMER,
        countdown: 0,
        think: 0,
        alive: true,
        shift: 0,
        cargo: None,
        size: EXPLOSION_SIZE,
        life: 0,
        exit: false,
        tag: None,
        parent: None,
        fraction: [0; 3],
        fresh: false,
    }];
    // F942: (F922(6) - 2) pods, then min(4, cargo >> 1) cargo pods.
    // F942: F922(6) - 2 pods of commodity 15, then, when the ship has cargo
    // space, F922(min(4, cargo >> 1)) - 1 pods of DATA_007212's cargo.
    let pods = (rng.bounded(6) as i32 - 2).max(0) as usize;
    let bound = u32::from((cargo >> 1).min(4));
    let cargo_pods = if bound > 0 {
        (rng.bounded(bound) as i32 - 1).max(0) as usize
    } else {
        0
    };
    for pod in 0..pods + cargo_pods {
        let load = if pod < pods {
            WRECK_CARGO
        } else {
            // F942's argument is `DATA_007212[F1530 & 0x1f]`.
            u16::from(WRECK_CARGO_TABLE[(draw16(rng) & 0x1f) as usize])
        };
        // F926 clones the ship (F1533), so a pod keeps its matrix: the
        // executed probe's pods share the destroyed ship's rows.
        let (forward, up) = (axes[2], axes[1]);
        // F926: two F1530 calls; the drift is the signed top ten bits of
        // `DATA_008818` after each (`sar 22`), and of `DATA_008820` for z,
        // added to the ship's velocity words.
        rng.advance();
        let x = (rng.a as i32) >> 22;
        rng.advance();
        let drift = [x, (rng.a as i32) >> 22, (rng.b as i32) >> 22];
        effects.push(Effect {
            kind: Kind::Debris,
            model: DEBRIS_MODEL,
            // F926 places the pod with F1514, the drift vector added to the
            // clone's position (the executed probe: pod = ship + drift).
            position: [
                position[0] + i64::from(drift[0]),
                position[1] + i64::from(drift[1]),
                position[2] + i64::from(drift[2]),
            ],
            velocity: words_to_speed([
                ship_words[0].wrapping_add(drift[0]),
                ship_words[1].wrapping_add(drift[1]),
                ship_words[2].wrapping_add(drift[2]),
            ]),
            forward,
            up,
            timer: 0,
            countdown: DEBRIS_SPARK_TICKS,
            think: 0,
            alive: true,
            shift: 0,
            cargo: Some(load),
            // F926: `+0x9e = string + 0x7200` (16 bits) with the string
            // `0x8e00 + cargo`, so global word 1 is the cargo code (15 for a
            // wreck pod, as the executed death probe shows).
            size: load,
            life: 0,
            exit: false,
            tag: None,
            parent: None,
            fraction: [0; 3],
            fresh: false,
        });
    }
    effects
}

/// The hyperspace cloud's model (`0x9a`).
pub const CLOUD_MODEL: u32 = 0x9a;
/// F738/F1094's distance word `+0x7e = 8` for a cloud.
pub const CLOUD_SHIFT: i8 = 8;
/// Game tics per game day: `DATA_008804` is the day's fraction in 2^-32.
pub const TICS_PER_DAY: u64 = 1 << 32;

/// F738/F761: the cloud a ship leaves when it jumps out. The ship itself
/// becomes the model-0x9a object, with `+0x9e = 0xffff` and `+0xa4 = -1`, and
/// stays until its arrival time elsewhere (`+0xa8/+0xac` plus the jump time
/// `+0xfa`); `life` is that jump time in game tics.
pub fn departure_cloud(position: [i64; 3], life: u64) -> Effect {
    cloud(position, false, 0xffff, life)
}

/// How long the player's exit cloud stays: F1094 copies the player's `+0xa8/+0xac` (the game
/// time) into the cloud and adds `0xff9e8e` tics, about 337 game seconds, before F763 deletes
/// it. A ship's (F740 stores `0xff9e8e` as the absolute target, long past) goes at the next
/// control tick.
pub const PLAYER_EXIT_CLOUD_TICS: u64 = 0x00ff_9e8e;

/// F1094 (the player) and F740 (a ship): the exit cloud left at the arrival
/// point, identity matrix (F1093), `+0x9e = 1` growing, `+0xa4 = -1`. F763
/// removes it once the game time passes its target (`life` game tics away):
/// [`PLAYER_EXIT_CLOUD_TICS`] for the player, the next control tick for a ship.
pub fn exit_cloud(position: [i64; 3], life: u64) -> Effect {
    cloud(position, true, 1, life)
}

/// F1098's derelict: the old ship's object, drifting on its velocity.
pub fn derelict(model: u32, position: [i64; 3], velocity: [f64; 3], axes: [[f64; 3]; 3]) -> Effect {
    Effect {
        kind: Kind::Derelict,
        model,
        position,
        velocity: words_to_speed(to_velocity_words(velocity)),
        forward: axes[2],
        up: axes[1],
        timer: 0,
        countdown: 0,
        think: 0,
        alive: true,
        shift: 0,
        cargo: None,
        size: 0,
        life: 0,
        exit: false,
        tag: None,
        parent: None,
        fraction: [0; 3],
        fresh: false,
    }
}

fn cloud(position: [i64; 3], exit: bool, size: u16, life: u64) -> Effect {
    Effect {
        kind: Kind::Cloud,
        model: CLOUD_MODEL,
        position,
        velocity: [0.0; 3],
        forward: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        timer: 0xffff,
        countdown: -1,
        think: 0,
        alive: true,
        shift: CLOUD_SHIFT,
        cargo: None,
        size,
        life,
        exit,
        tag: None,
        parent: None,
        fraction: [0; 3],
        fresh: false,
    }
}

/// F291's jettisoned canister: model 0x99 in F926's drifting state `0x16`
/// (no sparks), pushed out behind the ship. `push` is the velocity F291
/// takes off the ship's (its forward row `>> 11` plus a random word, `>> 12`).
pub fn canister(
    position: [i64; 3],
    velocity: [f64; 3],
    axes: [[f64; 3]; 3],
    push: [f64; 3],
    cargo: u16,
    rng: &mut Random,
) -> Effect {
    let (forward, up) = random_axes(axes[2], rng);
    Effect {
        kind: Kind::Debris,
        model: DEBRIS_MODEL,
        position,
        velocity: words_to_speed(to_velocity_words([
            velocity[0] - push[0],
            velocity[1] - push[1],
            velocity[2] - push[2],
        ])),
        forward,
        up,
        timer: 0,
        countdown: -1,
        think: 0,
        alive: true,
        shift: 0,
        cargo: Some(cargo),
        size: 0,
        life: 0,
        exit: false,
        tag: None,
        parent: None,
        fraction: [0; 3],
        fresh: false,
    }
}

/// F1530's returned word: `DATA_008818 & 0xffff` after one advance of the pair.
fn draw16(rng: &mut Random) -> u32 {
    rng.advance() & 0xffff
}

/// F1675 (`MatBuildYZT`) for two 16-bit binary angles: the rows are
/// `[ca*cb, sa, -(sb*ca)]`, `[-(sa*cb), ca, sa*sb]` and `[sb, 0, cb]` (see
/// `model_animation::build_yzt`). Returns the forward (row 2) and up (row 1)
/// axes; the right row is their cross product, as [`Effect::axes`] takes it.
pub fn yzt_axes(a: u32, b: u32) -> ([f64; 3], [f64; 3]) {
    let angle = |word: u32| f64::from(word & 0xffff) / 65_536.0 * std::f64::consts::TAU;
    let (sa, ca) = angle(a).sin_cos();
    let (sb, cb) = angle(b).sin_cos();
    ([sb, 0.0, cb], [-(sa * cb), ca, sa * sb])
}

/// A random orientation like F1675_MatBuildYZT's two random binary angles: the
/// given forward tilted away from `base` by a random yaw and pitch.
fn random_axes(base: [f64; 3], rng: &mut Random) -> ([f64; 3], [f64; 3]) {
    let yaw = f64::from(rng.bounded(65_536)) / 65_536.0 * std::f64::consts::TAU;
    let pitch = f64::from(rng.bounded(65_536)) / 65_536.0 * std::f64::consts::PI;
    // A helper axis perpendicular to `base`, so the tilt is not degenerate for
    // a ship pointing straight up.
    let helper = if base[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let right = normalise(cross(helper, base));
    let up = normalise(cross(base, right));
    let forward = normalise([
        base[0] * pitch.cos() + (right[0] * yaw.cos() + up[0] * yaw.sin()) * pitch.sin(),
        base[1] * pitch.cos() + (right[1] * yaw.cos() + up[1] * yaw.sin()) * pitch.sin(),
        base[2] * pitch.cos() + (right[2] * yaw.cos() + up[2] * yaw.sin()) * pitch.sin(),
    ]);
    // The up row needs a helper that is not along the new forward row, or
    // a forward near the first helper gives a zero cross product.
    let helper = if forward[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let right = normalise(cross(helper, forward));
    (forward, normalise(cross(forward, right)))
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(v: [f64; 3]) -> [f64; 3] {
    let length = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if length < 1e-9 {
        [0.0, 0.0, 1.0]
    } else {
        v.map(|x| x / length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparks_keep_their_throwers_global_word_one() {
        // The executed death probe: explosion sparks have `+0x9e` 0xffff and
        // the distance word +2; debris sparks 15 (a wreck pod) and -1.
        let mut rng = Random::start();
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let burst = explosion([0; 3], [0.0; 3], 0, axes, &mut rng);
        let spark = burst[0].spark(&mut rng);
        assert_eq!((spark.size, spark.shift), (0xffff, 2));
        // A seed whose explosion throws a wreck pod.
        let pod = (1u32..)
            .find_map(|seed| {
                let mut rng = Random::new(seed, seed.rotate_left(13));
                explosion([0; 3], [0.0; 3], 8, axes, &mut rng)
                    .into_iter()
                    .find(|e| e.kind == Kind::Debris && e.cargo == Some(WRECK_CARGO))
            })
            .unwrap();
        assert_eq!(pod.size, 15);
        let spark = pod.spark(&mut rng);
        assert_eq!((spark.size, spark.shift), (pod.size, -1));
    }

    /// The executed original's death at the Ross 154 pad (`jjffe-smoke --death-probe`
    /// with `FFE_PAIR=12345678,9abcdef0 FFE_DEATH_OBJECTS=1`, frame 250; the full
    /// 450-frame comparison is `tests/death_replay.rs`): F942's draws, F926's
    /// placement and the first frame's sparks.
    #[test]
    fn f942_and_the_first_sweep_match_the_executed_death_probe() {
        let ship = [-5_594_992_173, -2_625_307_481, 473_426_443];
        let rows = [
            [-906_354_405.0, 1_945_245_414.0, 76_650_739.0],
            [-1_938_310_550.0, -909_597_149.0, 163_923_668.0],
            [180_966_058.0, 0.0, 2_139_827_798.0],
        ]
        .map(|row: [f64; 3]| row.map(|v| v / 2_147_483_648.0));
        let mut rng = Random::new(0x1234_5678, 0x9abc_def0);
        let burst = explosion(ship, [0.0; 3], 0, rows, &mut rng);
        // Two F1530 draws, F922(6), then two F1530 draws a pod.
        assert_eq!((rng.a, rng.b), (936_602_760, 3_400_778_596));
        assert_eq!(burst.len(), 4);
        assert_eq!(burst[0].position, ship);
        assert_eq!(burst[0].size, EXPLOSION_SIZE);
        // F926's F1514: the pod sits at the ship plus its drift, and keeps the
        // ship's own rows.
        // In creation order, so slots 102, 101, 100.
        let drift = [[122, -240, -120], [144, 110, -144], [108, 223, -214]];
        for (pod, drift) in burst[1..].iter().zip(drift) {
            assert_eq!(pod.velocity_words(), drift);
            assert_eq!(
                pod.position,
                [0, 1, 2].map(|k| ship[k] + i64::from(drift[k]))
            );
            assert_eq!((pod.forward, pod.up), (rows[2], rows[1]));
        }
        // The other objects hold slots 104..=114 (the player's, 107, goes at the
        // end of F942); the explosion takes 103 and the pods 102..=100.
        let mut table = EffectTable::new();
        for _ in 1..OBJECT_SLOTS {
            let mut filler = test_effect(Kind::Derelict);
            filler.tag = Some(0);
            table.spawn(filler);
        }
        for slot in 1..104 {
            table.free(slot);
        }
        let mut burst = burst.into_iter();
        assert_eq!(table.spawn(burst.next().unwrap()), Some(103));
        for pod in burst {
            table.spawn_pod(pod);
        }
        table.free(107);
        // The sweep walks the slots downwards: pod 102 throws its spark into 107,
        // 101 into 99 and 100 into 98 (the last two are visited again this frame).
        let born = table.sweep(1043, Some(2_147_576_475), 0.0, ship, &mut rng);
        assert_eq!(born, vec![107, 99, 98]);
        assert_eq!((rng.a, rng.b), (3_604_963_699, 3_960_172_047));
        assert_eq!(table.get(107).unwrap().timer, 50_771);
        // 37,126 and 57,435 (`F1530 | 0x8000`) less this frame's `1043 >> 1`:
        // slots 99 and 98 were visited again after their births.
        assert_eq!(table.get(99).unwrap().timer, 37_126 - 521);
        assert_eq!(table.get(98).unwrap().timer, 57_435 - 521);
        // F708: `1043 >> 2` off the explosion's word.
        assert_eq!(table.get(103).unwrap().timer, 0xffff - 260);
    }

    #[test]
    fn a_derelict_drifts_until_f767_removes_it() {
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        // 100 m/s along z from the origin: 1030 source words.
        let mut hull = derelict(0x24, [0; 3], [0.0, 0.0, 100_000.0], axes);
        assert_eq!(hull.velocity_words(), [0, 0, 1030]);
        assert_eq!(hull.tick(1.0, 49_700), Update::Nothing);
        // 49700 >> 7 = 388; 1030 * 388 >> 2 = 99,910 mm, not 100,000.
        assert_eq!(hull.position, [0, 0, 99_910]);
        assert!(hull.alive() && !hull.out_of_range([0; 3]));
        // F767: gone at distance class 0xd (2^26 mm, about 67 km).
        hull.position[2] = DEBRIS_RANGE;
        assert!(hull.out_of_range([0; 3]));
    }

    #[test]
    fn f1511_advances_by_integer_velocity_words() {
        // 1000 mm/s is ten words of 49700/512 mm/s each.
        let mut hull = derelict(0x24, [0; 3], [1000.0, 0.0, -1000.0], axes());
        assert_eq!(hull.velocity_words(), [10, 0, -10]);
        // F1511: 4970 >> 7 = 38, then 10 * 38 >> 2 = 95 on each axis.
        hull.tick(0.1, 4970);
        assert_eq!(hull.position, [95, 0, -95]);
        // The time is truncated to 128-tic units: 127 tics do not move it,
        // 128 tics move by 10 * 1 >> 2 = 2 (and -10 * 1 >> 2 = -3).
        let before = hull.position;
        hull.tick(0.003, 127);
        assert_eq!(hull.position, before);
        hull.tick(0.003, 128);
        assert_eq!(hull.position, [before[0] + 2, 0, before[2] - 3]);
    }

    #[test]
    fn random_axes_stay_unit_for_every_draw() {
        // Every forward row the draws can give, including ones along the
        // helper axis, makes a finite orthonormal pair.
        let mut rng = Random::start();
        for base in [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]] {
            for _ in 0..20_000 {
                let (forward, up) = random_axes(base, &mut rng);
                let length = |v: [f64; 3]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
                assert!((length(forward) - 1.0).abs() < 1e-9, "{forward:?}");
                assert!((length(up) - 1.0).abs() < 1e-9, "{up:?}");
                let dot: f64 = (0..3).map(|k| forward[k] * up[k]).sum();
                assert!(dot.abs() < 1e-9);
            }
        }
    }

    fn axes() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    #[test]
    fn f939_spawns_one_particle_with_the_ships_velocity() {
        let mut rng = Random::new(0x1234_5678, 0x4567_89ab);
        let particle = hit(
            [10, 20, 30],
            [100.0, 200.0, 300.0],
            axes(),
            PARTICLE_SPEED,
            &mut rng,
        );
        assert_eq!(particle.model, HIT_MODEL);
        assert_eq!(particle.kind, Kind::Hit);
        assert_eq!(particle.position, [10, 20, 30]);
        // The float field is the same three velocity words F939 cloned.
        assert_eq!(particle.velocity_words(), [1, 2, 3]);
        assert!(particle.timer >= 0x8000);
        // A slow source leaves the particle where it was hit (F939 zeroes it).
        let mut rng = Random::new(1, 2);
        let slow = hit([0; 3], [1.0, 2.0, 3.0], axes(), 0.0, &mut rng);
        assert_eq!(slow.velocity, [0.0; 3]);
    }

    #[test]
    fn f942_spawns_the_explosion_and_f926_debris_with_its_counts() {
        let mut rng = Random::new(0x1234_5678, 0x4567_89ab);
        let effects = explosion([100, 200, 300], [10.0; 3], 8, axes(), &mut rng);
        let burst = &effects[0];
        assert_eq!(burst.model, EXPLOSION_MODEL);
        assert_eq!(burst.kind, Kind::Explosion);
        assert_eq!(burst.timer, EXPLOSION_TIMER);
        // F922(6) - 2 pods (0..3) plus F922(min(4, 8 >> 1)) - 1 cargo pods
        // (0..2).
        let debris = effects.iter().filter(|e| e.kind == Kind::Debris).count();
        assert!(debris <= 5, "{debris}");
        // No cargo, and F922(6) can still give 0..3 pods.
        let mut rng = Random::new(3, 4);
        let light = explosion([0; 3], [0.0; 3], 0, axes(), &mut rng);
        assert!(light.len() <= 4);
        // A pod keeps the ship's velocity words and adds F926's signed
        // `DATA_008818 >> 22` drift (-512..=511) to each of them.
        let mut rng = Random::new(5, 6);
        let pods = explosion([0; 3], [1000.0; 3], 16, axes(), &mut rng);
        for pod in pods.iter().skip(1) {
            assert_eq!(pod.model, DEBRIS_MODEL);
            for (axis, word) in pod.velocity_words().iter().enumerate() {
                let drift = word - 10;
                assert!((-512..=511).contains(&drift), "{drift}");
                assert!((pod.velocity[axis] - word_speed(*word)).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn f719_spawns_one_explosion_and_no_debris() {
        let mut rng = Random::new(7, 8);
        let burst = missile_explosion([1, 2, 3], axes(), &mut rng);
        assert_eq!(burst.model, EXPLOSION_MODEL);
        assert_eq!(burst.kind, Kind::Explosion);
        assert_eq!(burst.position, [1, 2, 3]);
        assert_eq!(burst.velocity, [0.0; 3], "F719 leaves the velocity zero");
        assert_eq!(burst.timer, EXPLOSION_TIMER);
    }

    #[test]
    fn effects_move_and_expire() {
        let mut rng = Random::new(5, 6);
        let mut particle = hit([0; 3], [1000.0, 0.0, 0.0], axes(), PARTICLE_SPEED, &mut rng);
        // Ten words over 4970 tics: 4970 >> 7 = 38, 10 * 38 >> 2 = 95.
        particle.tick(0.1, 4970);
        assert_eq!(particle.position, [95, 0, 0]);
        assert!(particle.alive());
        // F709: at most 0xffff / (994 >> 1) updates of 20 ms, 2.6 s.
        for _ in 0..132 {
            particle.tick(0.02, 994);
        }
        assert!(!particle.alive());
        // A zero-length frame does nothing.
        let before = particle.position;
        particle.tick(0.0, 0);
        assert_eq!(particle.position, before);
    }

    #[test]
    fn f708_runs_the_explosion_timer_and_its_spark_window() {
        let mut rng = Random::new(7, 8);
        let mut burst = explosion([0; 3], [0.0; 3], 0, axes(), &mut rng).remove(0);
        let mut sparks = Vec::new();
        let mut updates = 0;
        while burst.alive() {
            if burst.tick(0.02, 994) == Update::Spark {
                sparks.push(updates);
            }
            updates += 1;
        }
        // 0xffff / (994 >> 2 = 248) is 264 updates of 20 ms: 5.3 s.
        assert_eq!(updates, 265);
        // 0xffff - 0xc800 .. 0xffff - 0xbc00 at 248 a step.
        assert_eq!(sparks.first(), Some(&57));
        assert_eq!(sparks.last(), Some(&69));
    }

    #[test]
    fn f926_pods_spark_for_seven_think_ticks_then_drift() {
        let mut rng = Random::new(5, 6);
        let mut pod = loop {
            let mut pods = explosion([0; 3], [0.0; 3], 16, axes(), &mut rng);
            if pods.len() > 1 {
                break pods.pop().unwrap();
            }
        };
        assert_eq!(pod.kind, Kind::Debris);
        let mut sparking = 0.0f64;
        while pod.tick(0.02, 994) == Update::Spark {
            sparking += 0.02;
        }
        // Seven day ticks of 0xc22e tics at 994 a 20 ms update: 7.0 s.
        assert!((sparking - 7.0).abs() < 0.05, "{sparking}");
        assert!(pod.alive());
        assert!(!pod.out_of_range([0; 3]));
        assert!(pod.out_of_range([2 * DEBRIS_RANGE, 0, 0]));
    }

    #[test]
    fn f714_scoops_with_the_tractor_or_the_scoop() {
        let mut rng = Random::new(9, 10);
        let pod = canister([0, 0, 10_000], [0.0; 3], axes(), [0.0; 3], 7, &mut rng);
        let mut ship = Scoop {
            position: [0; 3],
            forward: [0.0, 0.0, 1.0],
            radius: 10_000.0,
            tractor: false,
            scoop: false,
        };
        // No scoop fitted: nothing.
        assert!(!pod.scooped_by(&ship));
        // The Cargo Scoop takes a canister touching the nose.
        ship.scoop = true;
        assert!(pod.scooped_by(&ship));
        // Not one behind the ship (a jettison leaves it there).
        let behind = canister([0, 0, -10_000], [0.0; 3], axes(), [0.0; 3], 7, &mut rng);
        assert!(!behind.scooped_by(&ship));
        // The Tractor Beam reaches 2^15 mm within 60° of the nose.
        ship.scoop = false;
        ship.tractor = true;
        let far = canister([0, 10_000, 30_000], [0.0; 3], axes(), [0.0; 3], 7, &mut rng);
        assert!(far.scooped_by(&ship));
        let wide = canister([30_000, 0, 10_000], [0.0; 3], axes(), [0.0; 3], 7, &mut rng);
        assert!(!wide.scooped_by(&ship));
        let out = canister([0, 0, 40_000], [0.0; 3], axes(), [0.0; 3], 7, &mut rng);
        assert!(!out.scooped_by(&ship));
        // An explosion's flash carries no cargo.
        let burst = explosion([0, 0, 1000], [0.0; 3], 0, axes(), &mut rng).remove(0);
        assert!(!burst.scooped_by(&ship));
    }

    #[test]
    fn f926_pods_carry_alloys_then_the_wreck_table() {
        let mut rng = Random::new(11, 12);
        for _ in 0..50 {
            let effects = explosion([0; 3], [0.0; 3], 16, axes(), &mut rng);
            for pod in effects.iter().filter(|e| e.kind == Kind::Debris) {
                let cargo = pod.cargo.expect("a pod carries cargo");
                assert!(
                    cargo == WRECK_CARGO || WRECK_CARGO_TABLE.contains(&(cargo as u8)),
                    "{cargo}"
                );
            }
        }
    }

    fn test_effect(kind: Kind) -> Effect {
        Effect {
            kind,
            model: 0,
            position: [0; 3],
            velocity: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            timer: 0,
            countdown: 0,
            think: 0,
            alive: true,
            shift: 0,
            cargo: None,
            size: 0,
            life: 0,
            exit: false,
            tag: None,
            parent: None,
            fraction: [0; 3],
            fresh: false,
        }
    }

    #[test]
    fn f926_adds_its_drift_to_the_velocity_words() {
        let mut rng = Random::new(5, 6);
        let pod = loop {
            let mut effects = explosion([0; 3], [1000.0; 3], 16, axes(), &mut rng);
            if effects.len() > 1 {
                break effects.pop().unwrap();
            }
        };
        assert_eq!(pod.kind, Kind::Debris);
        // The ship's ten words plus F926's signed 10-bit drift.
        let words = pod.velocity_words();
        for (axis, word) in words.iter().enumerate() {
            assert!((-512..=511).contains(&(word - 10)), "{}", word - 10);
            assert!((pod.velocity[axis] - word_speed(*word)).abs() < 1e-9);
        }
    }

    #[test]
    fn effect_table_reuses_a_freed_slot_from_the_top() {
        let mut table = EffectTable::new();
        assert_eq!(table.spawn(test_effect(Kind::Debris)), Some(0x72));
        assert_eq!(table.spawn(test_effect(Kind::Explosion)), Some(0x71));
        assert_eq!(table.len(), 2);
        assert_eq!(table.counted(), 0);
        // F924 frees the slot; F928's next spawn takes it again.
        assert_eq!(table.free(0x72).unwrap().kind, Kind::Debris);
        assert_eq!(table.spawn(test_effect(Kind::Hit)), Some(0x72));
        assert_eq!(table.len(), 2);
        assert_eq!(table.counted(), 1, "F928 counts the 0x20 flag");
        assert_eq!(table.free(0x72).unwrap().kind, Kind::Hit);
        assert_eq!(table.counted(), 0, "F924 decrements it again");
    }

    #[test]
    fn f927_keeps_the_low_slots_for_ships() {
        let mut table = EffectTable::new();
        for _ in 0..OBJECT_SLOTS - SHIP_SLOT_FLOOR {
            assert!(table.spawn_pod(test_effect(Kind::Debris)).is_some());
        }
        assert_eq!(table.len(), OBJECT_SLOTS - SHIP_SLOT_FLOOR);
        // No free slot at or above the floor, and no counted explosion there.
        assert_eq!(table.spawn_pod(test_effect(Kind::Debris)), None);
        // F928 has no floor: a particle takes the topmost free slot anywhere.
        assert_eq!(
            table.spawn(test_effect(Kind::Hit)),
            Some(SHIP_SLOT_FLOOR - 1)
        );
    }

    #[test]
    fn a_full_table_replaces_the_topmost_counted_explosion() {
        let mut table = EffectTable::new();
        for _ in 1..OBJECT_SLOTS {
            assert!(table.spawn(test_effect(Kind::Debris)).is_some());
        }
        assert_eq!(table.len(), OBJECT_SLOTS - 1);
        // Put a counted explosion at the top.
        assert!(table.free(0x72).is_some());
        assert_eq!(table.spawn(test_effect(Kind::Hit)), Some(0x72));
        assert_eq!(table.counted(), 1);
        // Full: the topmost 0x20 object is replaced, so the count is unchanged.
        assert_eq!(table.spawn(test_effect(Kind::Hit)), Some(0x72));
        assert_eq!(table.counted(), 1);
    }

    #[test]
    fn f927_does_not_recycle_a_counted_slot_below_the_floor() {
        let mut table = EffectTable::new();
        // Fill 0x14..=0x72 with F926 pods and 0x01..=0x13 with F928 objects.
        for _ in 0..OBJECT_SLOTS - SHIP_SLOT_FLOOR {
            assert!(table.spawn_pod(test_effect(Kind::Debris)).is_some());
        }
        for _ in 1..SHIP_SLOT_FLOOR {
            assert!(table.spawn(test_effect(Kind::Debris)).is_some());
        }
        assert_eq!(table.len(), OBJECT_SLOTS - 1);
        // A counted explosion below the floor.
        assert!(table.free(0x13).is_some());
        assert_eq!(table.spawn(test_effect(Kind::Hit)), Some(0x13));
        assert_eq!(table.counted(), 1);
        // F927 may not replace it; F928 may.
        assert_eq!(table.spawn_pod(test_effect(Kind::Debris)), None);
        assert_eq!(table.spawn(test_effect(Kind::Debris)), Some(0x13));
        assert_eq!(table.counted(), 0);
    }

    #[test]
    fn a_dead_effects_slot_is_recycled_by_the_next_spawn() {
        let mut table = EffectTable::new();
        let mut particle = test_effect(Kind::Hit);
        particle.timer = 1;
        assert_eq!(table.spawn(particle), Some(0x72));
        let mut burst = test_effect(Kind::Explosion);
        burst.timer = EXPLOSION_TIMER;
        assert_eq!(table.spawn(burst), Some(0x71));
        let mut rng = Random::new(1, 2);
        // F709: 994 >> 1 = 497 is more than the particle's timer.
        assert!(table.tick(0.02, 994, [0; 3], &mut rng).is_empty());
        assert!(table.get(0x72).is_none());
        assert_eq!(table.len(), 1);
        assert_eq!(table.counted(), 0);
        assert_eq!(table.spawn(test_effect(Kind::Hit)), Some(0x72));
    }

    #[test]
    fn f712_sparks_are_allocated_into_free_slots() {
        let mut table = EffectTable::new();
        let mut pod = test_effect(Kind::Debris);
        pod.countdown = DEBRIS_SPARK_TICKS;
        assert_eq!(table.spawn_pod(pod), Some(0x72));
        let mut rng = Random::new(3, 4);
        // F712 throws one F939 spark from the pod's centre each update.
        assert_eq!(table.tick(0.02, 994, [0; 3], &mut rng), vec![0x71]);
        assert_eq!(table.get(0x71).unwrap().kind, Kind::Hit);
        assert_eq!(table.counted(), 1);
    }

    #[test]
    fn the_effects_facade_hands_a_freed_slot_to_the_next_push() {
        let mut effects = Effects::new();
        // The first push takes F928's topmost slot, the next the one below.
        let mut particle = test_effect(Kind::Hit);
        particle.timer = 1;
        effects.push(particle);
        effects.push(test_effect(Kind::Derelict));
        assert_eq!(
            effects.slots().map(|(slot, _)| slot).collect::<Vec<_>>(),
            vec![0x71, 0x72]
        );
        // F709: the top particle's one-tick timer ends in this update, and F924
        // frees its slot before the frame ends.
        let mut rng = Random::new(1, 2);
        effects.tick(0.02, 994, [0; 3], &mut rng);
        assert_eq!(effects.len(), 1);
        assert!(effects.get_slot(0x72).is_none());
        assert_eq!(effects.counted(), 0);
        // The next F928 spawn takes the freed slot, not a new one.
        effects.push(test_effect(Kind::Hit));
        assert_eq!(effects.get_slot(0x72).unwrap().kind, Kind::Hit);
        assert_eq!(effects.counted(), 1);
        assert_eq!(
            effects.slots().map(|(slot, _)| slot).collect::<Vec<_>>(),
            vec![0x71, 0x72]
        );
    }

    #[test]
    fn a_burst_takes_f928s_slot_and_its_pods_f927s() {
        let mut effects = Effects::new();
        let mut rng = Random::new(4, 5);
        let burst = explosion([0; 3], [0.0; 3], 16, axes(), &mut rng);
        let explosion = effects
            .push_burst(burst)
            .expect("the model-0x9b object takes a slot");
        assert_eq!(explosion, 0x72);
        assert_eq!(
            effects.get_slot(explosion).unwrap().kind,
            Kind::Explosion,
            "F928 allocates the explosion"
        );
        for (slot, effect) in effects
            .slots()
            .filter(|(_, effect)| effect.kind != Kind::Explosion)
        {
            assert_eq!(effect.kind, Kind::Debris);
            assert!(slot >= SHIP_SLOT_FLOOR, "F927 keeps {slot:#x} back");
        }
    }

    /// F152's event 6 is global: a pod counts the next `0xc22e`-tic boundary
    /// of the game clock, not a full second after it was born, so the facade
    /// carries that clock with the tics it advances the objects by.
    #[test]
    fn a_pod_counts_the_next_f152_boundary_not_a_second_after_its_birth() {
        let mut effects = Effects::new();
        let mut rng = Random::new(1, 2);
        // 50 frames of 20 ms: the F152 clock is at 49,700 tics, just short of
        // the 49,710 (`0xc22e`) boundary.
        for _ in 0..50 {
            effects.tick(0.02, 994, [0; 3], &mut rng);
        }
        let mut pod = test_effect(Kind::Debris);
        pod.countdown = DEBRIS_SPARK_TICKS;
        effects.push_pod(pod);
        effects.tick(0.02, 994, [0; 3], &mut rng);
        let pod = effects
            .iter()
            .find(|effect| effect.kind == Kind::Debris)
            .expect("the pod is still drifting");
        assert_eq!(
            pod.countdown,
            DEBRIS_SPARK_TICKS - 1,
            "the frame crossed F152's boundary, so F768 counted the pod"
        );
    }

    /// A pod's F768 count runs on the frame tics alone: a caller that ticks the
    /// effects without advancing a world clock still sees it stop at seven
    /// boundaries (the regression that left a pod sparking for ever).
    #[test]
    fn a_pod_stops_sparking_after_seven_f152_boundaries() {
        let mut effects = Effects::new();
        let mut pod = test_effect(Kind::Debris);
        pod.countdown = DEBRIS_SPARK_TICKS;
        effects.push_pod(pod);
        let mut rng = Random::new(1, 2);
        // 400 frames of 20 ms: 8 s of game tics, well past seven boundaries.
        for _ in 0..400 {
            effects.tick(0.02, 994, [0; 3], &mut rng);
        }
        let pod = effects
            .iter()
            .find(|effect| effect.kind == Kind::Debris)
            .expect("the pod is still drifting");
        assert!(pod.countdown < 0, "the pod still sparks: {pod:?}");
    }

    /// F1094 adds `0xff9e8e` tics (about 337 game seconds) to the player's exit cloud's target
    /// time, so it stays behind the ship for minutes; a ship's (F740) goes at the next control tick.
    #[test]
    fn the_players_exit_cloud_stays_behind_the_ship_for_minutes() {
        let frame = 1043u32;
        let mut player = exit_cloud([0; 3], PLAYER_EXIT_CLOUD_TICS);
        let mut ship = exit_cloud([0; 3], u64::from(THINK_TICS));
        for _ in 0..15_000 {
            player.tick(0.02, frame);
            ship.tick(0.02, frame);
        }
        assert!(player.alive, "still there after about 310 game seconds");
        assert!(
            !ship.alive,
            "a ship's cloud is gone after its next control tick"
        );
        for _ in 0..2_000 {
            player.tick(0.02, frame);
        }
        assert!(!player.alive, "gone after about 337 game seconds");
    }

    #[test]
    fn saved_effects_return_to_their_source_slots() {
        let mut effects = Effects::new();
        effects.push(departure_cloud([1, 2, 3], 100_000));
        let mut rng = Random::new(6, 7);
        effects.push_burst(explosion([0; 3], [0.0; 3], 8, axes(), &mut rng));
        effects.push(canister(
            [1000, 0, 0],
            [1.0; 3],
            axes(),
            [0.0; 3],
            15,
            &mut rng,
        ));
        let slots: Vec<usize> = effects.slots().map(|(slot, _)| slot).collect();
        // A save keeps slot order; rebuilding it in reverse gives every object
        // the slot it had.
        let loaded = Effects::from_saved(&effects.to_vec());
        assert_eq!(loaded, effects);
        assert_eq!(
            loaded.slots().map(|(slot, _)| slot).collect::<Vec<_>>(),
            slots
        );
    }
}
