//! Missiles, mines and the missile lock (guide: `docs/notes/combat.md`).
//!
//! Source: JJFFE 2.8a7 `ffe.asm`/`ffedat.asm`.
//!
//! - **Ammo.** A ship carries ten pylon words at `ship+0xd6..+0xdf`
//!   (`plstate.txt`). A missile code's low nibble is its type; `DATA_007220`
//!   (`ffedat.asm:18441`) maps types 0..9 to models 4..13. Bit `0x40` marks a
//!   mine (F941, `ffe.asm:70399`). The new game's Eagle has two `0x82` words
//!   (`FUNC_000998`, `ffe.asm:74099`): type 2, model 6.
//! - **Launch.** The `M` key (F1054, `ffe.asm:78204`) finds the first non-zero
//!   pylon, stores its 1-based index at `player+0xc4`, and F1096
//!   (`ffe.asm:80303`) calls F941 with the combat target `DATA_008875` when a
//!   missile and a target are present (`0x98c1` otherwise). F941 clears the
//!   pylon word and allocates an object with control `0xb`, AI state `0x13`
//!   (missile), `0x14` (mine) or `0x16` (model-4 mine), and sets the owner at
//!   `+0x100`. Its initial velocity is the launcher's collision radius along
//!   `-up` plus (or minus, by pylon parity) `+right * 0x2000`; F1514 adds that
//!   once to the position, the separation impulse.
//! - **Guidance.** The missile's AI is F758 (`ffe.asm:50035`, event 6) and F717
//!   (`ffe.asm:47399`, event 5). F758 ages the missile at `+0x9e` and explodes
//!   it above `0x3c` (60 updates); it gets the target from `+0xfe`, calls F596
//!   (lead-pursuit direction), and while the distance exponent is above 4 adds
//!   the F599 thrust vector (plus `0x1108` on the local z) and F586's turn rates
//!   (`x >> 3` yaw, `-(y >> 3)` pitch). F717 integrates the acceleration and
//!   rotation. `docs/notes/combat.md` §8 records the original's unreachable
//!   pieces.
//! - **Fuse and damage.** F758 detonates at distance exponent <= 4, which is
//!   under `2^(13+4)` mm (131 m); F718/`F721` also detonate on the F569 contact
//!   test. F720 damages the target with `DATA_006269[model]`
//!   (`ffedat.asm:10124`) through F953. The player's model 6 does 200.
//!
//! The port keeps the original's tables and rules in floating point: the
//! original's Q15 thrust words become mm/s² with the AI's 0x20 event-5 tics (0.59 mm/s² per word),
//! and its quantised F585 turn words become a maximum turn rate. The lead angle
//! is a continuous-time version of F597's relative-velocity fudge. This module
//! has no I/O and no cache access; `ffe-rs/src/missile.rs` builds [`Spec`]s from
//! the cache's 48-byte model statistics.

/// The number of pylon words (`ship+0xd6..+0xdf`).
pub const PYLONS: usize = 10;
/// The player's object id in the missile target space. `NpcShip` ids are small
/// and the F941 object ids are 0-based, so a reserved high word keeps the
/// player's target distinct from every ship (`DATA_008857` in the source).
pub const PLAYER_TARGET: u32 = u32::MAX;
/// The first chaff/decoy object id (`DATA_007253`). F1057 stores the chaff
/// object's id in the missile's `+0xfe`; the port puts decoys above every ship
/// and the player so a target id says which list to look in.
pub const DECOY_BASE: u32 = 0x8000_0000;
/// F1054's chaff object model (`0xa4`, 164), the invisible decoy the source
/// spawns before F1057 retargets missiles to it.
pub const DECOY_MODEL: u32 = 0xa4;

/// True when `id` is a chaff/decoy target.
pub fn is_decoy(id: u32) -> bool {
    id >= DECOY_BASE
}
/// The original's type-to-model table `DATA_007220`.
pub const MODELS: [u32; 10] = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
/// A mine code's flag (`0x40`); F941's targeted branch.
pub const MINE_FLAG: u8 = 0x40;
/// The new game's missile code (F998): two `0x82` pylons.
pub const PLAYER_MISSILE: u8 = 0x82;
/// F941's AI state for a missile (`+0xff`).
pub const AI_MISSILE: u8 = 0x13;
/// F941's AI state for a mine that is not model 4.
pub const AI_MINE: u8 = 0x14;
/// F941's AI state for the model-4 mine.
pub const AI_MINE_MODEL_4: u8 = 0x16;
/// `DATA_006269` (`ffedat.asm:10124`), indexed by model.
pub const DAMAGE: [u32; 14] = [
    250, 300, 400, 500, 10, 100, 200, 300, 400, 600, 800, 1000, 100, 111,
];
/// F758's lifetime: it explodes above 60 event-6 updates.
pub const LIFETIME_UPDATES: u32 = 0x3c;
/// F758's event-6 rate: the executed new game updates a missile's guidance
/// about every 20 frames (0.4 s) while F717's physics runs every frame.
pub const GUIDANCE_INTERVAL: f64 = 0.4;
/// The event-5 physics `tics` the AI passes to F601: 0x20, not the 994-tic
/// frame time, which is why a model-6 missile gains about 4 velocity units a
/// frame.
pub const AI_TICS: u32 = 0x20;
/// F758's lifetime in world seconds (60 guidance updates).
pub const LIFETIME_SECONDS: f64 = LIFETIME_UPDATES as f64 * GUIDANCE_INTERVAL;
/// F758's proximity fuse: a truncated distance exponent of 4 or less, i.e.
/// under `2^(13+4)` mm (about 131 m).
pub const PROXIMITY_MM: f64 = 131_072.0;
/// F599's local-z bias (`0x1108`) added to the aim direction before clamping.
pub const FORWARD_BIAS: f64 = 0x1108 as f64 / 32_768.0;
/// F601/F586: mm/s² per unit of a model's thrust word, with the AI's 0x20
/// `tics`: `word * ((tics >> 3) * 2) / 65536` velocity units per frame, one
/// velocity unit `49_700/512` mm/s, 50 frames a second.
pub const ACCEL_PER_WORD: f64 =
    ((AI_TICS >> 3) as f64 * 2.0 / 65_536.0) * (49_700.0 / 512.0) * 50.0;
