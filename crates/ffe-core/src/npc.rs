//! NPC ships in system flight: the original's ship-type tables, its traffic
//! spawn counts, and the flight/combat AI states it runs them through.
//!
//! Source: JJFFE 2.8a7 `ffe.asm`/`ffedat.asm` (the DOS release's `FIRSTENC.EXE`
//! has the same tables and routines; the one behavioural difference found is
//! noted at F724 below).
//!
//! - **F635** (`ffe.asm:43576`) picks a ship model: `DATA_006199` is a table of
//!   nineteen `(count, model-list)` records and `RND(count)` selects an entry.
//!   Types 0 and 1 are shifted by `2 * DATA_008996` (the system allegiance
//!   index), so a trader in an Imperial system comes from the Imperial shipyard
//!   list (type 1 + 2*2 = 5).
//! - **F688** (`ffe.asm:45818`) is the system traffic spawner. It reads the
//!   four bytes and two words F869 returns from the galaxy record (`+5..+9`,
//!   `+46`, `+50`) and spawns traders (F689, type 1), Thargoids (F690, type
//!   0x11), ships in station bays (F691), auxiliary bulk carriers (F699, type
//!   0xf) and patrol groups (F698). `DATA_008997` is **not** a cache ship-data
//!   table: F617/F618 build it at system creation by walking the objects with
//!   `+0x14c & 0x10` (the stations and starports) and copying their object
//!   slots into the 18-record `DATA_008999` starport array. F700 draws one of
//!   those records, so the F691 count is `RND(4 * places)` (capped 25) and the
//!   port derives it from the generated system's place bodies. See §2.
//! - **The Thargoid count is `RND(F35(0x16))`** (`ffe.asm:45858`, a
//!   global-variable read; F34/F35 are `ffe.asm:3507`/`3524`). `F35` is a
//!   limit, not the exact count: F688 passes it to F922 like the trader counts.
//!   It is not a system field: the only setter is the mission-script bytecode's
//!   variable assignment (F37, `ffe.asm:3646`, calling F34), and the 71-dword
//!   block is saved with the commander (`F65`, `ffe.asm:6898`). No ported system
//!   loads a mission script, so the normal limit is zero; [`System::thargoids`]
//!   carries it when a mission hook supplies one. F690 itself is F689 with the
//!   type byte 0x11 and the model list `DATA_006197`; it has no military, system
//!   or random (beyond the count draw) gate.
//! - **F694/F695/F696/F693** (F698's four patrol choices) are the pirate
//!   spawns: a lone type-0xa ship in state 5, a type-0xb leader with two
//!   formation escorts in state 0xb (+0x102..+0x106 hold their offsets from the
//!   leader), and two further variants. F697 fits the pirate equipment and
//!   rolls `+0x11a`, the bounty, from `DATA_006268`.
//! - **F691** (`ffe.asm:46113`) is the station-bay spawner: it creates a
//!   type-1 trader (F772), draws a random place with F700, asks F650 for a bay
//!   and, on success, copies the starport's position/orientation (the `0x20`
//!   branch), adds the bay offset, marks the ship state 0x24 with F747's object
//!   time (without the `+2` days F747 adds for an arriving ship) and the
//!   station in `+0xfe`. On a full station it calls F940, which removes the
//!   ship. The port spawns such a ship as `State::Docked` with a pending bay;
//!   `Navigator` asks the station's traffic record for the bay when it installs
//!   the ships.
//! - **F699** (`ffe.asm:46729`) is F701 (a station must exist), F772(0xf), F702
//!   on the `0x5000/0x18` orbit, state 0x0d and `+0x56` = the station's object
//!   slot. Table row 0x0d has no control or movement routine (only F790's damage
//!   response), so these ships hold; the source gives them no route, station
//!   target or dock request. Its type-0xf model list is `DATA_006195`
//!   (`Lynx Bulk Carrier`/`Long-range Cruiser`), not model 0x0f (the
//!   `Interplanetary Shuttle`, which is table record 0xd).
//! - **F702/F1507** (`ffe.asm:46868`, `89917`) place a spawned ship at a random
//!   phase on an orbit of the raw ffp word `0x5000/0x33`.
//! - **F773** (`ffe.asm:50526`) creates the ship: type byte `+0x118`, equipment,
//!   drive, hull (`+0xe4 = mass * 4`) and shields (`+0xe2 = random * 16`).
//! - **F953/F942** (`ffe.asm:71190`, `70381`) apply damage and destroy the ship;
//!   F1536 (`ffe.asm:91274`) draws model 0x9d (157) at a ship whose shields
//!   absorbed a hit.
//! - **The AI table** is `DATA_006258` (`ffedat.asm:9862`): 0x25 states, four
//!   routine slots each, dispatched by F706/F704/F778/F780 on object `+0xff`.
//!   [`AI_STATES`] is that table. The routines the port models are the common
//!   movement F725/F727 (F1512 aims, F585 applies the limited turn, F587
//!   computes the rates), the arrival test F753, the pirate attack states 5/0xa/
//!   0xb (F759/F751/F757) and the jump-out states 0x1f/0x21 (F787/F747).
//! - **F931** (`ffe.asm:69550`) is the AI's laser too: F724 (state 1) calls it
//!   from the ship's front mount, and F930/F953 give the damage the player
//!   takes. JJFFE patched that call out ("Laser invulnerability", `ffe.asm:
//!   47856`) and only sets the fired flag; the port follows DOS FFE and fires.
//! - **F674/F675/F680** (`ffe.asm:45338`, `45374`, `45536`) persist the ships
//!   that survive a hyperspace jump. F675 keeps a pirate (class 0xfb) only when
//!   it is within 0x10 of the player and keeps escape pods (model 0x9a); F680
//!   restores them and then F688 spawns the new system's traffic.
//! - **F927** (`ffe.asm:69170`) allocates a physics slot from 0x72 down, never
//!   below 0x14, recycling the highest explosion/debris slot when full.
//! - **F1035/F1036** (`ffe.asm:76306`, `76393`) are *surface* spawns (they test
//!   `DATA_008867`, a planet altitude, and spawn at state 0x19 near a building
//!   model 0x72). They are not the orbital traffic and are not ported here.
//!
//! **What this module models and what it leaves out.** The model and type tables,
//! the F688 counts and the spawn roles are the original's. The flight AI is a
//! state machine over the original's own states (approach/attack/pass/hold/flee/
//! dead) with its turn limiter, braking rule and laser gates; it is not a
//! transcription of every state routine. The police response is a reconstruction
//! from `DATA_008646/8647` (the wanted/alert words) because the original's
//! police spawn path is the same F688 count as the traffic. Docking, station
//! bays, missiles and ECM remain. Thargoid traffic exists as the F690 rule and
//! role, but the source's count variable is mission-script driven (see above),
//! so no ordinary system spawns any. Placement uses the original's rule (a
//! random phase on an orbit around an anchor) with a radius in the port's unit
//! system that keeps the ships reachable. See `docs/notes/npc.md`.

use crate::galaxy::Details;
use crate::laser::{self, Damage, DamageState};
use crate::model_animation::TICS_PER_FRAME;
use crate::object_record::ObjectRecord;
use serde::{Deserialize, Serialize};

/// The original's ship-type table, `DATA_006199`/`DATA_006180..6198`: nineteen
/// records of a count and a model list, in table order.
pub const SHIP_TYPES: [&[u32]; 19] = [
    // 0x0: DATA_006184, nine entries
    &[0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x24, 0x22, 0x2a, 0x28],
    // 0x1: DATA_006185, eighteen (traders)
    &[
        0x1c, 0x1d, 0x1e, 0x22, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2c, 0x35, 0x29, 0x2f, 0x39, 0x3b,
        0x3d, 0x41, 0x44,
    ],
    // 0x2: DATA_006182, ten (Imperial military)
    &[0x19, 0x13, 0x1e, 0x13, 0x22, 0x20, 0x20, 0x22, 0x38, 0x38],
    // 0x3: DATA_006183, eight (Imperial shipyard)
    &[0x17, 0x19, 0x38, 0x38, 0x3a, 0x3a, 0x41, 0x41],
    // 0x4: DATA_006180, twelve (Federal military)
    &[
        0x17, 0x18, 0x1f, 0x14, 0x21, 0x15, 0x16, 0x2b, 0x1e, 0x22, 0x23, 0x27,
    ],
    // 0x5: DATA_006181, sixteen (Federal shipyard)
    &[
        0x1d, 0x1e, 0x22, 0x24, 0x26, 0x28, 0x27, 0x37, 0x35, 0x3c, 0x2d, 0x36, 0x43, 0x2c, 0x3e,
        0x44,
    ],
    // 0x6: DATA_006186, twenty-three
    &[
        0x1f, 0x1a, 0x1d, 0x14, 0x1d, 0x1e, 0x14, 0x17, 0x18, 0x1f, 0x2b, 0x22, 0x23, 0x2a, 0x22,
        0x24, 0x23, 0x2a, 0x30, 0x30, 0x28, 0x29, 0x29,
    ],
    // 0x7: DATA_006187, twenty-seven
    &[
        0x1d, 0x1f, 0x24, 0x1f, 0x2a, 0x2c, 0x2a, 0x26, 0x27, 0x28, 0x2c, 0x29, 0x2f, 0x40, 0x2f,
        0x29, 0x2d, 0x39, 0x2d, 0x3b, 0x43, 0x3d, 0x41, 0x43, 0x44, 0x41, 0x44,
    ],
    // 0x8: DATA_006188, eleven (Thargoids)
    &[
        0x31, 0x31, 0x32, 0x32, 0x33, 0x33, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f,
    ],
    // 0x9: DATA_006189, twelve (Thargoids and a transporter)
    &[
        0x45, 0x31, 0x31, 0x32, 0x32, 0x33, 0x33, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f,
    ],
    // 0xa: DATA_006190, fifteen (single pirates)
    &[
        0x1c, 0x2b, 0x22, 0x1d, 0x24, 0x26, 0x25, 0x2a, 0x30, 0x28, 0x29, 0x2f, 0x27, 0x38, 0x41,
    ],
    // 0xb: DATA_006191, twenty-four (grouped pirates)
    &[
        0x13, 0x14, 0x1f, 0x21, 0x20, 0x15, 0x16, 0x17, 0x18, 0x19, 0x21, 0x1b, 0x2b, 0x1e, 0x1a,
        0x22, 0x23, 0x23, 0x2a, 0x2a, 0x1c, 0x1d, 0x24, 0x30,
    ],
    // 0xc: DATA_006192, ten
    &[0x20, 0x1d, 0x23, 0x24, 0x2a, 0x26, 0x25, 0x27, 0x27, 0x28],
    // 0xd: DATA_006193, two (shuttle and lifter)
    &[0x0f, 0x12],
    // 0xe: DATA_006194, four (police)
    &[0x1e, 0x1e, 0x22, 0x22],
    // 0xf: DATA_006195, three (Lynx, LRC)
    &[0x46, 0x46, 0x47],
    // 0x10: DATA_006196, twenty (assassination targets)
    &[
        0x37, 0x40, 0x2c, 0x2f, 0x35, 0x3c, 0x36, 0x3e, 0x39, 0x2d, 0x3b, 0x3d, 0x43, 0x3e, 0x43,
        0x38, 0x41, 0x3a, 0x41, 0x44,
    ],
    // 0x11: DATA_006197, twelve (Thargoids and a transporter)
    &[
        0x45, 0x31, 0x31, 0x32, 0x32, 0x33, 0x33, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f,
    ],
    // 0x12: DATA_006198, one
    &[0x11],
];

/// One row of the original's AI state table, `DATA_006255` (`ffedat.asm:9854`):
/// four routine slots per state, `0x25` rows.
///
/// The slots are `(control, movement, notify, damage)`: `F706` (event 6, the
/// per-frame controller) indexes `DATA_006255`, `F704` (event 5, the physics
/// step) indexes `DATA_006256`, `F780` (another object's death) indexes
/// `DATA_006257` and `F778` (this object taking a hit) indexes `DATA_006258`.
/// The three labels between `DATA_006255` and `DATA_006258` are the first row's
/// remaining slots, not separate tables: the row data is
/// `F742, F727, 0, F789` for state 0. A `None` slot is a `dd 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiState {
    /// `object+0xff`.
    pub code: u8,
    /// What the original uses the state for.
    pub name: &'static str,
    /// F706's event-6 controller.
    pub control: Option<u16>,
    /// F704's event-5 physics step.
    pub movement: Option<u16>,
    /// F780's death notification for the target this ship wants dead.
    pub notify: Option<u16>,
    /// F778's response to a hit on this ship (F785-F790).
    pub damage: Option<u16>,
}

/// `DATA_006255`: the AI states, in `object+0xff` order.
///
/// The first rows are the flight cycle: F742's navigation (0x00), F752/F753's
/// attack turns (0x01/0x02), F755/F756's evasion (0x03), F750's interception
/// (0x04), F749's reposition (0x05), F759's static hold (0x06), F736's departure
/// (0x07) and F741's new target after a jump (0x08). F770 (0x09) is the trading
/// approach, F743 (0x0a) the patrol/station-keeping offset and F751 (0x0b) the
/// pirate formation. Rows 0x10-0x12 are F760's trading timer, F737's jump
/// countdown and F740's jump exit; rows 0x1d/0x1e are the hyperspace clouds.
///
/// The station cycle is 0x1f F734 (request launch), 0x20 F747 (docked: mark the
/// object and wait for traffic control), 0x21 F735 (launching: wait for the
/// station's door, then state 7), 0x23 F746 (final approach and pad contact,
/// with F745's F655 request) and 0x24 F733 (docked: wait out the object time,
/// then request launch). 0x22 F748 is the player autopilot's launch check.
pub const AI_STATES: [AiState; 0x25] = [
    AiState {
        code: 0x00,
        name: "navigation",
        control: Some(742),
        movement: Some(727),
        notify: None,
        damage: Some(789),
    },
    AiState {
        code: 0x01,
        name: "attack turn",
        control: Some(752),
        movement: Some(725),
        notify: Some(782),
        damage: Some(785),
    },
    AiState {
        code: 0x02,
        name: "firing",
        control: Some(752),
        movement: Some(724),
        notify: Some(782),
        damage: Some(785),
    },
    AiState {
        code: 0x03,
        name: "evade",
        control: Some(755),
        movement: Some(725),
        notify: Some(782),
        damage: Some(785),
    },
    AiState {
        code: 0x04,
        name: "intercept",
        control: Some(750),
        movement: Some(727),
        notify: Some(782),
        damage: Some(785),
    },
    AiState {
        code: 0x05,
        name: "reposition",
        control: Some(749),
        movement: Some(727),
        notify: Some(782),
        damage: Some(785),
    },
    AiState {
        code: 0x06,
        name: "static",
        control: Some(759),
        movement: Some(725),
        notify: None,
        damage: Some(790),
    },
    AiState {
        code: 0x07,
        name: "depart station",
        control: Some(736),
        movement: Some(727),
        notify: None,
        damage: Some(788),
    },
    AiState {
        code: 0x08,
        name: "new target",
        control: Some(741),
        movement: Some(727),
        notify: None,
        damage: Some(789),
    },
    AiState {
        code: 0x09,
        name: "trading approach",
        control: Some(770),
        movement: Some(727),
        notify: None,
        damage: Some(785),
    },
    AiState {
        code: 0x0a,
        name: "patrol",
        control: Some(743),
        movement: Some(727),
        notify: None,
        damage: Some(789),
    },
    AiState {
        code: 0x0b,
        name: "formation",
        control: Some(751),
        movement: Some(729),
        notify: Some(782),
        damage: Some(785),
    },
    AiState {
        code: 0x0c,
        name: "missile evasion",
        control: Some(757),
        movement: Some(725),
        notify: Some(781),
        damage: None,
    },
    AiState {
        code: 0x0d,
        name: "none",
        control: None,
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x0e,
        name: "parked shuttle",
        control: None,
        movement: None,
        notify: None,
        damage: Some(790),
    },
    AiState {
        code: 0x0f,
        name: "parked police",
        control: None,
        movement: None,
        notify: None,
        damage: Some(790),
    },
    AiState {
        code: 0x10,
        name: "trading timer",
        control: Some(760),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x11,
        name: "jump countdown",
        control: Some(737),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x12,
        name: "jump exit",
        control: Some(740),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x13,
        name: "missile",
        control: Some(758),
        movement: Some(717),
        notify: Some(784),
        damage: None,
    },
    AiState {
        code: 0x14,
        name: "mine",
        control: None,
        movement: Some(716),
        notify: Some(784),
        damage: None,
    },
    AiState {
        code: 0x15,
        name: "missile",
        control: Some(768),
        movement: Some(712),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x16,
        name: "missile",
        control: Some(767),
        movement: Some(713),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x17,
        name: "missile",
        control: Some(732),
        movement: Some(714),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x18,
        name: "cargo",
        control: Some(766),
        movement: Some(710),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x19,
        name: "asteroid",
        control: Some(765),
        movement: Some(711),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x1a,
        name: "missile",
        control: None,
        movement: Some(709),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x1b,
        name: "space dust",
        control: Some(769),
        movement: Some(715),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x1c,
        name: "explosion",
        control: None,
        movement: Some(708),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x1d,
        name: "hyperspace cloud",
        control: Some(764),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x1e,
        name: "cloud remnant",
        control: Some(763),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x1f,
        name: "request launch",
        control: Some(734),
        movement: Some(727),
        notify: None,
        damage: Some(787),
    },
    AiState {
        code: 0x20,
        name: "docked",
        control: Some(747),
        movement: Some(723),
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x21,
        name: "launching",
        control: Some(735),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x22,
        name: "launch check",
        control: Some(748),
        movement: None,
        notify: None,
        damage: None,
    },
    AiState {
        code: 0x23,
        name: "docking",
        control: Some(746),
        movement: Some(727),
        notify: None,
        damage: Some(790),
    },
    AiState {
        code: 0x24,
        name: "docked countdown",
        control: Some(733),
        movement: None,
        notify: None,
        damage: Some(786),
    },
];

/// `DATA_006255`'s row for a state byte, when the byte is in the table.
pub fn ai_state(code: u8) -> Option<&'static AiState> {
    AI_STATES.get(usize::from(code))
}

/// `DATA_008818`/`DATA_008820` after F146/F147: the one pair F922, F1530 and
/// F1476 all advance.
pub const SHARED_SEED: [u32; 2] = [0x1234_5678, 0x4567_89ab];

/// F922 (`ffe.asm:68974`): the original's bounded random number. `a` and `b`
/// are `DATA_008818`/`DATA_008820`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Random {
    pub a: u32,
    pub b: u32,
}

impl Random {
    pub const fn new(a: u32, b: u32) -> Self {
        Self { a, b }
    }

    /// The game's shared pair as F147 sets it for a new game
    /// (`ffe.asm:14923`, start modes below 5) and F146 at start-up.
    pub const fn start() -> Self {
        Self::new(SHARED_SEED[0], SHARED_SEED[1])
    }

    /// The same words as F1530's/F1476's callers see them (`ffe_core::RandomState`).
    pub const fn state(self) -> crate::RandomState {
        crate::RandomState::new(self.a, self.b)
    }

    /// Take the words back from a lent [`crate::RandomState`].
    pub fn take(&mut self, lent: crate::RandomState) {
        self.a = lent.a;
        self.b = lent.b;
    }

    /// F922's bounded draw: a new state, then `((a & 0xffff) * limit) >> 16`.
    pub fn bounded(&mut self, limit: u32) -> u32 {
        let b = self.b;
        let rotated = b.rotate_left(5);
        let a = self.a.wrapping_add(b).wrapping_add(rotated);
        self.a = a.rotate_left(16);
        self.b = rotated;
        ((u64::from(self.a & 0xffff) * u64::from(limit)) >> 16) as u32
    }

    /// F1530 (`ffe.asm:90913`): the object-time random word F747/F691 read from
    /// `DATA_008818`. It advances the same `a`/`b` words as [`Self::bounded`]
    /// with a different mixing step and returns the full new `a`; the original
    /// returns its low word and callers such as F691 then read `[DATA_008818]`.
    pub fn advance(&mut self) -> u32 {
        let old_a = self.a;
        let mixed = (self.b ^ 0x1234_567f).rotate_right(7);
        self.a = self.a.wrapping_add(mixed);
        self.b = old_a.rotate_left(13);
        self.a
    }
}

/// Which of the original's traffic roles a ship has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    Trader,
    /// F699's auxiliary bulk carrier (`F772(0xf)`: Lynx Bulk Carrier/LRC). It
    /// is state 0x0d, which has no control routine, so it holds on its orbit.
    Shuttle,
    Police,
    /// A lone pirate (F694, type 0xa in F772) or a group member (F695, 0xb).
    Pirate,
    Military,
    Thargoid,
}

impl Role {
    /// The F635 type-table index, before the allegiance shift for types 0 and 1.
    pub fn type_code(self) -> usize {
        match self {
            Role::Trader => 0x1,
            // F699 calls F772(0xf), the Lynx/LRC list. Do not confuse the table
            // index 0xf with model 0x0f (the Interplanetary Shuttle, record 0xd).
            Role::Shuttle => 0xf,
            Role::Police => 0xe,
            Role::Pirate => 0xa,
            Role::Military => 0x2,
            Role::Thargoid => 0x11,
        }
    }

    /// F773 writes the role into `+0x118`; the values it uses for the traffic
    /// roles are 1 (traders/station ships), 0xf (F699 aux traders), 0x11 (the
    /// F690 branch) and 0xfb (the F697 equipment branch).
    pub fn ai_class(self) -> u8 {
        match self {
            Role::Trader => 0x1,
            Role::Shuttle | Role::Police | Role::Military => 0xf,
            Role::Pirate => 0xfb,
            Role::Thargoid => 0x11,
        }
    }

    pub fn hostile(self) -> bool {
        matches!(self, Role::Pirate | Role::Thargoid)
    }

    /// A ship the law protects: attacking one is a crime (F931's illegal-fire
    /// flag, `DATA_008860` bit 2, is set for any player shot; the wanted state
    /// that makes police react is the port's reconstruction from
    /// `DATA_008646/8647`). Pirates and Thargoids are fair game.
    pub fn clean(self) -> bool {
        matches!(
            self,
            Role::Trader | Role::Shuttle | Role::Police | Role::Military
        )
    }
}

/// F635: the model for a role. `allegiance` is `DATA_008996` (the system
/// record's allegiance word `>> 6`). Out-of-range types fall back to the
/// original's first table so a hand-made system cannot panic.
pub fn model_for(role: Role, allegiance: u32, rng: &mut Random) -> u32 {
    model_for_type(role.type_code(), allegiance, rng)
}

/// F635 for F772's type-table index `index` (F658, which the mission
/// scripts' F772 and token 0x35 call).
pub fn model_for_type(mut index: usize, allegiance: u32, rng: &mut Random) -> u32 {
    if index <= 1 {
        index += 2 * (allegiance.min(4) as usize);
    }
    let table = SHIP_TYPES
        .get(index)
        .or_else(|| SHIP_TYPES.first())
        .expect("the table is not empty");
    if table.is_empty() {
        return 0;
    }
    table[rng.bounded(table.len() as u32) as usize]
}

/// F635 with a fixed draw instead of F922's random word: `fraction` is the
/// 16-bit value F658 passes for kind 0x10 (F460/F506's `F470` ship), and the
/// source indexes the list with `(fraction * count) >> 16`.
pub fn model_for_fraction(mut index: usize, allegiance: u32, fraction: u16) -> u32 {
    if index <= 1 {
        index += 2 * (allegiance.min(4) as usize);
    }
    let table = SHIP_TYPES
        .get(index)
        .or_else(|| SHIP_TYPES.first())
        .expect("the table is not empty");
    if table.is_empty() {
        return 0;
    }
    let at = (u32::from(fraction) * table.len() as u32) >> 16;
    table[at as usize]
}

/// The fields of an original ship the NPC needs: its model's radii, its
/// statistics' thrust, mass, shield and cargo words, and its gun mounts. Built
/// by `ffe-rs` from the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub model: u32,
    pub name: String,
    pub radius: i64,
    /// F1533's collision radius (`+0x140`), the model's other radius word. Zero
    /// means "the same as `radius`".
    pub collision_radius: i64,
    /// F1533's class byte (`+0x14c`) from the model records: light index,
    /// dockable/starport/open-air and the complex-shape bit.
    pub class: u8,
    pub mass: u16,
    pub shield_strength: u16,
    /// The statistics' forward thrust word (`+0x0`, object `+0xc4`).
    pub thrust: i16,
    /// The statistics' drive word's low byte (`+0x14`), the drive class.
    pub drive: u8,
    /// The statistics' gun mounts (`+0x4`).
    pub gun_mounts: i8,
    /// The statistics' `+0x16` word, added to `DATA_008911` by F942.
    pub rating: i16,
    /// The statistics' missile mounts (`+0x12`), F773's count ceiling.
    pub missiles: u16,
    /// The cargo space (`+0x116`): F942 uses `min(4, cargo >> 1)` extra debris
    /// pods. F773 also spends it on the ship's fitted equipment.
    pub cargo: u16,
    /// The statistics' six thrust limits (F598/F600), when known: the ship then
    /// flies by the original's object physics (F597 guidance, F601, F585,
    /// F1512) instead of the port's speed model.
    pub limits: Option<crate::thrust::Limits>,
}

impl Spec {
    /// `+0xe4`: F1533 stores the statistics' mass word times four.
    pub fn hull(&self) -> u16 {
        self.mass.saturating_mul(4)
    }

    /// The shield maximum. F773 gives an AI ship `random * 16` from its equipment
    /// budget; the port uses the statistics `+0xc` shield word's low byte times
    /// four, which gives the traffic ships a shield that a 1MW pulse can absorb
    /// (see `docs/notes/npc.md`).
    pub fn shield_max(&self) -> u16 {
        (self.shield_strength & 0xff).saturating_mul(4)
    }

    /// F601's thrust-to-mass ratio sets how a ship accelerates, so the port makes
    /// its top speed proportional to the same ratio. The scale is calibrated on
    /// the new-game Eagle (thrust 13625, mass 25), which keeps the old pirate
    /// speed. Big hulls are slower and turn less sharply, as F585's halving of
    /// the turn limit for models 0x35 and 0x3b says.
    pub fn top_speed(&self) -> f64 {
        let ratio = f64::from(self.thrust.max(0)) / f64::from(self.mass.max(1));
        (SPEED_SCALE * ratio).clamp(MIN_SPEED, MAX_SPEED)
    }

    /// The acceleration that reaches [`Spec::top_speed`] in [`ACCEL_SECONDS`].
    /// The original integrates the thrust word itself (F601); its units are the
    /// original's position units, which do not carry over to the port's scaled
    /// system frame, so the ramp is the port's and the ratio above sets the top.
    pub fn accel(&self) -> f64 {
        self.top_speed() / ACCEL_SECONDS
    }
}

/// Where traffic spawns: a station or planet with its interaction radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub position: [i64; 3],
    pub radius: i64,
    /// The system body's index when the anchor is a place to dock (F689's
    /// `+0x100` destination in `Navigator::bodies` order), so a trader can ask
    /// for a bay there. `None` for a star or a plain orbit.
    pub station: Option<usize>,
    /// Capital traffic uses orbital stations as landmarks.
    pub orbital_station: bool,
}

/// The radius of the orbit a trader or shuttle holds around its anchor. The
/// original's F702 words put a ship on a system-wide orbit; this radius keeps
/// the same "random phase around an anchor" rule but is near enough to see and
/// shoot (the laser's range is [`laser::RANGE`]).
pub const ANCHOR_ORBIT: f64 = 2.5e6;
/// The base distance ahead of the player a pirate group appears at.
pub const PIRATE_DISTANCE: f64 = 3.5e6;
/// How far outside the anchor's orbit a system entry's pirate group spawns
/// (25 km): beyond [`laser::RANGE`] of the port, near enough to come in.
pub const PIRATE_LURK: f64 = 25.0e6;
/// F702's orbit for a pirate group: the raw ffp word `0x5000/0x33` (mantissa shifted by
/// exponent - 15, F1495), round the star. The executed original (`--npc-trace` at the default
/// start) has its pirates at 1.3e15 from the root with parent 0, further out than Hope's planet
/// (9.3e14), and they never reach a player on the pad.
pub const PIRATE_ORBIT: f64 = (0x5000_i64 << 36) as f64;
/// Ships farther than this from the player are despawned (F927 recycles slots).
pub const DESPAWN_RANGE: f64 = 30.0e6;
/// The half-size of the box the port tests a hunting ship's beam against. The port has the model's
/// bounding radius (32 m for the Saker) and no collision stream. In the executed original
/// (`FFE_POLICE_TRACE`'s `phull`, `--flight idle` at Hope) every hull hit came while the beam
/// passed 6.5 to 9 m from the player's origin, in bursts of two or three frames per pass
/// ([`MAX_HITS_PER_PASS`]), never at a wider miss.
pub const PLAYER_HIT_BOX: i64 = 8_000;
/// How many frames of one state-2 pass reach the hull: the original's four passes at Hope hit
/// two, three, three and three times (a pass is about 90 frames in the corridor).
pub const MAX_HITS_PER_PASS: u8 = 3;
/// Port addition: police patrols per system, one at the place nearest the
/// player and one at each other place up to this count. The original's F238
/// sends police after a wanted player; the port keeps a fixed patrol force and
/// calls it in (see [`NpcShip::provoke`]) instead of spawning more.
pub const MAX_POLICE_PATROLS: usize = 4;
/// Port addition: a called-in police or military ship's cruise speed toward
/// the player (2 km/s), its acceleration, and the range it slows to its own
/// combat speed at.
pub const RESPONSE_SPEED: f64 = 2.0e6;
pub const RESPONSE_ACCEL: f64 = 2.0e5;
pub const RESPONSE_ARRIVE: f64 = 5.0e6;
/// F750's switch to combat: F596's distance exponent below 10, i.e. inside
/// `2^(13+10)` mm (the same length as F931's laser range).
pub const COMBAT_RANGE: f64 = (1u64 << 23) as f64;
/// F770's trading approach ends at F610 distance class 0xa: `2^(13+10)` mm.
pub const TRADING_RANGE: f64 = (1u64 << 23) as f64;
/// F1536's model-0x9d flash stays for this many seconds (the original clears the
/// flag on the next frame; a short visible flash replaces it in the port).
pub const FLASH_SECONDS: f64 = 0.25;
/// How long an NPC's laser shot stays drawn: one frame of the original at about 18
/// frames a second (F1536 draws F1776's beam on the frame the gun fired).
pub const BEAM_SECONDS: f64 = 1.0 / 18.0;

/// The laser shot an NPC just fired, for F1776's visible beam (model 0x9c at its
/// gun): the laser equipment code (the beam model's global 4) and F931's `+0xe6`
/// length (the hit distance >> 7, or `0xffff` for a miss), kept for
/// [`BEAM_SECONDS`]. F931 sets the flag whether or not the ray hits.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShotMark {
    pub code: u8,
    pub length: u16,
    pub seconds: f64,
}

impl ShotMark {
    /// F931's `+0xe6` for a miss.
    pub const MISS: u16 = 0xffff;

    fn new(code: u8, hit: Option<&laser::Hit>) -> Self {
        ShotMark {
            code,
            length: hit.map_or(Self::MISS, |hit| {
                ((hit.distance.max(0.0) as i64) >> 7) as u16
            }),
            seconds: BEAM_SECONDS,
        }
    }
}
/// How long the port keeps a destroyed ship's marker before removing it.
pub const DEATH_SECONDS: f64 = 0.6;
/// The most ships one system keeps: the port's object budget, not F688's rolls.
/// F688 draws every kind's count itself (the executed original's default start
/// holds 17 ships: six bay ships, an aux trader, a trader and two pirate
/// groups), so this only bounds a rare high roll: four traders, thirty pirate
/// attempts of several ships and 25 bay ships can exceed the object array.
pub const MAX_SHIPS: usize = 40;
/// F691's cap on its `RND(4 * places)` roll (`cmp ebx,0x19`, `ffe.asm:45956`).
pub const MAX_BAY_SHIPS: usize = 25;
/// Station traffic's cruise floor. [`Spec::top_speed`]'s thrust/mass ratio
/// clamps a heavy freighter to 2,000 mm/s, which would put F745's five-kilometre
/// approach half an hour away; FFE's own units do not scale to the port's
/// millimetre frame, so traders and shuttles get this floor for the docking
/// cycle. It is a port decision, not a source constant.
pub const STATION_CRUISE_SPEED: f64 = 4.0e4;
/// The acceleration that goes with [`STATION_CRUISE_SPEED`]: a stop from it in
/// 40 m, so the final approach can still brake at the pad.
pub const STATION_ACCEL: f64 = 2.0e4;

/// F1511's velocity unit in millimetres per second (`v * t / 512` with the
/// game's 49,700 tics a second).
pub const MM_PER_VELOCITY_UNIT: f64 = TICS_PER_SECOND / 512.0;
/// The original's tics per second at 1x (`DATA_009155` is 994 per 20 ms).
pub const TICS_PER_SECOND: f64 = TICS_PER_FRAME as f64 * 50.0;
/// The original's AI step, once per game second: the whole-day tick F152 drives
/// event 6 with (`0xc22e` tics, see `docs/notes/npc.md`). The executed original's
/// pirate leader (state 4) re-aims at frames 532, 579, 627, 675, ... of a 1,043-tic
/// frame run, 47.67 frames apart, and the port's `guide` reproduces the words it
/// writes there (`tests/npc_pirate_replay.rs`).
pub const THINK_TICS: u32 = 0xc22e;

fn think_due() -> u32 {
    THINK_TICS
}