/// F586/F585: radians per second per turn word (`word/8` per update, 65536
/// angle units per turn, 50 updates per second).
pub const TURN_PER_WORD: f64 = std::f64::consts::TAU * 50.0 / 65_536.0;
/// F767's removal range for the model-4 mine (state 0x16): distance class 0xd
/// is `2^(13+13)` mm (67 km) from the camera.
pub const SEEKING_RANGE_MM: f64 = (1u64 << 26) as f64;

/// What one of `DATA_006255`'s missile/object rows does, as `ffe_core` can run
/// it. The rows 0x13-0x1b are the same four-slot table as the flight states
/// (`npc::AI_STATES`), but their movement routines belong to the missile and
/// effect objects rather than to `NpcShip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    /// F758 + F717: F758's guidance every 0.4 s (60 updates), the proximity
    /// fuse and F717's per-frame F603/F601/F585/F1512 physics.
    Guided,
    /// F716: F603 and F718's contact test only. A mine has no control slot, so
    /// it never guides, never applies thrust and never times out.
    Mine,
    /// F767 + F713: F713's F1512/F603/F714 seeker every frame, with F767's
    /// removal at distance class 0xd and no F758 lifetime.
    Seeking,
    /// An object the port's `effects` object list owns (debris, asteroids,
    /// dust), not a [`Missile`].
    Object,
}

/// One row of `DATA_006255`/`DATA_006256` (control, movement) for the missile
/// and object states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiRow {
    /// `object+0xff`.
    pub code: u8,
    pub name: &'static str,
    /// F706's event-6 controller (0x13 F758, 0x15 F768, 0x16 F767, 0x17
    /// F732, 0x18 F766, 0x19 F765, 0x1b F769).
    pub control: Option<u16>,
    /// F704's event-5 physics step (0x13 F717, 0x14 F716, 0x15 F712, 0x16
    /// F713, 0x17 F714, 0x18 F710, 0x19 F711, 0x1a F709, 0x1b F715).
    pub movement: Option<u16>,
    pub behaviour: Behaviour,
}

/// `DATA_006255` rows 0x13-0x1b (`ffedat.asm:9854`). The port's `MissileField`
/// creates 0x13 (F941's missile), 0x14 (a mine) and 0x16 (the model-4 mine);
/// 0x15 and 0x17-0x1b are objects `ffe_core::effects` owns or the port never
/// creates.
pub const AI_ROWS: [AiRow; 9] = [
    AiRow {
        code: 0x13,
        name: "missile",
        control: Some(758),
        movement: Some(717),
        behaviour: Behaviour::Guided,
    },
    AiRow {
        code: 0x14,
        name: "mine",
        control: None,
        movement: Some(716),
        behaviour: Behaviour::Mine,
    },
    AiRow {
        code: 0x15,
        name: "missile",
        control: Some(768),
        movement: Some(712),
        behaviour: Behaviour::Object,
    },
    AiRow {
        code: 0x16,
        name: "model-4 mine",
        control: Some(767),
        movement: Some(713),
        behaviour: Behaviour::Seeking,
    },
    AiRow {
        code: 0x17,
        name: "missile",
        control: Some(732),
        movement: Some(714),
        behaviour: Behaviour::Object,
    },
    AiRow {
        code: 0x18,
        name: "cargo",
        control: Some(766),
        movement: Some(710),
        behaviour: Behaviour::Object,
    },
    AiRow {
        code: 0x19,
        name: "asteroid",
        control: Some(765),
        movement: Some(711),
        behaviour: Behaviour::Object,
    },
    AiRow {
        code: 0x1a,
        name: "missile",
        control: None,
        movement: Some(709),
        behaviour: Behaviour::Object,
    },
    AiRow {
        code: 0x1b,
        name: "space dust",
        control: Some(769),
        movement: Some(715),
        behaviour: Behaviour::Object,
    },
];

/// The row for a missile's `+0xff`, when it is one of the missile rows.
pub fn ai_row(code: u8) -> Option<&'static AiRow> {
    AI_ROWS.iter().find(|row| row.code == code)
}

/// The lifecycle [`Missile::tick_with`] runs for a row. An unknown code keeps
/// F758's guided behaviour (the only launch the source makes through F941 is
/// 0x13/0x14/0x16).
pub fn behaviour(code: u8) -> Behaviour {
    ai_row(code).map_or(Behaviour::Guided, |row| row.behaviour)
}