/// The base turn rate in radians per second. F585 limits the per-frame turn and
/// halves that limit for models 0x35 and above and quarters it from 0x3b; the
/// base is the port's, calibrated so a Cobra can track a crossing Eagle.
pub const TURN_RATE: f64 = 1.2;
/// The speed scale: `top_speed = 71 * thrust / mass`, which gives the Eagle
/// (13625/25) the 38,800 units/s the traffic slice used.
const SPEED_SCALE: f64 = 71.0;
const MIN_SPEED: f64 = 2_000.0;
const MAX_SPEED: f64 = 80_000.0;
/// Seconds to reach [`Spec::top_speed`].
const ACCEL_SECONDS: f64 = 3.0;
/// The distance a pirate stops closing at (F725/F753's approach distance).
pub const STANDOFF: f64 = 1.2e6;
/// Within this of an orbit or waypoint the ship is there.
const ARRIVE: f64 = 0.35e6;
/// How long a F597 overshoot pass runs before the ship turns back in.
const PASS_SECONDS: f64 = 1.4;
/// How long a ship runs for before F787/F738's jump-out.
pub const FLEE_SECONDS: f64 = 3.0;
/// A hull at or below this fraction runs (the port's trigger; see the module
/// docs: the original has the jump-out states but no low-hull trigger).
const FLEE_HULL: f64 = 0.25;
/// `+0xfa` counts 1/65536 of a day (F738 splits it into `+0xa8`'s tics and
/// `+0xac`'s days); one day of game time is 86,400 s, as `clock_words` uses.
pub const JUMP_TIME_PER_DAY: f64 = 65_536.0;

/// F738's `+0xfa` in game seconds: `jump_time / 65536` days. A jump of 73,500
/// units is 1.12 days, the value `Navigator::hyperspace_to_destination` already
/// reports.
pub fn jump_seconds(jump_time: u32) -> f64 {
    f64::from(jump_time) * 86_400.0 / JUMP_TIME_PER_DAY
}

/// F953 sets `+0x14e = 0xa` on hull damage; F938 counts it down a second at a
/// time and F725/F727/F729 emit F939's hit particle while it is clear.
pub const SMOKE_TIMEOUT: u8 = 0x0a;
/// F732 resets `+0x14d` to `0x13` after F606's frame-of-reference update.
pub const FOR_TIMEOUT: u8 = 0x13;

/// `DATA_009013` by the console's time-acceleration step (F208: 1x, 10x, 100x, 1000x, 10000x).
pub const TIME_SHIFTS: [i32; 5] = [0, 3, 7, 10, 12];

/// What a hunting ship reads from the player each frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pursuit {
    /// `DATA_009013`, the time-acceleration shift ([`TIME_SHIFTS`]); 0 at 1x.
    pub time_shift: i32,
    /// The player's velocity in object units: F597's accelerated branch gives a placed ship
    /// its target's velocity (F594).
    pub velocity: [i32; 3],
    /// `DATA_008839`, the combat velocity F726 relaxes a fighting ship towards.
    pub combat_velocity: [i32; 3],
}
/// F610's `+0x150` reset: `(time-acceleration exponent + 1) << 4`. The port's
/// NPC flight is 1x, so the counter runs 16 navigation frames.
pub const AVOID_TIMEOUT: u8 = 16;
/// F675 keeps a pirate within 0x10 (16) of the player across a jump. The
/// original's 0x10 is in its own 32-bit frame; the port's 2,000 km keeps the
/// same "close enough to follow you" intent in its scaled frame.
pub const PERSIST_RANGE: f64 = 2.0e6;

/// F747's docked interval: `[+0xac] = [D8807] + 2` days and
/// `[+0xa8] = [D8804] + [D8818]`, a random fraction of a day in tics. The
/// object time is the original's absolute game clock; the port counts the
/// interval down in game seconds so time acceleration (`Navigator::clock`)
/// shortens the wait exactly as it does in the original.
pub const DOCKED_DAYS: f64 = 2.0;
/// The day fraction F747 adds from `[D8818]` (tics in a day, 49,700 a second).
pub const DOCKED_JITTER_SECONDS: f64 = 86_400.0;

/// F691/F747's random object time from the F1530 word: the high nibble is whole
/// days (`[+0xac]`) and the low 28 bits shifted left four are the day's tics
/// (`[+0xa8]`). F747 adds the two days on top; F691 uses the word as it is, so
/// an initial bay ship can launch within its first fraction of a day.
pub fn docked_object_seconds(word: u32) -> f64 {
    let days = f64::from(word >> 28);
    let tics = word.wrapping_shl(4);
    days * 86_400.0 + f64::from(tics) / TICS_PER_SECOND
}

/// How long a trader waits at a station for a bay before F762 picks another
/// station: the source retries F745 every frame for ever, so this give-up is
/// the port's (see `docs/notes/npc.md`, "Owner decisions").
pub const DOCK_WAIT_SECONDS: f64 = 30.0;
/// F745/F734's retry after traffic control refused a request.
pub const DOCK_RETRY_SECONDS: f64 = 1.0;
/// How long F736 flies out of the slot before F761/F762 pick the next station.
/// The original decides in the same frame (it is already at the station); the
/// port gives the departure a short visible run first.
const DEPART_SECONDS: f64 = 1.0;

/// A station as the station-traffic cycle needs it: its centre, docking axis
/// and face (F745's `+0x102..+0x106` target), the bay F650 gave this ship, and
/// the traffic record's clearance and door words. The caller owns the record
/// (`ffe_core::traffic::Starport`) and supplies a snapshot for the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    /// The system body's index (`Navigator::bodies`).
    pub index: usize,
    pub position: [i64; 3],
    /// The outward docking axis (the object's +z row, F650's frame).
    pub axis: [f64; 3],
    /// Distance from the centre to the docking face (millimetres).
    pub front: f64,
    /// The model's bounding radius (millimetres).
    pub radius: f64,
    /// F650's pad point in the system frame for an open-air starport; indoor
    /// stations have no visible pad, so the port uses the slot instead.
    pub pad: Option<[i64; 3]>,
    /// The bay F650 allocated to this ship (`+0xd0..`), if any.
    pub bay: Option<u16>,
    /// F657: this ship is the record's current one.
    pub cleared: bool,
    /// F652's mode 3: the door is closing behind the ship.
    pub closing: bool,
    /// F865: the door word has reached 0.
    pub door_open: bool,
}

impl Station {
    /// The point F745 aims at: the allocated pad, or just inside the slot of an
    /// indoor station.
    pub fn bay_point(&self) -> [i64; 3] {
        match self.pad {
            Some(pad) => pad,
            None => self.point(self.front * 0.9),
        }
    }

    /// The point `along` from the centre on the docking axis.
    pub fn point(&self, along: f64) -> [i64; 3] {
        std::array::from_fn(|k| self.position[k] + (self.axis[k] * along).round() as i64)
    }

    /// F745/F736's "distance 6" test: the ship asks for a bay at the station's
    /// interaction reach, not at the pad.
    pub fn request_range(&self) -> f64 {
        self.front + self.radius
    }

    /// F1088/F735: where a launch puts the ship, outside the face and hull.
    pub fn launch_point(&self) -> [i64; 3] {
        self.point(self.front + 2.0 * self.radius + 2.0e6)
    }
}

/// A request the station-traffic record must answer for a ship. The caller owns
/// the `Starport` (F649-F657) and applies these; the ship only flies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockRequest {
    /// F655/F654/F650: ask for clearance and a bay.
    Request { ship: u32, station: usize },
    /// F652: the ship has touched the pad (only the current ship may).
    Touch { ship: u32, station: usize },
    /// F747/F653: the door has closed and the handling record is cleared.
    Settle { ship: u32, station: usize },
    /// F654 again: ask traffic control to open the door (F734).
    Launch { ship: u32, station: usize },
    /// F651/F788: free the bay.
    Release { ship: u32, station: usize },
    /// F761/F762: pick the next station, or leave the system when there is none.
    ChooseStation { ship: u32, station: usize },
}

/// The traffic record's ship byte for an NPC id: 0 is an empty bay and 1 is the
/// player (`traffic::PLAYER`), so NPCs start at 2.
pub fn traffic_id(id: u32) -> u8 {
    (id % 250) as u8 + 2
}

/// `DATA_006268` (`ffedat.asm`): the pirate bounty table, in credits. F697 picks
/// an entry with `DATA_006268[(random & 0xf) + offset]`, offset 4 for F694's
/// lone pirate and 2 for F695's group, so a lone pirate can carry 50..150 CR
/// and a group member up to 100.
pub const PIRATE_BOUNTIES: [u32; 22] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 100, 125, 150, 175, 200,
];

/// `DATA_006260` (`ffedat.asm:10051`), the sixteen missile codes F773 fits,
/// indexed by the top nibble of its random word. The low end is the two mine
/// codes, the middle is the KL760 (`0x82`, model 6) and the top entries are the
/// LV111 (`0x83`, model 7) and MV1 (`0x84`, model 8).
pub const MISSILE_CODES: [u8; 16] = [
    0xc0, 0xc1, 0xc1, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x82, 0x83, 0x83, 0x83, 0x84,
];

/// F754's launch band. F753 calls it only for F610 distance exponents 5..8;
/// with F758's `2^(13+e)` convention that is 2^18..2^22 mm (262 m to 4.2 km).
pub const MISSILE_LAUNCH_MIN: f64 = 262_144.0;
pub const MISSILE_LAUNCH_MAX: f64 = 4_194_304.0;
/// F756 (state 3) calls F754 for any distance exponent up to 10, so its band is
/// wider than F753's: 0..2^23 mm (8.4 km).
pub const MISSILE_LAUNCH_PASS_MAX: f64 = 8_388_608.0;
/// F754's random gate: `F1530() < 0x2000` (one in eight event-6 calls).
pub const LAUNCH_GATE: u16 = 0x2000;
/// F757's ECM gate: `F1530() > 0xa000` (0.375 of the evasion updates).
pub const ECM_GATE: u16 = 0xa000;
/// The original's event-6 interval (`DATA_009155` is 994 tics per 20 ms frame).
/// F754/F757 are controllers, so their rolls run once per 20 ms of game time.
pub const CONTROL_SECONDS: f64 = 0.02;
/// The lateral distance the port's F757 evasion aims across the missile's line.
pub const EVADE_OFFSET: f64 = 2.0e6;
/// F773's general-equipment bit for the energy bomb (`+0xc8` bit 26). F754
/// calls F937 when F941 found no pylon; the port does not model F937's blast
/// (reported in `docs/notes/npc.md`).
pub const ENERGY_BOMB_BIT: u32 = 1 << 26;

/// F941's launch request for an NPC. The core has no cache, so `ffe-rs` turns
/// the code into a missile [`crate::missile::Spec`] and calls F941's launch.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MissileLaunch {
    /// The launcher's [`NpcShip::id`] (`+0x100`).
    pub owner: u32,
    /// The target (`+0xfe`): the player, in the port's traffic AI.
    pub target: u32,
    /// The pylon word's missile code (`ship+0xd6..+0xdf`).
    pub code: u8,
    /// F941's 1-based pylon index.
    pub pylon: usize,
    pub position: [i64; 3],
    /// Right, up, forward.
    pub axes: [[f64; 3]; 3],
    /// F1533's launcher radius, which F941 uses for the separation impulse.
    pub launcher_radius: i64,
}

/// A ship's AI state in this port. Each variant names the original states it
/// stands in for, so [`State::original`] can be checked against [`AI_STATES`]:
///
/// - `Transit` = 0x00/0x08/0x09 (F742/F741/F770's navigation to the target).
/// - `Approach` = 0x23 (F746's final approach; F745 asks F655 for the bay).
/// - `Docked` = 0x20/0x24 (F747 marks the docked object, F733 waits out F747's
///   two-to-three day object time and requests launch).
/// - `Launch` = 0x1f/0x21 (F734's F654 request, F735 waiting for the door).
/// - `Depart` = 0x07 (F736 flies out, F651 releases the bay, F761/F762 choose
///   the next station or jump out).
/// - `Hold` = 0x06/0x0d/0x0e/0x0f (static holding, F759; the parked shuttle and
///   police states have only their F790 damage response).
/// - `Escort` = 0x0a/0x0b (F743's station-keeping and F751's formation join).
/// - `Attack` = 0x01/0x02/0x04/0x05 (F752/F753 attack, F724 firing, F750
///   interception, F749's transition).
/// - `Pass` = 0x03 (F755/F756's evasion/overshoot).
/// - `Evade` = 0x0c (F757: F941 puts a ship a missile is aimed at into this
///   state; F757 jinks and may call F935's ECM burst).
/// - `Flee` = 0x1d (F761 hides the ship as F738's hyperspace cloud).
/// - `Dead` = 0x1c (F708/F942's explosion object).
/// - `Escaped`: the cloud is gone and F927 frees the slot; no live row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum State {
    /// Fly to the station or waypoint, then hold.
    Transit,
    /// F770's trading approach (state 0x09): close on `+0xfe` in this state so
    /// no attack or shot is taken; `scripts::approaches` is F770's class-0xa
    /// test and hands the ship to state 1 (with event 0x17 and the 0x98d0 hail).
    Trading,
    /// Fly the final leg to F650's bay and touch the pad (F745/F746).
    Approach,
    /// In the bay, waiting out F747's docked interval (F733).
    Docked,
    /// Clearance given, waiting for the door to open (F734/F735).
    Launch,
    /// Fly out of the slot and choose the next station (F736/F761).
    Depart,
    /// Circle the spawn anchor (F689/F699's station traffic).
    Hold,
    /// Station-keep in a leader's frame (F695's escorts, F743).
    Escort,
    /// Close on the player (F694/F695's pirates, type 0xa/0xb).
    Attack,
    /// A fast pass through the target: F597's "cannot stop in time" overshoot.
    Pass,
    /// A missile is locked on this ship: F757's evasive turn and F935's ECM.
    Evade,
    /// Run from the player and jump out (F787/F738).
    Flee,
    /// F682/F737/F740: the delayed exit cloud a ship carried through the
    /// player's jump re-enters. It waits out `+0xfa`, then returns in the
    /// `+0xf8` mode F682 saved (0x11 the waiting cloud, 0x12 its exit).
    Jump,
    /// Destroyed; the death marker counts down (F942).
    Dead,
    /// Jumped out: the slot is free (F738/F927).
    Escaped,
}

impl State {
    /// The port's state for an original `+0xff` code a mission script sets
    /// (target 0x3e): F669's 9 is F770's trading approach; the attack codes
    /// close on the target; 0x17, the scripts' parked objects, holds its place
    /// like 0x06/0x0d-0x0f.
    pub fn from_original(code: u8) -> Option<State> {
        Some(match code {
            0x00 | 0x08 => State::Transit,
            0x09 => State::Trading,
            0x01 | 0x02 | 0x04 | 0x05 => State::Attack,
            0x03 => State::Pass,
            0x06 | 0x0d | 0x0e | 0x0f | 0x17 => State::Hold,
            0x07 => State::Depart,
            0x0a | 0x0b => State::Escort,
            0x0c => State::Evade,
            0x11 | 0x12 => State::Jump,
            0x1d => State::Flee,
            0x1f | 0x21 => State::Launch,
            0x20 | 0x24 => State::Docked,
            0x23 => State::Approach,
            _ => return None,
        })
    }

    /// The original `+0xff` values this state stands in for.
    pub fn original(self) -> &'static [u8] {
        match self {
            State::Transit => &[0x00, 0x08],
            State::Trading => &[0x09],
            State::Approach => &[0x23],
            State::Docked => &[0x20, 0x24],
            State::Launch => &[0x1f, 0x21],
            State::Depart => &[0x07],
            State::Hold => &[0x06, 0x0d, 0x0e, 0x0f],
            State::Escort => &[0x0a, 0x0b],
            State::Attack => &[0x01, 0x02, 0x04, 0x05],
            State::Pass => &[0x03],
            State::Evade => &[0x0c],
            State::Flee => &[0x1d],
            State::Jump => &[0x11, 0x12],
            State::Dead => &[0x1c],
            State::Escaped => &[],
        }
    }

    /// Whether the ship is in the station's docking cycle.
    pub fn docking(self) -> bool {
        matches!(
            self,
            State::Approach | State::Docked | State::Launch | State::Depart
        )
    }
}

/// One NPC ship in the system flight frame.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NpcShip {
    /// A stable id for the target lock (the slot index is not stable when a
    /// ship is destroyed and the list is compacted).
    pub id: u32,
    /// `+0x86`: the object's slot in the 0x152-byte object list, F927's
    /// topmost-free allocation (see `object_record::ShipSlots`).
    #[serde(default)]
    pub slot: u8,
    pub model: u32,
    pub role: Role,
    pub name: String,
    /// The ship type's name without the port's numbering (the model's
    /// `stats+0xe` string, which F1109 prints). Empty means `name`.
    #[serde(default)]
    pub type_name: String,
    /// Position in the system frame, original units (millimetres as the port's
    /// bodies use them).
    pub position: [i64; 3],
    pub velocity: [f64; 3],
    /// Unit forward and up axes; the right axis is `up x forward`.
    pub forward: [f64; 3],
    pub up: [f64; 3],
    pub radius: i64,
    pub damage: DamageState,
    /// The statistics' shield word (`+0xe`), which F1099 shows as
    /// "Shield Strength d".
    pub shield_strength: i16,
    /// The statistics' `+0x16` word, F942's Elite rating points.
    pub rating: i16,
    /// The ship's cargo space (`+0x116`), which F942 turns into debris pods.
    pub cargo: u16,
    pub state: State,
    pub anchor: [i64; 3],
    pub orbit_radius: f64,
    pub phase: f64,
    /// The top speed from the model's thrust/mass ratio (see [`Spec::top_speed`]).
    pub max_speed: f64,
    /// The acceleration that reaches it (see [`Spec::accel`]).
    pub accel: f64,
    /// The turn limit from F585: [`TURN_RATE`], halved for models 0x35..0x3a and
    /// quartered from 0x3b.
    pub turn_rate: f64,
    /// [`Spec::limits`]: with them the ship flies by the original physics.
    #[serde(default)]
    pub limits: Option<crate::thrust::Limits>,
    /// The object's physics words (`+0x0..+0x11e` as F601/F585/F1512 use them):
    /// the velocity accumulators, rates and thrust carried between frames.
    #[serde(default)]
    pub motion: crate::physics::Motion,
    /// The current speed along `forward`.
    pub speed: f64,
    /// Whether the ship attacks the player. Pirates and Thargoids start true;
    /// police turn true when the player is wanted.
    pub hostile: bool,
    /// The front laser (`+0xd2`), when the model has a gun mount. The port fits
    /// one 1MW pulse to a combat role; see `docs/notes/npc.md`.
    pub laser: Option<u8>,
    /// F931's mutable words for that laser (`+0xe8` refire, `+0xea` heat,
    /// `+0x11e` energy).
    pub guns: laser::State,
    /// F773's general-equipment dword (`ship+0xc8`): the E.C.M./Naval E.C.M.
    /// and energy-bomb bits the source's random outfitting can set.
    pub general: u32,
    /// F773's ten pylon words (`ship+0xd6..+0xdf`), with the code `DATA_006260`
    /// chose. F941 clears the one it launches.
    pub pylons: [u8; crate::missile::PYLONS],
    /// F1530's per-ship stream, for F754/F757's rolls and F773's outfitting.
    pub missile_rng: crate::RandomState,
    /// Seconds of event-6 time not yet rolled for a launch/ECM attempt.
    pub control_clock: f64,
    /// Game tics since this ship last re-aimed. The original recomputes an NPC's
    /// guidance (F596's thrust words, speed caps and turn rates) only on its AI
    /// step, once per game second ([`THINK_TICS`]); F727's physics applies the
    /// stored words every frame in between. Starts due, so a new ship aims at once.
    #[serde(default = "think_due")]
    pub think_tics: u32,
    /// F239's police ship in state 0x22 (F748): it waits in the starport until the
    /// record's door has opened, then attacks (state 1).
    #[serde(default)]
    pub scrambled: bool,
    /// The hunting AI's original state while the ship is [`State::Attack`] or [`State::Pass`] with
    /// the original's physics: 0 is F750's interception (state 4), 1 and 2 are F753's attack
    /// states (2 fires), 3 is F755's break-away ([`State::Pass`]).
    #[serde(default)]
    pub attack_phase: u8,
    /// F753/F756 set `DATA_009046` bit 1 on a step at class 10 or nearer: F725 then relaxes the
    /// ship's velocity towards [`Self::combat_velocity`] every frame (F726).
    #[serde(default)]
    pub in_fudge: bool,
    /// Frames of the current state-2 pass that have hit the player ([`MAX_HITS_PER_PASS`]).
    #[serde(default)]
    pub pass_hits: u8,
    /// `DATA_008839`: the velocity F726 relaxes towards (the player's, or zero when it is in a
    /// station's shelter or near the ground). The caller sets it before each tick.
    #[serde(default)]
    pub combat_velocity: [i32; 3],
    /// F749's waypoint words (`+0x102..+0x106`, each shifted left 10 into the aim point).
    #[serde(default)]
    pub waypoint: [i16; 3],
    /// What the ship reads from the player this frame (set by the caller before each tick).
    #[serde(skip)]
    pub pursuit: Pursuit,
    /// F597's accelerated branch placed the ship at its aim point this tick; the caller moves
    /// its escorts with it ([`Self::take_placed`]).
    #[serde(skip)]
    pub placed: bool,
    /// Tics since the scrambled ship's last AI step. F748 only looks at the door on the ship's
    /// own step (once per [`THINK_TICS`] from its creation), so the executed original attacks
    /// at the first step after the door opens: 143 frames after the launch on a 126-frame door.
    #[serde(default)]
    pub scramble_tics: u32,
    /// Set when `scrambled` ended: the police turned hostile this tick.
    /// Taken by [`Self::take_turned`].
    #[serde(default)]
    pub turned: bool,
    /// The missile this ship is evading (`+0xfe` while state 0x0c), from F941.
    pub missile_target: Option<u32>,
    /// F757's F935 burst has already been tried this evasion.
    pub ecm_attempted: bool,
    /// A launch F941 made this tick, drained by `tick_ships`.
    pub launch: Option<MissileLaunch>,
    /// F935's naval flag for a burst made this tick, drained by `tick_ships`.
    pub ecm_burst: Option<bool>,
    /// `+0x11a`, the bounty F942 pays when the player kills it.
    pub bounty: u32,
    /// The leader this ship escorts (F695's `+0xfe` points at it) and the offset
    /// in the leader's frame (`+0x102..+0x106`).
    pub leader: Option<u32>,
    pub formation: [i64; 3],
    /// Seconds of a [`State::Pass`] left.
    pub pass: f64,
    /// Port addition: a police or military ship called in on a wanted
    /// player, flying in at [`RESPONSE_SPEED`] from wherever it patrolled.
    pub responding: bool,
    /// F750: this hostile has closed into [`COMBAT_RANGE`] of the player and
    /// entered combat (state 1); cleared when it stops attacking the player.
    pub in_combat: bool,
    /// F750's battle alert, waiting for the caller ([`Self::take_alert`]).
    pub alert: bool,
    /// Port addition (Advanced traffic): another ship this one is fighting
    /// or running from, flown at in place of the player.
    pub npc_target: Option<u32>,
    /// The assassination contract this ship is the target of: destroying it
    /// completes the contract (F291's target object).
    pub contract: Option<u32>,
    /// Seconds of a [`State::Flee`] left before the jump.
    pub flee: f64,
    /// Seconds of the F1536 shield-hit flash left.
    pub flash: f64,
    /// The laser shot this ship just fired, drawn as F1776's beam.
    #[serde(default)]
    pub shot: Option<ShotMark>,
    /// Seconds of the death marker left.
    pub death: f64,
    /// The station body this ship is trading with (F689's `+0x100`, F762's
    /// choice), when it has one.
    pub station: Option<usize>,
    /// F650's bay (`+0xd0..`), held from the request until the departure's F651.
    pub bay: Option<u16>,
    /// The bay's point in the system frame (F650's offset), when the station
    /// supplied one; indoor stations use their slot instead.
    pub pad: Option<[i64; 3]>,
    /// F747's remaining docked object time in game seconds.
    pub dock_timer: f64,
    /// F691's initial station-bay object time: `docked_object_seconds(F1530)`
    /// without F747's extra two days. Kept on the ship so the caller can occupy
    /// the bay and then start the countdown.
    pub dock_span: f64,
    /// F745/F734's retry delay after traffic control said no.
    pub dock_retry: f64,
    /// Seconds spent waiting for a bay or clearance.
    pub dock_wait: f64,
    /// `+0xa0`: the word a mission script gave the ship (F672's name seed or
    /// target 0x38).
    #[serde(default)]
    pub seed: i32,
    /// `+0xfe` as a mission script set it (target 0x41, statements 41-43
    /// and 60), in the scripts' ids.
    #[serde(default)]
    pub script_target: i32,
    /// The `+0xff` code a mission script set (target 0x3e), reported while
    /// the ship is still in the state it stands for.
    #[serde(default)]
    pub ai_code: Option<u8>,
    /// F1533's class byte (`+0x14c`): light index (0..7), 0x10 dockable,
    /// 0x20 starport, 0x40 open air, 0x80 complex shape. Stored from the model
    /// records, not inferred from the role.
    #[serde(default)]
    pub class_flags: u8,
    /// F1533's collision radius (`+0x140`). Zero in saves older than the field;
    /// the record then reports [`Self::radius`].
    #[serde(default)]
    pub collision_radius: i64,
    /// `+0xd0`: the statistics' drive class.
    #[serde(default)]
    pub drive: u8,
    /// `+0x119`: cargo fuel in tonnes, F773's outfit draw. F776/F901 reads it
    /// when the ship picks a jump destination and spends what the jump needs.
    #[serde(default)]
    pub cargo_fuel: u8,
    /// `+0xcc`: F953's damaged-equipment flags.
    #[serde(default)]
    pub damaged: u32,
    /// `+0x101`: 0-2 depending on retrospective power, from the AI's thrust
    /// words (F601 reads it to pick the retro acceleration).
    #[serde(default)]
    pub retro: u8,
    /// `+0x14d`: the frame-of-reference update timeout.
    #[serde(default)]
    pub for_timeout: u8,
    /// `+0x14e`: the smoke timeout.
    #[serde(default)]
    pub smoke_timeout: u8,
    /// `+0x14f`: the E.C.M./energy-bomb timeout.
    #[serde(default)]
    pub ecm_timeout: u8,
    /// `+0x150`: the autopilot avoidance-check timeout.
    #[serde(default)]
    pub avoid_timeout: u8,
    /// `+0x56`: the frame of reference F606 last chose (an object slot; zero
    /// for the root). The port keeps [`Self::position`] in the system frame and
    /// stores the chosen frame's offset separately, so the record can report the
    /// source's relative reading.
    #[serde(default)]
    pub frame_parent: u8,
    /// `+0x3e` relative to [`Self::frame_parent`]: `position - parent.position`.
    #[serde(default)]
    pub frame_position: [i64; 3],
    /// F732's `+0x14d` reached below zero this tick, so F606 must run on the
    /// ship. Cleared by [`Self::take_frame_update`].
    #[serde(skip)]
    pub frame_due: bool,
    /// F610's `+0x150` reached zero this tick: F802's collision check is due.
    /// Cleared by [`Self::take_avoid_due`].
    #[serde(skip)]
    pub avoid_due: bool,
    /// F610's avoidance aim for the next guidance step (away from the
    /// obstruction F802 found). The port keeps it here rather than in the
    /// transient `+0x108..` words until F597 recomputes them.
    #[serde(skip)]
    pub avoid_target: Option<[i64; 3]>,
    /// The `+0xf2..+0xfd` union read as a jump (F738/F761): the destination,
    /// range, post-jump AI mode and time. Ordinary objects keep their motion
    /// caps there instead; [`Self::object_record`] writes this reading while
    /// the ship flees, waits out a carried jump or has escaped.
    #[serde(default)]
    pub jump: crate::object_record::HyperspaceWords,
    /// `+0xa8`/`+0xac`: the object time F738 adds `+0xfa` to and F737/F740/F764
    /// compare with the game clock (low word first, as `clock_words`).
    #[serde(default)]
    pub jump_clock: [u32; 2],
    /// Game seconds since F732/F938's last one-second pass (the source calls
    /// them once per event-6 second).
    #[serde(default)]
    pub timer_clock: f64,
}

impl NpcShip {
    /// Fit what the original's F773 drew for this ship instead of the port's own equipment pass:
    /// the front laser byte (`+0xd2`), the pylon words (`+0xd6`, 1-based) and the equipment dword
    /// (`+0xc8`). The drive class (`+0xd0`) and cargo fuel (`+0x119`) are F773's too. The executed
    /// original's police carry a 0xa0 beam and no missiles.
    pub fn apply_outfit(&mut self, outfit: &crate::spawn::Outfit) {
        self.laser = (outfit.laser != 0).then_some(outfit.laser);
        for (index, pylon) in self.pylons.iter_mut().enumerate() {
            *pylon = outfit.pylons.get(index + 1).copied().unwrap_or(0);
        }
        self.general = outfit.equipment;
        self.drive = outfit.drive;
        self.cargo_fuel = outfit.fuel;
    }

    /// A ship of `spec` at `position`, orbiting `anchor` (the traffic spawn's constructor).
    pub fn new(
        spec: &Spec,
        role: Role,
        position: [i64; 3],
        anchor: [i64; 3],
        rng: &mut Random,
    ) -> Self {
        let shields = spec.shield_max();
        let phase = (rng.bounded(65_536) as f64) / 65_536.0 * std::f64::consts::TAU;
        let orbit_radius = ANCHOR_ORBIT + f64::from(rng.bounded(1_000_000));
        let hostile = role.hostile();
        let laser = role.laser_code(spec, rng);
        // F773's equipment pass runs after the model's laser choice; the ship's
        // own F1530 stream starts from the spawn draw.
        let (general, pylons) = fit_equipment(spec, role, laser, rng);
        let seed = rng.bounded(0x1_0000);
        Self {
            id: 0,
            slot: 0,
            model: spec.model,
            role,
            name: spec.name.clone(),
            type_name: spec.name.clone(),
            position,
            velocity: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            radius: spec.radius,
            damage: DamageState {
                shields,
                shields_max: shields,
                hull: spec.hull(),
                mass: spec.mass,
            },
            shield_strength: spec.shield_strength as i16,
            rating: spec.rating,
            cargo: spec.cargo,
            state: if hostile { State::Attack } else { State::Hold },
            anchor,
            orbit_radius,
            phase,
            max_speed: spec.top_speed(),
            accel: spec.accel(),
            turn_rate: TURN_RATE / f64::from(turn_divisor(spec.model)),
            limits: spec.limits,
            motion: crate::physics::Motion {
                model: spec.model,
                drift: 0x3c,
                ..crate::physics::Motion::default()
            },
            speed: 0.0,
            hostile,
            laser,
            guns: laser::State {
                refire: 0,
                heat: 0,
                energy: drive_energy(spec.drive),
            },
            general,
            pylons,
            missile_rng: crate::RandomState::new(seed, seed.rotate_left(13) ^ 0x4567_89ab),
            control_clock: 0.0,
            think_tics: THINK_TICS,
            scrambled: false,
            scramble_tics: 0,
            attack_phase: 0,
            in_fudge: false,
            pass_hits: 0,
            combat_velocity: [0; 3],
            waypoint: [0; 3],
            pursuit: Pursuit::default(),
            placed: false,
            turned: false,
            missile_target: None,
            ecm_attempted: false,
            launch: None,
            ecm_burst: None,
            bounty: if role == Role::Pirate {
                pirate_bounty(rng, false)
            } else {
                0
            },
            leader: None,
            formation: [0; 3],
            pass: 0.0,
            flee: 0.0,
            responding: false,
            in_combat: false,
            alert: false,
            npc_target: None,
            contract: None,
            flash: 0.0,
            shot: None,
            death: 0.0,
            station: None,
            bay: None,
            pad: None,
            dock_timer: 0.0,
            dock_span: 0.0,
            dock_retry: 0.0,
            dock_wait: 0.0,
            seed: 0,
            script_target: 0,
            ai_code: None,
            class_flags: spec.class,
            collision_radius: spec.collision_radius,
            drive: spec.drive,
            cargo_fuel: 0,
            damaged: 0,
            retro: 0,
            for_timeout: 0,
            smoke_timeout: 0,
            ecm_timeout: 0,
            avoid_timeout: 0,
            frame_parent: 0,
            frame_position: [0; 3],
            frame_due: false,
            avoid_due: false,
            avoid_target: None,
            jump: crate::object_record::HyperspaceWords::default(),
            jump_clock: [0; 2],
            timer_clock: 0.0,
        }
    }

    /// `+0x140`: the collision radius, falling back to the interaction radius in
    /// saves made before the field existed.
    pub fn collision_radius(&self) -> i64 {
        if self.collision_radius > 0 {
            self.collision_radius
        } else {
            self.radius
        }
    }

    /// A source-shaped `PhysObj` record for this ship, as the game keeps it in
    /// the 0x152-byte object list (`object_record`). The slot is the ship's
    /// stored `+0x86`; the source-only fields (class, drive, damaged equipment,
    /// timeouts, retro power, jump words) are the ones the live ship carries.
    /// `+0xfe`/`+0x100` stay zero: use [`Self::object_record_in`] with the live
    /// list's [`crate::object_record::ObjectIndexes`].
    pub fn object_record(&self) -> ObjectRecord {
        self.object_record_in(&crate::object_record::ObjectIndexes::default())
    }