/// `DATA_007220[code & 0xf]`, or `None` past the table.
pub fn model(code: u8) -> Option<u32> {
    MODELS.get(usize::from(code & 0x0f)).copied()
}

/// `DATA_006269[model]`; anything outside the table does no damage.
pub fn damage(model: u32) -> u32 {
    DAMAGE.get(model as usize).copied().unwrap_or(0)
}

/// F941's mine test (`code & 0x40`).
pub fn is_mine(code: u8) -> bool {
    code & MINE_FLAG != 0
}

/// F718's ECM dispatch table, `DATA_000029` (`ffe.asm:47490`), indexed by the
/// missile model. The value is the branch: 0 destroys on any ECM burst, 1 is
/// immune, 2 needs the naval system's extra `DATA_008835` bit 3.
///
/// The table is `[0, 0, 0, 0, 1, 0, 0, 2, 1, 2, 2, 1, 1, 1]`, so models 5 and
/// 6 are destroyed by the standard E.C.M. and models 7, 9 and 10 only by the
/// Naval E.C.M.; 4, 8, 11, 12 and 13 are immune (`ffe.asm:47495-47566`,
/// `DATA_000030`'s three jump entries). Models above 13 take the source's
/// `ja` branch and are destroyed.
pub const ECM_BRANCH: [u8; 14] = [0, 0, 0, 0, 1, 0, 0, 2, 1, 2, 2, 1, 1, 1];

/// The source's NN500 Naval Missile model: equipment code `0x84`
/// (`ffedat.asm:8380`, `DATA_005775`) takes `DATA_007220[4]` = 8.
pub const NN500_MODEL: u32 = 8;

/// F718: does an ECM burst of `kind` destroy a missile of `model`?
pub fn ecm_destroys(model: u32, naval: bool) -> bool {
    match ECM_BRANCH.get(model as usize) {
        Some(0) => true,
        Some(2) => naval,
        Some(_) => false,
        None => true,
    }
}

/// [`ecm_destroys`] with `enhancement-8` (`enhancement.ecm_nn500`): the
/// NN500's `DATA_000029[8] = 1` immunity becomes the table's branch 0, so
/// either E.C.M. destroys it. All other models keep the source branch.
pub fn ecm_destroys_with(model: u32, naval: bool, nn500: bool) -> bool {
    if nn500 && model == NN500_MODEL {
        return true;
    }
    ecm_destroys(model, naval)
}

/// F1057's retarget range: only models 6..=11 (`cmp esi,6; jc`, `cmp esi,0xb;
/// ja`). A chaff cloud can pull these away from the player.
pub fn retargetable(model: u32) -> bool {
    (6..=11).contains(&model)
}

/// F941's AI state for a launched code.
pub fn ai_state(code: u8) -> u8 {
    if is_mine(code) {
        if model(code) == Some(4) {
            AI_MINE_MODEL_4
        } else {
            AI_MINE
        }
    } else {
        AI_MISSILE
    }
}

/// The fields a launched missile takes from its model's statistics. Built by
/// `ffe-rs` from the cache; [`Spec::ballistic`] is the no-statistics fallback.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Spec {
    pub model: u32,
    /// F720's damage word.
    pub damage: u32,
    /// F599's z maximum, mm/s².
    pub forward: f64,
    /// F599's z minimum, mm/s² (negative).
    pub reverse: f64,
    /// F599's x/y limit, mm/s² (positive).
    pub side: f64,
    /// F586/F585's maximum turn rate, rad/s.
    pub turn: f64,
    /// F928/F1533's collision radius (`+0x140`), mm.
    pub radius: i64,
}

impl Spec {
    /// Build from a model's first two statistics words (`+0x0` forward,
    /// `+0x2` reverse) and its F1533 collision radius.
    pub fn from_words(model: u32, forward_word: i16, reverse_word: i16, radius: i64) -> Self {
        let side_word = i32::from(reverse_word).unsigned_abs().min(32_767) as f64;
        Self {
            model,
            damage: damage(model),
            forward: f64::from(forward_word.max(0)) * ACCEL_PER_WORD,
            reverse: f64::from(reverse_word.min(0)) * ACCEL_PER_WORD,
            side: side_word * ACCEL_PER_WORD,
            turn: (side_word / 8.0) * TURN_PER_WORD,
            radius,
        }
    }

    /// A missile with no engine statistics (models 4 and 5): it keeps the
    /// launch impulse and flies straight.
    pub fn ballistic(model: u32, radius: i64) -> Self {
        Self::from_words(model, 0, 0, radius)
    }
}

/// An object the missile can see, in the system frame (millimetres, mm/s).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub id: u32,
    pub position: [i64; 3],
    pub velocity: [f64; 3],
    pub radius: i64,
}