    /// [`Self::object_record`] with the live object list's indexes: `+0x86` from
    /// [`crate::object_record::ShipSlots`], `+0xfe` from the ship's enemy or its
    /// station, `+0x100` from its destination body.
    pub fn object_record_in(
        &self,
        indexes: &crate::object_record::ObjectIndexes<'_>,
    ) -> ObjectRecord {
        use crate::object_record::{AgentRecord, ObjectRecord, CONTROL_AI_SHIP};
        let slot = indexes
            .slots
            .and_then(|slots| slots.slot_of(self.id))
            .unwrap_or(self.slot);
        // `+0xfe` is the starport/docked object index for a ship in the traffic
        // cycle, else the AI's enemy (F695/F750). `+0x100` is the navigation
        // target, the destination body.
        let station = indexes.station;
        let target = if self.state.docking() || self.bay.is_some() || self.pad.is_some() {
            station.unwrap_or(0)
        } else if let Some(missile) = self.missile_target {
            // F941/F757: an evading ship's `+0xfe` is the missile object, and
            // that replaces any ship target it was fighting.
            indexes
                .slots
                .and_then(|slots| slots.slot_of_missile(missile))
                .unwrap_or(0)
        } else if let Some(enemy) = self.npc_target {
            indexes
                .slots
                .and_then(|slots| slots.slot_of(enemy))
                .unwrap_or(0)
        } else if self.state_hunts() {
            indexes.player.unwrap_or(0)
        } else {
            0
        };
        let mut agent = AgentRecord {
            target,
            ai_mode: self.ai_state(),
            nav_target: station.unwrap_or(0),
            retro: self.retro,
            capacity: self.cargo,
            kind: self.role.ai_class(),
            cargo_fuel: self.cargo_fuel,
            bounty: self.bounty,
            fuel: self.guns.energy,
            radius: self.radius,
            collision_radius: self.collision_radius(),
            physics_timeout: self.motion.switch_delay.max(0) as u32,
            flags: self.class_flags,
            for_timeout: self.for_timeout,
            smoke_timeout: self.smoke_timeout,
            ecm_timeout: self.ecm_timeout,
            avoid_timeout: self.avoid_timeout,
            laser_flags: (if self.flash > 0.0 {
                crate::object_record::LASER_SHIELDS_HIT
            } else {
                0
            }) | (if self.shot.is_some() {
                crate::object_record::LASER_FIRED | crate::object_record::LASER_FRONT
            } else {
                0
            }),
            ..AgentRecord::default()
        };
        agent.set_name(&self.name);
        let mut record = ObjectRecord {
            orientation: self.motion.orientation,
            drift: self.motion.drift,
            position_valid: 1,
            // F606's frame: `+0x3e` is relative to `+0x56`. The port keeps the
            // absolute position in `Self::position` and reports the source's
            // reading from the frame the ship's `+0x14d` pass chose.
            position: if self.frame_parent == 0 {
                self.position
            } else {
                self.frame_position
            },
            parent: self.frame_parent,
            rotating: self.motion.rotating,
            orientation_valid: self.motion.updated,
            model: self.model,
            object: slot,
            control: CONTROL_AI_SHIP,
            velocity: self.motion.velocity,
            rates: self.motion.rate,
            thrust: self.motion.thrust,
            equipment: self.general,
            damaged: self.damaged,
            drive: self.drive,
            gun_mounts: self.motion.gun_mounts as u8,
            lasers: [self.laser.unwrap_or(0), 0, 0, 0],
            pylons: self.pylons,
            shields_total: self.damage.shields_max,
            shields: self.damage.shields,
            hull: self.damage.hull,
            refire: self.guns.refire,
            heat: self.guns.heat,
            fraction: self.motion.fraction,
            agent,
            ..ObjectRecord::default()
        };
        if matches!(self.state, State::Flee | State::Escaped | State::Jump) {
            // F682/F738 read `+0xf2..+0xfd` as the jump: the destination,
            // range, post-jump mode and time F775/F776 stored. F738/F682 also
            // write the object time to `+0xa8`/`+0xac` (the `+0xa8` the source
            // calls the creation time for ordinary objects).
            record.set_hyperspace(self.jump);
            record.clock_tics = self.jump_clock[0];
            record.clock_days = self.jump_clock[1];
        } else {
            record.set_caps(self.motion.cap);
        }
        record
    }

    pub fn alive(&self) -> bool {
        !matches!(self.state, State::Dead | State::Escaped)
    }

    /// The `+0xff` code the scripts read (token 0x3e): the code a script
    /// set while the ship is still in that state, else the state's first.
    pub fn ai_state(&self) -> u8 {
        self.ai_code
            .filter(|code| State::from_original(*code) == Some(self.state))
            .unwrap_or_else(|| self.state.original().first().copied().unwrap_or(0))
    }

    /// A mission script sets the ship's AI state `code` (target 0x3e) with
    /// its target `target` (`+0xfe`): `Some(None)` the player, `Some(Some(id))`
    /// another ship, `None` unchanged (an attack then closes on the player).
    pub fn set_ai(&mut self, code: u8, target: Option<Option<u32>>) {
        let Some(state) = State::from_original(code) else {
            return;
        };
        self.ai_code = Some(code);
        self.state = state;
        match state {
            State::Attack | State::Pass => match target {
                Some(Some(ship)) => self.npc_target = Some(ship),
                _ => {
                    self.hostile = true;
                    self.npc_target = None;
                }
            },
            State::Trading => {
                // F770 flies at `+0xfe`; the caller passes that target's
                // position to the tick.
                if let Some(target) = target {
                    self.npc_target = target;
                }
            }
            State::Escort => {
                if let Some(target) = target {
                    self.leader = target;
                    if target.is_none() {
                        self.formation = [0; 3];
                    }
                }
            }
            State::Hold => {
                self.anchor = self.position;
                self.orbit_radius = 0.0;
            }
            _ => {}
        }
    }

    /// Whether the ship should be drawn, locked or hit this frame. A docked or
    /// just-launched ship is inside the station (F747 hides the object and the
    /// bay is not in the outside view); a jump cloud (`State::Jump`, the
    /// carried ship's `+0x9a` model) is not a ship either.
    pub fn visible(&self) -> bool {
        self.alive() && !matches!(self.state, State::Docked | State::Launch | State::Jump)
    }

    /// F935's equipment test for an AI ship, in the source's order: Naval
    /// E.C.M. (`+0xca` bit 7) first, then the standard system (bit 3).
    pub fn fitted_ecm(&self) -> Option<crate::equipment::Ecm> {
        if self.general & crate::equipment::NAVAL_ECM_BIT != 0 {
            Some(crate::equipment::Ecm::Naval)
        } else if self.general & crate::equipment::ECM_BIT != 0 {
            Some(crate::equipment::Ecm::Standard)
        } else {
            None
        }
    }

    /// F941's `+0xff = 0x0c` side effect: a missile is now aimed at this ship.
    /// Only a live, non-docked target goes evasive.
    pub fn evade_missile(&mut self, missile: u32) {
        if self.alive() && !self.state.docking() && self.state != State::Evade {
            self.state = State::Evade;
            self.missile_target = Some(missile);
            self.ecm_attempted = false;
        }
    }

    /// F780/F781: the missile this ship was evading died. The source retargets
    /// its `+0xfe` to the missile's owner (the player) and returns to state 5;
    /// the port returns to [`State::Attack`].
    pub fn missile_gone(&mut self, missile: u32) {
        if self.missile_target == Some(missile) {
            self.missile_target = None;
            self.ecm_attempted = false;
            if self.state == State::Evade {
                self.state = State::Attack;
            }
        }
    }

    /// Take this tick's F941 launch, if any.
    pub fn take_launch(&mut self) -> Option<MissileLaunch> {
        self.launch.take()
    }

    /// Take this tick's F935 bust, if any; `true` is the Naval system.
    pub fn take_ecm(&mut self) -> Option<bool> {
        self.ecm_burst.take()
    }

    /// F754: roll for a launch at `target` and, on success, F941's pylon clear.
    /// `state` 1/2's distance band is checked by the caller. Returns the launch
    /// for `ffe-rs` to turn into a `Spec`.
    fn attempt_launch(&mut self, target: u32) {
        let roll = self.missile_rng.next_u16();
        if roll >= LAUNCH_GATE {
            return;
        }
        let pylon = (roll as usize & 7) + 1;
        let code = self.pylons[pylon - 1];
        if code == 0 {
            // F941 returned 0, so F754 calls F937. The energy-bomb blast is not
            // modelled; the rare fitted bit is still set and reported.
            return;
        }
        self.pylons[pylon - 1] = 0;
        self.launch = Some(MissileLaunch {
            owner: self.id,
            target,
            code,
            pylon,
            position: self.position,
            axes: self.axes(),
            launcher_radius: self.radius,
        });
    }

    /// F650/F745: the station gave this ship a bay; fly the final leg to it.
    pub fn begin_approach(&mut self, bay: u16, pad: Option<[i64; 3]>) {
        if matches!(self.state, State::Transit | State::Approach) {
            self.state = State::Approach;
            self.bay = Some(bay);
            self.pad = pad;
            self.dock_retry = 0.0;
            self.dock_wait = 0.0;
        }
    }

    /// F652 refused the touch (another ship is current, or the clearance ran
    /// out): go back to transit and ask again.
    pub fn touch_failed(&mut self) {
        if matches!(self.state, State::Approach | State::Docked) {
            self.state = State::Transit;
            self.bay = None;
            self.pad = None;
            self.dock_timer = 0.0;
            self.dock_retry = DOCK_RETRY_SECONDS;
        }
    }

    /// F747/F733: the ship is in its bay. Hold it at the pad and start the
    /// two-to-three day object time (F747's `+2` days and `[D8818]` fraction).
    /// The `phase` is the ship's spawn-time random draw, so the wait is stable.
    pub fn begin_docked(&mut self, seat: [i64; 3]) {
        let fraction = self.phase / std::f64::consts::TAU;
        let timer = DOCKED_DAYS * 86_400.0 + fraction * DOCKED_JITTER_SECONDS;
        self.begin_docked_for(seat, timer);
    }

    /// [`Self::begin_docked`] with an explicit object time. F691's initial bay
    /// ships use this with [`Self::dock_span`] (no F747 `+2` days).
    pub fn begin_docked_for(&mut self, seat: [i64; 3], seconds: f64) {
        self.state = State::Docked;
        self.position = seat;
        self.velocity = [0.0; 3];
        self.speed = 0.0;
        self.dock_timer = seconds.max(0.0);
        self.dock_retry = 0.0;
        self.dock_wait = 0.0;
    }

    /// F751 (state 0x0b): the leader has entered its attack state, so the
    /// escort drops its formation offsets and attacks the leader's target. The
    /// port's [`State::Attack`] always means the player, which is F751's
    /// `+0x100` for the pirate group. It keeps the 0x0a/0x0b rows via
    /// [`State::Escort`] until this hand-off fires.
    pub fn join_attack(&mut self) -> bool {
        if self.leader.is_none() || !matches!(self.state, State::Escort) {
            return false;
        }
        self.state = State::Attack;
        self.leader = None;
        self.formation = [0; 3];
        self.pass = 0.0;
        true
    }

    /// F735: the door is open and the bay is free; fly out.
    pub fn begin_departure(&mut self, seat: [i64; 3]) {
        self.state = State::Depart;
        self.position = seat;
        self.velocity = [0.0; 3];
        self.speed = 0.0;
        self.bay = None;
        self.pad = None;
        self.dock_timer = 0.0;
    }

    /// F761/F762: head for station `station` and ask for a bay there.
    pub fn begin_transit(&mut self, station: usize, anchor: [i64; 3]) {
        self.state = State::Transit;
        self.station = Some(station);
        self.anchor = anchor;
        self.bay = None;
        self.pad = None;
        self.dock_timer = 0.0;
        self.dock_retry = 0.0;
        self.dock_wait = 0.0;
    }

    /// F925: no station to trade with; leave the system (the HS cloud is the
    /// port's `Escaped` marker).
    pub fn escape(&mut self) {
        self.state = State::Escaped;
        self.velocity = [0.0; 3];
    }

    /// F738: the ship becomes a hyperspace cloud until the object time
    /// `+0xa8`/`+0xac` runs out. The jump time is added to the clock
    /// (`+0xa8 += +0xfa << 16`, `+0xac += +0xfa >> 16`) and F764/F740 wait for
    /// it; `now` is the game clock in `clock_words` order. `+0xf4`/`+0xf8`'s
    /// `post_mode` comes from the caller so F682/F761 keep it.
    pub fn begin_jump(&mut self, words: crate::object_record::HyperspaceWords, now: [u32; 2]) {
        self.jump = words;
        self.jump_clock = [
            now[0].wrapping_add(words.jump_time.wrapping_shl(16)),
            now[1].wrapping_add(words.jump_time >> 16),
        ];
        if words.jump_time > 0 {
            // F764/F740's object time is the jump itself, in game seconds.
            self.flee = jump_seconds(words.jump_time);
        }
    }

    /// F761/F738: a ship with a reachable destination leaves the system. The
    /// mode it is in is saved in `+0xf8` before F738 turns it into the
    /// entrance cloud that F764 removes at the object time.
    pub fn jump_out(&mut self, words: crate::object_record::HyperspaceWords, now: [u32; 2]) {
        let post_mode = self.ai_state();
        self.state = State::Flee;
        self.speed = 0.0;
        self.velocity = [0.0; 3];
        self.begin_jump(
            crate::object_record::HyperspaceWords { post_mode, ..words },
            now,
        );
    }

    /// `+0x14e != 0`: F725/F727/F729 emit F939's hit particle at the object
    /// every frame while the smoke timeout is non-zero.
    pub fn smoking(&self) -> bool {
        self.smoke_timeout != 0
    }

    /// F939's smoke particle for this frame, drawn from the shared F1530 pair in
    /// object order as F725/F727/F729 call it (two words per frame). `None`
    /// while the ship is not smoking. This is a deliberate RNG-ledger change:
    /// the source draws these words on the frame's pair.
    pub fn smoke_spark(&self, rng: &mut Random) -> Option<crate::effects::Effect> {
        if !self.smoking()
            || !self.alive()
            || matches!(self.state, State::Docked | State::Launch | State::Jump)
        {
            return None;
        }
        Some(crate::effects::hit(
            self.position,
            self.velocity,
            self.axes(),
            self.speed,
            rng,
        ))
    }

    /// F732: `+0x14d` reached below zero this tick, so F606 must run.
    pub fn take_frame_update(&mut self) -> bool {
        std::mem::take(&mut self.frame_due)
    }

    /// F606: choose the frame this ship belongs in from the live object list
    /// and store the chosen parent and its relative position. Returns the
    /// parent's slot (zero for the root).
    pub fn update_frame_of_reference(&mut self, candidates: &[crate::frames::FrameBody]) -> u8 {
        // F606 scans the live object list; the ship itself cannot be its own
        // frame, so its own slot is not a candidate.
        let filtered;
        let candidates = if candidates.iter().any(|body| body.slot == self.slot) {
            filtered = candidates
                .iter()
                .filter(|body| body.slot != self.slot)
                .copied()
                .collect::<Vec<_>>();
            &filtered[..]
        } else {
            candidates
        };
        let parent =
            crate::frames::select_frame_of_reference(self.position, self.frame_parent, candidates);
        self.frame_parent = parent;
        self.frame_position = if parent == 0 {
            self.position
        } else {
            match candidates.iter().find(|body| body.slot == parent) {
                Some(body) => std::array::from_fn(|axis| {
                    self.position[axis].wrapping_sub(body.position[axis])
                }),
                None => self.position,
            }
        };
        parent
    }

    /// F610: `+0x150` reached zero, so F802's collision check is due.
    pub fn take_avoid_due(&mut self) -> bool {
        std::mem::take(&mut self.avoid_due)
    }

    /// F610/F802: steer away from `obstacle`. The source replaces the thrust
    /// direction with the collision normal plus a quarter of the desired
    /// direction and re-arms the check (`+0x150 = 0`); the port aims the next
    /// guidance step at a point that way.
    pub fn steer_avoid(&mut self, obstacle: [i64; 3], reach: f64) {
        let away = normalise(std::array::from_fn(|axis| {
            (self.position[axis] - obstacle[axis]) as f64
        }));
        // DATA_008721..23 >> 0x11 + dir >> 2: mostly the normal, plus the
        // original heading so the ship keeps making progress.
        let desired = normalise(std::array::from_fn(|axis| {
            away[axis] + self.forward[axis] * 0.25
        }));
        let distance = distance_between(self.position, obstacle).max(reach);
        self.avoid_target = Some(add(
            self.position,
            desired.map(|value| (value * distance) as i64),
        ));
        // F597 recomputes the thrust words on this call.
        self.think_tics = THINK_TICS;
        self.avoid_timeout = 0;
    }

    /// Whether the ship attacks the player. Pirates and Thargoids start true;
    /// police and military turn true when the player is wanted ([`Self::provoke`]).
    pub fn is_hostile(&self) -> bool {
        self.hostile
    }

    /// The player attacked a clean ship: the police response. F783 clears the
    /// original's `DATA_008647` alert when a police ship's target is gone; the
    /// port's counterpart is that police and military ships engage a wanted
    /// player. Traders and shuttles do not fight back.
    pub fn provoke(&mut self) -> bool {
        if !self.alive() || self.hostile || !matches!(self.role, Role::Police | Role::Military) {
            return false;
        }
        self.hostile = true;
        self.state = State::Attack;
        self.pass = 0.0;
        self.flee = 0.0;
        self.responding = true;
        true
    }

    /// The wanted bit went clear (F783, or F238 running out of police): the executed original
    /// puts every police ship that was attacking into state 8, which flies it home, in the
    /// frame it clears. A police ship still waiting in the starport (state 0x22) stays.
    pub fn stand_down(&mut self) {
        if self.alive() && self.role == Role::Police && self.hostile {
            self.hostile = false;
            self.responding = false;
            self.in_combat = false;
            self.state = State::Transit;
            self.station = None;
        }
    }

    /// Whether a scrambled police ship turned hostile since the last call (F748).
    pub fn take_turned(&mut self) -> bool {
        std::mem::take(&mut self.turned)
    }

    /// F750's battle alert since the last call: this ship entered combat
    /// with the player.
    pub fn take_alert(&mut self) -> bool {
        std::mem::take(&mut self.alert)
    }

    /// The right/up/forward axes, as `screen` and the camera use them.
    pub fn axes(&self) -> [[f64; 3]; 3] {
        [
            normalise(cross(self.up, self.forward)),
            self.up,
            self.forward,
        ]
    }

    /// F953: shields first, then hull. A shield hit sets the F1536 flash flag
    /// (`+0x151 & 1`); F942's destruction removes the ship after its marker.
    pub fn hit(&mut self, damage: u32) -> Damage {
        if !self.alive() {
            return Damage::Hull;
        }
        let result = laser::apply_damage(&mut self.damage, damage);
        match result {
            Damage::Shields => self.flash = FLASH_SECONDS,
            // F953 sets `+0x14e = 0xa` in the hull-damage branch (ffe.asm
            // 0x3e3a4): the ship smokes for ten seconds of F938's countdown.
            Damage::Hull => self.smoke_timeout = SMOKE_TIMEOUT,
            Damage::Destroyed => {
                self.smoke_timeout = SMOKE_TIMEOUT;
                self.state = State::Dead;
                self.death = DEATH_SECONDS;
                self.flash = 0.0;
            }
        }
        result
    }

    pub fn hull_per_mille(&self) -> i32 {
        laser::hull_per_mille(&self.damage)
    }

    pub fn shield_percent(&self) -> i32 {
        laser::shield_percent(&self.damage)
    }

    /// The remaining hull as a fraction of `mass * 4`.
    pub fn hull_fraction(&self) -> f64 {
        let full = self.damage.mass.saturating_mul(4).max(1);
        f64::from(self.damage.hull.min(full)) / f64::from(full)
    }

    /// One frame of the flight AI: F730/F938's laser upkeep, F585's limited
    /// turn, F1512's aim and F597's "can it stop?" speed rule. `dt` is
    /// accelerated world seconds, `tics` the frame's game tics, `player` the
    /// player's position in the system frame and `escort` an escort's station in
    /// its leader's frame (F695's `+0x102..+0x106`), when it has one.
    pub fn tick(&mut self, dt: f64, tics: u32, player: [i64; 3], escort: Option<[i64; 3]>) {
        let mut requests = Vec::new();
        self.tick_with_station(dt, tics, player, escort, None, &mut requests);
    }

    /// [`Self::tick`] with the station-traffic cycle. `station` is the current
    /// station's snapshot (its face, the bay F650 gave this ship, F657's
    /// clearance and F865's door); requests are appended for the caller to apply
    /// to its `traffic::Starport`.
    pub fn tick_with_station(
        &mut self,
        dt: f64,
        tics: u32,
        player: [i64; 3],
        escort: Option<[i64; 3]>,
        station: Option<&Station>,
        requests: &mut Vec<DockRequest>,
    ) {
        if !self.alive() {
            self.death = (self.death - dt).max(0.0);
            return;
        }
        self.flash = (self.flash - dt).max(0.0);
        if let Some(mark) = self.shot.as_mut() {
            mark.seconds -= dt;
            if mark.seconds <= 0.0 {
                self.shot = None;
            }
        }
        laser::tick_refire(&mut self.guns, tics);
        laser::cool(&mut self.guns, dt, false);
        self.dock_retry = (self.dock_retry - dt).max(0.0);
        // F732/F938 are event-6 updates: one pass per game second. F732 counts
        // `+0x14d` down from 0x13 and then runs F606's frame-of-reference
        // update; F938 counts the smoke (`+0x14e`) and E.C.M./bomb (`+0x14f`)
        // timeouts down to zero.
        self.timer_clock += dt;
        while self.timer_clock >= 1.0 {
            self.timer_clock -= 1.0;
            self.smoke_timeout = self.smoke_timeout.saturating_sub(1);
            self.ecm_timeout = self.ecm_timeout.saturating_sub(1);
            if self.for_timeout == 0 {
                // F606: the period is the source's; the chosen frame is applied
                // by the caller through `update_frame_of_reference` (it needs
                // the system's live object list).
                self.frame_due = true;
                self.for_timeout = FOR_TIMEOUT;
            } else {
                self.for_timeout -= 1;
            }
        }
        // F610's avoidance cadence (`+0x150`): the autopilot collision check
        // runs every 16 navigation frames at 1x and resets the counter. F610
        // sets `+0x150 = 0` again after an avoidance, forcing the next frame's
        // check; the caller decides what F802 found.
        if matches!(self.state, State::Transit | State::Depart | State::Escort) {
            if self.avoid_timeout == 0 {
                self.avoid_due = true;
                self.avoid_timeout = AVOID_TIMEOUT;
            } else {
                self.avoid_timeout -= 1;
            }
        }
        // The original's jump-out states hide the model and move it out
        // (F738/F787). The low-hull trigger is the port's.
        if self.hostile
            && matches!(self.state, State::Attack | State::Pass)
            && self.hull_fraction() <= FLEE_HULL
        {
            // F682 stores the AI mode it is leaving in `+0xf8` before the
            // object switches to the jump state; F761 only fills the jump
            // words when F775 finds a destination.
            self.jump.post_mode = self.ai_state();
            self.jump.jump_time = 0;
            self.jump_clock = [0; 2];
            self.state = State::Flee;
            self.flee = FLEE_SECONDS;
        }
        // F737/F740: a carried ship waits out its `+0xfa` in the exit cloud and
        // returns in the `+0xf8` mode F682 saved (F740 clears `+0xf2..+0xfa`).
        if self.state == State::Jump {
            self.speed = 0.0;
            self.velocity = [0.0; 3];
            self.flee -= dt;
            if self.flee <= 0.0 {
                self.state = State::from_original(self.jump.post_mode).unwrap_or(State::Attack);
                self.jump = crate::object_record::HyperspaceWords::default();
                self.jump_clock = [0; 2];
                self.flee = 0.0;
            }
            return;
        }
        if self.state == State::Flee {
            self.flee -= dt;
            if self.flee <= 0.0 {
                self.state = State::Escaped;
                self.velocity = [0.0; 3];
                return;
            }
        }
        if self.state == State::Pass && !self.original_hunt() {
            self.pass -= dt;
            if self.pass <= 0.0 {
                self.state = State::Attack;
            }
        }
        // F754/F757 are event-6 controllers: their random rolls run once per
        // 20 ms of game time, not once per rendered frame.
        self.control_clock += dt;
        while self.control_clock >= CONTROL_SECONDS {
            self.control_clock -= CONTROL_SECONDS;
            match self.state {
                // The original's hunting AI rolls F754 on its own step (`hunt_step`).
                State::Attack | State::Pass if self.original_hunt() => {}
                State::Attack | State::Pass => {
                    let range = distance_between(self.position, player);
                    let (min, max) = if self.state == State::Pass {
                        (0.0, MISSILE_LAUNCH_PASS_MAX)
                    } else {
                        (MISSILE_LAUNCH_MIN, MISSILE_LAUNCH_MAX)
                    };
                    if self.is_hostile() && (min..max).contains(&range) {
                        self.attempt_launch(crate::missile::PLAYER_TARGET);
                    }
                }
                State::Evade => {
                    // F757: `F1530() > 0xa000` calls F935 when the ship has an
                    // E.C.M. system. One burst is enough to end the evasive
                    // state for the models the table defeats; the source may
                    // repeat the roll while the missile survives.
                    let burst = if !self.ecm_attempted && self.missile_rng.next_u16() > ECM_GATE {
                        self.fitted_ecm()
                    } else {
                        None
                    };
                    if let Some(ecm) = burst {
                        self.ecm_burst = Some(ecm == crate::equipment::Ecm::Naval);
                        self.ecm_attempted = true;
                    }
                }
                _ => {}
            }
        }
        // F747/F733: in the bay. The object stays on its pad while the station's
        // own motion carries it (the pad snapshot is in the system frame), and
        // F733's object time counts down. At zero F734 asks F654 for the door.
        if self.state == State::Docked {
            self.speed = 0.0;
            self.velocity = [0.0; 3];
            let Some(station) = station else {
                self.touch_failed();
                requests.push(DockRequest::ChooseStation {
                    ship: self.id,
                    station: self.station.unwrap_or(0),
                });
                return;
            };
            self.position = station
                .pad
                .or(self.pad)
                .unwrap_or_else(|| station.bay_point());
            // F747 waits for F652's mode 3 to end (the door closed) and then
            // calls F653; only then does F733's object time run.
            if station.closing {
                return;
            }
            if station.cleared && self.bay.is_some() {
                requests.push(DockRequest::Settle {
                    ship: self.id,
                    station: station.index,
                });
                return;
            }
            self.dock_timer = (self.dock_timer - dt).max(0.0);
            if self.dock_timer <= 0.0 {
                self.dock_wait += dt;
                if self.dock_retry <= 0.0 {
                    requests.push(DockRequest::Launch {
                        ship: self.id,
                        station: station.index,
                    });
                    self.dock_retry = DOCK_RETRY_SECONDS;
                }
                // The source retries F654 for ever; after the port's give-up the
                // bay is freed (F651) and F762 picks another station.
                if self.dock_wait >= DOCK_WAIT_SECONDS {
                    requests.push(DockRequest::Release {
                        ship: self.id,
                        station: station.index,
                    });
                    requests.push(DockRequest::ChooseStation {
                        ship: self.id,
                        station: station.index,
                    });
                    self.state = State::Transit;
                    self.bay = None;
                    self.pad = None;
                    self.dock_wait = 0.0;
                }
            }
            return;
        }
        // F734/F735: launch clearance is in the record; wait for the door word
        // (F865) to reach zero, then F651 frees the bay and F736 flies out.
        if self.state == State::Launch {
            self.speed = 0.0;
            self.velocity = [0.0; 3];
            let Some(station) = station else {
                self.touch_failed();
                return;
            };
            self.dock_wait += dt;
            self.position = station
                .pad
                .or(self.pad)
                .unwrap_or_else(|| station.bay_point());
            if self.scrambled {
                self.scramble_tics = self
                    .scramble_tics
                    .saturating_add((dt * TICS_PER_SECOND).round().max(0.0) as u32);
            }
            let step = self.scrambled && self.scramble_tics >= THINK_TICS;
            if step {
                self.scramble_tics %= THINK_TICS;
            }
            if station.door_open && self.scrambled && !step {
                // F748 only looks at the door on the ship's own step.
            } else if station.door_open && self.scrambled {
                // F748: the record's mode is 0xfe or 0, the door is open: state 1.
                self.scrambled = false;
                self.turned = true;
                self.hostile = true;
                self.responding = true;
                self.state = State::Attack;
                self.position = station.launch_point();
                self.station = None;
                self.dock_wait = 0.0;
            } else if station.door_open {
                let seat = station
                    .pad
                    .or(self.pad)
                    .unwrap_or_else(|| station.bay_point());
                requests.push(DockRequest::Release {
                    ship: self.id,
                    station: station.index,
                });
                self.begin_departure(seat);
                self.dock_wait = DEPART_SECONDS;
            } else if !self.scrambled && self.dock_wait >= DOCK_WAIT_SECONDS {
                // The door never opened: free the bay and pick a new station.
                requests.push(DockRequest::Release {
                    ship: self.id,
                    station: station.index,
                });
                requests.push(DockRequest::ChooseStation {
                    ship: self.id,
                    station: station.index,
                });
                self.state = State::Transit;
                self.bay = None;
                self.pad = None;
                self.dock_wait = 0.0;
            }
            return;
        }
        // F745/F746: the final leg to the pad. A lost clearance sends the ship
        // back to transit (the traffic record timed out in F867).
        if self.state == State::Approach && station.is_some_and(|s| !s.cleared || s.bay.is_none()) {
            self.touch_failed();
        }
        if self.state == State::Transit {
            if let Some(station) = station {
                let range = distance_between(self.position, station.position);
                if range <= station.request_range() {
                    self.dock_wait += dt;
                    if self.dock_retry <= 0.0 {
                        requests.push(DockRequest::Request {
                            ship: self.id,
                            station: station.index,
                        });
                        self.dock_retry = DOCK_RETRY_SECONDS;
                    }
                    if self.dock_wait >= DOCK_WAIT_SECONDS {
                        requests.push(DockRequest::ChooseStation {
                            ship: self.id,
                            station: station.index,
                        });
                        self.dock_wait = 0.0;
                    }
                } else {
                    self.dock_wait = 0.0;
                }
            }
        }
        // The leader's frame gives F695's escorts their station.
        let target = match self.state {
            State::Trading => player,
            State::Hold => {
                self.phase += (self.max_speed * 0.35 / self.orbit_radius.max(1.0)) * dt;
                offset(self.anchor, self.phase, self.orbit_radius)
            }
            State::Transit if self.role == Role::Shuttle && station.is_none() => {
                let outward = away(self.position, self.anchor);
                add(self.anchor, outward.map(|v| (v * self.orbit_radius) as i64))
            }
            State::Transit => station.map_or(self.anchor, Station::bay_point),
            State::Approach => station.map_or(self.anchor, |station| {
                station
                    .pad
                    .or(self.pad)
                    .unwrap_or_else(|| station.bay_point())
            }),
            State::Depart => station.map_or(self.anchor, Station::launch_point),
            State::Escort => escort.map_or(player, |point| {
                if !self.hostile || distance_between(player, self.position) > 4.0 * STANDOFF {
                    point
                } else {
                    player
                }
            }),
            State::Attack | State::Pass => player,
            // F757's evasive turn: aim across the missile's line rather than at
            // the player. The source adds random roll/pitch words (F588); the
            // port turns toward a point one `EVADE_OFFSET` to the ship's right.
            State::Evade => {
                let right = self.axes()[0];
                add(player, right.map(|value| (value * EVADE_OFFSET) as i64))
            }
            State::Flee => {
                let direction = away(self.position, player);
                add(self.position, direction.map(|value| (value * 4.0e6) as i64))
            }
            State::Docked | State::Launch | State::Dead | State::Escaped | State::Jump => {
                self.position
            }
        };
        // F610's avoidance aim wins over the state's target for the guidance
        // step that follows an F802 collision (the caller set it after the
        // `+0x150` check). The source replaces the thrust direction for that
        // F597 call and clears `+0x150`; the port clears the aim here.
        let target = self.avoid_target.take().unwrap_or(target);
        let to = sub(target, self.position);
        let distance = length(to);
        // F750: a hostile attacking the player enters combat inside
        // COMBAT_RANGE; the caller drops time to 1x and sounds the alert.
        let hunting = self.hostile
            && self.npc_target.is_none()
            && matches!(self.state, State::Attack | State::Pass);
        if !hunting {
            self.in_combat = false;
        } else if !self.in_combat && distance < COMBAT_RANGE {
            self.in_combat = true;
            self.alert = true;
        }
        if let Some(limits) = self.limits {
            // The original's object physics (F727/F725): F597's guidance to the
            // state's target, F601's thrust integration within the statistics'
            // limits, F585's turn (halved for the larger models) and F1512's
            // position step. A pass aims past the target, as F756's evasion
            // flies through instead of braking onto it.
            let hunting = self.original_hunt();
            let aim = if self.state == State::Pass && !hunting {
                add(self.position, self.forward.map(|v| (v * 4.0e6) as i64))
            } else {
                target
            };
            self.fly(aim, dt, &limits);
            if !hunting && self.state == State::Attack && distance < STANDOFF {
                // F753: within the close range the attacker breaks off into
                // state 3's evasion (the port's pass) and comes round again.
                self.state = State::Pass;
                self.pass = PASS_SECONDS;
            }
        } else {
            let desired = if distance > 1.0 {
                scale(to, 1.0 / distance)
            } else {
                self.forward
            };
            self.forward = turn_toward(self.forward, desired, self.turn_rate * dt);
            self.up = upright(self.forward);
            let wanted = match self.state {
                State::Hold => {
                    (self.max_speed * 0.5).min(0.7 * (2.0 * self.accel * distance).sqrt())
                }
                State::Transit | State::Depart => {
                    if distance <= ARRIVE {
                        0.0
                    } else {
                        self.max_speed
                            .min(0.7 * (2.0 * self.accel * (distance - ARRIVE)).sqrt())
                    }
                }
                State::Trading => self
                    .max_speed
                    .min(0.7 * (2.0 * self.accel * distance).sqrt()),
                State::Approach => self
                    .max_speed
                    .min(0.7 * (2.0 * self.accel * distance).sqrt()),
                State::Attack => {
                    let room = (distance - STANDOFF).max(0.0);
                    let combat = self.max_speed.min(0.7 * (2.0 * self.accel * room).sqrt());
                    if self.responding {
                        // Called in: cruise to the player and brake into range.
                        let run = (distance - RESPONSE_ARRIVE).max(0.0);
                        combat.max(RESPONSE_SPEED.min(0.7 * (2.0 * RESPONSE_ACCEL * run).sqrt()))
                    } else {
                        combat
                    }
                }
                State::Escort => {
                    let room = (distance - STANDOFF).max(0.0);
                    self.max_speed.min(0.7 * (2.0 * self.accel * room).sqrt())
                }
                State::Pass | State::Flee | State::Evade => self.max_speed,
                State::Docked | State::Launch | State::Dead | State::Escaped | State::Jump => 0.0,
            };
            // F597: if the ship cannot stop before the standoff, it commits to the
            // pass (the original's overshoot branch) instead of braking.
            if self.state == State::Attack
                && self.speed > 0.5 * self.max_speed
                && (STANDOFF..4.0 * STANDOFF).contains(&distance)
                && self.speed * self.speed > 2.0 * self.accel * (distance - STANDOFF).max(0.0)
            {
                self.state = State::Pass;
                self.pass = PASS_SECONDS;
            }
            let accel = if self.responding {
                self.accel.max(RESPONSE_ACCEL)
            } else {
                self.accel
            };
            let change = (wanted - self.speed).clamp(-accel * dt, accel * dt);
            self.speed = (self.speed + change).max(0.0);
            self.velocity = scale(self.forward, self.speed);
            for axis in 0..3 {
                self.position[axis] += (self.velocity[axis] * dt).round() as i64;
            }
        }
        if self.state == State::Transit && distance <= ARRIVE && station.is_none() {
            self.state = State::Hold;
            self.speed = 0.0;
            self.velocity = [0.0; 3];
        }
        // F652: the pad contact. The caller's F652 decides whether this ship is
        // the record's current one, then F747 starts the docked object time.
        if self.state == State::Approach && distance <= ARRIVE {
            if let Some(station) = station {
                requests.push(DockRequest::Touch {
                    ship: self.id,
                    station: station.index,
                });
                self.speed = 0.0;
                self.velocity = [0.0; 3];
            }
        }
        // F736/F761: the departure run is over, so pick the next station or jump
        // out. The original's state 7 is already at the slot and calls F761 at
        // once; the port gives it `DEPART_SECONDS` (or the hull clearance).
        if self.state == State::Depart {
            self.dock_wait -= dt;
            if let Some(station) = station {
                if self.dock_wait <= 0.0
                    || distance_between(self.position, station.position) > station.radius + 2.0e6
                {
                    requests.push(DockRequest::ChooseStation {
                        ship: self.id,
                        station: station.index,
                    });
                    self.station = None;
                    self.state = State::Hold;
                    self.speed = 0.0;
                    self.velocity = [0.0; 3];
                }
            }
        }
    }

    /// F931 for an AI ship: the front laser's gates, then F817's ray against the
    /// player's interaction box. The shot is fired (heat, energy and refire
    /// spent) even when the ray misses, as the original's F931 does. `None` when
    /// the ship has no laser, is not hostile, or F931's gates refuse.
    pub fn fire(&mut self, tics: u32, player: [i64; 3], player_radius: i64) -> Option<Shot> {
        if !self.alive()
            || !self.is_hostile()
            || matches!(self.state, State::Evade | State::Jump | State::Trading)
        {
            return None;
        }
        // F724 (state 2) is the only state that asks F931 for a shot, once a frame.
        if self.original_hunt() && self.attack_phase != 2 {
            return None;
        }
        let code = self.laser?;
        let spec = laser::fire(code, &mut self.guns, tics).ok()?;
        let targets = [laser::Target {
            index: 0,
            centre: player,
            radius: player_radius.min(PLAYER_HIT_BOX),
        }];
        let hit = laser::hit(self.position, self.forward, &targets);
        self.shot = Some(ShotMark::new(code, hit.as_ref()));
        let hit = hit?;
        if self.original_hunt() {
            if self.pass_hits >= MAX_HITS_PER_PASS {
                return None;
            }
            self.pass_hits += 1;
        }
        Some(Shot {
            ship: self.id,
            name: self.name.clone(),
            code,
            damage: spec.damage,
            beam: spec.beam,
            point: hit.point,
            distance: hit.distance,
        })
    }

    /// A hostile hunting the player with the original's object physics: F750, F753 and F755 run
    /// its AI instead of the port's standoff approximation.
    fn original_hunt(&self) -> bool {
        self.limits.is_some() && self.state_hunts()
    }

    fn state_hunts(&self) -> bool {
        self.hostile
            && self.npc_target.is_none()
            && matches!(self.state, State::Attack | State::Pass)
    }

    /// F750 (state 4), F753 (states 1 and 2) and F755 (state 3) on the ship's AI step, after F610
    /// has stored the guidance: the distance class (F592's shift of the largest relative
    /// component) picks the state and the thrust request.
    fn hunt_step(&mut self, detail: &crate::autopilot::Detail, limits: &crate::thrust::Limits) {
        let class = detail.guidance.distance_shift;
        if self.attack_phase != 2 {
            self.pass_hits = 0;
        }
        self.in_fudge = self.attack_phase != 0 && class <= 10;
        match (self.state, self.attack_phase) {
            // F750: inside class 10 it becomes state 1; further out F599 and F587 stand.
            (State::Attack, 0) => {
                if class < 10 {
                    self.attack_phase = 1;
                    self.waypoint = [0; 3];
                }
            }
            // F755 = F732 + F756.
            (State::Pass, _) => self.break_away(detail, limits),
            // F752 = F732 + F753.
            _ => {
                if class > 10 {
                    self.reposition();
                } else if class < 5 {
                    self.state = State::Pass;
                    self.attack_phase = 3;
                    self.break_away(detail, limits);
                } else if class < 9 {
                    // Classes 5 to 8: F587 (the stored turns), F598's full deflection, F754.
                    self.motion.thrust = crate::thrust::request_clamp(limits, detail.local)
                        .words
                        .map(|w| w as i16);
                    self.f754(detail.direction);
                } else {
                    // Classes 9 and 10: state 1 with F599's clamp (already stored).
                    self.attack_phase = 1;
                }
            }
        }
    }

    /// F749: back to the interception (state 4) with a random waypoint round the target.
    fn reposition(&mut self) {
        let angle = f64::from(self.missile_rng.a & 0xffff);
        self.missile_rng.next_u16();
        // F1913's Sin16 is +-0x7fff over a 16-bit turn; the words are its value shifted right 5.
        let sin16 = |x: f64| (32767.0 * (x * std::f64::consts::TAU / 65536.0).sin()) as i32;
        self.waypoint = [
            (sin16(angle) >> 5) as i16,
            (sin16(angle + 16384.0) >> 5) as i16,
            (self.missile_rng.b >> 22) as i16,
        ];
        self.state = State::Attack;
        self.attack_phase = 0;
    }

    /// F597's accelerated placement: the ship moves to `position` with `velocity` (object
    /// units), its thrust words and turn rates cleared. Also used for an escort that moves with
    /// its placed leader.
    pub fn place_at(&mut self, position: [i64; 3], velocity: [i32; 3]) {
        self.position = position;
        self.motion.position = position;
        self.motion.velocity = velocity;
        self.motion.thrust = [0; 3];
        self.motion.rate = [0; 3];
        self.motion.autopilot = [0; 3];
        self.velocity = velocity.map(|v| f64::from(v) * MM_PER_VELOCITY_UNIT);
        self.speed = length(self.velocity);
        self.placed = true;
    }

    /// Whether F597's accelerated branch placed the ship this tick (cleared by the call).
    pub fn take_placed(&mut self) -> bool {
        std::mem::take(&mut self.placed)
    }

    /// F756 (state 3): turn away (F588) and fly forward at full thrust with the sideways thrust
    /// that cancels the velocity relative to the target (the player's own velocity is zero here),
    /// back to state 1 once the ship is beyond class 6.
    fn break_away(&mut self, detail: &crate::autopilot::Detail, limits: &crate::thrust::Limits) {
        let class = detail.guidance.distance_shift;
        let matrix = self.motion.orientation;
        self.motion.autopilot =
            crate::autopilot::face_away(matrix, detail.direction.map(|v| v as i16));
        let mul = crate::model::multiply;
        let r = self.motion.velocity;
        let up = mul(r[0], matrix[1][0])
            .wrapping_neg()
            .wrapping_add(mul(r[1], matrix[1][1]))
            .wrapping_add(mul(r[2], matrix[1][2]))
            .wrapping_shl(2);
        let right = mul(r[0], matrix[0][0])
            .wrapping_neg()
            .wrapping_add(mul(r[1], matrix[0][1]))
            .wrapping_add(mul(r[2], matrix[0][2]))
            .wrapping_shl(2);
        self.motion.thrust = crate::thrust::request_clamp(limits, [up, right, 0x7fff])
            .words
            .map(|w| w as i16);
        if class > 10 {
            self.reposition();
        } else {
            if class > 6 {
                self.state = State::Attack;
                self.attack_phase = 1;
            }
            self.f754(detail.direction);
        }
    }

    /// F754: in states 1 and 2 the ship faces the target (state 2 within 60 degrees); then a one
    /// in eight chance of a missile (F941, else F937).
    fn f754(&mut self, direction: [i32; 3]) {
        if self.attack_phase == 1 || self.attack_phase == 2 {
            let words = direction.map(|v| i32::from(v as i16));
            let unit = crate::physics::normalise(words).map(|v| v >> 16);
            let forward = self.motion.orientation[2];
            let dot = (0..3).fold(0i32, |sum, k| {
                sum.wrapping_add((forward[k] >> 16).wrapping_mul(unit[k]))
            });
            self.attack_phase = if dot > 0x2000_0000 { 2 } else { 1 };
        }
        self.attempt_launch(crate::missile::PLAYER_TARGET);
    }

    /// One frame of the original's object physics toward `target`: F597
    /// (`autopilot::guide`) turns the target into thrust words, a velocity cap
    /// and turn rates; F601 integrates the thrust within `limits`, F585 turns
    /// the rows and F1512 moves the ship. The port's float fields (position,
    /// velocity in mm/s, forward/up) are kept in step with the object words.
    fn fly(&mut self, target: [i64; 3], dt: f64, limits: &crate::thrust::Limits) {
        let tics = (dt * TICS_PER_SECOND).round().max(0.0) as u32;
        if tics == 0 {
            return;
        }
        let q31 = |v: f64| (v.clamp(-1.0, 1.0) * 2_147_483_647.0) as i32;
        let right = normalise(cross(self.up, self.forward));
        let motion = &mut self.motion;
        motion.model = self.model;
        motion.position = self.position;
        motion.velocity = self
            .velocity
            .map(|v| (v / MM_PER_VELOCITY_UNIT).round() as i32);
        // The object's own rows are the source of truth once it flies: rebuilding them from the
        // float axes every frame loses the low bits the original keeps. Only an axis changed from
        // outside (a spawn, a launch) is loaded.
        let kept = motion.orientation.map(|row| normalise(row.map(f64::from)));
        let unchanged =
            |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1.0e-7);
        if !(unchanged(kept[2], self.forward) && unchanged(kept[1], self.up)) {
            motion.orientation = [right, self.up, self.forward].map(|row| row.map(q31));
        }
        // F727 applies the stored thrust, caps and turn rates every frame; F750's
        // F596 refreshes them only on the AI step.
        self.think_tics = self.think_tics.saturating_add(tics);
        let hunting = self.state_hunts();
        let mut hunt = None;
        if self.think_tics >= THINK_TICS {
            self.think_tics %= THINK_TICS;
            // F593 adds F749's waypoint to the interception's aim point.
            let aim: [i64; 3] = if hunting && self.attack_phase == 0 {
                std::array::from_fn(|k| target[k] + (i64::from(self.waypoint[k]) << 10))
            } else {
                target
            };
            // F750 passes F596's modifier 1 (the autopilot's own call passes 0).
            let detail = crate::autopilot::guide_detail(
                &self.motion,
                self.position,
                aim,
                [0; 3],
                limits,
                1,
                0,
            );
            // F597's accelerated branch: with time accelerated, an interceptor (state 4, a
            // waypoint set) near enough for the step is placed at its aim point with the
            // target's velocity instead of flying the distance. Without it the frame-sized
            // steps at 1000x and 10000x carry a pirate past a moving player and it never
            // closes. F694's leader starts in state 5, whose F749 sets the first waypoint; the
            // port's leader starts in state 4 without one, so F749 runs here first.
            if hunting && self.attack_phase == 0 {
                if let Some(reach) = crate::autopilot::accelerated_reach(
                    detail.guidance.distance_shift,
                    detail.velocity_shift,
                    self.pursuit.time_shift,
                ) {
                    // `+0x14d = 0`: F606 picks the ship's frame on its next pass.
                    self.for_timeout = 0;
                    if reach < 6 {
                        if self.waypoint == [0; 3] {
                            self.reposition();
                        }
                        let aim: [i64; 3] = std::array::from_fn(|k| {
                            target[k] + (i64::from(self.waypoint[k]) << 10)
                        });
                        self.place_at(aim, self.pursuit.velocity);
                        return;
                    }
                }
            }
            let guidance = detail.guidance;
            self.motion.thrust = guidance.thrust;
            self.motion.cap = guidance.cap;
            self.motion.autopilot = guidance.turns;
            hunt = Some(detail);
        }
        if let (true, Some(detail)) = (hunting, hunt) {
            self.hunt_step(&detail, limits);
        }
        let motion = &mut self.motion;
        // F725: F585 turns the rows first, then F601 applies the thrust along the new rows.
        let mut scanner = crate::physics::Scanner::default();
        motion.orient(tics as i32, true, tics as i32, &mut scanner);
        motion.accelerate(tics);
        // F726: in combat the velocity relaxes towards the target's by `(v - w) * (tics << 13)`
        // of one, on each axis that is more than 100 units away.
        if self.in_fudge && tics < 0xc22e {
            for k in 0..3 {
                let away = motion.velocity[k].wrapping_sub(self.combat_velocity[k]);
                if away.wrapping_abs() > 100 {
                    motion.velocity[k] = motion.velocity[k]
                        .wrapping_sub(crate::model::multiply(away, (tics << 13) as i32));
                }
            }
        }
        motion.advance(i64::from(tics));
        motion.renormalise();
        self.position = motion.position;
        self.velocity = motion.velocity.map(|v| f64::from(v) * MM_PER_VELOCITY_UNIT);
        let rows = motion.orientation.map(|row| normalise(row.map(f64::from)));
        self.forward = rows[2];
        self.up = rows[1];
        self.speed = length(self.velocity);
        // `+0x101`: 0-2 depending on retro power. F601 reads the stored power
        // rather than the sign of the thrust word; with the port's guidance
        // words it follows the backward thrust, at F1533's two thresholds.
        self.retro = retro_power(self.motion.thrust[2]);
    }

    /// Port addition (Advanced traffic): F931's shot at another ship, with
    /// no hostility gate (the caller decides who fights whom).
    pub fn fire_at(&mut self, tics: u32, centre: [i64; 3], radius: i64) -> Option<Shot> {
        if !self.alive() || self.state == State::Evade {
            return None;
        }
        let code = self.laser?;
        let spec = laser::fire(code, &mut self.guns, tics).ok()?;
        let targets = [laser::Target {
            index: 0,
            centre,
            radius,
        }];
        let hit = laser::hit(self.position, self.forward, &targets);
        self.shot = Some(ShotMark::new(code, hit.as_ref()));
        let hit = hit?;
        Some(Shot {
            ship: self.id,
            name: self.name.clone(),
            code,
            damage: spec.damage,
            beam: spec.beam,
            point: hit.point,
            distance: hit.distance,
        })
    }

    /// F927's slot recycle / the original ship vanishing: beyond
    /// [`DESPAWN_RANGE`] of the player, killed, or jumped out.
    pub fn finished(&self, player: [i64; 3]) -> bool {
        match self.state {
            State::Dead => self.death <= 0.0,
            State::Escaped => true,
            // A ship with business in the system stays wherever it is: a
            // trader flying to, docked at or leaving a station, and a hostile
            // hunting the player. The original keeps every object until F927
            // needs its slot; the port only recycles idle ships far away.
            // Keep police patrols and capital convoys available across the
            // system, including when the player starts on a surface pad.
            _ if self.station.is_some()
                || self.hostile
                || matches!(self.role, Role::Police | Role::Shuttle)
                || (self.role == Role::Military && self.leader.is_some()) =>
            {
                false
            }
            _ => distance_between(self.position, player) > DESPAWN_RANGE,
        }
    }

    /// The hit-test box target for `laser::hit`, in the absolute system frame.
    pub fn target(&self, index: usize) -> laser::Target {
        laser::Target {
            index,
            centre: self.position,
            radius: self.radius,
        }
    }

    /// FFE's ground collision (`F1037`/`F1041`'s object contact, state `0x1c`):
    /// a ship that flies into a world is destroyed. The port's AI has no
    /// terrain, so `keep_ships_out_of_worlds` keeps ordinary traffic clear and
    /// only an attacker that would have hit the ground is removed.
    pub fn hits_ground(&self, centre: [i64; 3], radius: f64) -> bool {
        self.alive()
            && matches!(
                self.state,
                State::Attack | State::Pass | State::Evade | State::Flee | State::Trading
            )
            && distance_between(self.position, centre) < radius
    }
}

/// An NPC laser hit on the player, from F930/F931 and F817.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    /// The firing ship's [`NpcShip::id`].
    pub ship: u32,
    pub name: String,
    /// The laser equipment code (`0x88`, ...).
    pub code: u8,
    /// F930's damage, for F953.
    pub damage: u32,
    /// F930's beam branch (the original does not set the refire word for beams).
    pub beam: bool,
    /// Where the ray entered the player's box.
    pub point: [i64; 3],
    pub distance: f64,
}

/// One record of F236's offence table `DATA_005039` (`ffedat.asm:4362-4382`):
/// eleven 14-byte records. `points` is `+0` (added to the per-starport
/// `DATA_008908` word), `fine` is `+4` (the number F236 prints in the message),
/// `witness` is `+8` (the base of F234's starport-witness roll) and `flags`
/// `+0xc`: bit 0 sets the wanted state, bit 1 shows the message and bit 2 asks
/// F234 for a nearby starport before the offence is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offence {
    pub name: &'static str,
    pub points: u32,
    pub fine: u32,
    pub witness: u8,
    pub flags: u8,
}

/// `DATA_005039`, in the original's order. The fines are the `+4` words; F236
/// charges ten times the printed amount into the starport's ledger
/// (`0x50f8 + 4*starport`), while the message shows `fine` itself (the executed
/// original prints "$600" for unlawful discharge and records 6000).
pub const OFFENCES: [Offence; 11] = [
    Offence {
        name: "Failure to obey Traffic Control",
        points: 5,
        fine: 1000,
        witness: 0,
        flags: 3,
    },
    Offence {
        name: "Launching without clearance",
        points: 8,
        fine: 500,
        witness: 0,
        flags: 2,
    },
    Offence {
        name: "Evading payment of fine",
        points: 15,
        fine: 2000,
        witness: 0,
        flags: 3,
    },
    Offence {
        name: "Unlawful discharge of weapon",
        points: 20,
        fine: 600,
        witness: 8,
        flags: 7,
    },
    Offence {
        name: "Illegal Mining",
        points: 20,
        fine: 200,
        witness: 16,
        flags: 7,
    },
    Offence {
        name: "Trading in illegal goods",
        points: 20,
        fine: 0,
        witness: 8,
        flags: 0,
    },
    Offence {
        name: "Attempted bribery",
        points: 25,
        fine: 0,
        witness: 0,
        flags: 0,
    },
    Offence {
        name: "Smuggling illegal goods",
        points: 30,
        fine: 2500,
        witness: 0,
        flags: 2,
    },
    Offence {
        name: "Unlawful dumping",
        points: 40,
        fine: 3000,
        witness: 22,
        flags: 7,
    },
    Offence {
        name: "Permit Violation",
        points: 80,
        fine: 4000,
        witness: 99,
        flags: 7,
    },
    Offence {
        name: "Piracy and murder",
        points: 300,
        fine: 10_000,
        witness: 20,
        flags: 7,
    },
];

/// The index of "Unlawful discharge of weapon" in [`OFFENCES`]: F931/F941's
/// `F237(ship, 3)`.
pub const UNLAWFUL_DISCHARGE: usize = 3;
/// "Trading in illegal goods" (F542's `F237(5, station)` and the fake
/// smuggler advertisement's row action).
pub const TRADING_ILLEGAL: usize = 5;
/// "Smuggling illegal goods" (F233's event-5 scan).
pub const SMUGGLING: usize = 7;
/// `DATA_005039` record 1: F1056's action 9 from the pad without clearance.
pub const LAUNCHING_WITHOUT_CLEARANCE: usize = 1;
/// `DATA_005039` record 2: F1056 adds it when the starport's ledger is over 5000.
pub const EVADING_FINE: usize = 2;
/// The police fine F543 adds for a fake smuggler advertisement (`0x1388` in
/// tenths of a credit: 500.0 CR).
pub const POLICE_FINE: u32 = 0x1388;

/// F236's message (`DATA_005040`/`DATA_005041`, `ffedat.asm:4384/4386`). The
/// first offence at a starport uses "is not permitted here"; a later one, when
/// that starport's ledger is already non-zero, uses "can't be tolerated ...
/// Pay an extra".
pub fn offence_message(offence: usize, additional: bool) -> String {
    let record = &OFFENCES[offence];
    if additional {
        format!(
            "{} can't be tolerated. Pay an extra ${} fine.",
            record.name, record.fine
        )
    } else {
        format!(
            "{} is not permitted here. Pay a ${} fine.",
            record.name, record.fine
        )
    }
}

/// What F236 did for one offence, for the caller's message log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffenceReport {
    pub message: String,
    /// The amount printed in the message (`+4`).
    pub fine: u32,
    /// The amount added to the jurisdiction's ledger: ten times `fine` (F236's
    /// `lea eax,[eax+eax*4]` after doubling `+4`).
    pub charged: u32,
    /// A later offence in the same jurisdiction used the "extra" message.
    pub additional: bool,
}

/// The player's legal standing: the port's reconstruction of `DATA_008646` bit
/// 0 (`wanted`), `DATA_008647` (`alert`, the police chase), `DATA_008860` bit 2
/// (`illegal_fire`, F931/F941's once-only unlawful-discharge report), the
/// offence count, F236's six-dword fine ledger `DATA_009095` and the six
/// `DATA_009093` offence bitmasks.
///
/// The six ledger slots are F869's system allegiance (`DATA_008996`, the system
/// word's `>> 6`), not the starport ordinal: 0 Interpol/Independent, 1 Imperial,
/// 2 Federal, 3 Alliance, 4 Thargoid. F655 reads the current system's slot and
/// F312 prints slots 2, 1 and 0 as Federal, Imperial and Interpol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Legal {
    pub wanted: bool,
    /// `DATA_008647 != 0`: the police chase is running or dying away.
    pub alert: bool,
    /// `DATA_008647` itself: 0, [`crate::police::CHASE`] (1) or [`crate::police::ENDING`]
    /// (`0xff`). F238 starts it, F240 ends it and F783/F233 clear it.
    #[serde(default)]
    pub chase: i8,
    pub illegal_fire: bool,
    pub offences: u32,
    pub fines: [u32; 6],
    /// `DATA_009093`: the offences recorded by each jurisdiction, one bit per
    /// [`OFFENCES`] index.
    pub records: [u32; 6],
    /// `DATA_008908`: each jurisdiction's offence points (F236 adds the
    /// offence's `+0`; the mission scripts read and write them, token 0x30).
    #[serde(default)]
    pub points: [u32; 6],
}

/// F544/F545's outcome when the player tries to pay the local fine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinePayment {
    /// The fine the current jurisdiction asked for.
    pub amount: u32,
    /// The amount actually taken from the player's cash (0 on refusal).
    pub paid: u32,
}

impl FinePayment {
    /// F544 reached `sub [0x114], fine`.
    pub fn ok(&self) -> bool {
        self.paid == self.amount
    }
}

impl Legal {
    /// F236 for any offence record: the points and fine go to the starport's
    /// ledger (ten times the printed fine), flags bit 0 sets the wanted state
    /// and bit 1 appends the message. A record whose flags ask for a starport
    /// witness (bit 2) is left unrecorded when `starport` is `None`, exactly as
    /// F234 returning 0 does.
    pub fn record(&mut self, offence: usize, starport: Option<usize>) -> Option<OffenceReport> {
        let record = OFFENCES.get(offence)?;
        if record.flags & 4 != 0 && starport.is_none() {
            return None;
        }
        let slot = starport.unwrap_or(0) % self.fines.len();
        let additional = self.fines[slot] != 0;
        let charged = record.fine.saturating_mul(10);
        self.fines[slot] = self.fines[slot].saturating_add(charged);
        self.points[slot] = self.points[slot].saturating_add(record.points);
        if offence < 32 {
            self.records[slot] |= 1 << offence;
        }
        if record.flags & 1 != 0 {
            self.wanted = true;
        }
        self.offences += 1;
        Some(OffenceReport {
            message: if record.flags & 2 != 0 {
                offence_message(offence, additional)
            } else {
                String::new()
            },
            fine: record.fine,
            charged,
            additional,
        })
    }

    /// F543's extra charge on F542's police screen: the amount is already in
    /// the cash unit (tenths of a credit) and goes straight into the starport's
    /// ledger, as `[D9095 + 4*D8996] += 0x1388` does.
    pub fn charge(&mut self, amount: u32, starport: Option<usize>) {
        let slot = starport.unwrap_or(0) % self.fines.len();
        self.fines[slot] = self.fines[slot].saturating_add(amount);
    }

    /// F931/F941: the player's shot or missile has passed F931's gates. The
    /// first shot in a system (`DATA_008860` bit 2 clear) is reported through
    /// F237(ship, 3); later shots are not. F236 needs a nearby starport (the
    /// caller's `witness`); without one it returns before the fine and the
    /// wanted bit, leaving the flag set so the system's one report is spent.
    ///
    /// `witness` is `Some(DATA_008996)` when F234 found a starport in range; the
    /// value is the current system's **allegiance**, the ledger slot F236 uses.
    /// `None` means no starport is in range.
    pub fn player_fired(&mut self, witness: Option<usize>) -> Option<OffenceReport> {
        if self.illegal_fire {
            return None;
        }
        self.illegal_fire = true;
        self.record(UNLAWFUL_DISCHARGE, witness)
    }

    /// F783: the police object (`+0x118 == 0xe`) has gone. It clears
    /// `DATA_008647` and `DATA_008646` bit 0. The illegal-fire flag
    /// (`DATA_008860` bit 2) is not cleared by F783.
    pub fn clear_alert(&mut self) -> bool {
        let was_set = self.wanted || self.alert;
        self.wanted = false;
        self.alert = false;
        self.chase = 0;
        was_set
    }

    /// Set `DATA_008647`, keeping the `alert` mirror of it.
    pub fn set_chase(&mut self, chase: i8) {
        self.chase = chase;
        self.alert = chase != 0;
    }

    /// The fine ledger for a jurisdiction, as F655's `owes` argument. `None`
    /// means no local starport, so nothing is owed.
    pub fn owed(&self, jurisdiction: Option<usize>) -> u32 {
        jurisdiction.map_or(0, |index| self.fines[index % self.fines.len()])
    }

    /// `DATA_009093`: the offence bits recorded by `jurisdiction`.
    pub fn offence_bits(&self, jurisdiction: usize) -> u32 {
        self.records[jurisdiction % self.records.len()]
    }

    /// F311's legal-status word for the jurisdiction's fine.
    pub fn status(&self, jurisdiction: usize) -> &'static str {
        crate::status::legal_status(self.owed(Some(jurisdiction)))
    }

    /// F544/F545: pay the current jurisdiction's fine if there is enough cash.
    /// The cash is the caller's (`DATA_008888`) and is only touched here, as
    /// F236 leaves it alone when the debt is recorded.
    pub fn pay_fine(&mut self, cash: u32, jurisdiction: usize) -> FinePayment {
        let slot = jurisdiction % self.fines.len();
        let amount = self.fines[slot];
        if amount > cash {
            return FinePayment { amount, paid: 0 };
        }
        self.fines[slot] = 0;
        FinePayment {
            amount,
            paid: amount,
        }
    }
}

/// The commander's standing beyond the law: `DATA_008911`'s Elite score,
/// `DATA_008912`/`DATA_008913`'s Federal/Imperial military points and
/// `DATA_009091`'s medal and permit bits.
///
/// F942 adds a destroyed ship's statistics `+0x16` word to the Elite score;
/// F300's mission rewards call F304/F305, which add a signed amount to the
/// military points and print F306's promotion or demotion notice when the rank
/// word changes. `docs/notes/rank-and-fines.md` has the call sites.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Career {
    pub federal: u32,
    pub imperial: u32,
    pub elite: u32,
    pub medals: u32,
}

/// One rank-word change from a military reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankChange {
    pub from: usize,
    pub to: usize,
}

impl RankChange {
    pub fn promoted(&self) -> bool {
        self.to > self.from
    }
}

impl Career {
    /// F942: add a destroyed ship's rating word to `DATA_008911`. Returns
    /// whether the high word changed, exactly as F942's `sar 0x10` comparison
    /// does (the Elite Federation message's trigger).
    pub fn award_kill(&mut self, rating: i16) -> bool {
        let before = self.elite >> 16;
        self.elite = if rating >= 0 {
            self.elite.saturating_add(u32::from(rating as u16))
        } else {
            self.elite.saturating_sub(u32::from(rating.unsigned_abs()))
        };
        before != self.elite >> 16
    }

    /// F304/F306 for the Federal points. `points` is the signed reward F300's
    /// row carries. Returns the rank-index change, if the word changed.
    pub fn award_federal(&mut self, points: i32) -> Option<RankChange> {
        let from = crate::status::military_rank_index(self.federal);
        self.federal = add_signed(self.federal, points);
        let to = crate::status::military_rank_index(self.federal);
        (from != to).then_some(RankChange { from, to })
    }

    /// F305/F306 for the Imperial title.
    pub fn award_imperial(&mut self, points: i32) -> Option<RankChange> {
        let from = crate::status::military_rank_index(self.imperial);
        self.imperial = add_signed(self.imperial, points);
        let to = crate::status::military_rank_index(self.imperial);
        (from != to).then_some(RankChange { from, to })
    }

    /// F307: award the medal or permit whose bit is `index`. `false` when the
    /// bit was already set (the original's `DATA_009091` test).
    pub fn award_medal(&mut self, index: usize) -> bool {
        let Some(bit) = 1u32.checked_shl(index as u32) else {
            return false;
        };
        if self.medals & bit != 0 {
            return false;
        }
        self.medals |= bit;
        true
    }

    pub fn federal_rank(&self) -> &'static str {
        crate::status::federal_rank(self.federal)
    }

    pub fn imperial_title(&self) -> &'static str {
        crate::status::imperial_title(self.imperial)
    }

    pub fn elite_rating(&self) -> &'static str {
        crate::status::elite_rating(self.elite)
    }
}

/// F306's signed add: positive values saturate at `0xffff`, negatives clamp at
/// zero.
fn add_signed(value: u32, delta: i32) -> u32 {
    if delta >= 0 {
        value.saturating_add(delta as u32).min(0xffff)
    } else {
        value.saturating_sub(delta.unsigned_abs()).min(0xffff)
    }
}

/// F675/F680's carried ship: a hostile ship close to the player survives a jump.
/// The offset is from the player, so it can be restored in the new system.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Persisted {
    pub model: u32,
    pub role: Role,
    pub name: String,
    pub type_name: String,
    pub offset: [i64; 3],
    pub velocity: [f64; 3],
    pub forward: [f64; 3],
    pub up: [f64; 3],
    pub radius: i64,
    pub damage: DamageState,
    pub shield_strength: i16,
    /// The statistics' `+0x16` word, kept for F942's rating on a later kill.
    pub rating: i16,
    /// The statistics' cargo space (`+0x116`), kept for F942's debris on a
    /// later destruction.
    pub cargo: u16,
    pub hostile: bool,
    pub laser: Option<u8>,
    pub guns: laser::State,
    /// F773's general-equipment dword and pylon words, so a carried ship keeps
    /// its E.C.M. and remaining missiles across the jump.
    pub general: u32,
    pub pylons: [u8; crate::missile::PYLONS],
    pub bounty: u32,
    /// The model-derived limits, so a restored ship flies and turns like itself.
    pub max_speed: f64,
    pub accel: f64,
    pub turn_rate: f64,
    pub limits: Option<crate::thrust::Limits>,
    pub motion: crate::physics::Motion,
    /// F1533's class byte (`+0x14c`), collision radius (`+0x140`) and drive
    /// (`+0xd0`), kept so a carried ship's object record stays the same.
    #[serde(default)]
    pub class_flags: u8,
    #[serde(default)]
    pub collision_radius: i64,
    #[serde(default)]
    pub drive: u8,
    /// `+0x119`: cargo fuel in tonnes, kept for F776/F901 after a carried jump.
    #[serde(default)]
    pub cargo_fuel: u8,
    /// `+0xcc`: the damaged-equipment flags.
    #[serde(default)]
    pub damaged: u32,
    /// F682/F680: the `+0xf2..+0xfd` jump words the carried ship had when the
    /// player jumped. F682 re-runs F901 with the player's jump distance and
    /// F680 writes `DATA_008916` to `+0xf2`; the port keeps the ship's own
    /// F776 words, which is the destination it had chosen.
    #[serde(default)]
    pub jump: crate::object_record::HyperspaceWords,
}