/// What a missile did in a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// F720/F721: the missile reached its target or a contact. The caller
    /// applies F953's shields→hull→death.
    Hit { target: u32, damage: u32 },
    /// F758's 60-update timeout (state 0x13) or F767's distance-class
    /// removal (state 0x16): F719's explosion, no damage.
    Expired { target: Option<u32> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum State {
    Flying,
    Spent,
}

/// Runtime-only collision switches for one missile step. These are never
/// serialized: they are read from `config.cfg` at the frame boundary, so
/// toggling an enhancement cannot change a saved missile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickOptions {
    /// `enhancement-2`: ignore the launcher (`owner`) in F718's **swept (ray)**
    /// contact test. The source's F575 skips the missile's parent object at
    /// `+0x56` (`plstate.txt`); this port has no separate parent field, so the
    /// launcher is `owner`, and the start-of-step contact test always skips it
    /// (`contact_point`). Only the ray added by `enhancement-6` can still meet
    /// the launcher, so this switch guards that ray.
    pub ignore_launcher: bool,
    /// `enhancement-6`: test the path the missile swept over this step rather
    /// than only its position at the start of the step.
    pub swept: bool,
}

impl TickOptions {
    /// The port's pre-enhancement missile step: skip the launcher, test the
    /// start-of-step position. `ffe-rs` callers that have no config use this;
    /// the live game passes the two toggles from `config.cfg`.
    pub const PORT_DEFAULT: Self = Self {
        ignore_launcher: true,
        swept: false,
    };
}

/// A launched missile object (`control 0xb`, AI state 0x13/0x14/0x16).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Missile {
    pub spec: Spec,
    pub ai_state: u8,
    /// F941's object id (`+0x86`). `DATA_008877` (the missile view's object,
    /// `player+0xef`) holds this id, and F1057's retarget keeps the missile's
    /// `+0xfe` separate from it. The field is assigned by
    /// `ffe-rs::missile::MissileField` (the core has no object slots).
    pub id: u32,
    /// The launcher's object id (`+0x100`).
    pub owner: u32,
    /// The combat target (`+0xfe`), `DATA_008875` at launch.
    pub target: Option<u32>,
    pub position: [i64; 3],
    pub velocity: [f64; 3],
    /// Right, up, forward.
    pub axes: [[f64; 3]; 3],
    pub age: f64,
    /// Seconds since the last F758 guidance update.
    pub guidance: f64,
    /// F758's `+0x9e` update count.
    pub updates: u32,
    /// F599's body-frame thrust words, applied by F717 every frame.
    pub thrust: [f64; 3],
    pub state: State,
}

impl Missile {
    /// F941/F1514: allocate at the launcher, with the separation impulse. The
    /// `pylon` is 1-based; odd pylons get the `+right` offset, even ones the
    /// negative, as F941's `test byte [ebp+0xc], 1` does. A launcher with a
    /// collision radius of at least `0x4650` doubles the lateral word.
    #[allow(clippy::too_many_arguments)]
    pub fn launch(
        spec: Spec,
        ai_state: u8,
        owner: u32,
        target: Option<u32>,
        position: [i64; 3],
        axes: [[f64; 3]; 3],
        pylon: usize,
        launcher_radius: i64,
    ) -> Self {
        let radius = if launcher_radius > 0 {
            launcher_radius as f64
        } else {
            spec.radius as f64
        };
        let side = if launcher_radius >= 0x4650 {
            16_384.0
        } else {
            8_192.0
        };
        let sign = if pylon % 2 == 1 { 1.0 } else { -1.0 };
        let impulse = [
            -axes[1][0] * radius + axes[0][0] * side * sign,
            -axes[1][1] * radius + axes[0][1] * side * sign,
            -axes[1][2] * radius + axes[0][2] * side * sign,
        ];
        let position = [
            position[0] + impulse[0].round() as i64,
            position[1] + impulse[1].round() as i64,
            position[2] + impulse[2].round() as i64,
        ];
        Self {
            spec,
            ai_state,
            id: 0,
            owner,
            target,
            position,
            velocity: impulse,
            axes,
            age: 0.0,
            guidance: 0.0,
            updates: 0,
            // F941 leaves the missile accelerating along its nose until F758's
            // first guidance update (the executed original gains 4 velocity
            // units a frame from the launch).
            thrust: [0.0, 0.0, spec.forward / ACCEL_PER_WORD],
            state: State::Flying,
        }
    }

    pub fn alive(&self) -> bool {
        self.state == State::Flying
    }

    /// One F717 physics step over `dt` world seconds, with F758's guidance
    /// every [`GUIDANCE_INTERVAL`]. Returns an [`Event`] when the missile hits
    /// or expires. This is [`Self::tick_with`] with the source's start-of-step
    /// contact test (which skips the launcher) and no enhancement switches.
    pub fn tick(&mut self, dt: f64, targets: &[Target]) -> Option<Event> {
        self.tick_with(dt, targets, TickOptions::default())
    }