/// F675: the ships that follow the player through a jump. The original keeps a
/// pirate (class 0xfb) within 0x10 of the player and escape pods (model 0x9a);
/// the port has no pods and uses [`PERSIST_RANGE`].
pub fn persist(ships: &[NpcShip], player: [i64; 3]) -> Vec<Persisted> {
    ships
        .iter()
        .filter(|ship| ship.alive())
        .filter(|ship| ship.is_hostile())
        .filter(|ship| distance_between(ship.position, player) <= PERSIST_RANGE)
        .map(|ship| Persisted {
            model: ship.model,
            role: ship.role,
            name: ship.name.clone(),
            type_name: ship.type_name.clone(),
            offset: [
                ship.position[0] - player[0],
                ship.position[1] - player[1],
                ship.position[2] - player[2],
            ],
            velocity: ship.velocity,
            forward: ship.forward,
            up: ship.up,
            radius: ship.radius,
            damage: ship.damage,
            shield_strength: ship.shield_strength,
            rating: ship.rating,
            cargo: ship.cargo,
            hostile: true,
            laser: ship.laser,
            guns: ship.guns,
            general: ship.general,
            pylons: ship.pylons,
            bounty: ship.bounty,
            max_speed: ship.max_speed,
            accel: ship.accel,
            turn_rate: ship.turn_rate,
            limits: ship.limits,
            motion: ship.motion,
            class_flags: ship.class_flags,
            collision_radius: ship.collision_radius,
            drive: ship.drive,
            cargo_fuel: ship.cargo_fuel,
            damaged: ship.damaged,
            // F682 stores the AI mode the ship is leaving in `+0xf8`; persist
            // runs on the jump's frame, so the current mode is that mode.
            jump: crate::object_record::HyperspaceWords {
                post_mode: ship.ai_state(),
                ..ship.jump
            },
        })
        .collect()
}

/// F680/F682: what the carried ships do when the player arrives. F680 writes
/// `DATA_008916` (the system two back, i.e. the one just left) to every carried
/// ship's `+0xf2`; F682 then makes its *second* F901 call with the player's
/// hyperspace distance (`DATA_009115`), so the ship's `+0xf6` range and `+0xfa`
/// time are the ones the player's jump gives it. A ship whose drive or fuel
/// cannot make that distance makes F682 return non-zero, which the source
/// answers with F940 (kill): the record is dropped. With no distance (a direct
/// `Navigator::jump`), the ship keeps its own F776 words.
pub fn carried_jump(
    records: Vec<Persisted>,
    distance: i32,
    origin: u32,
    drives: &[crate::sector::Drive],
) -> Vec<Persisted> {
    records
        .into_iter()
        .filter_map(|mut record| {
            record.jump.destination = origin;
            if distance <= 0 {
                return Some(record);
            }
            let (max_range, jump_time, left) = crate::hyperspace::ai_jump(
                drives,
                i32::from(record.damage.mass),
                record.drive,
                distance,
                record.cargo_fuel,
            )?;
            record.jump.max_range = max_range;
            record.jump.jump_time = jump_time;
            record.cargo_fuel = left;
            Some(record)
        })
        .collect()
}

/// F680: restore the carried ships at the player's arrival point, keeping their
/// offset from the player. `first_id` keeps the ids clear of the new system's
/// traffic. A record whose offset is beyond [`DESPAWN_RANGE`] is dropped.
pub fn restore(records: &[Persisted], player: [i64; 3], first_id: u32) -> Vec<NpcShip> {
    records
        .iter()
        .filter(|record| {
            let offset = record.offset.map(|v| v as f64);
            length(offset) <= DESPAWN_RANGE
        })
        .enumerate()
        .map(|(index, record)| {
            // F680/F682: a carried ship re-enters the delayed exit cloud
            // (`+0xff = 0x11` with `+0xf8` holding the mode it left in) and
            // F737/F740 return it after the jump time. A record without a jump
            // time (F776 found no destination) keeps the port's attack.
            let (state, flee) = if record.jump.jump_time > 0 {
                (State::Jump, jump_seconds(record.jump.jump_time))
            } else {
                (State::Attack, 0.0)
            };
            NpcShip {
                id: first_id + index as u32,
                slot: 0,
                model: record.model,
                role: record.role,
                name: record.name.clone(),
                type_name: record.type_name.clone(),
                position: add(player, record.offset),
                velocity: record.velocity,
                forward: normalise(record.forward),
                up: normalise(record.up),
                radius: record.radius,
                damage: record.damage,
                shield_strength: record.shield_strength,
                rating: record.rating,
                cargo: record.cargo,
                state,
                anchor: player,
                orbit_radius: ANCHOR_ORBIT,
                phase: 0.0,
                max_speed: record.max_speed,
                accel: record.accel,
                turn_rate: record.turn_rate,
                limits: record.limits,
                motion: record.motion,
                speed: length(record.velocity),
                hostile: record.hostile,
                laser: record.laser,
                guns: record.guns,
                general: record.general,
                pylons: record.pylons,
                missile_rng: crate::RandomState::new(
                    first_id ^ index as u32 ^ 0x5eed_1234,
                    (first_id + index as u32).rotate_left(13) ^ 0x2468_ace0,
                ),
                control_clock: 0.0,
                think_tics: THINK_TICS,
                scrambled: false,
                scramble_tics: 0,
                attack_phase: 0,
                in_fudge: false,
                pass_hits: 0,
                combat_velocity: [0; 3],
                waypoint: [0; 3],
                pursuit: Pursuit::default(),
                placed: false,
                turned: false,
                missile_target: None,
                ecm_attempted: false,
                launch: None,
                ecm_burst: None,
                bounty: record.bounty,
                leader: None,
                formation: [0; 3],
                pass: 0.0,
                flee,
                responding: false,
                in_combat: false,
                alert: false,
                npc_target: None,
                contract: None,
                flash: 0.0,
                shot: None,
                death: 0.0,
                station: None,
                bay: None,
                pad: None,
                dock_timer: 0.0,
                dock_span: 0.0,
                dock_retry: 0.0,
                dock_wait: 0.0,
                seed: 0,
                script_target: 0,
                ai_code: None,
                class_flags: record.class_flags,
                collision_radius: record.collision_radius,
                drive: record.drive,
                cargo_fuel: record.cargo_fuel,
                damaged: record.damaged,
                retro: 0,
                for_timeout: 0,
                smoke_timeout: 0,
                ecm_timeout: 0,
                avoid_timeout: 0,
                frame_parent: 0,
                frame_position: [0; 3],
                frame_due: false,
                avoid_due: false,
                avoid_target: None,
                jump: record.jump,
                jump_clock: [0; 2],
                timer_clock: 0.0,
            }
        })
        .collect()
}

/// F802's collision check for F610's avoidance: the first obstacle whose
/// centre lies inside the forward ray's stopping distance and within the
/// combined radii. Returns the centre and the reach (obstacle radius plus the
/// ship's collision radius). The source scans the whole object list with the
/// ship's radius, speed and the time-acceleration step; this is the port's
/// body/ship-centre test over the supplied list.
pub fn obstruction_along(
    position: [i64; 3],
    forward: [f64; 3],
    speed: f64,
    accel: f64,
    ship_radius: f64,
    obstacles: &[([i64; 3], f64)],
) -> Option<([i64; 3], f64)> {
    let forward = normalise(forward);
    let stopping = (0.5 * speed * speed / accel.max(1.0)).max(1.0);
    let mut best: Option<(f64, [i64; 3], f64)> = None;
    for (centre, radius) in obstacles {
        let to: [f64; 3] = std::array::from_fn(|axis| (centre[axis] - position[axis]) as f64);
        let square = to.iter().map(|value| value * value).sum::<f64>();
        if square < 1.0 {
            continue;
        }
        let along = (0..3).fold(0.0, |sum, axis| sum + to[axis] * forward[axis]);
        if along <= 0.0 || along > stopping + radius + ship_radius {
            continue;
        }
        let miss = (square - along * along).max(0.0).sqrt();
        if miss >= radius + ship_radius {
            continue;
        }
        if best.is_none_or(|(best_along, _, _)| along < best_along) {
            best = Some((along, *centre, *radius));
        }
    }
    best.map(|(_, centre, radius)| (centre, radius + ship_radius))
}

/// F1533's `+0x101` retro power from the forward thrust word: the source
/// negates the retro acceleration and compares it with 0x1329 and 0xaa5.
/// Negative forward thrust is a retro burn, so the port reads its magnitude.
pub fn retro_power(forward_thrust: i16) -> u8 {
    let retro = -i32::from(forward_thrust);
    if retro >= 0x1329 {
        2
    } else if retro >= 0x0aa5 {
        1
    } else {
        0
    }
}

/// F585's turn-limit divisor: the model's size halves the limit from 0x35 and
/// quarters it from 0x3b.
pub fn turn_divisor(model: u32) -> u32 {
    if model >= 0x3b {
        4
    } else if model >= 0x35 {
        2
    } else {
        1
    }
}

/// F773's energy word: `DATA_006266[drive] << 24`.
pub fn drive_energy(drive: u8) -> u32 {
    const DRIVE_FUEL: [u32; 15] = [
        0, 0x20, 0x20, 0x40, 0x60, 0x80, 0xa0, 0xc0, 0xe0, 0xff, 0x20, 0x40, 0x60, 0x80, 0,
    ];
    DRIVE_FUEL.get(usize::from(drive)).copied().unwrap_or(0) << 24
}

/// F697's `DATA_006268` draw: `offset` 4 for F694's lone pirate and 2 for
/// F695's group members; `random & 0xf` is the port's F922 draw.
pub fn pirate_bounty(rng: &mut Random, grouped: bool) -> u32 {
    let offset = if grouped { 2 } else { 4 };
    let index = (rng.bounded(16) as usize + offset).min(PIRATE_BOUNTIES.len() - 1);
    PIRATE_BOUNTIES[index]
}

/// `DATA_006264` (`ffedat.asm:10082`): the per-class laser-capacity proportion
/// F774 adds to its `RND(0x33)` draw. The table has eighteen dwords; class
/// 0x11 (a Thargoid) reads `0xb2` at offset `0x44`.
pub const LASER_PROPORTION: [u16; 18] = [
    0xb2, 0x0c, 0xb2, 0x0c, 0xb2, 0x0c, 0xb2, 0x0c, 0xb2, 0x0c, 0x66, 0x99, 0x99, 0x00, 0xb2, 0x07,
    0x7f, 0xb2,
];

/// `DATA_006265` (`ffedat.asm:10092`): the laser type and tonnage table F774
/// walks. These are the laser bytes F773 writes to the mount, not the
/// equipment-shop codes; the shop's Thargoid Laser is
/// [`crate::equipment::THARGOID_LASER`] (0xd5).
pub const LASER_TONNAGE: [(u8, u32); 9] = [
    (0x00, 0),
    (0x88, 1),
    (0x91, 3),
    (0xa0, 5),
    (0xa9, 20),
    (0xb3, 75),
    (0xbc, 200),
    (0xc6, 500),
    (0xcf, 900),
];

/// F774 (`ffe.asm:50765`): pick the laser a ship of `class` fits from its
/// remaining equipment `budget`. `RND(0x33) + DATA_006264[class]` times the
/// budget, shifted right eight, picks the largest `DATA_006265` tonnage that
/// fits; the chosen tonnage is returned in the tuple with the laser byte.
///
/// The port has no equipment-budget loop, so a caller passes the model's cargo
/// word as the budget. That is the same starting quantity F774 receives for
/// these models (F1533's drive-mass subtraction is not modelled).
pub fn f774_laser(class: u8, budget: u32, rng: &mut Random) -> Option<(u8, u32)> {
    let proportion = u32::from(*LASER_PROPORTION.get(usize::from(class))?);
    let draw = rng.bounded(0x33).saturating_add(proportion);
    let capacity = ((u64::from(draw) * u64::from(budget)) >> 8) as u32;
    let mut index = 1;
    while index < LASER_TONNAGE.len() && capacity >= LASER_TONNAGE[index].1 {
        index += 1;
    }
    // The original decrements after the loop, so a capacity below the first
    // tonnage returns entry 0 (no laser).
    Some(LASER_TONNAGE[index - 1])
}

/// The tonnage of a laser byte in F774's `DATA_006265` table.
fn laser_mass(code: u8) -> u32 {
    LASER_TONNAGE
        .iter()
        .find(|(entry, _)| *entry == code)
        .map_or(0, |(_, mass)| *mass)
}

/// F773's equipment fit for an AI ship: the E.C.M./Naval E.C.M. and energy-bomb
/// branch and the missile branch of its cargo-tonnage budget (`+0x116`).
///
/// The port has no shield-spending loop (the shield maximum is the model's
/// shield word, see [`Spec::shield_max`]), so the budget here is the cargo word
/// less the laser's [`LASER_TONNAGE`] mass. F773 draws the E.C.M. gate before
/// the missiles; the missile count is `RND(0..=mounts)` (zero means none) and the code comes from
/// [`MISSILE_CODES`]. The pylon fill is capped at eight because F754's pylon
/// roll is `(F1530 & 7) + 1` and never reaches pylons 9/10.
pub fn fit_equipment(
    spec: &Spec,
    role: Role,
    laser: Option<u8>,
    rng: &mut Random,
) -> (u32, [u8; crate::missile::PYLONS]) {
    let mut budget = i32::from(spec.cargo) - laser.map_or(0, |code| laser_mass(code) as i32);
    // F773's `or edi,0x4000` gives every AI ship atmospheric shielding (bit 30).
    let mut general = 1 << 30;
    if budget >= 2 {
        let roll = rng.bounded(0x1_0000) as u16;
        if roll >= 0xc000 {
            general |= crate::equipment::NAVAL_ECM_BIT;
            budget -= 2;
        } else if roll >= 0x4000 {
            general |= crate::equipment::ECM_BIT;
            budget -= 2;
        }
    }
    let mounts = spec.missiles.min(8) as u32;
    let mut pylons = [0; crate::missile::PYLONS];
    let count = if mounts == 0 {
        0
    } else {
        rng.bounded(mounts + 1)
    };
    let code = MISSILE_CODES[(rng.bounded(0x1_0000) >> 12) as usize];
    let mut filled = 0u32;
    while filled < count && budget > 0 && filled < 8 {
        pylons[filled as usize] = code;
        budget -= 1;
        filled += 1;
    }
    // F773's rare energy-bomb branch: `F1530 >= 0xfc00`, four tonnes, not police.
    if budget >= 4 && role != Role::Police && rng.bounded(0x1_0000) >= 0xfc00 {
        general |= ENERGY_BOMB_BIT;
    }
    (general, pylons)
}

impl Role {
    /// The front laser an NPC of this role carries. F773/F774 fit it from the
    /// model's equipment budget; the port has no budget loop, so pirates a 1MW
    /// pulse, police and military a 5MW pulse, and Thargoids F774's selection
    /// from their class proportion and cargo. The equipment shop's separate
    /// Thargoid Laser (0xd5) is not one of F774's table entries.
    pub fn laser_code(self, spec: &Spec, rng: &mut Random) -> Option<u8> {
        if spec.gun_mounts <= 0 {
            return None;
        }
        match self {
            Role::Pirate => Some(laser::PULSE_1MW),
            Role::Police | Role::Military => Some(laser::PULSE_5MW),
            Role::Thargoid => f774_laser(self.ai_class(), u32::from(spec.cargo), rng)
                .and_then(|(code, _)| (code != 0).then_some(code)),
            Role::Trader | Role::Shuttle => None,
        }
    }
}

/// The F869 system report plus `F35(0x16)`, the mission-script Thargoid limit
/// F688 passes to F922 before F690. A normal system has `thargoids == 0`: the
/// port has no mission scripts, so nothing writes that global variable, and
/// `RND(0)` spawns none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct System {
    pub details: Details,
    /// `F35(0x16)` (`ffe.asm:45858`): the exclusive limit of F688's Thargoid
    /// draw, not the number of ships.
    pub thargoids: usize,
    /// F686's flag (`DATA_009046` bit 0): F688's first-pass variant. It drops
    /// the patrol roll's `* 4` and skips F691/F699. The executed original's
    /// new game does **not** run it (every start key draws RND(4 * danger) for
    /// pirates and runs F691's bay roll), so the port never sets it.
    pub start: bool,
    /// Port only: the new game's own spawn. The executed original holds no
    /// police ship at its start and F688 has no police draw, so the port's
    /// patrol is not added to it.
    pub new_game: bool,
}

impl System {
    pub const fn new(details: Details, thargoids: usize) -> Self {
        Self {
            details,
            thargoids,
            start: false,
            new_game: false,
        }
    }
}

/// F688's counts and roles with the port's cap. The bay count is F691's
/// `RND(4 * places)` (see [`counts_with_places`]); `aux_traders` is F699's
/// count. F688 has no police draw; `police` is the port's one patrol group from
/// the record's `+7` byte (the source's F238 police response is not ported as a
/// spawn path, see `docs/notes/npc.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub traders: usize,
    /// `RND(F35(0x16))`; F690's spawned count, before [`MAX_SHIPS`].
    pub thargoids: usize,
    /// F691: `min(RND(4 * places), 25)` docked traders.
    pub bay_ships: usize,
    /// F699's aux traders (`RND(+8)`), subject to the source's quirk below.
    pub aux_traders: usize,
    pub pirates: usize,
    pub police: usize,
}

/// The counts F688 draws from a system record, before [`MAX_SHIPS`]. `details`
/// is F869's report: `bytes[3]` is `+8`, `bytes[1]` is `+6` and `bytes[2]` is
/// `+7`. `places` is `DATA_008997`, the number of station/starport records
/// F617/F618 collected (the port's generated place bodies, capped at the array's
/// 18 records); pass 0 when the caller has no stations to offer.
///
/// The source's exact draws are: `RND(+8)` traders (F689), `RND(F35(0x16))`
/// Thargoids (F690), F691's `RND(4 * places)` capped at 25, F699's `RND(+8)`
/// aux traders, `RND(+6*4)` F698 pirate-group attempts and no police draw. F688
/// has one quirk preserved here: with places present, F699 runs only when the
/// F691 roll was non-zero; when the F691 roll is zero the shuttle/aux branch is
/// skipped entirely (and a system without places runs F699 but not F691). The
/// port keeps its own trader/pirate caps; the pirate `+6` count without F698's
/// `RND(*4)` remains a simplification.
pub fn counts(system: &System, rng: &mut Random) -> Counts {
    counts_with_places(system, 0, rng)
}

/// F688's counts with `places` = `DATA_008997`. See [`counts`].
pub fn counts_with_places(system: &System, places: usize, rng: &mut Random) -> Counts {
    // F688: RND(byte +8) traders, RND(F35(0x16)) Thargoids (F690), then (with
    // F686/F687's flag clear) F691's bay roll and F699's aux count, byte +6
    // patrol groups and byte +7 police (the port's reconstruction).
    let details = &system.details;
    let thargoid_limit = u32::try_from(system.thargoids).unwrap_or(u32::MAX);
    let traders = rng.bounded(details.bytes[3]) as usize;
    let thargoids = rng.bounded(thargoid_limit) as usize;
    let places_word = u32::try_from(places.min(18)).unwrap_or(18);
    let bay_ships = if places_word == 0 {
        0
    } else {
        (rng.bounded(places_word.saturating_mul(4)) as usize).min(0x19)
    };
    let aux_traders = if places_word == 0 || bay_ships > 0 {
        rng.bounded(details.bytes[3]) as usize
    } else {
        0
    };
    // F688: `RND(+6 << 2)` patrol groups (F698), without the shift on F686's
    // new-game pass.
    let patrol_limit = if system.start {
        details.bytes[1]
    } else {
        details.bytes[1] << 2
    };
    let pirates = rng.bounded(patrol_limit) as usize;
    Counts {
        traders,
        thargoids,
        bay_ships,
        aux_traders,
        pirates,
        police: (details.bytes[2] > 0) as usize,
    }
}

/// F688's spawn switches that are the port's rather than the source's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOptions {
    /// Force at least one trader and one pirate group. This is the port's
    /// testability/capture guarantee, not F688: the original can roll zero
    /// traffic for a system. Only a scripted capture or a unit fixture should
    /// set it; a real system entry must not, or a system the source leaves
    /// empty gets a hostile group it should not have.
    pub guarantee: bool,
    /// Place the pirate group `PIRATE_DISTANCE` ahead of the player instead of
    /// on the anchor orbit. This is the port's capture framing (it puts the
    /// pirate in the camera and inside F931's range). F702 places a spawned
    /// ship at a random system-wide point, so a normal entry must not use the
    /// player's position: otherwise a real system's pirate group is forced
    /// beside the pad or the arrival point and attacks immediately.
    pub near_player: bool,
    /// Run F688's station-traffic branches (F691 bay ships and F699 aux
    /// traders). A normal system entry has them; the scripted combat/capture
    /// fixtures turn them off so their frames keep the ships they were built
    /// against.
    pub station_traffic: bool,
}

/// A trader out of hyperspace, `id` its stable id, flying to `station` (a
/// place at `anchor`) for a bay: the assassination contract's target
/// ([`contract_target`]) and its escorts.
pub fn hyperspace_arrival(
    spec: &Spec,
    id: u32,
    position: [i64; 3],
    anchor: [i64; 3],
    station: usize,
    rng: &mut Random,
) -> NpcShip {
    let mut spec = spec.clone();
    let type_name = spec.name.clone();
    spec.name = format!("{} {}", spec.name, id + 1);
    let mut ship = NpcShip::new(&spec, Role::Trader, position, anchor, rng);
    ship.type_name = type_name;
    ship.id = id;
    ship.forward = normalise(sub(anchor, position));
    ship.begin_transit(station, anchor);
    ship
}

/// A ship docked at a place's pad or bay (its bay is the caller's to grant),
/// leaving after `seconds`: F672's mission-script ship at a starport.
pub fn pad_trader(
    spec: &Spec,
    id: u32,
    anchor: [i64; 3],
    station: usize,
    seconds: f64,
    rng: &mut Random,
) -> NpcShip {
    let mut ship = hyperspace_arrival(spec, id, anchor, anchor, station, rng);
    ship.state = State::Docked;
    ship.dock_span = seconds;
    ship.dock_timer = seconds;
    ship
}

/// A mission script's attacker (F669 for statements 41-43): a hostile ship
/// that closes on the player (`+0xff` 9 with the player as `+0xfe`).
pub fn script_attacker(spec: &Spec, id: u32, position: [i64; 3], rng: &mut Random) -> NpcShip {
    let mut spec = spec.clone();
    let type_name = spec.name.clone();
    spec.name = format!("{} {}", spec.name, id + 1);
    let mut ship = NpcShip::new(&spec, Role::Pirate, position, position, rng);
    ship.type_name = type_name;
    ship.id = id;
    ship.hostile = true;
    ship.state = State::Attack;
    ship
}

/// A ship a mission script made with F772 (statement 19): created where the
/// script will place it (statement 54), holding its place until a script
/// steers it. F773 gives it no target.
pub fn script_ship(spec: &Spec, id: u32, position: [i64; 3], rng: &mut Random) -> NpcShip {
    let mut spec = spec.clone();
    let type_name = spec.name.clone();
    spec.name = format!("{} {}", spec.name, id + 1);
    let mut ship = NpcShip::new(&spec, Role::Trader, position, position, rng);
    ship.type_name = type_name;
    ship.id = id;
    ship.state = State::Hold;
    ship.orbit_radius = 0.0;
    ship.velocity = [0.0; 3];
    ship.speed = 0.0;
    ship
}

/// An object a mission script puts in the system (F297's surface buildings,
/// F40's orbiting objects): it does not fly or fire, attacking it is no
/// crime, and the scripts count its damage themselves (F953's signal).
pub fn script_object(spec: &Spec, id: u32, position: [i64; 3], rng: &mut Random) -> NpcShip {
    let mut ship = NpcShip::new(spec, Role::Pirate, position, position, rng);
    ship.type_name = spec.name.clone();
    ship.id = id;
    ship.hostile = false;
    ship.laser = None;
    ship.state = State::Hold;
    ship.orbit_radius = 0.0;
    ship.max_speed = 0.0;
    ship.accel = 0.0;
    ship.turn_rate = 0.0;
    ship.limits = None;
    ship.velocity = [0.0; 3];
    ship.speed = 0.0;
    ship
}

/// `DATA_001705` (`ffedat.asm:699`): F41 statement 60's formation places,
/// 27 offsets `(x, y, z)` in the leader's frame (`+0x102..+0x106`). The
/// first nine are a vee behind the leader; the scripts add 9 for the other
/// groupings.
pub const SCRIPT_FORMATIONS: [[i16; 3]; 27] = [
    [500, 0, -500],
    [-500, 0, -500],
    [1000, 0, -1000],
    [0, 0, -1000],
    [-1000, 0, -1000],
    [500, 0, -1500],
    [-500, 0, -1500],
    [0, 0, -2000],
    [0, 0, 500],
    [-1000, -1000, 0],
    [-1000, 0, 0],
    [-1000, 1000, 0],
    [0, -1000, 0],
    [0, 1000, 0],
    [1000, -1000, 0],
    [1000, 0, 0],
    [1000, 1000, 0],
    [-500, -500, -500],
    [500, -500, -500],
    [-500, 500, -500],
    [500, 500, -500],
    [-500, -500, 500],
    [500, -500, 500],
    [-500, 500, 500],
    [500, 500, 500],
    [0, 0, 1000],
    [0, 0, -1000],
];

/// The port's scale from the original formation words to its frame, as
/// F695's pirate escorts use it (500 becomes 0.4e6 mm).
pub const FORMATION_SCALE: i64 = 800;

/// An assassination contract's target (F291): a trader leaving `anchor`'s
/// place for `station`'s, carrying the contract id.
pub fn contract_target(
    spec: &Spec,
    id: u32,
    position: [i64; 3],
    anchor: [i64; 3],
    station: usize,
    contract: u32,
    rng: &mut Random,
) -> NpcShip {
    let mut ship = hyperspace_arrival(spec, id, position, anchor, station, rng);
    ship.contract = Some(contract);
    ship
}

/// The port's capital-ship orbit off a station's surface (F699's aux trader).
pub const CAPITAL_ORBIT: f64 = 20.0e6;

impl SpawnOptions {
    /// The most ships a spawn keeps: [`MAX_SHIPS`] and the police patrol.
    pub fn cap(&self) -> usize {
        MAX_SHIPS + 1
    }
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            guarantee: false,
            near_player: false,
            station_traffic: true,
        }
    }
}

impl SpawnOptions {
    /// The scripted traffic capture: something to shoot and a pirate in frame.
    /// It does not add the station's own traffic, so the capture frames are not
    /// crowded by F691/F699 ships unrelated to the fixture.
    pub const CAPTURE: Self = Self {
        guarantee: true,
        near_player: true,
        station_traffic: false,
    };
}

/// Spawn one system's traffic. `spec` and `name` look models up in the cache;
/// models without a statistics block are skipped. The result is capped at
/// [`MAX_SHIPS`] and is deterministic for a given `rng` state. F691's bay ships
/// are returned in `State::Docked` with `station` set and no `bay`: the caller
/// owns the traffic record and must ask it for a bay, then call
/// [`NpcShip::begin_docked_for`] with [`NpcShip::dock_span`]. [`SpawnOptions`]
/// carries the port's capture-only guarantee; without it the counts are F688's
/// (including zero) capped for legibility. `system.thargoids` is F35(0x16)'s
/// limit; zero unless a mission script set it.
#[allow(clippy::too_many_arguments)]
pub fn spawn(
    player: [i64; 3],
    forward: [f64; 3],
    anchors: &[Anchor],
    system: &System,
    allegiance: u32,
    rng: &mut Random,
    options: SpawnOptions,
    spec: impl Fn(u32) -> Option<Spec>,
) -> Vec<NpcShip> {
    let place_count = if options.station_traffic {
        anchors
            .iter()
            .filter(|anchor| anchor.station.is_some())
            .count()
    } else {
        0
    };
    let mut counts = counts_with_places(system, place_count, rng);
    // The port's stand-in for F702's system-wide placement is an orbit of one
    // place: the one nearest the player, so the system's traffic, police and
    // pirates are where the player is rather than at the first port listed.
    let anchor = anchors
        .iter()
        .copied()
        .min_by(|a, b| {
            distance_between(a.position, player).total_cmp(&distance_between(b.position, player))
        })
        .unwrap_or(Anchor {
            position: player,
            radius: 1_000_000,
            station: None,
            orbital_station: false,
        });
    let orbital_station = anchors
        .iter()
        .filter(|place| place.orbital_station)
        .min_by(|a, b| {
            distance_between(a.position, player).total_cmp(&distance_between(b.position, player))
        });
    if options.guarantee {
        // The capture/test guarantee: a fresh system has something to label and
        // shoot and the F953 damage path is reachable.
        counts.traders = counts.traders.clamp(1, 2);
        // The capture keeps the record's patrol count, not F688's roll, so
        // its pirate group is the same every run.
        counts.pirates = (system.details.bytes[1] as usize).clamp(1, 2);
    } else {
        // F688 can roll zero of every kind; its draws stand.
    }
    counts.thargoids = counts.thargoids.min(MAX_SHIPS);
    counts.bay_ships = counts.bay_ships.min(MAX_BAY_SHIPS);
    if !options.station_traffic {
        counts.bay_ships = 0;
        counts.aux_traders = 0;
    }
    // F688 has no police draw, and nothing in the original spawns a police ship (type 0xe)
    // outside F238's response, so no entry brings a patrol. Only the port's scripted capture
    // keeps the patrol its frames were built with.
    counts.police = if system.new_game || !options.guarantee {
        0
    } else {
        counts.police.min(1)
    };
    let limit = MAX_SHIPS;

    let orbit = (anchor.radius as f64 * 1.2).clamp(ANCHOR_ORBIT, laser::RANGE * 0.75);
    let mut ships: Vec<NpcShip> = Vec::new();
    let mut next_id = 0u32;
    let mut add = |role: Role,
                   kind: usize,
                   position: [i64; 3],
                   anchor,
                   rng: &mut Random|
     -> Option<NpcShip> {
        // Police patrols are outside the cap ([`MAX_POLICE_PATROLS`]).
        if next_id as usize >= limit && role != Role::Police {
            return None;
        }
        let mut model = model_for(role, allegiance, rng);
        if model == 0 {
            return None;
        }
        let mut picked = spec(model);
        // A model the cache cannot draw or describe is replaced by the first
        // usable entry of the same type list.
        if picked.is_none() {
            let table = SHIP_TYPES.get(kind).copied().unwrap_or(&[]);
            for candidate in table {
                if let Some(found) = spec(*candidate) {
                    model = *candidate;
                    picked = Some(found);
                    break;
                }
            }
        }
        let mut spec = picked?;
        spec.model = model;
        let type_name = spec.name.clone();
        spec.name = format!("{} {}", spec.name, next_id + 1);
        let mut ship = NpcShip::new(&spec, role, position, anchor, rng);
        ship.type_name = type_name;
        ship.id = next_id;
        next_id += 1;
        if !role.hostile() {
            // Hold the radius the station spawn used, not the default.
            ship.orbit_radius = orbit;
        }
        if matches!(role, Role::Trader | Role::Shuttle) {
            ship.max_speed = ship.max_speed.max(STATION_CRUISE_SPEED);
            ship.accel = ship.accel.max(STATION_ACCEL);
        }
        Some(ship)
    };

    let spread = |centre: [i64; 3], rng: &mut Random, angle: f64, radius: f64| {
        let mut a = angle;
        a += (rng.bounded(16_384) as f64 / 16_384.0 - 0.5) * 0.8;
        let lift = (rng.bounded(1_000_000) as f64) / 1_000_000.0 * 0.4e6;
        [
            centre[0] + (a.cos() * radius) as i64,
            centre[1] + lift as i64,
            centre[2] + (a.sin() * radius) as i64,
        ]
    };

    for i in 0..counts.traders {
        let position = spread(anchor.position, rng, i as f64 * 2.1, orbit);
        if let Some(mut ship) = add(Role::Trader, 1, position, anchor.position, rng) {
            // F689: a trader appears with `+0x100` set to a station and flies to
            // it; the port starts it on that station's docking cycle instead of
            // an endless orbit.
            if let Some(station) = anchor.station {
                ship.begin_transit(station, anchor.position);
            }
            ships.push(ship);
        }
    }
    // F690: F689's setup with type 0x11 (DATA_006197's models). The source uses
    // the same anchor/orbit placement and departure timer, then calls script
    // event 0x17; the port starts them hostile (see `Role::hostile`).
    for i in 0..counts.thargoids {
        let position = spread(anchor.position, rng, 1.0 + i as f64 * 2.1, orbit);
        if let Some(ship) = add(Role::Thargoid, 0x11, position, anchor.position, rng) {
            ships.push(ship);
        }
    }
    // F691: station-bay ships. F700 draws a place (`RND(DATA_008997)`), F650
    // allocates a bay on it and the ship starts in F747's state 0x24 with the
    // bay's object time (F691 does not add F747's two days). One full station
    // just loses that ship (F940), so the loop keeps going.
    let places: Vec<Anchor> = if options.station_traffic {
        anchors
            .iter()
            .copied()
            .filter(|anchor| anchor.station.is_some())
            .collect()
    } else {
        Vec::new()
    };
    for _ in 0..counts.bay_ships {
        let Some(place) = places
            .get(rng.bounded(places.len() as u32) as usize)
            .copied()
        else {
            break;
        };
        let Some(mut ship) = add(Role::Trader, 1, place.position, place.position, rng) else {
            continue;
        };
        // The bay is granted by the caller's traffic record, which owns F650;
        // `spawn` marks the ship and keeps F1530's object time.
        ship.state = State::Docked;
        ship.station = place.station;
        ship.bay = None;
        ship.pad = None;
        ship.dock_span = docked_object_seconds(rng.advance());
        ship.dock_timer = ship.dock_span;
        ship.dock_retry = 0.0;
        ship.dock_wait = 0.0;
        ships.push(ship);
    }
    // F699: F701 requires a station, F772(0xf) creates the aux trader, F702 puts
    // it on the 0x5000/0x18 orbit and state 0x0d is a hold row (no control or
    // movement routine). The source gives it no station target or dock request.
    // The port's orbit for a capital ship: 20 km off the station's surface, or
    // twice the ship's radius if larger.
    let capital_orbit = |place: &Anchor, ship: &NpcShip| {
        place.radius as f64 + CAPITAL_ORBIT.max(2.0 * ship.radius as f64)
    };
    if let Some(place) = orbital_station {
        for i in 0..counts.aux_traders {
            if let Some(mut ship) = add(Role::Shuttle, 0xf, place.position, place.position, rng) {
                ship.orbit_radius = capital_orbit(place, &ship);
                ship.position =
                    spread(place.position, rng, 1.0 + i as f64 * 2.1, ship.orbit_radius);
                ships.push(ship);
            }
        }
    }
    for i in 0..counts.police {
        let position = spread(anchor.position, rng, 2.6 + i as f64 * 2.1, orbit);
        if let Some(ship) = add(Role::Police, 0xe, position, anchor.position, rng) {
            ships.push(ship);
        }
    }
    // F694/F695: the pirate group. The leader is F694's lone type-0xa ship in
    // state 5; the escort uses F695's grouped type 0xb list, state 0xb, spawned
    // at its leader's formation offset and carrying the group bounty (F697's
    // offset 2). A scripted capture puts the group `PIRATE_DISTANCE` ahead of
    // the player; a real system entry uses the anchor orbit, the port's scaled
    // stand-in for F702's system-wide random placement, so the source's pirate
    // count cannot force a group beside the pad/arrival point.
    let right = normalise(cross([0.0, 1.0, 0.0], forward));
    let mut leader = None;
    for i in 0..counts.pirates {
        let (side, back) = if i == 0 { (1.0, 0.0) } else { (-1.0, 0.8e6) };
        let distance = PIRATE_DISTANCE + f64::from(rng.bounded(1_000_000));
        let position = if options.near_player {
            [
                player[0]
                    + (forward[0] * distance + right[0] * side * 0.8e6 - forward[0] * back) as i64,
                player[1]
                    + (forward[1] * distance + right[1] * side * 0.8e6 - forward[1] * back) as i64,
                player[2]
                    + (forward[2] * distance + right[2] * side * 0.8e6 - forward[2] * back) as i64,
            ]
        } else {
            // The group lurks well outside the port's orbit, so a commander
            // on the pad is not beside it (the executed original's start pad
            // has no attacker); it comes for the player in flight.
            spread(
                anchor.position,
                rng,
                3.6 + i as f64 * 2.1,
                orbit + PIRATE_LURK,
            )
        };
        let holder = if options.near_player {
            player
        } else {
            anchor.position
        };
        let Some(mut ship) = add(
            Role::Pirate,
            if i == 0 { 0xa } else { 0xb },
            position,
            holder,
            rng,
        ) else {
            continue;
        };
        let id = ship.id;
        ship.bounty = pirate_bounty(rng, i > 0);
        if i == 0 {
            leader = Some(id);
        } else {
            // F695's escort: +0xfe is the leader, +0x100 the target, and the
            // +0x102..+0x106 words a station in the leader's frame. The words are
            // 500/-500 in the original's position units; the port scales them to
            // its frame (0.4e6/0.6e6) so the escorts are visibly apart.
            ship.state = State::Escort;
            ship.leader = leader;
            ship.formation = if i == 1 {
                [400_000, 0, -400_000]
            } else {
                [-400_000, 0, -600_000]
            };
        }
        ships.push(ship);
    }
    ships
}

/// F688's ships as [`crate::spawn::run`] drew them: the original's roles,
/// models and places, with the port's own positions and flight state (F702's
/// placement hashes its own explicit state, so it is not part of the shared
/// pair). The ships' private streams are seeded from each object's seed (the
/// pair word F703 stored), never from the shared pair.
///
/// `anchors` are the system's places in the order [`crate::spawn::World`]
/// numbers them; `player` picks the anchor the system-wide ships orbit.
pub fn spawn_plan(
    player: [i64; 3],
    anchors: &[Anchor],
    plan: &[crate::spawn::Spawned],
    police: Option<u32>,
    spec: impl Fn(u32) -> Option<Spec>,
) -> Vec<NpcShip> {
    use crate::spawn::Role as Planned;
    let nearest = anchors
        .iter()
        .copied()
        .min_by(|a, b| {
            distance_between(a.position, player).total_cmp(&distance_between(b.position, player))
        })
        .unwrap_or(Anchor {
            position: player,
            radius: 1_000_000,
            station: None,
            orbital_station: false,
        });
    let orbit_of =
        |anchor: &Anchor| (anchor.radius as f64 * 1.2).clamp(ANCHOR_ORBIT, laser::RANGE * 0.75);
    let spread = |centre: [i64; 3], rng: &mut Random, angle: f64, radius: f64| {
        let a = angle + (rng.bounded(16_384) as f64 / 16_384.0 - 0.5) * 0.8;
        let lift = (rng.bounded(1_000_000) as f64) / 1_000_000.0 * 0.4e6;
        [
            centre[0] + (a.cos() * radius) as i64,
            centre[1] + lift as i64,
            centre[2] + (a.sin() * radius) as i64,
        ]
    };
    let mut ships: Vec<NpcShip> = Vec::new();
    let mut leader: Option<u32> = None;
    let mut escorts = 0usize;
    for (n, planned) in plan.iter().enumerate() {
        let Some(mut spec) = spec(planned.outfit.model) else {
            continue;
        };
        spec.model = planned.outfit.model;
        let id = ships.len() as u32;
        let type_name = spec.name.clone();
        spec.name = format!("{} {}", spec.name, id + 1);
        let mut rng = Random::new(
            planned.outfit.seed ^ 0x5bd1_e995,
            (n as u32 + 1).wrapping_mul(0x9e37_79b9),
        );
        let anchor = planned
            .place
            .and_then(|place| anchors.get(place as usize).copied());
        let (role, position, holder) = match planned.role {
            Planned::Trader | Planned::Thargoid => {
                let home = anchor.unwrap_or(nearest);
                let position = spread(home.position, &mut rng, n as f64 * 2.1, orbit_of(&home));
                (
                    if planned.role == Planned::Trader {
                        Role::Trader
                    } else {
                        Role::Thargoid
                    },
                    position,
                    home.position,
                )
            }
            Planned::Bay => {
                let home = anchor.unwrap_or(nearest);
                (Role::Trader, home.position, home.position)
            }
            Planned::Aux => {
                let home = anchor.unwrap_or(nearest);
                (Role::Shuttle, home.position, home.position)
            }
            Planned::PirateLeader | Planned::PirateEscort => {
                // F702/F1507: a random phase on the system-wide orbit round the star (the
                // origin), with the original's spread in height (its two traced pirates sit at
                // 0.3 and 0.05 of the radius above the plane).
                let phase = f64::from(rng.bounded(16_384)) / 16_384.0 * std::f64::consts::TAU;
                let height = (f64::from(rng.bounded(16_384)) / 16_384.0 - 0.5) * 0.8;
                let (flat, up) = (height.cos() * PIRATE_ORBIT, height.sin() * PIRATE_ORBIT);
                let position = [
                    (phase.cos() * flat) as i64,
                    up as i64,
                    (phase.sin() * flat) as i64,
                ];
                (Role::Pirate, position, nearest.position)
            }
        };
        let mut ship = NpcShip::new(&spec, role, position, holder, &mut rng);
        ship.type_name = type_name;
        ship.id = id;
        // F773's outfit chose the drive and the fuel; F776/F901 need them.
        ship.drive = planned.outfit.drive;
        ship.cargo_fuel = planned.outfit.fuel;
        if !role.hostile() {
            ship.orbit_radius = orbit_of(&anchor.unwrap_or(nearest));
        }
        if matches!(role, Role::Trader | Role::Shuttle) {
            ship.max_speed = ship.max_speed.max(STATION_CRUISE_SPEED);
            ship.accel = ship.accel.max(STATION_ACCEL);
        }
        match planned.role {
            Planned::Trader => {
                // F689: it flies to a station for a bay.
                if let Some(station) = anchor.and_then(|home| home.station) {
                    ship.begin_transit(station, anchor.map_or(holder, |home| home.position));
                }
            }
            Planned::Bay => {
                // F691: parked in a bay the caller grants (F650).
                ship.state = State::Docked;
                ship.station = anchor.and_then(|home| home.station);
                ship.bay = None;
                ship.pad = None;
                ship.dock_span = docked_object_seconds(rng.advance());
                ship.dock_timer = ship.dock_span;
                ship.dock_retry = 0.0;
                ship.dock_wait = 0.0;
            }
            Planned::Aux => {
                if let Some(home) = anchor {
                    let around = home.radius as f64 + CAPITAL_ORBIT.max(2.0 * ship.radius as f64);
                    ship.orbit_radius = around;
                    ship.position = spread(home.position, &mut rng, 1.0 + n as f64 * 2.1, around);
                }
            }
            Planned::PirateLeader => {
                ship.bounty = planned
                    .bounty_group
                    .and_then(|row| PIRATE_BOUNTIES.get(usize::from(row)).copied())
                    .unwrap_or(ship.bounty);
                leader = Some(id);
                escorts = 0;
            }
            Planned::PirateEscort => {
                ship.bounty = planned
                    .bounty_group
                    .and_then(|row| PIRATE_BOUNTIES.get(usize::from(row)).copied())
                    .unwrap_or(ship.bounty);
                escorts += 1;
                ship.state = State::Escort;
                ship.leader = leader;
                ship.formation = match escorts {
                    1 => [400_000, 0, -400_000],
                    2 => [-400_000, 0, -600_000],
                    _ => [0, 0, -800_000],
                };
                // F695 spawns an escort at its leader's formation offset, so the group
                // arrives together rather than scattered round the whole orbit.
                if let Some(head) = leader.and_then(|id| ships.get(id as usize)) {
                    ship.position = add(head.position, ship.formation);
                }
            }
            Planned::Thargoid => {}
        }
        ships.push(ship);
    }
    // The port's one police patrol (F688 draws none; the response to a wanted
    // player is the port's reconstruction, see EXP-015). `police` is the
    // allegiance; the new game's spawn passes `None`.
    if let Some(allegiance) = police {
        let mut rng = Random::new(0x504f_4c49, anchors.len() as u32 + 1);
        let model = model_for(Role::Police, allegiance, &mut rng);
        if let Some(mut spec) = spec(model) {
            spec.model = model;
            let id = ships.len() as u32;
            let type_name = spec.name.clone();
            spec.name = format!("{} {}", spec.name, id + 1);
            let position = spread(nearest.position, &mut rng, 2.6, orbit_of(&nearest));
            let mut ship = NpcShip::new(&spec, Role::Police, position, nearest.position, &mut rng);
            ship.type_name = type_name;
            ship.id = id;
            ship.orbit_radius = orbit_of(&nearest);
            ships.push(ship);
        }
    }
    ships
}

fn offset(anchor: [i64; 3], phase: f64, radius: f64) -> [i64; 3] {
    [
        anchor[0] + (phase.cos() * radius) as i64,
        anchor[1] + (phase.sin() * radius * 0.12) as i64,
        anchor[2] + (phase.sin() * radius) as i64,
    ]
}

fn sub(a: [i64; 3], b: [i64; 3]) -> [f64; 3] {
    [
        (a[0] - b[0]) as f64,
        (a[1] - b[1]) as f64,
        (a[2] - b[2]) as f64,
    ]
}