    /// [`Self::tick`] with the `enhancement-2`/`enhancement-6` switches.
    pub fn tick_with(
        &mut self,
        dt: f64,
        targets: &[Target],
        options: TickOptions,
    ) -> Option<Event> {
        if !self.alive() || dt <= 0.0 {
            return None;
        }
        // F941 gives a launched object one of DATA_006255's rows: 0x13 is
        // F758/F717, 0x14 is F716's contact-only mine and 0x16 is F767/F713's
        // seeking model-4 mine. The row decides whether guidance, thrust and a
        // lifetime run at all.
        let behaviour = behaviour(self.ai_state);
        if behaviour == Behaviour::Object {
            return None;
        }
        self.age += dt;
        self.guidance += dt;
        let from = self.position;
        // F718/F721: the contact test runs every physics step, before the fuse.
        // The swept option replaces it with the segment test after F717 moves
        // the missile.
        if !options.swept {
            if let Some(target) = self.contact_point(targets) {
                return Some(self.hit(target));
            }
        }
        let target = self
            .target
            .filter(|id| *id != self.owner)
            .and_then(|id| targets.iter().find(|candidate| candidate.id == id));
        // F767 removes the seeking mine beyond distance class 0xd. The source
        // uses `+0x88` (the object's distance from the camera); the port's
        // nearest stand-in is the player target of the tick.
        if behaviour == Behaviour::Seeking {
            if let Some(player) = targets.iter().find(|t| t.id == PLAYER_TARGET) {
                if distance(self.position, player.position) > SEEKING_RANGE_MM {
                    self.state = State::Spent;
                    return Some(Event::Expired {
                        target: self.target,
                    });
                }
            }
        }
        if behaviour != Behaviour::Mine && self.guidance >= GUIDANCE_INTERVAL {
            self.guidance -= GUIDANCE_INTERVAL;
            self.updates += 1;
            // F758: `++w[+0x9e] > 0x3c` explodes the missile. Only state 0x13
            // has F758 as its controller; F767's row has no lifetime.
            if behaviour == Behaviour::Guided && self.updates > LIFETIME_UPDATES {
                self.state = State::Spent;
                return Some(Event::Expired {
                    target: self.target,
                });
            }
            if let Some(target) = target {
                if behaviour == Behaviour::Guided
                    && distance(self.position, target.position) <= PROXIMITY_MM
                {
                    return Some(self.hit(target.id));
                }
                // F596/F597: aim at the target's lead position.
                let to = sub(target.position, self.position);
                let distance = length(to);
                let speed = length(self.velocity).max(1.0);
                let lead = (distance / speed).clamp(0.0, 2.0);
                let aim = normalise(add(to, scale(target.velocity, lead)));
                // F599 in the missile frame, with F758's local-z bias.
                let local = [
                    dot(aim, self.axes[0]),
                    dot(aim, self.axes[1]),
                    dot(aim, self.axes[2]) + FORWARD_BIAS,
                ];
                // F586 turns the missile by its lateral thrust words.
                let turn = self.spec.turn * GUIDANCE_INTERVAL;
                let forward = turn_toward(self.axes[2], aim, turn);
                self.axes = basis(self.axes[0], forward);
                let limit = |value: f64, low: f64, high: f64| value.clamp(low, high);
                let side = self.spec.side / ACCEL_PER_WORD;
                self.thrust = [
                    limit(local[0] * 32_768.0, -side, side),
                    limit(local[1] * 32_768.0, -side, side),
                    limit(
                        local[2] * 32_768.0,
                        self.spec.reverse / ACCEL_PER_WORD,
                        self.spec.forward / ACCEL_PER_WORD,
                    ),
                ];
            }
        }
        // F717/F601: integrate the last F599 thrust vector every frame. F716's
        // mine row has no thrust source, so its launch impulse carries it.
        let local_accel = if behaviour == Behaviour::Mine {
            [0.0; 3]
        } else {
            self.thrust.map(|word| word * ACCEL_PER_WORD)
        };
        for (value, axis) in self.velocity.iter_mut().zip(self.axes) {
            *value += dot(local_accel, axis) * dt;
        }
        for axis in 0..3 {
            self.position[axis] += (self.velocity[axis] * dt).round() as i64;
        }
        // Enhancement-6: the missile's path over this step, not just where it
        // started. The trajectory is unchanged; only the contact test differs.
        if options.swept {
            if let Some(target) = self.contact_swept(from, self.position, targets, options) {
                return Some(self.hit(target));
            }
        }
        None
    }

    /// F569/F575's point contact. The launcher is always skipped: F941 copies
    /// the launcher's record and F575 skips the object that is the missile's
    /// parent (`+0x56`), which the port keeps as `owner`. Without the skip a
    /// player missile spawns inside its own ship's collision sphere and
    /// detonates on the launcher on the launch step.
    fn contact_point(&self, targets: &[Target]) -> Option<u32> {
        for target in targets {
            if target.id == self.owner {
                continue;
            }
            if distance(self.position, target.position) <= contact(target.radius, self.spec.radius)
            {
                return Some(target.id);
            }
        }
        None
    }

    /// F569/F575's contact along the segment the missile travelled, so a fast
    /// missile cannot tunnel through a target between two point tests.
    /// `enhancement-2` keeps the launcher out of this ray too.
    fn contact_swept(
        &self,
        from: [i64; 3],
        to: [i64; 3],
        targets: &[Target],
        options: TickOptions,
    ) -> Option<u32> {
        for target in targets {
            if options.ignore_launcher && target.id == self.owner {
                continue;
            }
            let reach = contact(target.radius, self.spec.radius);
            if segment_distance(from, to, target.position) <= reach {
                return Some(target.id);
            }
        }
        None
    }

    fn hit(&mut self, target: u32) -> Event {
        self.state = State::Spent;
        Event::Hit {
            target,
            damage: self.spec.damage,
        }
    }
}

/// A launched missile's spec from a model's statistics: `forward`/`reverse` are
/// the two words the original's object set-up copies, `radius` the F1533
/// collision radius.
pub fn spec_from_words(model: u32, forward: i16, reverse: i16, radius: i64) -> Spec {
    Spec::from_words(model, forward, reverse, radius)
}

fn contact(a: i64, b: i64) -> f64 {
    (a + b).max(0) as f64
}