fn add(a: [i64; 3], b: [i64; 3]) -> [i64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// The unit direction away from `from`, as seen from `of`.
fn away(of: [i64; 3], from: [i64; 3]) -> [f64; 3] {
    normalise(sub(of, from))
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    vector.map(|v| v * factor)
}

fn length(vector: [f64; 3]) -> f64 {
    vector.iter().map(|v| v * v).sum::<f64>().sqrt()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(vector: [f64; 3]) -> [f64; 3] {
    let length = length(vector);
    if length < 1e-9 {
        [1.0, 0.0, 0.0]
    } else {
        scale(vector, 1.0 / length)
    }
}

fn turn_toward(from: [f64; 3], to: [f64; 3], amount: f64) -> [f64; 3] {
    let dot = from[0] * to[0] + from[1] * to[1] + from[2] * to[2];
    let angle = dot.clamp(-1.0, 1.0).acos();
    if angle < 1e-6 {
        return from;
    }
    let t = (amount / angle).clamp(0.0, 1.0);
    // Spherical interpolation of unit vectors. Nearly opposite vectors have no
    // stable plane (the interpolation cancels back to `from` and the ship never
    // turns), so they swing round at once, as exactly opposite ones always did.
    let sin = angle.sin();
    if sin.abs() < 1e-6 {
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

fn upright(forward: [f64; 3]) -> [f64; 3] {
    let world_up = [0.0, 1.0, 0.0];
    let right = cross(world_up, forward);
    if length(right) < 1e-6 {
        return [0.0, 0.0, 1.0];
    }
    normalise(cross(forward, normalise(right)))
}

fn distance_between(a: [i64; 3], b: [i64; 3]) -> f64 {
    length(sub(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::{Reply, Starport, CLEARANCE_SECONDS};

    fn details() -> Details {
        // Ross 154's F869 report: bytes [128, 1, 2, 3, 0].
        Details {
            words: [34032, 32779],
            bytes: [128, 1, 2, 3, 0],
            allegiance: 134,
        }
    }

    fn system(thargoids: usize) -> System {
        System::new(details(), thargoids)
    }

    fn spec(model: u32) -> Option<Spec> {
        let names = ["Eagle", "Viper", "Cobra Mk III", "Shuttle"];
        (14..=0x50).contains(&model).then(|| Spec {
            limits: None,
            model,
            name: names[(model as usize) % names.len()].to_owned(),
            radius: 10_000,
            collision_radius: 8_000,
            class: 0,
            mass: if model >= 0x35 { 800 } else { 25 },
            shield_strength: 0x4014,
            thrust: if model >= 0x35 { 3_270 } else { 13_625 },
            drive: 2,
            gun_mounts: 1,
            rating: 10,
            missiles: 2,
            cargo: 8,
        })
    }

    fn tick(ship: &mut NpcShip, seconds: f64, player: [i64; 3]) {
        // 20 ms at 1x is 994 tics.
        let tics = (994.0 * seconds / 0.02) as u32;
        ship.tick(seconds, tics, player, None);
    }

    #[test]
    fn a_ships_object_record_carries_the_fields_the_port_holds() {
        use crate::object_record::{
            ObjectRecord, CONTROL_AI_SHIP, KIND_PIRATE, LASER_FIRED, LASER_FRONT, LASER_SHIELDS_HIT,
        };
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(
            &spec,
            Role::Pirate,
            [1_000, 2_000, 3_000],
            [0; 3],
            &mut Random::new(1, 2),
        );
        ship.damage.shields = 12;
        ship.damage.hull = 34;
        ship.flash = 0.5;
        ship.shot = Some(ShotMark {
            code: 0xa0,
            length: ShotMark::MISS,
            seconds: 0.1,
        });
        ship.slot = 7;
        let record = ship.object_record();
        assert_eq!(record.model, spec.model);
        assert_eq!(record.object, 7);
        assert_eq!(record.control, CONTROL_AI_SHIP);
        assert_eq!(record.position, [1_000, 2_000, 3_000]);
        assert_eq!(record.agent.kind, KIND_PIRATE);
        assert_eq!(record.agent.name_string(), spec.name);
        assert_eq!(record.agent.radius, ship.radius);
        assert_eq!(record.agent.collision_radius, spec.collision_radius);
        assert_eq!(record.agent.flags, spec.class);
        assert_eq!(record.drive, spec.drive);
        assert_eq!(record.agent.ai_mode, State::Attack.original()[0]);
        assert_eq!(record.agent.capacity, ship.cargo);
        assert_eq!(record.agent.bounty, ship.bounty);
        assert_eq!(record.shields, 12);
        assert_eq!(record.hull, 34);
        assert_eq!(
            record.agent.laser_flags,
            LASER_SHIELDS_HIT | LASER_FIRED | LASER_FRONT
        );
        // The snapshot is a real 0x152-byte record, not a parallel struct.
        let mut bytes = vec![0u8; crate::objects::OBJECT_BYTES];
        record.write(&mut bytes);
        assert_eq!(ObjectRecord::from_bytes(&bytes), Some(record));
    }

    #[test]
    fn the_live_slots_and_object_indexes_reach_the_record() {
        use crate::object_record::{ObjectIndexes, ShipSlots};
        let spec = spec(0x17).unwrap();
        let mut rng = Random::new(1, 2);
        let mut pirate = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut rng);
        pirate.id = 4;
        let mut trader = NpcShip::new(&spec, Role::Trader, [1; 3], [0; 3], &mut rng);
        trader.id = 9;
        let mut ships = vec![pirate, trader];
        let mut slots = ShipSlots::new();
        slots.assign(&mut ships);
        assert_eq!(ships[0].slot, 0x72);
        assert_eq!(ships[1].slot, 0x71);

        // The pirate fights the trader: `+0xfe` is the trader's object slot.
        ships[0].npc_target = Some(9);
        let record = ships[0].object_record_in(&ObjectIndexes {
            slots: Some(&slots),
            player: Some(0x70),
            station: None,
        });
        assert_eq!(record.object, 0x72);
        assert_eq!(record.agent.target, 0x71);
        assert_eq!(record.agent.nav_target, 0);

        // A trader approaching a starport (body slot 3) indexes it in both.
        ships[1].station = Some(0);
        ships[1].state = State::Approach;
        let record = ships[1].object_record_in(&ObjectIndexes {
            slots: Some(&slots),
            player: None,
            station: Some(3),
        });
        assert_eq!(record.object, 0x71);
        assert_eq!(record.agent.target, 3);
        assert_eq!(record.agent.nav_target, 3);

        // A hostile with no ship target attacks the player object.
        ships[0].npc_target = None;
        let record = ships[0].object_record_in(&ObjectIndexes {
            slots: Some(&slots),
            player: Some(0x70),
            station: None,
        });
        assert_eq!(record.agent.target, 0x70);

        // F927's freed slot is the next one the list hands out.
        ships.truncate(1);
        slots.assign(&mut ships);
        assert_eq!(slots.slot_of(9), None);
        assert_eq!(slots.slot_of(4), Some(0x72));
    }

    #[test]
    fn a_hostile_reports_the_player_and_an_evading_ship_reports_its_missile() {
        use crate::object_record::{ObjectIndexes, ShipSlots};
        let spec = spec(0x17).unwrap();
        let mut rng = Random::new(1, 2);
        let mut pirate = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut rng);
        pirate.id = 4;
        let mut slots = ShipSlots::new();
        // F1074 reserves `DATA_008857` before F688's traffic.
        slots.reserve_player();
        slots.assign(std::slice::from_mut(&mut pirate));
        assert_eq!(slots.player_slot(), Some(0x72));
        assert_eq!(pirate.slot, 0x71);
        assert!(pirate.state_hunts());

        // An attacker with no ship target reports the player object's index.
        let record = pirate.object_record_in(&ObjectIndexes {
            slots: Some(&slots),
            player: slots.player_slot(),
            station: None,
        });
        assert_eq!(record.agent.target, 0x72);

        // F941/F757: an evading ship's `+0xfe` is the missile's object slot.
        pirate.evade_missile(7);
        slots.allocate_missile(7);
        let missile_slot = slots.slot_of_missile(7).expect("the missile's slot");
        let record = pirate.object_record_in(&ObjectIndexes {
            slots: Some(&slots),
            player: slots.player_slot(),
            station: None,
        });
        assert_eq!(record.agent.target, missile_slot);
        assert_eq!(record.agent.ai_mode, State::Evade.original()[0]);
    }

    #[test]
    fn a_fleeing_ship_records_the_jump_reading_of_the_tail() {
        use crate::object_record::HyperspaceWords;
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut Random::new(1, 2));
        ship.motion.cap = [1, 2, 3];
        assert_eq!(ship.object_record().caps(), [1, 2, 3]);
        // F682 saves the mode it leaves in `+0xf8` before the flee state.
        ship.state = State::Attack;
        ship.ai_code = Some(0x04);
        ship.state = State::Flee;
        ship.jump = HyperspaceWords {
            destination: 0x1234,
            max_range: 9,
            post_mode: 4,
            jump_time: 77,
        };
        let record = ship.object_record();
        assert_eq!(record.hyperspace(), ship.jump);
        assert_eq!(record.caps()[0], 0x1234);
    }

    #[test]
    fn f953_smokes_after_hull_damage_and_f938_counts_it_down() {
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut Random::new(1, 2));
        ship.state = State::Hold;
        ship.damage.shields = 0;
        ship.damage.hull = 100;
        assert!(!ship.smoking());
        // F953's hull branch (`ffe.asm` 0x3e3a4): `+0x14e = 0xa`.
        assert_eq!(ship.hit(1), crate::laser::Damage::Hull);
        assert_eq!(ship.smoke_timeout, SMOKE_TIMEOUT);
        assert!(ship.smoking());
        // F938 runs once a second and counts the smoke and E.C.M. timeouts
        // down; F732's `+0x14d` starts a 19-second period.
        ship.ecm_timeout = 3;
        for _ in 0..SMOKE_TIMEOUT {
            tick(&mut ship, 1.0, [0; 3]);
        }
        assert_eq!(ship.smoke_timeout, 0);
        assert!(!ship.smoking());
        assert_eq!(ship.ecm_timeout, 0);
    }

    #[test]
    fn f939_smoke_sparks_draw_two_shared_words_at_the_ship() {
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(
            &spec,
            Role::Pirate,
            [1_000, 2_000, 3_000],
            [0; 3],
            &mut Random::new(1, 2),
        );
        let mut rng = Random::new(0x1234_5678, 0x9abc_def0);
        ship.speed = 20_000.0;
        assert!(ship.smoke_spark(&mut rng).is_none());
        ship.smoke_timeout = SMOKE_TIMEOUT;
        let before = (rng.a, rng.b);
        let spark = ship.smoke_spark(&mut rng).expect("a smoking ship sparks");
        assert_eq!(spark.kind, crate::effects::Kind::Hit);
        assert_eq!(spark.position, ship.position);
        // F939 draws two F1530 words from the frame's shared pair.
        assert_ne!((rng.a, rng.b), before);
        // F733/F734 and F737 do not run a movement routine, so no particle.
        ship.state = State::Docked;
        assert!(ship.smoke_spark(&mut rng).is_none());
        ship.state = State::Hold;
        ship.smoke_timeout = 0;
        assert!(ship.smoke_spark(&mut rng).is_none());
    }

    #[test]
    fn f732_counts_the_for_timeout_to_zero_then_resets_to_0x13() {
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Trader, [0; 3], [0; 3], &mut Random::new(1, 2));
        assert_eq!(ship.for_timeout, 0);
        tick(&mut ship, 1.0, [0; 3]);
        // The clear byte runs F606 on this pass.
        assert!(ship.take_frame_update());
        assert_eq!(ship.for_timeout, FOR_TIMEOUT);
        for _ in 0..FOR_TIMEOUT {
            tick(&mut ship, 1.0, [0; 3]);
        }
        // 19 decrements later the byte is zero; the next second runs F606's
        // update again and resets it.
        assert_eq!(ship.for_timeout, 0);
        assert!(!ship.take_frame_update());
        tick(&mut ship, 1.0, [0; 3]);
        assert_eq!(ship.for_timeout, FOR_TIMEOUT);
        assert!(ship.take_frame_update());
    }

    #[test]
    fn f606_puts_the_ship_in_the_closer_bodys_frame() {
        use crate::frames::FrameBody;
        use crate::object_record::CONTROL_AI_SHIP;
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(
            &spec,
            Role::Trader,
            [0, 0, 1 << 20],
            [0; 3],
            &mut Random::new(1, 2),
        );
        ship.state = State::Transit;
        tick(&mut ship, 1.0, [0; 3]);
        assert!(ship.take_frame_update());
        // A body centred on the root scores (7 - 20) << 4 = -208 against the
        // root's 0x32, so the ship belongs in its frame.
        let candidates = [FrameBody {
            slot: 5,
            parent: 0,
            weight: 20,
            position: [0; 3],
        }];
        assert_eq!(ship.update_frame_of_reference(&candidates), 5);
        let record = ship.object_record();
        assert_eq!(record.parent, 5);
        assert_eq!(record.position, ship.position);
        assert_eq!(record.control, CONTROL_AI_SHIP);
        // F606 scans the live list: the ship's own slot is not a frame.
        ship.slot = 9;
        ship.frame_parent = 0;
        let candidates = [
            FrameBody {
                slot: 5,
                parent: 0,
                weight: 20,
                position: [0; 3],
            },
            FrameBody {
                slot: 9,
                parent: 0,
                weight: 20,
                position: ship.position,
            },
        ];
        assert_eq!(ship.update_frame_of_reference(&candidates), 5);
    }

    #[test]
    fn f610s_avoidance_timeout_runs_sixteen_navigation_frames() {
        let spec = spec(0x17).unwrap();
        // The anchor is far away so the ship stays in navigation instead of
        // turning into F753's arrival hold.
        let mut ship = NpcShip::new(
            &spec,
            Role::Trader,
            [0; 3],
            [4_000_000, 0, 0],
            &mut Random::new(1, 2),
        );
        ship.state = State::Transit;
        assert_eq!(ship.avoid_timeout, 0);
        tick(&mut ship, 0.0, [0; 3]);
        assert!(ship.take_avoid_due());
        assert_eq!(ship.avoid_timeout, AVOID_TIMEOUT);
        for _ in 0..AVOID_TIMEOUT {
            tick(&mut ship, 0.0, [0; 3]);
            assert!(!ship.take_avoid_due());
        }
        assert_eq!(ship.avoid_timeout, 0);
        tick(&mut ship, 0.0, [0; 3]);
        assert!(ship.take_avoid_due());
        assert_eq!(ship.avoid_timeout, AVOID_TIMEOUT);
        // A combat state does not run F610.
        ship.state = State::Attack;
        ship.avoid_timeout = 5;
        tick(&mut ship, 0.0, [0; 3]);
        assert_eq!(ship.avoid_timeout, 5);
    }

    #[test]
    fn f802_steering_aims_away_and_rearms_the_check() {
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Trader, [0; 3], [0; 3], &mut Random::new(1, 2));
        ship.state = State::Transit;
        ship.forward = [0.0, 0.0, 1.0];
        ship.speed = 10_000.0;
        ship.avoid_timeout = 3;
        // A body ahead: the normal points back, plus a quarter of the heading.
        ship.steer_avoid([0, 0, 1_000_000], 20_000.0);
        let aim = ship.avoid_target.expect("an avoidance aim");
        assert!(aim[2] < 0, "the aim is away from the body: {aim:?}");
        assert_eq!(ship.avoid_timeout, 0);
        assert_eq!(ship.think_tics, THINK_TICS);
        // The next guidance step consumes the aim.
        ship.tick(0.02, 994, [0, 0, 10_000_000], None);
        assert!(ship.avoid_target.is_none());
        assert!(ship.forward[2] < 1.0, "{:?}", ship.forward);
    }

    #[test]
    fn f802s_obstruction_test_uses_the_stopping_distance_and_radii() {
        let ahead = [([0, 0, 4_000], 1_000.0)];
        // 10 km/s, 10 km/s^2: the stopping distance is 5,000 mm.
        let found = obstruction_along([0; 3], [0.0, 0.0, 1.0], 10_000.0, 10_000.0, 1_000.0, &ahead)
            .expect("the body is on the path");
        assert_eq!(found.0, [0, 0, 4_000]);
        assert_eq!(found.1, 2_000.0);
        // Behind the ship, out of reach, or off to the side: no obstruction.
        assert!(obstruction_along(
            [0; 3],
            [0.0, 0.0, 1.0],
            10_000.0,
            10_000.0,
            1_000.0,
            &[([0, 0, -4_000], 1_000.0)]
        )
        .is_none());
        assert!(obstruction_along(
            [0; 3],
            [0.0, 0.0, 1.0],
            10_000.0,
            10_000.0,
            1_000.0,
            &[([0, 0, 50_000], 1_000.0)]
        )
        .is_none());
        assert!(obstruction_along(
            [0; 3],
            [0.0, 0.0, 1.0],
            10_000.0,
            10_000.0,
            1_000.0,
            &[([5_000, 0, 4_000], 1_000.0)]
        )
        .is_none());
    }

    #[test]
    fn f680_and_f682_give_the_carried_ship_the_players_jump_words() {
        use crate::sector::Drive;
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut Random::new(1, 2));
        ship.state = State::Attack;
        ship.cargo_fuel = 5;
        let mode = ship.ai_state();
        let records = persist(&[ship], [0; 3]);
        assert_eq!(records[0].jump.post_mode, mode);
        let mut drives = vec![
            Drive {
                power: 0,
                fuel_modifier: 0,
                mass: 0,
                name: 0,
            };
            3
        ];
        drives[2] = Drive {
            power: 30_000,
            fuel_modifier: 0,
            mass: 4,
            name: 7,
        };
        // F680 writes the system just left to `+0xf2`; F682's second F901 uses
        // the player's 802-unit jump with the pirate's own mass and drive.
        let jumped = carried_jump(records.clone(), 802, 0x2a, &drives);
        assert_eq!(jumped[0].jump.destination, 0x2a);
        assert_eq!(jumped[0].jump.max_range, 1_200); // 30,000 / 25
        assert!(jumped[0].jump.jump_time > 0);
        assert_eq!(jumped[0].cargo_fuel, 4);
        // A jump the pirate's drive cannot make is F682's failure: F940 kills
        // the carried ship, so the record is dropped.
        assert!(carried_jump(records, 2_000, 0x2a, &drives).is_empty());
    }

    #[test]
    fn f738_adds_the_jump_time_to_the_object_clock() {
        use crate::object_record::HyperspaceWords;
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut Random::new(1, 2));
        // `+0xfa = 0x1_8000`: one whole day (`>> 16`) and 0x8000 << 16 of the
        // low word.
        let words = HyperspaceWords {
            destination: 0x10,
            max_range: 0x20,
            post_mode: 0x01,
            jump_time: 0x0001_8000,
        };
        ship.begin_jump(words, [0x1000_0000, 5]);
        assert_eq!(ship.jump, words);
        assert_eq!(ship.jump_clock, [0x9000_0000, 6]);
        assert!((ship.flee - jump_seconds(0x0001_8000)).abs() < 1e-9);
        ship.state = State::Flee;
        let record = ship.object_record();
        assert_eq!(record.hyperspace(), words);
        // F738's object time lands in `+0xa8`/`+0xac`.
        assert_eq!(record.clock_tics, 0x9000_0000);
        assert_eq!(record.clock_days, 6);
    }

    #[test]
    fn a_fleeing_ship_waits_out_the_jump_time_not_the_port_three_seconds() {
        use crate::object_record::HyperspaceWords;
        let spec = spec(0x17).unwrap();
        let mut ship = NpcShip::new(&spec, Role::Pirate, [0; 3], [0; 3], &mut Random::new(1, 2));
        ship.state = State::Attack;
        ship.ai_code = Some(0x01);
        ship.damage.shields = 0;
        ship.damage.hull = 1;
        tick(&mut ship, 0.02, [0; 3]);
        assert_eq!(ship.state, State::Flee);
        assert_eq!(ship.jump.post_mode, 0x01);
        let words = HyperspaceWords {
            destination: 7,
            max_range: 9,
            post_mode: 0x01,
            jump_time: 100,
        };
        ship.begin_jump(words, [0; 2]);
        let time = jump_seconds(100);
        assert!(time > FLEE_SECONDS);
        tick(&mut ship, time - 1.0, [0; 3]);
        assert_eq!(ship.state, State::Flee);
        tick(&mut ship, 1.5, [0; 3]);
        assert_eq!(ship.state, State::Escaped);
    }

    #[test]
    fn a_carried_ship_keeps_its_f2_jump_and_returns_in_its_mode() {
        use crate::object_record::HyperspaceWords;
        let spec = spec(0x17).unwrap();
        let mut pirate = NpcShip::new(
            &spec,
            Role::Pirate,
            [5_000, 0, 0],
            [0; 3],
            &mut Random::new(1, 2),
        );
        pirate.state = State::Attack;
        pirate.ai_code = Some(0x01);
        pirate.jump = HyperspaceWords {
            destination: 7,
            max_range: 9,
            post_mode: 0x01,
            jump_time: 100,
        };
        let records = persist(std::slice::from_ref(&pirate), [0; 3]);
        assert_eq!(records.len(), 1);
        // F682/F680: `+0xf2..+0xfd` crosses the jump.
        assert_eq!(records[0].jump, pirate.jump);
        let mut ship = restore(&records, [0; 3], 10).remove(0);
        // F682 leaves the ship in the delayed exit cloud (0x11) with `+0xf8`
        // holding the mode to restore, and F737/F740 wait the jump time.
        assert_eq!(ship.state, State::Jump);
        assert_eq!(ship.jump.post_mode, 0x01);
        assert!(!ship.visible());
        assert_eq!(ship.object_record().hyperspace().destination, 7);
        tick(&mut ship, jump_seconds(100) - 1.0, [0; 3]);
        assert_eq!(ship.state, State::Jump);
        tick(&mut ship, 2.0, [0; 3]);
        assert_eq!(ship.state, State::Attack);
        assert_eq!(ship.jump, HyperspaceWords::default());
        assert!(ship.visible());
    }

    #[test]
    fn the_retro_power_follows_the_backward_thrust() {
        assert_eq!(retro_power(0), 0);
        assert_eq!(retro_power(0x7fff), 0);
        assert_eq!(retro_power(-0x0aa4), 0);
        assert_eq!(retro_power(-0x0aa5), 1);
        assert_eq!(retro_power(-0x1329), 2);
    }

    #[test]
    fn a_ship_facing_directly_away_still_turns() {
        let turned = turn_toward([1.0, 0.0, 0.0], [-0.9999999999999999, 0.0, 0.0], 0.1);
        assert_eq!(turned, [-0.9999999999999999, 0.0, 0.0]);
    }

    #[test]
    fn the_ai_table_is_data_006255() {
        assert_eq!(AI_STATES.len(), 0x25);
        for (index, row) in AI_STATES.iter().enumerate() {
            assert_eq!(usize::from(row.code), index);
        }
        // State 0 is the plain-navigation row: F742 (event 6), F727 (event 5),
        // no death notification and F789's damage response.
        let row = ai_state(0x00).unwrap();
        assert_eq!(
            (row.control, row.movement, row.notify, row.damage),
            (Some(742), Some(727), None, Some(789))
        );
        // F752/F725/F782/F785: attack turn; F724 is the firing state's event-5
        // routine, not a third "combat" slot.
        let row = ai_state(0x01).unwrap();
        assert_eq!(
            (row.control, row.movement, row.notify, row.damage),
            (Some(752), Some(725), Some(782), Some(785))
        );
        assert_eq!(ai_state(0x02).unwrap().movement, Some(724));
        // F749/F727/F782/F785: F694's pirate state 5 (the transition to F750).
        let row = ai_state(0x05).unwrap();
        assert_eq!(
            (row.control, row.movement, row.notify, row.damage),
            (Some(749), Some(727), Some(782), Some(785))
        );
        // F751/F729/F782/F785: F695's formation state 0xb.
        let row = ai_state(0x0b).unwrap();
        assert_eq!(
            (row.control, row.movement, row.notify, row.damage),
            (Some(751), Some(729), Some(782), Some(785))
        );
        // 0xd is all zero; 0xe/0xf are the parked ships with only F790's
        // damage response; 0x11 is F737's jump countdown and 0x12 F740's exit.
        assert_eq!(
            ai_state(0x0d).unwrap(),
            &AiState {
                code: 0x0d,
                name: "none",
                control: None,
                movement: None,
                notify: None,
                damage: None,
            }
        );
        assert_eq!(ai_state(0x0e).unwrap().damage, Some(790));
        assert_eq!(ai_state(0x11).unwrap().control, Some(737));
        assert_eq!(ai_state(0x12).unwrap().control, Some(740));
        // The station cycle: F734 request, F747 docked, F735 launch, F746
        // final approach, F733's docked object time.
        assert_eq!(ai_state(0x1f).unwrap().control, Some(734));
        assert_eq!(ai_state(0x20).unwrap().control, Some(747));
        assert_eq!(ai_state(0x21).unwrap().control, Some(735));
        assert_eq!(ai_state(0x23).unwrap().control, Some(746));
        assert_eq!(ai_state(0x24).unwrap().control, Some(733));
        assert!(ai_state(0x25).is_none());
        // Every port state names only real rows.
        for state in [
            State::Transit,
            State::Approach,
            State::Docked,
            State::Launch,
            State::Depart,
            State::Hold,
            State::Escort,
            State::Attack,
            State::Pass,
            State::Evade,
            State::Flee,
            State::Jump,
            State::Dead,
            State::Escaped,
        ] {
            for code in state.original() {
                assert!(ai_state(*code).is_some(), "{state:?} names {code:#x}");
            }
        }
    }

    #[test]
    fn f585_halves_the_turn_limit_for_big_hulls() {
        assert_eq!(turn_divisor(0x17), 1);
        assert_eq!(turn_divisor(0x35), 2);
        assert_eq!(turn_divisor(0x3b), 4);
        let eagle = NpcShip::new(
            &spec(0x17).unwrap(),
            Role::Pirate,
            [0; 3],
            [0; 3],
            &mut Random::new(1, 2),
        );
        let big = NpcShip::new(
            &spec(0x3b).unwrap(),
            Role::Pirate,
            [0; 3],
            [0; 3],
            &mut Random::new(1, 2),
        );
        assert!((eagle.turn_rate - TURN_RATE).abs() < 1e-12);
        assert!((big.turn_rate - TURN_RATE / 4.0).abs() < 1e-12);
        // F601's thrust/mass ratio: the light Eagle outruns the big hull.
        assert!(eagle.max_speed > big.max_speed);
        assert!(eagle.accel > big.accel);
    }

    #[test]
    fn the_pirate_bounty_comes_from_data_006268() {
        let mut rng = Random::new(7, 9);
        let mut seen = Vec::new();
        for _ in 0..64 {
            seen.push(pirate_bounty(&mut rng, false));
            seen.push(pirate_bounty(&mut rng, true));
        }
        assert!(seen.iter().all(|bounty| PIRATE_BOUNTIES.contains(bounty)));
        assert!(seen.iter().any(|bounty| *bounty > 0));
        // A grouped pirate draws from offset 2, a lone one from offset 4, so the
        // lone range includes the higher entries.
        assert!(PIRATE_BOUNTIES[16..].iter().all(|bounty| *bounty > 0));
    }

    #[test]
    fn a_pirate_holds_its_laser_energy_and_gates() {
        let ship = NpcShip::new(
            &spec(0x17).unwrap(),
            Role::Pirate,
            [0; 3],
            [0; 3],
            &mut Random::new(1, 2),
        );
        assert_eq!(ship.laser, Some(laser::PULSE_1MW));
        // The drive word gives F773's energy: class 2 is 0x20 << 24.
        assert_eq!(ship.guns.energy, 0x2000_0000);
        assert_eq!(drive_energy(2), 0x2000_0000);
        assert_eq!(drive_energy(0), 0);
    }

    fn aimed_ship(role: Role, from: [i64; 3], at: [i64; 3]) -> NpcShip {
        let mut ship = NpcShip::new(
            &spec(0x17).unwrap(),
            role,
            from,
            from,
            &mut Random::new(1, 2),
        );
        ship.forward = normalise(sub(at, from));
        ship.up = upright(ship.forward);
        ship
    }

    /// F597's accelerated branch: at 10000x a pirate 40,000 km out is placed at its waypoint
    /// beside the player with the player's velocity; at 1x the same ship flies.
    #[test]
    fn at_accelerated_time_a_hunting_pirate_is_placed_beside_the_player() {
        let player = [0, 0, 0];
        let far = [0, 0, 40_000_000_000];
        let limits = crate::thrust::Limits {
            axes: [[3270, -3270], [3270, -3270], [8720, -3270]],
        };
        let mut ship = aimed_ship(Role::Pirate, far, player);
        assert!(ship.hostile && ship.state == State::Attack);
        ship.limits = Some(limits);
        ship.pursuit = Pursuit {
            time_shift: TIME_SHIFTS[4],
            velocity: [100, 0, -50],
            combat_velocity: [0; 3],
        };
        ship.tick(0.02, 994, player, None);
        assert!(ship.take_placed());
        assert_ne!(ship.waypoint, [0; 3], "F749 picks the waypoint first");
        let distance = length(ship.position.map(|v| v as f64));
        assert!(distance < 2.0e6, "placed {distance} mm away");
        assert_eq!(ship.motion.velocity, [100, 0, -50]);
        assert_eq!(ship.motion.thrust, [0; 3]);

        let mut slow = aimed_ship(Role::Pirate, far, player);
        slow.limits = Some(limits);
        slow.tick(0.02, 994, player, None);
        assert!(!slow.take_placed());
        assert!(length(slow.position.map(|v| v as f64)) > 3.9e10);
        // Too far for the step at 10x: the ordinary guidance.
        assert_eq!(
            crate::autopilot::accelerated_reach(22, -14, TIME_SHIFTS[1]),
            None
        );
        assert_eq!(
            crate::autopilot::accelerated_reach(22, -14, TIME_SHIFTS[4]),
            Some(5)
        );
    }

    /// F931 sets the gun's fired flag whether or not the ray hits, and F1776 draws the beam
    /// for it: a shot that hit carries `distance >> 7`, a miss `0xffff`, for a frame or so.
    #[test]
    fn a_shot_is_remembered_for_the_beam_hit_or_miss() {
        let player = [0, 0, 0];
        let mut ship = aimed_ship(Role::Pirate, [0, 0, 2_000_000], player);
        assert!(ship.shot.is_none());
        let shot = ship.fire(994, player, 10_000).expect("a hit");
        let mark = ship.shot.expect("the beam of a hit");
        assert_eq!(mark.code, laser::PULSE_1MW);
        assert_eq!(i64::from(mark.length), (shot.distance as i64) >> 7);
        assert_eq!(mark.seconds, BEAM_SECONDS);
        // A miss still draws, at the full length.
        ship.guns.refire = 0;
        ship.forward = [0.0, 0.0, 1.0];
        assert!(ship.fire(994, player, 10_000).is_none());
        assert_eq!(ship.shot.map(|mark| mark.length), Some(ShotMark::MISS));
        // The beam lasts about one frame of the original.
        ship.tick(BEAM_SECONDS / 2.0, 1, player, None);
        assert!(ship.shot.is_some());
        ship.tick(BEAM_SECONDS, 1, player, None);
        assert!(ship.shot.is_none());
    }

    #[test]
    fn firing_obeys_f931s_gates_and_applies_f930s_damage() {
        let player = [0, 0, 0];
        let mut ship = aimed_ship(Role::Pirate, [0, 0, 2_000_000], player);
        let shot = ship.fire(994, player, 10_000).expect("a hit");
        assert_eq!(shot.damage, 50);
        assert!(!shot.beam);
        assert_eq!(shot.ship, ship.id);
        assert!(shot.distance > 0.0);
        // F930 set the refire word; the next shot is refused.
        assert_eq!(ship.guns.refire, laser::PULSE_REFIRE);
        assert!(ship.fire(994, player, 10_000).is_none());
        // F730's countdown clears it; then a shot behind the player misses but
        // still spends the refire word.
        ship.guns.refire = 0;
        ship.forward = [0.0, 0.0, 1.0];
        assert!(ship.fire(994, player, 10_000).is_none());
        assert_ne!(ship.guns.refire, 0);
        // Heat above the limit and no energy refuse before the ray is tested,
        // and an unarmed trader never fires.
        ship.guns.refire = 0;
        ship.guns.heat = laser::HEAT_LIMIT + 1;
        assert!(ship.fire(994, player, 10_000).is_none());
        let mut trader = aimed_ship(Role::Trader, [0, 0, 2_000_000], player);
        assert!(trader.fire(994, player, 10_000).is_none());
        // Clean ships do not fire even with a gun.
        let mut police = aimed_ship(Role::Police, [0, 0, 2_000_000], player);
        assert!(police.fire(994, player, 10_000).is_none());
        assert!(police.provoke());
        assert!(police.fire(994, player, 10_000).is_some());
    }

    #[test]
    fn a_damaged_hostile_runs_and_jumps_out() {
        let player = [0, 0, 0];
        let mut ship = aimed_ship(Role::Pirate, [0, 0, 2_000_000], player);
        ship.damage.hull = 1;
        tick(&mut ship, 0.02, player);
        assert_eq!(ship.state, State::Flee);
        let before = distance_between(ship.position, player);
        tick(&mut ship, 1.0, player);
        assert!(distance_between(ship.position, player) > before);
        tick(&mut ship, FLEE_SECONDS, player);
        assert_eq!(ship.state, State::Escaped);
        assert!(!ship.alive());
        assert!(ship.finished(player));
    }

    #[test]
    fn an_attack_pass_brakes_for_the_standoff() {
        let player = [0, 0, 0];
        let mut ship = aimed_ship(Role::Pirate, [0, 0, 3.5e6 as i64], player);
        let start = distance_between(ship.position, player);
        let mut closest = start;
        let mut peak: f64 = 0.0;
        for _ in 0..8000 {
            tick(&mut ship, 0.02, player);
            let distance = distance_between(ship.position, player);
            closest = closest.min(distance);
            peak = peak.max(ship.speed);
        }
        assert!(peak > 0.5 * ship.max_speed, "it never accelerated: {peak}");
        // The braking rule keeps it out of the player's lap, and the ship is
        // still alive and hostile after 8 s.
        assert!(closest > 0.5 * STANDOFF, "closed to {closest}");
        assert!(closest < 2.0 * STANDOFF, "never closed: {closest}");
        assert!(ship.alive());
        assert!(matches!(ship.state, State::Attack | State::Pass));
    }

    /// F750 re-aims once per game second (0xc22e tics); F727 applies the stored
    /// thrust words every frame in between. The executed original's pirate leader
    /// changes its words at frames 532, 579, 627, ... of a 1,043-tic frame run.
    #[test]
    fn guidance_is_refreshed_once_per_game_second_not_every_frame() {
        let player = [0, 0, 0];
        let mut ship = aimed_ship(Role::Pirate, [0, 0, 20_000_000], player);
        ship.limits = Some(crate::thrust::Limits {
            axes: [[3270, -3270], [3270, -3270], [8720, -3270]],
        });
        let mut changes = Vec::new();
        let mut previous = ship.motion.thrust;
        let frames = 400u32;
        for frame in 0..frames {
            // 994 tics a 20 ms frame: 400 frames are 397,600 tics, 8 thinks.
            ship.tick(0.02, 994, player, None);
            if ship.motion.thrust != previous {
                changes.push(frame);
                previous = ship.motion.thrust;
            }
        }
        assert!(!changes.is_empty(), "the ship never aimed");
        // The first frame thinks at once; later thinks are 0xc22e tics (49.1 frames
        // at 994 tics) apart, so at most 9 changes in 400 frames, never per frame.
        assert!(changes.len() <= 9, "guidance changed on {changes:?}");
        for pair in changes.windows(2) {
            assert!(pair[1] - pair[0] >= 40, "two thinks {pair:?} frames apart");
        }
    }

    /// F783 clears the wanted bit and the executed original sends every attacking police
    /// ship to state 8 in the same frame; a ship still waiting in the starport stays.
    /// F748 looks at the door only on the ship's own step (once per game second from its
    /// creation): the executed original's orbital-station police (`--flight idle` from
    /// `--start-key=0x04`, FFE_POLICE_TRACE) leave state 0x22 143 frames (three steps) after they
    /// are made, with the door opening on frame 126, 17 frames earlier.
    #[test]
    fn a_waiting_police_ship_attacks_on_its_first_step_after_the_door_opens() {
        let player = [0, 0, 0];
        let mut ship = aimed_ship(Role::Police, [0, 0, 2_000_000], player);
        ship.state = State::Launch;
        ship.scrambled = true;
        ship.scramble_tics = 0;
        let mut station = Station {
            index: 0,
            position: [0, 0, 2_000_000],
            axis: [0.0, 0.0, 1.0],
            front: 1.0e6,
            radius: 1.0e6,
            pad: None,
            bay: Some(0),
            cleared: false,
            closing: false,
            door_open: false,
        };
        let mut turned_at = None;
        for frame in 1..=300u32 {
            station.door_open = frame >= 126;
            let mut requests = Vec::new();
            ship.tick_with_station(
                1043.0 / TICS_PER_SECOND,
                1043,
                player,
                None,
                Some(&station),
                &mut requests,
            );
            if ship.take_turned() && turned_at.is_none() {
                turned_at = Some(frame);
            }
        }
        let frame = turned_at.expect("the police attack once the door is open");
        assert!((142..=144).contains(&frame), "turned on frame {frame}");
        assert_eq!(ship.state, State::Attack);
    }

    /// Replays one police ship of the executed original (AI states 1, 2 and 3: F753, F754, F755,
    /// F756) through `NpcShip::tick`. Local only: it needs a trace with raw object records:
    ///
    /// ```text
    /// FFE_POLICE_RAW=1 FFE_POLICE_TRACE=1 FFE_FIRE_FROM=200 FFE_FIRE_TO=200 \
    ///   jjffe-smoke.exe --flight idle 3200 --start-key=0x02 > police.jsonl
    /// FFE_POLICE_TRACE_FILE=police.jsonl cargo test -p ffe-core --lib \
    ///   police_attack_cycle -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "needs the reference trace in FFE_POLICE_TRACE_FILE"]
    fn police_attack_cycle_follows_the_executed_original() {
        let path = std::env::var("FFE_POLICE_TRACE_FILE").expect("FFE_POLICE_TRACE_FILE");
        let hex = |text: &str| -> Vec<u8> {
            (0..text.len() / 2)
                .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap())
                .collect()
        };
        let i16_at = |o: &[u8], at: usize| i16::from_le_bytes([o[at], o[at + 1]]);
        let i32_at = |o: &[u8], at: usize| i32::from_le_bytes(o[at..at + 4].try_into().unwrap());
        let i64_at = |o: &[u8], at: usize| i64::from_le_bytes(o[at..at + 8].try_into().unwrap());
        // (frame, ship record, player record) for the ship that appears first.
        let mut track: Vec<(u64, Vec<u8>, Vec<u8>)> = Vec::new();
        let mut slot = None;
        for line in std::fs::read_to_string(path).unwrap().lines() {
            if !line.starts_with("{\"police\"") {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            let Some(ships) = v["ships"].as_array() else {
                continue;
            };
            for s in ships {
                let id = s["slot"].as_u64().unwrap();
                if slot.is_none() {
                    slot = Some(id);
                }
                if slot == Some(id) {
                    track.push((
                        v["police"].as_u64().unwrap(),
                        hex(s["raw"].as_str().unwrap()),
                        hex(v["praw"].as_str().unwrap()),
                    ));
                }
            }
        }
        assert!(track.len() > 250, "{} frames", track.len());
        let first = &track[0].1;
        let mut model_spec = spec(0x17).unwrap();
        model_spec.model = u32::from_le_bytes(first[0x82..0x86].try_into().unwrap());
        model_spec.limits = Some(crate::thrust::Limits {
            axes: std::array::from_fn(|k| {
                [i16_at(first, 0xbc + 4 * k), i16_at(first, 0xbe + 4 * k)]
            }),
        });
        let player0 = &track[0].2;
        let origin: [i64; 3] = std::array::from_fn(|k| i64_at(player0, 0x3e + 8 * k));
        let mut ship = NpcShip::new(
            &model_spec,
            Role::Police,
            [0, 0, 0],
            [0, 0, 0],
            &mut Random::new(1, 2),
        );
        let q = |v: i32| f64::from(v) / 2_147_483_647.0;
        let rel =
            |o: &[u8]| -> [i64; 3] { std::array::from_fn(|k| i64_at(o, 0x3e + 8 * k) - origin[k]) };
        let load = |ship: &mut NpcShip, o: &[u8]| {
            ship.position = rel(o);
            ship.velocity =
                std::array::from_fn(|k| f64::from(i32_at(o, 0x8c + 4 * k)) * MM_PER_VELOCITY_UNIT);
            let row =
                |r: usize| -> [f64; 3] { std::array::from_fn(|c| q(i32_at(o, 4 * (3 * r + c)))) };
            ship.up = row(1);
            ship.forward = row(2);
            ship.motion.thrust = [i16_at(o, 0xb6), i16_at(o, 0xb8), i16_at(o, 0xba)];
            ship.motion.autopilot = [i16_at(o, 0x110), i16_at(o, 0x112), i16_at(o, 0x114)];
            // The rest of what F727 reads: caps, rates, sub-unit fractions, drift, mounts.
            ship.motion.cap = [i32_at(o, 0xf2), i32_at(o, 0xf6), i32_at(o, 0xfa)];
            for k in 0..3 {
                ship.motion.rate[k] = i16_at(o, 0xb0 + 2 * k);
                ship.motion.fraction[k] = u16::from_le_bytes([o[0xec + 2 * k], o[0xed + 2 * k]]);
            }
            ship.motion.drift = o[0x24] as i8;
            ship.motion.gun_mounts = o[0xd1] as i8;
            ship.motion.craft = o[0x86];
            ship.motion.parent = o[0x56];
            ship.motion.rotating = o[0x57];
            ship.motion.switch_delay = i32_at(o, 0x148);
            ship.motion.updated = o[0x58];
            ship.attack_phase = o[0xff];
            ship.in_fudge = matches!(o[0xff], 1..=3);
            ship.state = if o[0xff] == 3 {
                State::Pass
            } else {
                State::Attack
            };
            ship.hostile = true;
        };
        load(&mut ship, first);
        // The ship's next AI step is the first later frame where its timer byte changes.
        let step = track
            .iter()
            .position(|(_, o, _)| o[0x14d] != first[0x14d])
            .expect("a second AI step");
        ship.think_tics = (THINK_TICS + 1).saturating_sub(1043 * u32::try_from(step).unwrap());
        let state_of = |ship: &NpcShip| match (ship.state, ship.attack_phase) {
            (State::Pass, _) => 3,
            (_, phase) => phase,
        };
        let (mut equal_state, mut equal_thrust, mut total) = (0usize, 0usize, 0usize);
        let mut first_bad = None;
        let mut hits: Vec<u64> = Vec::new();
        ship.laser = Some(first[0xd2]);
        for (n, (frame, record, _)) in track.iter().enumerate().skip(1) {
            // The slot is reused when the ship is removed: only its own life is replayed.
            if *frame != track[n - 1].0 + 1 {
                break;
            }
            ship.tick(1043.0 / TICS_PER_SECOND, 1043, [0, 0, 0], None);
            // F931 asks for a shot each frame the ship is in state 2; the player is a 32 m box.
            if ship.fire(1043, [0, 0, 0], 32_000).is_some() {
                hits.push(*frame);
            }
            total += 1;
            if std::env::var_os("FFE_VERBOSE").is_some() && (n % 12 == 0) {
                let orig = rel(record);
                eprintln!(
                    "n {n} frame {frame}: port pos {:?} v {:?} turn {:?}; original pos {:?} v {:?} turn {:?}",
                    ship.position,
                    ship.motion.velocity,
                    ship.motion.autopilot,
                    orig,
                    [i32_at(record, 0x8c), i32_at(record, 0x90), i32_at(record, 0x94)],
                    [i16_at(record, 0x110), i16_at(record, 0x112), i16_at(record, 0x114)],
                );
            }
            let same_state = state_of(&ship) == record[0xff];
            let thrust = [
                i16_at(record, 0xb6),
                i16_at(record, 0xb8),
                i16_at(record, 0xba),
            ];
            let same_thrust = ship.motion.thrust == thrust;
            equal_state += usize::from(same_state);
            equal_thrust += usize::from(same_thrust);
            if (!same_state || !same_thrust) && first_bad.is_none() {
                first_bad = Some((
                    n,
                    *frame,
                    state_of(&ship),
                    record[0xff],
                    ship.motion.thrust,
                    thrust,
                    distance_between(ship.position, [0, 0, 0]) as i64,
                    (record.len() > 0x152).then_some(0),
                ));
            }
        }
        // The original's ship 103 hits the player on frames 1252 and 1253 only (`phull` 100, 92, 84).
        eprintln!(
            "{total} frames: state equal {equal_state}, thrust equal {equal_thrust}; first difference {first_bad:?}; port hits on frames {hits:?}"
        );
        assert!(equal_state * 10 > total * 9, "the state cycle drifted");
    }

    #[test]
    fn the_police_stand_down_when_the_wanted_bit_clears() {
        let mut attacker = aimed_ship(Role::Police, [0, 0, 2_000_000], [0; 3]);
        attacker.hostile = true;
        attacker.responding = true;
        attacker.state = State::Attack;
        attacker.stand_down();
        assert!(!attacker.hostile && !attacker.responding);
        assert_eq!(attacker.state, State::Transit);
        let mut waiting = aimed_ship(Role::Police, [0, 0, 2_000_000], [0; 3]);
        waiting.state = State::Launch;
        waiting.scrambled = true;
        waiting.stand_down();
        assert_eq!(waiting.state, State::Launch);
        assert!(waiting.scrambled);
        // A pirate is not the police's to recall.
        let mut pirate = aimed_ship(Role::Pirate, [0, 0, 2_000_000], [0; 3]);
        pirate.hostile = true;
        pirate.state = State::Attack;
        pirate.stand_down();
        assert!(pirate.hostile);
        assert_eq!(pirate.state, State::Attack);
    }

    #[test]
    fn a_transit_ship_reaches_its_anchor_and_holds() {
        let anchor = [2.0e6 as i64, 0, 0];
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Trader,
            [0, 0, 0],
            anchor,
            &mut Random::new(3, 4),
        );
        ship.state = State::Transit;
        let mut closest = f64::MAX;
        for _ in 0..8000 {
            tick(&mut ship, 0.02, [0, 0, 0]);
            closest = closest.min(distance_between(ship.position, anchor));
        }
        assert_eq!(ship.state, State::Hold);
        assert!(closest < 1.5 * ARRIVE, "never arrived: {closest}");
        assert!(distance_between(ship.position, anchor) < 6.0e6);
    }

    #[test]
    fn f675_keeps_a_close_pirate_and_f680_restores_it() {
        let mut rng = Random::new(11, 12);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[Anchor {
                position: [0, 0, 0],
                radius: 2_000_000,
                station: None,
                orbital_station: false,
            }],
            &system(0),
            0,
            &mut rng,
            SpawnOptions::CAPTURE,
            spec,
        );
        let mut pirate = ships
            .iter()
            .find(|s| s.role == Role::Pirate)
            .unwrap()
            .clone();
        pirate.position = [0, 0, 1_000_000];
        pirate.damage.hull = 5;
        let records = persist(&[pirate.clone()], [0; 3]);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].offset, [0, 0, 1_000_000]);
        // A pirate far away is not kept.
        pirate.position = [0, 0, 9_000_000];
        assert!(persist(&[pirate], [0; 3]).is_empty());
        // F680 restores the kept ship at the player's arrival point.
        let restored = restore(&records, [5_000_000, 0, 0], 7);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, 7);
        assert_eq!(restored[0].position, [5_000_000, 0, 1_000_000]);
        assert_eq!(restored[0].damage.hull, 5);
        assert!(restored[0].is_hostile());
        assert_eq!(restored[0].state, State::Attack);
    }

    /// The executed original's default start (`--npc-trace 260 0x39`) holds no
    /// police ship (type 0xe) and F688 has no police draw, so no entry spawns
    /// one; only the scripted capture (`guarantee`) keeps the port's patrol.
    #[test]
    fn the_new_game_pass_spawns_no_police() {
        let anchors = [anchor(4), anchor(7)];
        let run = |start: bool| {
            let mut system = system(0);
            system.new_game = start;
            spawn(
                [0, 0, 0],
                [0.0, 0.0, 1.0],
                &anchors,
                &system,
                0,
                &mut Random::new(7, 0x1234),
                SpawnOptions::default(),
                spec,
            )
        };
        assert!(run(true).iter().all(|ship| ship.role != Role::Police));
        assert!(run(false).iter().all(|ship| ship.role != Role::Police));
    }

    /// F688's rolls stand: the old per-kind caps (two traders, three bay
    /// ships, one aux trader) are gone, and only F691's own 25 and the object
    /// budget bound a spawn. A seed that rolls more than three bay ships keeps
    /// them all.
    #[test]
    fn f688s_rolls_are_not_capped_below_the_source() {
        let places = 12;
        let anchors: Vec<Anchor> = (0..places).map(anchor).collect();
        let found = (1..20_000u32).find_map(|a| {
            let b = a.wrapping_mul(0x9e37_79b9);
            let counts = counts_with_places(&system(0), places, &mut Random::new(a, b));
            (counts.bay_ships > 3).then_some((a, b, counts))
        });
        let (a, b, counts) = found.expect("a seed rolls more than three bay ships");
        assert!(counts.bay_ships <= 0x19);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &anchors,
            &system(0),
            0,
            &mut Random::new(a, b),
            SpawnOptions::default(),
            spec,
        );
        let docked = ships
            .iter()
            .filter(|ship| ship.state == State::Docked)
            .count();
        assert_eq!(docked, counts.bay_ships);
        assert!(ships.len() <= MAX_SHIPS + 1);
    }

    #[test]
    fn f686s_pass_rolls_no_pirates_at_danger_one() {
        let mut start = system(0);
        start.start = true;
        assert_eq!(start.details.bytes[1], 1);
        for seed in 0..64 {
            let c = counts(&start, &mut Random::new(seed, seed ^ 0x55));
            assert_eq!(c.pirates, 0, "seed {seed}");
        }
        let entered = (0..64)
            .map(|seed| counts(&system(0), &mut Random::new(seed, seed ^ 0x55)).pirates)
            .max();
        assert!(entered > Some(0), "a jump in can bring pirates");
    }

    #[test]
    fn f688_counts_come_from_the_system_record_and_are_capped() {
        let mut rng = Random::new(0x1234_5678, 0x4567_89ab);
        let c = counts(&system(0), &mut rng);
        assert!(c.traders <= 3);
        // RND(+6 << 2): 0..3 groups for danger byte 1.
        assert!(c.pirates < 4);
        assert_eq!(c.police, 1);
        assert_eq!(c.thargoids, 0);
        // F35(0x16) is an exclusive limit: F688 calls F922 on it exactly as it
        // does for the trader counts, so the result is below the limit.
        let c = counts(&system(3), &mut rng);
        assert!(c.thargoids < 3);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[Anchor {
                position: [1_000_000, 0, 0],
                radius: 4_500_000,
                station: None,
                orbital_station: false,
            }],
            &system(0),
            2,
            &mut rng,
            SpawnOptions::CAPTURE,
            spec,
        );
        assert!((1..=MAX_SHIPS).contains(&ships.len()));
        assert!(ships.iter().any(|s| s.role == Role::Trader));
        assert!(ships.iter().any(|s| s.role == Role::Pirate));
        // The pirate group is near the player, within the laser's range.
        let pirate = ships.iter().find(|s| s.role == Role::Pirate).unwrap();
        assert!(distance_between(pirate.position, [0, 0, 0]) < laser::RANGE);
        // The trader orbits the anchor, not the player.
        let trader = ships.iter().find(|s| s.role == Role::Trader).unwrap();
        assert!(distance_between(trader.position, [1_000_000, 0, 0]) < 10.0e6);
        // Every NPC that carries a laser can fire it, and the police do not
        // start hostile.
        let police = ships.iter().find(|s| s.role == Role::Police).unwrap();
        assert!(!police.is_hostile());
        assert!(police.laser.is_some());
    }

    /// Without the port's capture guarantee, F688's own counts decide: a record
    /// whose +6/+8 bytes are zero rolls no traffic at all. This is the rule the
    /// new game and an ordinary system entry use, so a system the source leaves
    /// empty cannot force a pirate group beside the player.
    #[test]
    fn the_system_counts_alone_can_spawn_no_traffic() {
        let mut rng = Random::new(7, 8);
        let empty = System::new(
            Details {
                words: [0, 0],
                bytes: [128, 0, 0, 0, 0],
                allegiance: 0,
            },
            0,
        );
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[Anchor {
                position: [1_000_000, 0, 0],
                radius: 4_500_000,
                station: None,
                orbital_station: false,
            }],
            &empty,
            0,
            &mut rng,
            SpawnOptions::default(),
            spec,
        );
        assert!(ships.is_empty(), "{ships:?}");
        // The capture guarantee is what adds the one trader/pirate group.
        let mut rng = Random::new(7, 8);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[Anchor {
                position: [1_000_000, 0, 0],
                radius: 4_500_000,
                station: None,
                orbital_station: false,
            }],
            &empty,
            0,
            &mut rng,
            SpawnOptions::CAPTURE,
            spec,
        );
        assert!(ships.iter().any(|ship| ship.role == Role::Trader));
        assert!(ships.iter().any(|ship| ship.role == Role::Pirate));
    }

    /// F1530 (`ffe.asm:539f4`) is the object-time random word F747/F691 read:
    /// the four steps below are the source's `rotl(b ^ 0x1234567f, 25)` add to
    /// `a` and `rotl(old a, 13)` into `b`.
    #[test]
    fn f1530_advances_the_object_time_words() {
        let mut rng = Random::new(0x1234_5678, 0x4567_89ab);
        let expected = [
            (0xbae2_fe37u32, 0x8acf_0246u32),
            (0x2e14_f4df, 0x5fc6_f75c),
            (0x74b0_da21, 0x9e9b_e5c2),
            (0xefca_3988, 0x1b44_2e96),
        ];
        for (a, b) in expected {
            assert_eq!(rng.advance(), a);
            assert_eq!((rng.a, rng.b), (a, b));
        }
    }

    /// F691's object time is `[D8818] << 4` tics plus `[D8818] >> 28` days. It
    /// has no `+2` days (F747 adds those for an arriving ship), so the first
    /// station-bay ship can launch within its first day.
    #[test]
    fn f691_object_time_is_the_f1530_word_without_f747s_two_days() {
        assert_eq!(docked_object_seconds(0), 0.0);
        assert_eq!(docked_object_seconds(1), 16.0 / TICS_PER_SECOND);
        assert_eq!(docked_object_seconds(0x1000_0000), 86_400.0);
        assert_eq!(
            docked_object_seconds(0x1000_0040),
            86_400.0 + 1024.0 / 49_700.0
        );
        assert_eq!(docked_object_seconds(0xf000_0000), 15.0 * 86_400.0);
        // F747's two days are a separate `[+0xac] += 2`.
        assert!(docked_object_seconds(u32::MAX) < 17.0 * 86_400.0);
        // The port's regular F747 wait still starts at two days.
        let ship = &mut spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[anchor(0)],
            &system(0),
            0,
            &mut Random::new(1, 2),
            SpawnOptions::CAPTURE,
            spec,
        )
        .pop()
        .expect("a trader");
        ship.begin_docked([1, 2, 3]);
        assert!((2.0..3.0).contains(&(ship.dock_timer / 86_400.0)));
    }

    fn anchor(place: usize) -> Anchor {
        Anchor {
            position: [1_000_000 + place as i64 * 2_000_000, 0, 0],
            radius: 4_500_000,
            station: Some(place),
            orbital_station: true,
        }
    }

    /// Find a seed whose F688 pass gives a non-zero F691 roll, returning the
    /// seed and the counts. Scanning keeps the test independent of one lucky
    /// constant while still exercising the real `RND(4 * places)` draw.
    fn seed_with_bay(places: usize) -> (u32, u32, Counts) {
        for a in 1..20_000u32 {
            let b = a.wrapping_mul(0x9e37_79b9);
            let mut rng = Random::new(a, b);
            let counts = counts_with_places(&system(0), places, &mut rng);
            if counts.bay_ships > 0 {
                return (a, b, counts);
            }
        }
        panic!("no seed drew a station-bay ship");
    }

    /// F691/F700: a non-zero `RND(4 * places)` spawns that many type-1 traders
    /// in `State::Docked`, each with a random place as `+0xfe` and no bay yet
    /// (F650 belongs to the traffic record). The port caps the branch at
    /// [`MAX_BAY_SHIPS`].
    #[test]
    fn f691_spawns_docked_traders_from_the_place_table() {
        let places = 2;
        let (a, b, counts) = seed_with_bay(places);
        let anchors = [anchor(4), anchor(7)];
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &anchors,
            &system(0),
            0,
            &mut Random::new(a, b),
            SpawnOptions::default(),
            spec,
        );
        let docked: Vec<&NpcShip> = ships
            .iter()
            .filter(|ship| ship.state == State::Docked)
            .collect();
        assert_eq!(
            docked.len(),
            counts.bay_ships.min(MAX_BAY_SHIPS),
            "F691 count {} with places {places}",
            counts.bay_ships
        );
        for ship in docked {
            assert_eq!(ship.role, Role::Trader);
            assert_eq!(ship.role.type_code(), 1);
            assert!(
                matches!(ship.station, Some(4) | Some(7)),
                "{:?}",
                ship.station
            );
            assert_eq!(ship.bay, None);
            assert_eq!(ship.pad, None);
            assert!((0.0..16.0 * 86_400.0).contains(&ship.dock_span));
            assert_eq!(ship.dock_timer, ship.dock_span);
            assert!(SHIP_TYPES[1].contains(&ship.model));
        }
        // The capture fixture asks for no station traffic, so its frames keep
        // only the trader/pirate group they were built against.
        let capture = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &anchors,
            &system(0),
            0,
            &mut Random::new(a, b),
            SpawnOptions::CAPTURE,
            spec,
        );
        assert!(capture
            .iter()
            .all(|ship| ship.state != State::Docked && ship.role != Role::Shuttle));
    }

    /// F688's station branch has a quirk: with places present, F699's aux draw
    /// runs only when F691's roll was non-zero. A zero bay roll skips the aux
    /// traders too.
    #[test]
    fn f691s_zero_roll_skips_f699_like_the_source() {
        for a in 1..20_000u32 {
            let b = a.wrapping_mul(0x9e37_79b9);
            let mut rng = Random::new(a, b);
            let counts = counts_with_places(&system(0), 2, &mut rng);
            if counts.bay_ships == 0 {
                assert_eq!(counts.aux_traders, 0, "seed {a:#x} skipped F699");
                return;
            }
        }
        panic!("no seed rolled a zero F691 count");
    }

    /// F699: F772(0xf) is the Lynx/LRC list, and state 0x0d has no control or
    /// movement routine, so the aux trader holds with no station target.
    #[test]
    fn f699_aux_traders_use_the_lynx_table_and_hold() {
        assert_eq!(Role::Shuttle.type_code(), 0xf);
        assert_eq!(Role::Shuttle.ai_class(), 0xf);
        assert_eq!(SHIP_TYPES[0xf], &[0x46, 0x46, 0x47][..]);
        let mut found = None;
        for a in 1..20_000u32 {
            let b = a.wrapping_mul(0x9e37_79b9);
            let mut rng = Random::new(a, b);
            let counts = counts_with_places(&system(0), 1, &mut rng);
            if counts.aux_traders > 0 {
                found = Some((a, b));
                break;
            }
        }
        let (a, b) = found.expect("no seed drew an aux trader");
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[anchor(4)],
            &system(0),
            0,
            &mut Random::new(a, b),
            SpawnOptions::default(),
            spec,
        );
        let aux = ships
            .iter()
            .find(|ship| ship.role == Role::Shuttle)
            .expect("F699 did not spawn");
        assert_eq!(aux.state, State::Hold);
        assert_eq!(aux.station, None);
        assert!(SHIP_TYPES[0xf].contains(&aux.model));
        assert!(aux.max_speed >= STATION_CRUISE_SPEED);
    }

    /// F751 hands the escort to the attack when its leader is in state 1/2; the
    /// formation offsets are dropped and the leader link goes with it.
    #[test]
    fn f751_turns_a_formation_escort_into_an_attacker() {
        // Two pirate groups are needed for F695's escort; the fixture's +6 byte
        // is 1, so ask for 2 explicitly.
        let mut details = details();
        details.bytes[1] = 2;
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[anchor(4)],
            &System::new(details, 0),
            0,
            &mut Random::new(11, 12),
            SpawnOptions::CAPTURE,
            spec,
        );
        let mut escort = ships
            .iter()
            .find(|ship| ship.state == State::Escort)
            .expect("F695's escort")
            .clone();
        assert!(escort.leader.is_some());
        assert_ne!(escort.formation, [0; 3]);
        assert!(escort.join_attack());
        assert_eq!(escort.state, State::Attack);
        assert_eq!(escort.leader, None);
        assert_eq!(escort.formation, [0; 3]);
        // Only an escort with a leader joins; a second call does nothing.
        assert!(!escort.join_attack());
    }

    /// A Thargoid model's statistics: DATA_006197's models have 0x31's 232 t
    /// hull/203 t cargo, 1-2 gun mounts and (except 0x45) the shared 0x800e
    /// drive word.
    fn thargoid_spec(model: u32, cargo: u16, guns: i8) -> Spec {
        Spec {
            limits: None,
            model,
            name: "Unknown".to_owned(),
            radius: 90_000,
            collision_radius: 90_000,
            class: 0,
            mass: 232,
            shield_strength: 16_430,
            thrust: 14_715,
            drive: 14,
            gun_mounts: guns,
            rating: 50,
            missiles: 10,
            cargo,
        }
    }

    #[test]
    fn f690_spawns_rnd_f35_0x16_thargoids_with_data_006197_models() {
        assert_eq!(Role::Thargoid.type_code(), 0x11);
        assert_eq!(Role::Thargoid.ai_class(), 0x11);
        assert!(Role::Thargoid.hostile());
        assert!(!Role::Thargoid.clean());
        assert_eq!(
            SHIP_TYPES[0x11],
            &[0x45, 0x31, 0x31, 0x32, 0x32, 0x33, 0x33, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f][..]
        );

        let anchor = Anchor {
            position: [0, 0, 0],
            radius: 2_000_000,
            station: None,
            orbital_station: false,
        };
        // The only source of the count is F35(0x16): zero spawns none.
        let mut rng = Random::new(0x2345_6789, 0x3456_789a);
        let none = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[anchor],
            &system(0),
            0,
            &mut rng,
            SpawnOptions::default(),
            spec,
        );
        assert!(none.iter().all(|ship| ship.role != Role::Thargoid));

        // F688 draws RND(F35(0x16)); use a limit large enough that this seed's
        // draw and the MAX_SHIPS cap still leave a Thargoid in the traffic.
        let limit = 12;
        let mut rng = Random::new(0x2345_6789, 0x3456_789a);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[anchor],
            &system(limit),
            0,
            &mut rng,
            SpawnOptions::default(),
            spec,
        );
        let thargoids: Vec<&NpcShip> = ships
            .iter()
            .filter(|ship| ship.role == Role::Thargoid)
            .collect();
        assert!(!thargoids.is_empty(), "RND({limit}) drew no Thargoids");
        assert!(
            thargoids.len() < limit,
            "the draw must stay below F35(0x16)"
        );
        for ship in thargoids {
            assert!(
                SHIP_TYPES[0x11].contains(&ship.model),
                "model {:#x}",
                ship.model
            );
            assert_eq!(ship.state, State::Attack);
            assert_eq!(ship.role.ai_class(), 0x11);
            assert!(ship.is_hostile());
            assert_eq!(ship.bounty, 0);
            // The test spec's cargo (8 t) reaches F774's 1MW beam entry.
            assert_eq!(ship.laser, Some(laser::BEAM_1MW));
        }
    }

    #[test]
    fn f774_picks_the_thargoid_laser_from_class_and_cargo() {
        let mut rng = Random::new(3, 4);
        // DATA_006264[0x11] is 0xb2. 203 t of cargo always reaches the 75 t
        // 20MW beam entry (0xb3), not the 900 t 0xcf entry.
        for _ in 0..32 {
            assert_eq!(
                f774_laser(0x11, 203, &mut rng),
                Some((laser::BEAM_20MW, 75))
            );
        }
        // The Thargoid Transporter's 13,000 t cargo reaches 0xcf.
        assert_eq!(
            f774_laser(0x11, 13_000, &mut rng),
            Some((laser::THARGOID, 900))
        );
        // No budget is DATA_006265[0], "no laser"; a class outside
        // DATA_006264's eighteen entries has no proportion.
        assert_eq!(f774_laser(0x11, 0, &mut rng), Some((0, 0)));
        assert_eq!(f774_laser(0x1f, 100, &mut rng), None);
        // A gun-less model gets no mount even when its cargo picks a laser.
        let mut rng = Random::new(5, 6);
        let unarmed = Role::Thargoid.laser_code(&thargoid_spec(0x45, 13_000, 0), &mut rng);
        assert_eq!(unarmed, None);
        let armed = Role::Thargoid.laser_code(&thargoid_spec(0x31, 203, 1), &mut rng);
        assert_eq!(armed, Some(laser::BEAM_20MW));
        let unarmed = Role::Thargoid.laser_code(&thargoid_spec(0x31, 0, 1), &mut rng);
        assert_eq!(unarmed, None);
    }

    #[test]
    fn a_thargoid_uses_the_model_shield_hull_and_drive_words() {
        let ship = NpcShip::new(
            &thargoid_spec(0x31, 203, 1),
            Role::Thargoid,
            [0, 0, 2_000_000],
            [0; 3],
            &mut Random::new(7, 8),
        );
        assert_eq!(ship.damage.shields, 0xb8); // 0x2e * 4
        assert_eq!(ship.damage.shields_max, 0xb8);
        assert_eq!(ship.damage.hull, 928); // 232 * 4
        assert_eq!(ship.shield_strength, 16_430);
        assert_eq!(ship.guns.energy, drive_energy(14));
        assert_eq!(ship.guns.energy, 0);
        // The source's 0x800e drive word has no DATA_006266 energy entry; the
        // port keeps that zero rather than inventing a loadout.
        assert_eq!(ship.laser, Some(laser::BEAM_20MW));
    }

    #[test]
    fn a_script_sets_original_ai_codes_and_reads_them_back() {
        let mut rng = Random::new(5, 6);
        let mut ship = script_ship(&spec(0x1c).unwrap(), 3, [0; 3], &mut rng);
        assert_eq!(ship.state, State::Hold);
        assert!(!ship.hostile);
        // 0x17, the scripts' parked state, holds the ship where it is and
        // reads back as 0x17 while it holds.
        ship.position = [7, 8, 9];
        ship.set_ai(0x17, None);
        assert_eq!(
            (ship.state, ship.anchor, ship.ai_state()),
            (State::Hold, [7, 8, 9], 0x17)
        );
        // F669's 9 is F770's trading approach: it closes on `+0xfe` without
        // attacking; `scripts::approaches` makes F770's class-0xa transition.
        ship.waypoint = [1, 2, 3];
        ship.set_ai(9, Some(None));
        assert_eq!(ship.state, State::Trading);
        assert!(!ship.hostile);
        assert_eq!(ship.ai_state(), 9);
        ship.position = [0; 3];
        tick(&mut ship, 0.02, [0, 0, TRADING_RANGE as i64 + 1_000]);
        assert_eq!(ship.state, State::Trading);
        ship.position = [0; 3];
        tick(&mut ship, 0.02, [0, 0, TRADING_RANGE as i64 - 1_000]);
        // In state 9 the ship does not hunt or fire; F724 only fires in state 2.
        assert_eq!(ship.state, State::Trading);
        assert!(!ship.hostile);
        ship.hostile = true;
        assert_eq!(ship.fire(994, [0; 3], 17_600), None);
        ship.hostile = false;
        assert_eq!(ship.waypoint, [1, 2, 3]);
        // Formation (0xb) behind ship 4.
        ship.set_ai(0xb, Some(Some(4)));
        assert_eq!((ship.state, ship.leader), (State::Escort, Some(4)));
        // A state the ship has left reads as the port's.
        ship.state = State::Flee;
        assert_eq!(ship.ai_state(), 0x1d);
        // Unknown codes change nothing.
        ship.set_ai(0x99, None);
        assert_eq!(ship.state, State::Flee);
    }

    #[test]
    fn a_script_object_stays_put_and_is_no_crime() {
        let mut rng = Random::new(5, 6);
        let object = script_object(&spec(0x1c).unwrap(), 2, [1, 2, 3], &mut rng);
        assert_eq!(object.state, State::Hold);
        assert_eq!(object.max_speed, 0.0);
        assert!(!object.hostile && !object.role.clean());
        assert!(object.laser.is_none());
    }

    #[test]
    fn f1037s_ground_contact_removes_an_attacker_not_traffic() {
        // The film's removal: a police or pirate that flies into a world goes
        // to state 0x1c; ordinary traffic is kept clear by the port's stand-in.
        let mut rng = Random::new(5, 6);
        let mut police = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Police,
            [0, 0, 0],
            [0, 0, 0],
            &mut rng,
        );
        police.state = State::Attack;
        assert!(police.hits_ground([0, 0, 0], 1_000.0));
        assert!(!police.hits_ground([0, 0, 0], 0.0));
        police.state = State::Transit;
        assert!(!police.hits_ground([0, 0, 0], 1_000.0));
        police.state = State::Dead;
        assert!(!police.hits_ground([0, 0, 0], 1_000.0));
    }

    #[test]
    fn an_escort_follows_its_leaders_formation() {
        let mut rng = Random::new(21, 22);
        // Two pirates, so F695's escort is spawned when the cap allows.
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[],
            &System::new(
                Details {
                    words: [0, 0],
                    bytes: [128, 2, 0, 3, 0],
                    allegiance: 0,
                },
                0,
            ),
            0,
            &mut rng,
            SpawnOptions::CAPTURE,
            spec,
        );
        let pirates: Vec<&NpcShip> = ships.iter().filter(|s| s.role == Role::Pirate).collect();
        assert!(pirates.len() >= 2, "no escort spawned");
        assert_eq!(pirates[0].state, State::Attack);
        assert_eq!(pirates[1].state, State::Escort);
        assert_eq!(pirates[1].leader, Some(pirates[0].id));
        assert_ne!(pirates[1].formation, [0; 3]);
    }

    #[test]
    fn damage_is_shields_then_hull_then_death_with_the_flash() {
        let mut rng = Random::new(3, 4);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[],
            &system(0),
            0,
            &mut rng,
            SpawnOptions::CAPTURE,
            spec,
        );
        let mut ship = ships.into_iter().next().unwrap();
        ship.damage = DamageState {
            shields: 100,
            shields_max: 100,
            hull: 100,
            mass: 25,
        };
        assert_eq!(ship.hit(50), Damage::Shields);
        assert!(ship.flash > 0.0);
        assert_eq!(ship.shield_percent(), 50);
        assert_eq!(ship.hit(80), Damage::Hull);
        assert_eq!(ship.damage.hull, 70);
        assert_eq!(ship.hit(70), Damage::Destroyed);
        assert!(!ship.alive());
        assert_eq!(ship.state, State::Dead);
        assert!(!ship.finished([0, 0, 0]));
        ship.tick(DEATH_SECONDS, 0, [0, 0, 0], None);
        assert!(ship.finished([0, 0, 0]));
    }

    #[test]
    fn holding_traffic_stays_near_its_anchor_and_pirates_close_in() {
        let mut rng = Random::new(5, 6);
        let ships = spawn(
            [0, 0, 0],
            [0.0, 0.0, 1.0],
            &[Anchor {
                position: [0, 0, 0],
                radius: 2_000_000,
                station: None,
                orbital_station: false,
            }],
            &system(0),
            0,
            &mut rng,
            SpawnOptions::CAPTURE,
            spec,
        );
        let mut trader = ships
            .iter()
            .find(|s| s.role == Role::Trader)
            .unwrap()
            .clone();
        for _ in 0..600 {
            tick(&mut trader, 0.02, [0, 0, 0]);
        }
        assert!(distance_between(trader.position, [0, 0, 0]) < 6.0e6);
        assert!(distance_between(trader.position, [0, 0, 0]) > 1.0e6);
        let mut pirate = ships
            .iter()
            .find(|s| s.role == Role::Pirate)
            .unwrap()
            .clone();
        let start = distance_between(pirate.position, [0, 0, 0]);
        for _ in 0..600 {
            tick(&mut pirate, 0.02, [0, 0, 0]);
        }
        assert!(distance_between(pirate.position, [0, 0, 0]) < start);
    }

    /// A station with its face at +1 km and a pad 1 km out on +z, for the
    /// docking-cycle tests. The traffic record is the caller's.
    fn test_station(port: &Starport, id: u8) -> Station {
        Station {
            index: 0,
            position: [0; 3],
            axis: [0.0, 0.0, 1.0],
            front: 1_000_000.0,
            radius: 4_000_000.0,
            pad: Some([0, 0, 1_000_000]),
            bay: port.bay_of(id),
            cleared: port.is_docking(id),
            closing: port.mode == crate::traffic::DOCKING_DONE,
            door_open: port.door == 0,
        }
    }

    /// Apply a ship's requests to the traffic record exactly as
    /// `Navigator::apply_dock_request` does, for one ship. Returns how many times
    /// F762 picked a station.
    fn service(
        ship: &mut NpcShip,
        port: &mut Starport,
        requests: &[DockRequest],
        next_station: Option<usize>,
    ) -> usize {
        let id = traffic_id(ship.id);
        let mut chosen = 0;
        for request in requests {
            match *request {
                DockRequest::Request { .. } => {
                    if let Reply::Granted(bay) = port.request_docking(id, false, 0) {
                        ship.begin_approach(bay.index, None);
                    }
                }
                DockRequest::Touch { .. } => {
                    if port.touch(id) {
                        let seat = ship.pad.unwrap_or([0, 0, 1_000_000]);
                        ship.begin_docked(seat);
                    } else {
                        ship.touch_failed();
                    }
                }
                DockRequest::Settle { .. } => port.reset(),
                DockRequest::Launch { .. } => {
                    if port.request(id) {
                        ship.state = State::Launch;
                        ship.dock_wait = 0.0;
                    }
                }
                DockRequest::Release { .. } => port.release(id),
                DockRequest::ChooseStation { .. } => {
                    chosen += 1;
                    match next_station {
                        Some(index) => ship.begin_transit(index, [0; 3]),
                        None => {
                            ship.station = None;
                            ship.state = State::Transit;
                            ship.bay = None;
                            ship.pad = None;
                        }
                    }
                }
            }
        }
        chosen
    }

    /// Drive one ship's station cycle; `stop` ends the run at a state.
    fn run_cycle(
        ship: &mut NpcShip,
        port: &mut Starport,
        frames: usize,
        stop: impl Fn(State) -> bool,
        next_station: Option<usize>,
    ) -> (Vec<State>, usize) {
        let mut states = vec![ship.state];
        let mut chosen = 0;
        for _ in 0..frames {
            // The real frame time is the tick's; the test's 0.5 s frames give the
            // door the matching tics so it opens in the original 1.3 s.
            port.move_door(24_850);
            let station = ship
                .station
                .map(|_| test_station(port, traffic_id(ship.id)));
            let mut requests = Vec::new();
            ship.tick_with_station(0.5, 24_850, [0; 3], None, station.as_ref(), &mut requests);
            chosen += service(ship, port, &requests, next_station);
            if states.last() != Some(&ship.state) {
                states.push(ship.state);
            }
            if stop(ship.state) {
                break;
            }
        }
        (states, chosen)
    }

    #[test]
    fn a_trader_requests_a_bay_approaches_and_docks() {
        let mut port = Starport::new(3);
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Trader,
            [0, 0, 4_900_000],
            [0; 3],
            &mut Random::new(3, 4),
        );
        ship.begin_transit(0, [0; 3]);
        // The source's speeds make a 4 km approach minutes long; the test keeps
        // the model's turn and bay arithmetic but accelerates the frame motion.
        ship.max_speed = 400_000.0;
        ship.accel = 2_000_000.0;
        let id = traffic_id(ship.id);
        assert!(
            id > 1,
            "an NPC uses the record's bytes above the player's 1"
        );
        let (states, _) = run_cycle(
            &mut ship,
            &mut port,
            4000,
            |state| state == State::Docked,
            Some(0),
        );
        assert_eq!(states, [State::Transit, State::Approach, State::Docked]);
        // F650's bay is occupied from the request until the departure's F651.
        assert!(port.pads.contains(&id));
        assert_eq!(port.bay_of(id), Some(3));
        // F652 set mode 3 (the door closing). F747 runs F653 once it is over;
        // the bay itself stays allocated.
        assert_eq!(port.mode, crate::traffic::DOCKING_DONE);
        run_cycle(&mut ship, &mut port, 16, |_| false, Some(0));
        assert_eq!((port.mode, port.current, port.timeout), (0, 0, 0));
        assert_eq!(port.bay_of(id), Some(3));
        assert!((ship.position[2] - 1_000_000).abs() < 1);
        assert!(!ship.visible(), "a docked ship is inside the station");
    }

    #[test]
    fn the_docked_interval_is_f747s_two_to_three_days() {
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Trader,
            [0; 3],
            [0; 3],
            &mut Random::new(5, 6),
        );
        ship.begin_docked([0, 0, 1_000_000]);
        assert_eq!(ship.state, State::Docked);
        assert!((2.0 * 86_400.0..3.0 * 86_400.0).contains(&ship.dock_timer));
        // F733's object time: the ship asks F654 only after it has run out.
        let mut port = Starport::new(3);
        let id = traffic_id(ship.id);
        port.request_docking(id, false, 0);
        ship.dock_timer = 0.0;
        let station = test_station(&port, id);
        let mut requests = Vec::new();
        ship.tick_with_station(0.5, 24_850, [0; 3], None, Some(&station), &mut requests);
        assert_eq!(
            requests,
            [DockRequest::Launch {
                ship: ship.id,
                station: 0
            }]
        );
    }

    #[test]
    fn a_docked_ship_launches_when_the_door_opens_and_resumes_transit() {
        let mut port = Starport::new(3);
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Trader,
            [0, 0, 4_900_000],
            [0; 3],
            &mut Random::new(7, 8),
        );
        ship.begin_transit(0, [0; 3]);
        ship.max_speed = 400_000.0;
        ship.accel = 2_000_000.0;
        let id = traffic_id(ship.id);
        run_cycle(
            &mut ship,
            &mut port,
            4000,
            |state| state == State::Docked,
            Some(0),
        );
        assert!(port.pads.contains(&id));
        // Run the two-to-three day interval down; F654 opens the door and F735
        // waits for F865's word to reach 0 before F651 releases the bay.
        ship.dock_timer = 0.01;
        let (states, chosen) = run_cycle(
            &mut ship,
            &mut port,
            4000,
            |state| state == State::Transit,
            Some(0),
        );
        assert_eq!(
            states,
            [State::Docked, State::Launch, State::Depart, State::Transit]
        );
        assert_eq!(chosen, 1, "F761/F762 picked the next station");
        assert!(port.pads.iter().all(|pad| *pad == 0), "F651 freed the bay");
        assert_eq!(port.bay_of(id), None);
        assert!(ship.visible(), "the launched ship is outside the station");
        assert_eq!(ship.station, Some(0), "it resumed transit to a station");
    }

    #[test]
    fn a_full_station_makes_the_trader_try_another() {
        let mut port = Starport::new(1);
        // Two bays, both taken by other ships.
        port.pads[0] = 20;
        port.pads[1] = 21;
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Trader,
            [0, 0, 4_900_000],
            [0; 3],
            &mut Random::new(9, 10),
        );
        ship.begin_transit(0, [0; 3]);
        ship.max_speed = 400_000.0;
        ship.accel = 2_000_000.0;
        let (_, chosen) = run_cycle(&mut ship, &mut port, 400, |_| false, None);
        assert_eq!(chosen, 1, "F761/F762 was asked to pick another station");
        assert_eq!(ship.station, None, "the ship gave up on this station");
        // F655's Full branch resets the handling record but not the occupants.
        assert_eq!((port.mode, port.current, port.timeout), (0, 0, 0));
    }

    #[test]
    fn a_clearance_timeout_frees_the_bay_and_recycles_the_ship() {
        let mut port = Starport::new(3);
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Trader,
            [0, 0, 4_900_000],
            [0; 3],
            &mut Random::new(11, 12),
        );
        ship.begin_transit(0, [0; 3]);
        let id = traffic_id(ship.id);
        let reply = port.request_docking(id, false, 0);
        assert!(matches!(reply, Reply::Granted(_)));
        ship.begin_approach(3, None);
        // F867: the clearance runs out while the ship is still on its way.
        assert_eq!(port.elapse(u32::from(CLEARANCE_SECONDS)), Some(id));
        ship.touch_failed();
        assert_eq!(ship.state, State::Transit);
        assert_eq!(ship.bay, None);
        assert!(port.pads.iter().all(|pad| *pad == 0));
        // The ship asks again after the retry delay.
        assert!(ship.dock_retry > 0.0);
    }

    #[test]
    fn f635s_fixed_fraction_indexes_the_kind_0x10_target_table() {
        // F460/F506 pass the offer's seed word; F635 multiplies it by the
        // list's count and shifts right 16.
        assert_eq!(model_for_fraction(0x10, 0, 0), SHIP_TYPES[0x10][0]);
        assert_eq!(
            model_for_fraction(0x10, 0, 0xffff),
            *SHIP_TYPES[0x10].last().unwrap()
        );
        assert_eq!(model_for_fraction(0x10, 0, 0x8000), SHIP_TYPES[0x10][10]);
        // Kinds 0 and 1 take the allegiance shift; 0x10 does not.
        assert_eq!(model_for_fraction(1, 2, 0), SHIP_TYPES[5][0]);
    }

    #[test]
    fn f754_launches_a_fitted_pylon_in_the_five_to_eight_distance_band() {
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Pirate,
            [0, 0, 0],
            [0; 3],
            &mut Random::new(7, 9),
        );
        ship.pylons = [0x82; crate::missile::PYLONS];
        ship.general = 0;
        // 1,000 km: F610 exponent 5..8.
        let player = [0, 0, 1_000_000];
        let mut launch = None;
        for _ in 0..400 {
            tick(&mut ship, 0.02, player);
            if let Some(found) = ship.take_launch() {
                launch = Some(found);
                break;
            }
        }
        let launch = launch.expect("F754 should roll a launch");
        assert_eq!(launch.owner, ship.id);
        assert_eq!(launch.target, crate::missile::PLAYER_TARGET);
        assert_eq!(launch.code, 0x82);
        assert!((1..=8).contains(&launch.pylon));
        assert_eq!(ship.pylons[launch.pylon - 1], 0, "F941 clears the pylon");
        assert_eq!(ship.pylons.iter().filter(|code| **code == 0x82).count(), 9);
        assert_eq!(launch.launcher_radius, ship.radius);
    }

    #[test]
    fn f754_does_not_launch_outside_the_band_or_with_empty_pylons() {
        for player in [[0, 0, 10_000], [0, 0, 10_000_000]] {
            let mut ship = NpcShip::new(
                &spec(0x1c).unwrap(),
                Role::Pirate,
                [0, 0, 0],
                [0; 3],
                &mut Random::new(11, 12),
            );
            ship.pylons = [0x82; crate::missile::PYLONS];
            for _ in 0..400 {
                tick(&mut ship, 0.02, player);
            }
            assert!(ship.take_launch().is_none(), "range {player:?}");
            assert_eq!(
                ship.pylons.iter().filter(|code| **code == 0x82).count(),
                crate::missile::PYLONS
            );
        }
        let mut empty = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Pirate,
            [0, 0, 0],
            [0; 3],
            &mut Random::new(13, 14),
        );
        for _ in 0..400 {
            tick(&mut empty, 0.02, [0, 0, 1_000_000]);
        }
        assert!(empty.take_launch().is_none());
    }

    #[test]
    fn f773_fits_pylons_from_the_mount_word_and_the_cargo_budget() {
        let mut spec = spec(0x1c).unwrap();
        spec.missiles = 4;
        spec.cargo = 2;
        let mut fitted_any = false;
        for seed in 0..64u32 {
            let (general, pylons) = fit_equipment(
                &spec,
                Role::Pirate,
                Some(laser::PULSE_1MW),
                &mut Random::new(
                    0x1234_5678u32.wrapping_add(seed.wrapping_mul(0x9e37_79b9)),
                    0x4567_89ab ^ seed.rotate_left(7),
                ),
            );
            let fitted: Vec<u8> = pylons.iter().copied().filter(|code| *code != 0).collect();
            assert!(fitted.len() <= 2, "{fitted:?}");
            assert!(fitted.iter().all(|code| MISSILE_CODES.contains(code)));
            if fitted.is_empty() {
                continue;
            }
            fitted_any = true;
            assert_eq!(
                fitted.len(),
                fitted.iter().filter(|code| **code == fitted[0]).count()
            );
            assert_eq!(
                general & (1 << 30),
                1 << 30,
                "F773 always fits atmospheric shielding"
            );
            // The E.C.M. gate needs two tonnes and only sets one of its two bits.
            let ecm = general & (crate::equipment::ECM_BIT | crate::equipment::NAVAL_ECM_BIT);
            assert!(matches!(
                ecm,
                0 | crate::equipment::ECM_BIT | crate::equipment::NAVAL_ECM_BIT
            ));
        }
        assert!(fitted_any, "the fit should fill a pylon at some seed");
        // No cargo (after the laser) means no pylons at all.
        let mut empty = spec;
        empty.cargo = 1;
        let (_, pylons) = fit_equipment(
            &empty,
            Role::Pirate,
            Some(laser::PULSE_1MW),
            &mut Random::new(23, 24),
        );
        assert!(pylons.iter().all(|code| *code == 0));
    }

    #[test]
    fn f757_ecm_needs_a_fitted_system_and_f781_ends_the_evasion() {
        let mut ship = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Pirate,
            [0, 0, 0],
            [0; 3],
            &mut Random::new(31, 32),
        );
        ship.pylons = [0; crate::missile::PYLONS];
        ship.general = crate::equipment::ECM_BIT;
        ship.evade_missile(7);
        assert_eq!(ship.state, State::Evade);
        assert_eq!(ship.missile_target, Some(7));
        assert_eq!(ship.fitted_ecm(), Some(crate::equipment::Ecm::Standard));
        ship.general |= crate::equipment::NAVAL_ECM_BIT;
        assert_eq!(ship.fitted_ecm(), Some(crate::equipment::Ecm::Naval));
        ship.general = crate::equipment::ECM_BIT;
        let player = [0, 0, 1_000_000];
        let mut burst = None;
        for _ in 0..400 {
            tick(&mut ship, 0.02, player);
            if let Some(naval) = ship.take_ecm() {
                burst = Some(naval);
                break;
            }
        }
        assert_eq!(burst, Some(false), "the standard E.C.M. bit");
        ship.missile_gone(7);
        assert_eq!(ship.state, State::Attack);
        assert_eq!(ship.missile_target, None);

        // No E.C.M. bit: F935 is a no-op, so no burst is ever reported.
        let mut bare = NpcShip::new(
            &spec(0x1c).unwrap(),
            Role::Pirate,
            [0, 0, 0],
            [0; 3],
            &mut Random::new(33, 34),
        );
        bare.general = 0;
        bare.evade_missile(9);
        for _ in 0..400 {
            tick(&mut bare, 0.02, player);
        }
        assert!(bare.take_ecm().is_none());
        assert_eq!(bare.state, State::Evade, "only the missile's death ends it");
    }

    #[test]
    fn f922_is_bounded_and_deterministic() {
        let mut a = Random::new(0x1234_5678, 0x4567_89ab);
        let mut b = a;
        let first: Vec<u32> = (0..20).map(|_| a.bounded(1000)).collect();
        assert_eq!(first, (0..20).map(|_| b.bounded(1000)).collect::<Vec<_>>());
        assert!(first.iter().all(|value| *value < 1000));
        assert!(first.windows(2).any(|pair| pair[0] != pair[1]));
        assert_eq!(Random::new(0, 0).bounded(10), 0);
    }

    #[test]
    fn the_type_table_has_the_originals_lengths() {
        let counts: Vec<usize> = SHIP_TYPES.iter().map(|table| table.len()).collect();
        assert_eq!(
            counts,
            vec![9, 18, 10, 8, 12, 16, 23, 27, 11, 12, 15, 24, 10, 2, 4, 3, 20, 12, 1]
        );
        // F635's spot values: a Federal trader table and the police/shuttle lists.
        assert_eq!(SHIP_TYPES[1][0], 0x1c);
        assert_eq!(SHIP_TYPES[5][0], 0x1d);
        assert_eq!(SHIP_TYPES[0xd], [0x0f, 0x12]);
        assert_eq!(SHIP_TYPES[0xe], [0x1e, 0x1e, 0x22, 0x22]);
    }

    #[test]
    fn the_allegiance_shifts_traders_to_the_imperial_shipyard() {
        // DATA_008996 is 2 for Ross 154 (allegiance 134 >> 6); type 1 becomes 5.
        let mut rng = Random::new(1, 2);
        let model = model_for(Role::Trader, 2, &mut rng);
        assert!(SHIP_TYPES[5].contains(&model));
        let mut rng = Random::new(1, 2);
        assert!(SHIP_TYPES[1].contains(&model_for(Role::Trader, 0, &mut rng)));
    }
}