fn sub(a: [i64; 3], b: [i64; 3]) -> [f64; 3] {
    [
        (a[0] - b[0]) as f64,
        (a[1] - b[1]) as f64,
        (a[2] - b[2]) as f64,
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    a.map(|value| value * factor)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn normalise(a: [f64; 3]) -> [f64; 3] {
    let length = length(a);
    if length < 1e-9 {
        [0.0, 0.0, 1.0]
    } else {
        scale(a, 1.0 / length)
    }
}

fn distance(a: [i64; 3], b: [i64; 3]) -> f64 {
    length(sub(a, b))
}

/// The closest distance from `point` to the segment `from`..`to`, in the same
/// units. Degenerate segments fall back to the point distance.
fn segment_distance(from: [i64; 3], to: [i64; 3], point: [i64; 3]) -> f64 {
    let segment = sub(to, from);
    let offset = sub(point, from);
    let length_squared = dot(segment, segment);
    if length_squared < 1e-12 {
        return distance(from, point);
    }
    let t = (dot(offset, segment) / length_squared).clamp(0.0, 1.0);
    let closest = [
        from[0] as f64 + segment[0] * t,
        from[1] as f64 + segment[1] * t,
        from[2] as f64 + segment[2] * t,
    ];
    let gap = [
        point[0] as f64 - closest[0],
        point[1] as f64 - closest[1],
        point[2] as f64 - closest[2],
    ];
    length(gap)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// An upright right/up/forward basis for a new forward axis, keeping the
/// previous right axis' side when it is not degenerate.
fn basis(previous_right: [f64; 3], forward: [f64; 3]) -> [[f64; 3]; 3] {
    let mut right = cross([0.0, 1.0, 0.0], forward);
    if length(right) < 1e-6 {
        right = previous_right;
    }
    let right = normalise(right);
    let up = normalise(cross(forward, right));
    [right, up, normalise(forward)]
}

fn turn_toward(from: [f64; 3], to: [f64; 3], amount: f64) -> [f64; 3] {
    let angle = dot(from, to).clamp(-1.0, 1.0).acos();
    if angle < 1e-6 {
        return from;
    }
    let t = (amount / angle).clamp(0.0, 1.0);
    let sin = angle.sin();
    if sin.abs() < 1e-9 {
        return to;
    }
    let a = ((1.0 - t) * angle).sin() / sin;
    let b = (t * angle).sin() / sin;
    normalise([
        from[0] * a + to[0] * b,
        from[1] * a + to[1] * b,
        from[2] * a + to[2] * b,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> Spec {
        // Model 6: forward 32700, reverse -6540, radius 2100.
        Spec::from_words(6, 32700, -6540, 2100)
    }

    fn axes() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn target(position: [i64; 3]) -> Target {
        Target {
            id: 7,
            position,
            velocity: [0.0; 3],
            radius: 1_000,
        }
    }

    #[test]
    fn the_type_table_and_damage_words_are_the_originals() {
        assert_eq!(model(0x82), Some(6));
        assert_eq!(model(0x80), Some(4));
        assert_eq!(model(0x89), Some(13));
        assert_eq!(model(0x8a), None);
        assert_eq!(damage(6), 200);
        assert_eq!(damage(4), 10);
        assert_eq!(damage(13), 111);
        assert!(!is_mine(0x82));
        assert!(is_mine(0xc0));
        assert_eq!(ai_state(0x82), AI_MISSILE);
        assert_eq!(ai_state(0xc0), AI_MINE_MODEL_4);
        assert_eq!(ai_state(0xc1), AI_MINE);
    }

    #[test]
    fn ecm_uses_f718s_model_table_and_the_naval_extra_bit() {
        // F718/DATA_000029: 0 destroys, 1 immune, 2 needs DATA_008835 bit 3.
        assert_eq!(ECM_BRANCH.len(), 14);
        for model in [0, 1, 2, 3, 5, 6] {
            assert!(ecm_destroys(model, false), "model {model}");
            assert!(ecm_destroys(model, true), "model {model}");
        }
        for model in [4, 8, 11, 12, 13] {
            assert!(!ecm_destroys(model, false), "model {model}");
            assert!(!ecm_destroys(model, true), "model {model}");
        }
        for model in [7, 9, 10] {
            assert!(!ecm_destroys(model, false), "model {model}");
            assert!(ecm_destroys(model, true), "model {model}");
        }
        // The source's `ja` past 13 destroys, though no missile uses it.
        assert!(ecm_destroys(14, false));
        // The new game's 0x82 is model 6, which the standard ECM defeats.
        assert!(ecm_destroys(model(0x82).unwrap(), false));
    }

    #[test]
    fn f1057_retargets_only_models_six_to_eleven() {
        assert!(!retargetable(4));
        assert!(!retargetable(5));
        for model in 6..=11 {
            assert!(retargetable(model), "model {model}");
        }
        assert!(!retargetable(12));
        assert!(!retargetable(13));
    }

    #[test]
    fn the_missile_rows_are_data_006255s_0x13_to_0x1b() {
        assert_eq!(AI_ROWS.len(), 9);
        for (offset, row) in AI_ROWS.iter().enumerate() {
            assert_eq!(usize::from(row.code), 0x13 + offset);
            assert_eq!(ai_row(row.code), Some(row));
        }
        let row = |code: u8| ai_row(code).copied().unwrap();
        assert_eq!(
            (row(0x13).control, row(0x13).movement),
            (Some(758), Some(717))
        );
        assert_eq!((row(0x14).control, row(0x14).movement), (None, Some(716)));
        assert_eq!(
            (row(0x16).control, row(0x16).movement),
            (Some(767), Some(713))
        );
        assert_eq!(
            (row(0x17).control, row(0x17).movement),
            (Some(732), Some(714))
        );
        assert_eq!(
            (row(0x18).control, row(0x18).movement),
            (Some(766), Some(710))
        );
        assert_eq!(
            (row(0x19).control, row(0x19).movement),
            (Some(765), Some(711))
        );
        assert_eq!((row(0x1a).control, row(0x1a).movement), (None, Some(709)));
        assert_eq!(
            (row(0x1b).control, row(0x1b).movement),
            (Some(769), Some(715))
        );
        assert_eq!(behaviour(AI_MISSILE), Behaviour::Guided);
        assert_eq!(behaviour(AI_MINE), Behaviour::Mine);
        assert_eq!(behaviour(AI_MINE_MODEL_4), Behaviour::Seeking);
        assert_eq!(behaviour(0x18), Behaviour::Object);
    }

    #[test]
    fn a_mine_keeps_its_launch_impulse_and_never_expires() {
        // F941's 0xc1 code is state 0x14: no control slot, so no F758 guidance
        // and no 60-update lifetime.
        let mut mine = Missile::launch(spec(), AI_MINE, 1, Some(7), [0; 3], axes(), 1, 0);
        mine.velocity = [0.0, 0.0, 1_000.0];
        let far = target([0, 0, 10_000_000]);
        for _ in 0..200 {
            assert_eq!(mine.tick(0.02, &[far]), None);
        }
        assert!(mine.alive());
        assert_eq!(mine.updates, 0);
        // F716 has no thrust source: the launch impulse alone carries it.
        assert!((mine.velocity[2] - 1_000.0).abs() < 1e-9);
        assert!(mine.position[2] > 3_000);
    }

    #[test]
    fn the_model_4_mine_has_no_lifetime_and_is_removed_at_67_km() {
        // F941's 0xc0 code is state 0x16: F767/F713, whose only exit is the
        // distance-class removal (no F758's 60 updates).
        let mut mine = Missile::launch(spec(), AI_MINE_MODEL_4, 1, None, [0; 3], axes(), 1, 0);
        mine.velocity = [0.0; 3];
        mine.thrust = [0.0; 3];
        // 50 km from the player: inside F767's class 0xd, outside the contact box.
        mine.position = [0, 0, 50_000_000];
        let player = Target {
            id: PLAYER_TARGET,
            position: [0, 0, 0],
            velocity: [0.0; 3],
            radius: 17_600,
        };
        for _ in 0..80 {
            assert_eq!(mine.tick(0.4, &[player]), None);
        }
        assert!(mine.alive());
        assert!(mine.updates > LIFETIME_UPDATES);
        // Beyond 2^26 mm from the player, F767 calls F925 and the object goes.
        mine.position = [0, 0, (SEEKING_RANGE_MM as i64) + 1];
        assert_eq!(
            mine.tick(0.02, &[player]),
            Some(Event::Expired { target: None })
        );
    }

    #[test]
    fn launch_clears_the_pylon_and_separates_below_the_ship() {
        let missile = Missile::launch(
            spec(),
            AI_MISSILE,
            1,
            Some(7),
            [100, 200, 300],
            axes(),
            1,
            5200,
        );
        // Down by the launcher radius, right by 0x2000 for an odd pylon.
        assert_eq!(missile.position, [100 + 0x2000, 200 - 5200, 300]);
        assert_eq!(missile.velocity, [8192.0, -5200.0, 0.0]);
        assert_eq!(missile.target, Some(7));
        assert_eq!(missile.spec.damage, 200);
        let even = Missile::launch(spec(), AI_MISSILE, 1, None, [0; 3], axes(), 2, 5200);
        assert_eq!(even.position, [-0x2000, -5200, 0]);
    }

    #[test]
    fn tracking_turns_toward_the_target_and_gains_speed() {
        let mut missile = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 2100);
        // Aim 20 degrees off the forward axis, 300 m away.
        let target = Target {
            position: [100_000, 0, 300_000],
            velocity: [0.0; 3],
            ..target([0; 3])
        };
        let start = missile.axes[2];
        for _ in 0..25 {
            missile.tick(0.02, &[target]);
        }
        assert!(dot(missile.axes[2], start) < 0.999, "did not turn");
        assert!(length(missile.velocity) > 1_000.0, "did not accelerate");
        assert!(missile.alive());
    }

    #[test]
    fn the_proximity_fuse_hits_within_131_metres_and_damage_is_the_table_word() {
        let mut missile = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 2100);
        let event = missile.tick(GUIDANCE_INTERVAL, &[target([0, 0, 130_000])]);
        assert_eq!(
            event,
            Some(Event::Hit {
                target: 7,
                damage: 200
            })
        );
        assert!(!missile.alive());
    }

    #[test]
    fn contact_with_any_object_damages_it() {
        let mut missile = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 2100);
        let other = Target {
            id: 9,
            position: [0, 0, 3_000],
            velocity: [0.0; 3],
            radius: 10_000,
        };
        assert_eq!(
            missile.tick(0.02, &[other]),
            Some(Event::Hit {
                target: 9,
                damage: 200
            })
        );
    }

    #[test]
    fn a_missile_expires_after_sixty_event_six_updates() {
        let mut missile = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 2100);
        // A distant target keeps the fuse from firing.
        let far = target([0, 0, 10_000_000]);
        for _ in 0..60 {
            assert_eq!(missile.tick(GUIDANCE_INTERVAL, &[far]), None);
        }
        assert!(missile.alive());
        assert_eq!(
            missile.tick(GUIDANCE_INTERVAL, &[far]),
            Some(Event::Expired { target: Some(7) })
        );
        assert!(!missile.alive());
        assert_eq!(missile.tick(GUIDANCE_INTERVAL, &[far]), None);
    }

    #[test]
    fn a_ballistic_missile_keeps_its_launch_impulse() {
        let mut missile = Missile::launch(
            Spec::ballistic(5, 1200),
            AI_MISSILE,
            1,
            Some(7),
            [0; 3],
            axes(),
            1,
            5200,
        );
        let velocity = missile.velocity;
        let far = target([0, 0, 10_000_000]);
        missile.tick(0.02, &[far]);
        assert!((missile.velocity[1] - velocity[1]).abs() < 1e-9);
        assert!((missile.velocity[0] - velocity[0]).abs() < 1e-9);
    }

    #[test]
    fn the_launch_enhancement_guards_the_swept_ray_not_the_point_test() {
        // F941 places the missile 9.1 m from the launcher's centre; a 20 m
        // launcher plus the missile's 2.1 m radius reaches it on the first
        // step. F575 always skips the missile's parent object, so F718's point
        // contact test never meets the launcher and the missile flies.
        let launcher = Target {
            id: 1,
            position: [0; 3],
            velocity: [0.0; 3],
            radius: 20_000,
        };
        let mut point = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 5200);
        assert_eq!(
            point.tick_with(0.02, &[launcher], TickOptions::default()),
            None
        );
        assert!(point.alive());

        // enhancement-6's ray starts at the launch point, so it does meet the
        // launcher; enhancement-2 removes the launcher from that ray.
        let mut ray = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 5200);
        assert_eq!(
            ray.tick_with(
                0.02,
                &[launcher],
                TickOptions {
                    ignore_launcher: false,
                    swept: true,
                }
            ),
            Some(Event::Hit {
                target: 1,
                damage: 200
            })
        );
        let mut guarded = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 5200);
        assert_eq!(
            guarded.tick_with(
                0.02,
                &[launcher],
                TickOptions {
                    ignore_launcher: true,
                    swept: true,
                }
            ),
            None
        );
        assert!(guarded.alive());
    }

    #[test]
    fn the_swept_enhancement_catches_a_target_between_two_positions() {
        let thin = Target {
            radius: 100,
            ..target([0, 0, 10_000])
        };
        let fast = |position: [i64; 3], velocity: f64| {
            let mut missile =
                Missile::launch(spec(), AI_MISSILE, 1, Some(7), position, axes(), 1, 2100);
            missile.position = position;
            missile.velocity = [0.0, 0.0, velocity];
            missile.thrust = [0.0; 3];
            missile
        };
        // The step covers 0..20 m; a 100 mm target at 10 m is between the two
        // point tests (the 131 m fuse only runs at a guidance update).
        let mut point = fast([0; 3], 1_000_000.0);
        assert_eq!(point.tick_with(0.02, &[thin], TickOptions::default()), None);
        assert!(point.alive());
        let mut swept = fast([0; 3], 1_000_000.0);
        assert_eq!(
            swept.tick_with(
                0.02,
                &[thin],
                TickOptions {
                    ignore_launcher: false,
                    swept: true,
                }
            ),
            Some(Event::Hit {
                target: 7,
                damage: 200
            })
        );
    }

    #[test]
    fn a_swept_step_that_hits_nothing_leaves_the_saved_missile_identical() {
        let base = Missile::launch(spec(), AI_MISSILE, 1, Some(7), [0; 3], axes(), 1, 2100);
        let far = target([0, 0, 10_000_000]);
        let mut point = base;
        let mut swept = base;
        assert_eq!(point.tick_with(0.02, &[far], TickOptions::default()), None);
        assert_eq!(
            swept.tick_with(
                0.02,
                &[far],
                TickOptions {
                    ignore_launcher: false,
                    swept: true,
                }
            ),
            None
        );
        // No new field and no changed trajectory: the serialized missile is
        // the same either way.
        assert_eq!(point, swept);
    }

    #[test]
    fn the_ecm_enhancement_adds_the_nn500_to_the_destroyed_models() {
        assert_eq!(NN500_MODEL, 8);
        assert!(!ecm_destroys(8, false));
        assert!(!ecm_destroys(8, true));
        assert!(ecm_destroys_with(8, false, true));
        assert!(ecm_destroys_with(8, true, true));
        // The rest of DATA_000029 is untouched.
        assert!(!ecm_destroys_with(4, false, true));
        assert!(!ecm_destroys_with(7, false, true));
        assert!(ecm_destroys_with(7, true, true));
        assert!(ecm_destroys_with(6, false, true));
        assert!(!ecm_destroys_with(8, false, false));
    }
}
