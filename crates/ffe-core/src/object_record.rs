//! The original's 0x152-byte physics/agent object record (`PhysObj`), as the
//! layout is documented in `jjffe-master/srctools/plstate.txt` and written by
//! `ffe.asm`:
//!
//! - `+0x000..+0x023` relative orientation (three Q31 rows: right, up, forward),
//! - `+0x026`/`+0x03e` global and relative position (three 64-bit words each),
//! - `+0x056`/`+0x057`/`+0x058` parent, rotating frame and orientation-valid,
//! - `+0x07e` exponential scale, `+0x082` model, `+0x086` own slot,
//!   `+0x087` control byte, `+0x088`/`+0x08a`/`+0x08b` view distances and
//!   update cadence,
//! - `+0x0c8` equipment, `+0x0d0` drive, `+0x0d2..+0x0df` lasers and pylons,
//! - `+0x0e0..+0x0f0` shield/hull/laser/velocity words,
//! - `+0x0f2..+0x0fd` the motion caps / hyperspace union, and
//! - `+0x0fe..+0x151` the [`AgentRecord`] block (AI state, target, aim,
//!   bounty, fuel, name, radii, timeouts and laser flags).
//!
//! `objects::PhysList` stores the raw bytes in exactly this layout, so
//! [`ObjectRecord::from_bytes`]/[`ObjectRecord::write`] move a slot between the
//! byte list and a typed view without changing the data. [`ShipSlots`] is the
//! live ship list's F927 allocator and [`ObjectIndexes`] carries the `+0x86`,
//! `+0xfe` and `+0x100` object indexes; `npc::NpcShip::object_record_in` builds a
//! record from the fields the port stores.
//!
//! `DATA_008561` is not part of this layout: it is the 0x96-entry module table
//! (0x24 bytes each) used by the event dispatcher, not an object array.

use crate::objects::{self, OBJECT_BYTES};

/// Length of the object name field (`+0x124..+0x137`).
pub const NAME_BYTES: usize = 0x14;

/// Every field offset `plstate.txt` documents, kept as names so tests and the
/// byte list agree on one table.
pub mod offsets {
    pub const ORIENTATION: usize = 0x000;
    pub const DRIFT: usize = 0x024;
    pub const POSITION_VALID: usize = 0x025;
    pub const GLOBAL_POSITION: usize = 0x026;
    pub const POSITION: usize = 0x03e;
    pub const PARENT: usize = 0x056;
    pub const ROTATING: usize = 0x057;
    pub const ORIENTATION_VALID: usize = 0x058;
    pub const GLOBAL_ORIENTATION: usize = 0x05a;
    pub const SCALE: usize = 0x07e;
    pub const MODEL: usize = 0x082;
    pub const OBJECT: usize = 0x086;
    pub const CONTROL: usize = 0x087;
    pub const DISTANCE: usize = 0x088;
    pub const DISTANCE_OUTSIDE: usize = 0x08a;
    pub const UPDATE_CADENCE: usize = 0x08b;
    pub const VELOCITY: usize = 0x08c;
    pub const SCREEN_X: usize = 0x098;
    pub const SCREEN_Y: usize = 0x09a;
    pub const ANIMATION: usize = 0x09c;
    pub const TIMER: usize = 0x09e;
    pub const SEED: usize = 0x0a0;
    pub const MASS: usize = 0x0a4;
    pub const CLOCK_TICS: usize = 0x0a8;
    pub const CLOCK_DAYS: usize = 0x0ac;
    pub const RATES: usize = 0x0b0;
    pub const THRUST: usize = 0x0b6;
    pub const ACCELERATION: usize = 0x0bc;
    pub const EQUIPMENT: usize = 0x0c8;
    pub const DAMAGED: usize = 0x0cc;
    pub const DRIVE: usize = 0x0d0;
    pub const GUN_MOUNTS: usize = 0x0d1;
    pub const LASERS: usize = 0x0d2;
    pub const PYLONS: usize = 0x0d6;
    pub const SHIELDS_TOTAL: usize = 0x0e0;
    pub const SHIELDS: usize = 0x0e2;
    pub const HULL: usize = 0x0e4;
    pub const CLOUD_MODEL: usize = 0x0e6;
    pub const REFIRE: usize = 0x0e8;
    pub const HEAT: usize = 0x0ea;
    pub const FRACTION: usize = 0x0ec;
    /// The `+0xf2..+0xfd` union: motion caps, or the hyperspace agent words.
    pub const TAIL: usize = 0x0f2;
    pub const AGENT: usize = 0x0fe;
    pub const TARGET: usize = 0x0fe;
    pub const AI_MODE: usize = 0x0ff;
    pub const NAV_TARGET: usize = 0x100;
    pub const RETRO: usize = 0x101;
    pub const TARGET_OFFSET: usize = 0x102;
    pub const TARGET_DIRECTION: usize = 0x108;
    pub const TARGET_DISTANCE: usize = 0x10e;
    pub const AUTOPILOT: usize = 0x110;
    pub const CAPACITY: usize = 0x116;
    pub const KIND: usize = 0x118;
    pub const CARGO_FUEL: usize = 0x119;
    pub const BOUNTY: usize = 0x11a;
    pub const FUEL: usize = 0x11e;
    pub const NAME: usize = 0x124;
    pub const RADIUS: usize = 0x138;
    pub const COLLISION_RADIUS: usize = 0x140;
    pub const PHYSICS_TIMEOUT: usize = 0x148;
    pub const FLAGS: usize = 0x14c;
    pub const FOR_TIMEOUT: usize = 0x14d;
    pub const SMOKE_TIMEOUT: usize = 0x14e;
    pub const ECM_TIMEOUT: usize = 0x14f;
    pub const AVOID_TIMEOUT: usize = 0x150;
    pub const LASER_FLAGS: usize = 0x151;
}

/// `+0x087`: the object control byte (`plstate.txt`).
pub const CONTROL_ORBITAL: u8 = 0x1;
pub const CONTROL_PLAYER: u8 = 0x3;
pub const CONTROL_AI_SHIP: u8 = 0xb;

/// `+0x118`: the ship-table kind F773 writes (`plstate.txt` and F697/F773).
pub const KIND_TRADER: u8 = 0x1;
pub const KIND_AUX_TRADER: u8 = 0x0f;
pub const KIND_THARGOID: u8 = 0x11;
pub const KIND_CARGO: u8 = 0xfa;
pub const KIND_PIRATE: u8 = 0xfb;
pub const KIND_MISSILE: u8 = 0xfd;
pub const KIND_HYPERSPACE_CLOUD: u8 = 0xfe;
pub const KIND_PLAYER: u8 = 0xff;

/// `+0x14c` flags (`plstate.txt`).
pub const FLAG_LIGHT: u8 = 0x07;
pub const FLAG_LIGHT_SET: u8 = 0x08;
pub const FLAG_DOCKABLE: u8 = 0x10;
pub const FLAG_STARPORT: u8 = 0x20;
pub const FLAG_OPEN_AIR: u8 = 0x40;
pub const FLAG_COMPLEX: u8 = 0x80;

/// `+0x151` laser/equipment flags (`plstate.txt`).
pub const LASER_SHIELDS_HIT: u8 = 0x01;
pub const LASER_FIRED: u8 = 0x02;
pub const LASER_FRONT: u8 = 0x04;
pub const LASER_REAR: u8 = 0x08;
pub const LASER_TURRET: u8 = 0x10;
pub const LASER_ECM: u8 = 0x20;
pub const LASER_ENERGY_BOMB: u8 = 0x40;

/// Slots in the object list (`0x00..=0x72`), F927's index space.
pub const SLOTS: usize = objects::OBJECTS;
/// The topmost slot F927 hands out.
pub const TOP_SLOT: u8 = 0x72;
/// F927 scans down to slot 1 but never accepts a free slot below `0x14`: those
/// hold the system's bodies (F880). A ship therefore gets `0x14..=0x72`.
pub const FIRST_SHIP_SLOT: u8 = 0x14;

/// The player object's key in [`ShipSlots`] (`DATA_008857`). The player is
/// allocated by F928/F1074 before any traffic, so it holds the topmost slot and
/// keeps it for the whole game; its own id is not in the ship list.
pub const PLAYER_OBJECT: u32 = u32::MAX;
/// Missiles (F941) share the same F927 slot space as the ships. Their ids are
/// small and start at zero, so the table keys them with this high bit set.
pub const MISSILE_OBJECT: u32 = 0x4000_0000;
/// F1054's chaff objects (`DATA_007253`) are F928 objects too (model 0xa4).
/// Their live ids already start at [`crate::missile::DECOY_BASE`], so the table
/// keys them with the id itself.
pub const DECOY_OBJECT: u32 = 0x8000_0000;

fn missile_key(id: u32) -> u32 {
    MISSILE_OBJECT | (id & 0x3fff_ffff)
}

fn is_player(key: u32) -> bool {
    key == PLAYER_OBJECT
}

fn is_missile(key: u32) -> bool {
    key != PLAYER_OBJECT && key & MISSILE_OBJECT != 0
}

fn is_decoy(key: u32) -> bool {
    key != PLAYER_OBJECT && !is_missile(key) && key >= DECOY_OBJECT
}

fn is_ship(key: u32) -> bool {
    !is_player(key) && !is_missile(key) && !is_decoy(key)
}

/// The live object list's slots, allocated as F927 does: the topmost free slot
/// from [`TOP_SLOT`] down, never below [`FIRST_SHIP_SLOT`]. The source keeps the
/// slot in the object's own `+0x86` byte; the port keys its stable ship ids,
/// the player object ([`PLAYER_OBJECT`]) and the live missiles to slots here so
/// [`ObjectIndexes`] can report the same indexes and the record layer
/// round-trips them.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ShipSlots {
    /// Indexed by slot; `None` is free. The key is a ship id, [`PLAYER_OBJECT`]
    /// or [`missile_key`]'s encoding of an F941 missile id.
    slots: Vec<Option<u32>>,
}

impl ShipSlots {
    pub fn new() -> Self {
        Self {
            slots: vec![None; SLOTS],
        }
    }

    fn sized(&mut self) -> &mut [Option<u32>] {
        if self.slots.len() < SLOTS {
            self.slots.resize(SLOTS, None);
        }
        &mut self.slots[..SLOTS]
    }

    /// F927: the topmost free slot from `0x72` down, or `None` when every slot
    /// from `0x14` up is taken. Allocating a key that already owns a slot gives
    /// that slot back.
    pub fn allocate(&mut self, key: u32) -> Option<u8> {
        if let Some(slot) = self.slot_of(key) {
            return Some(slot);
        }
        let slot = (FIRST_SHIP_SLOT..=TOP_SLOT)
            .rev()
            .find(|slot| self.key(*slot).is_none())?;
        self.set(slot, Some(key));
        Some(slot)
    }

    /// F927's free: the slot becomes available to the next [`Self::allocate`].
    pub fn free(&mut self, key: u32) -> Option<u8> {
        let slot = self.slot_of(key)?;
        self.set(slot, None);
        Some(slot)
    }

    /// The slot a ship id owns, if it is in the list.
    pub fn slot_of(&self, id: u32) -> Option<u8> {
        self.slot_of_key(id)
    }

    fn slot_of_key(&self, key: u32) -> Option<u8> {
        let len = self.slots.len().min(SLOTS);
        (0..len)
            .find(|slot| self.slots[*slot] == Some(key))
            .map(|slot| slot as u8)
    }

    /// The id in a ship slot, if it is used. The player object and the missiles
    /// have their own accessors.
    pub fn owner(&self, slot: u8) -> Option<u32> {
        self.key(slot).filter(|key| is_ship(*key))
    }

    /// The raw key in a slot: a ship id, [`PLAYER_OBJECT`] or a missile key.
    pub fn key(&self, slot: u8) -> Option<u32> {
        self.slots.get(usize::from(slot)).copied().flatten()
    }

    /// F1074/F928: reserve the player object's slot (`DATA_008857`). At new
    /// game it is the topmost free slot before any traffic exists.
    pub fn reserve_player(&mut self) -> Option<u8> {
        self.allocate(PLAYER_OBJECT)
    }

    /// The player object's slot, when it has been reserved.
    pub fn player_slot(&self) -> Option<u8> {
        self.slot_of_key(PLAYER_OBJECT)
    }

    /// Give an F941 missile its object slot.
    pub fn allocate_missile(&mut self, id: u32) -> Option<u8> {
        self.allocate(missile_key(id))
    }

    /// The slot of a live F941 missile, when it has one.
    pub fn slot_of_missile(&self, id: u32) -> Option<u8> {
        self.slot_of_key(missile_key(id))
    }

    /// Give F1054's chaff object its slot. Decoy ids are already in the table's
    /// decoy range ([`DECOY_OBJECT`]).
    pub fn allocate_decoy(&mut self, id: u32) -> Option<u8> {
        self.allocate(id)
    }

    /// The slot of a live chaff object, when it has one.
    pub fn slot_of_decoy(&self, id: u32) -> Option<u8> {
        self.slot_of_key(id)
    }

    fn set(&mut self, slot: u8, key: Option<u32>) {
        let index = usize::from(slot);
        if index >= SLOTS {
            return;
        }
        self.sized()[index] = key;
    }

    /// The slots in use, lowest first.
    pub fn iter(&self) -> impl Iterator<Item = (u8, u32)> + '_ {
        self.slots
            .iter()
            .enumerate()
            .take(SLOTS)
            .filter_map(|(slot, key)| Some((slot as u8, (*key)?)))
    }

    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// A fresh system's allocation (F674/F909): every object except the
    /// player's, which survives the jump, becomes free.
    pub fn clear(&mut self) {
        let player = self.player_slot();
        self.slots.clear();
        if let Some(slot) = player {
            self.set(slot, Some(PLAYER_OBJECT));
        }
    }

    /// Bring the table in line with the live ship list: free the slots of the
    /// ids that left it, then give every new id the next free slot in list
    /// order (the source's creation order). The player object and the missiles
    /// keep their slots; use [`Self::sync_missiles`] for the missile field.
    pub fn sync(&mut self, ids: impl IntoIterator<Item = u32>) {
        let ids: Vec<u32> = ids.into_iter().collect();
        let gone: Vec<u32> = self
            .iter()
            .map(|(_, key)| key)
            .filter(|key| is_ship(*key))
            .filter(|id| !ids.contains(id))
            .collect();
        for id in gone {
            self.free(id);
        }
        for id in ids {
            self.allocate(id);
        }
    }

    /// Bring the missile slots in line with F941's live list. Ships and the
    /// player keep their slots.
    pub fn sync_missiles(&mut self, ids: impl IntoIterator<Item = u32>) {
        let ids: Vec<u32> = ids.into_iter().map(missile_key).collect();
        let gone: Vec<u32> = self
            .iter()
            .map(|(_, key)| key)
            .filter(|key| is_missile(*key))
            .filter(|key| !ids.contains(key))
            .collect();
        for key in gone {
            self.free(key);
        }
        for key in ids {
            self.allocate(key);
        }
    }

    /// Bring the chaff objects' slots in line with F1054's live list. Ships,
    /// the player and the missiles keep their slots.
    pub fn sync_decoys(&mut self, ids: impl IntoIterator<Item = u32>) {
        let ids: Vec<u32> = ids.into_iter().collect();
        let gone: Vec<u32> = self
            .iter()
            .map(|(_, key)| key)
            .filter(|key| is_decoy(*key))
            .filter(|key| !ids.contains(key))
            .collect();
        for key in gone {
            self.free(key);
        }
        for key in ids {
            self.allocate(key);
        }
    }

    /// [`Self::sync`] plus storing each slot in the ship's own `+0x86`.
    pub fn assign(&mut self, ships: &mut [crate::npc::NpcShip]) {
        self.sync(ships.iter().map(|ship| ship.id));
        for ship in ships {
            ship.slot = self.slot_of(ship.id).unwrap_or(0);
        }
    }

    /// Rebuild from saved `(id, +0x86)` pairs: a saved slot is kept when it is
    /// inside the list and free, so a resumed run reports the same indexes.
    pub fn from_saved(pairs: impl IntoIterator<Item = (u32, u8)>) -> Self {
        let mut slots = ShipSlots::new();
        for (id, slot) in pairs {
            if slot >= FIRST_SHIP_SLOT && slots.key(slot).is_none() {
                slots.set(slot, Some(id));
            }
        }
        slots
    }

    /// [`Self::from_saved`] with the player object's slot reserved first, as
    /// F1074 allocates it before the system's traffic. A saved ship that used
    /// to hold the player's slot is left for [`Self::sync`] to reallocate.
    pub fn from_saved_with_player(pairs: impl IntoIterator<Item = (u32, u8)>) -> Self {
        let mut slots = ShipSlots::new();
        slots.reserve_player();
        for (id, slot) in pairs {
            if slot >= FIRST_SHIP_SLOT && slots.key(slot).is_none() {
                slots.set(slot, Some(id));
            }
        }
        slots
    }
}

/// The live object indexes a ship record needs beyond its own slot: `+0x86`
/// comes from [`ShipSlots`], `+0xfe`/`+0x100` from the target's and the
/// destination body's object indexes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectIndexes<'a> {
    /// The live list's slots. `None` keeps the ship's stored `slot`.
    pub slots: Option<&'a ShipSlots>,
    /// `DATA_008857`: the player object's index, an NPC's enemy.
    pub player: Option<u8>,
    /// The destination body's object slot (`+0x100`, and `+0xfe` while docked).
    pub station: Option<u8>,
}

/// The hyperspace-agent reading of the `+0xf2..+0xfd` union (an AI ship's
/// jump destination, range, post-jump mode and countdown).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct HyperspaceWords {
    /// `+0xf2`: destination system code.
    pub destination: u32,
    /// `+0xf6`: maximum jump range.
    pub max_range: u16,
    /// `+0xf8`: AI mode to restore after the jump.
    pub post_mode: u8,
    /// `+0xfa`: jump time, in tics shifted right 16.
    pub jump_time: u32,
}

/// The agent half of a `PhysObj`, `+0xfe..+0x151`.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentRecord {
    /// `+0xfe`: the starport/docked object index, or the AI's enemy index.
    pub target: u8,
    /// `+0xff`: the AI mode (see `npc::State`).
    pub ai_mode: u8,
    /// `+0x100`: the navigation target (or a pirate group's enemy index).
    pub nav_target: u8,
    /// `+0x101`: 0-2 depending on retro power.
    pub retro: u8,
    /// `+0x102..+0x106`: target x/y/z offsets.
    pub target_offset: [i16; 3],
    /// `+0x108..+0x10c`: target x/y/z direction words.
    pub target_direction: [i16; 3],
    /// `+0x10e`: exponential distance to the target.
    pub target_distance: u16,
    /// `+0x110..+0x114`: autopilot pitch/roll/yaw.
    pub autopilot: [i16; 3],
    /// `+0x116`: remaining internal capacity (AI only).
    pub capacity: u16,
    /// `+0x118`: ship-table kind, or the special object kinds above.
    pub kind: u8,
    /// `+0x119`: cargo fuel in tonnes (AI only).
    pub cargo_fuel: u8,
    /// `+0x11a`: the bounty F942 pays.
    pub bounty: u32,
    /// `+0x11e`: fuel tank; `0x2000_0000` is one tonne.
    pub fuel: u32,
    /// `+0x124`: the object name/ID (NUL-padded).
    pub name: [u8; NAME_BYTES],
    /// `+0x138`: interaction radius (64-bit).
    pub radius: i64,
    /// `+0x140`: primary collision radius (64-bit).
    pub collision_radius: i64,
    /// `+0x148`: physics timeout.
    pub physics_timeout: u32,
    /// `+0x14c`: class flags (light, dockable, starport, open air, complex).
    pub flags: u8,
    /// `+0x14d`: frame-of-reference update timeout.
    pub for_timeout: u8,
    /// `+0x14e`: smoke timeout.
    pub smoke_timeout: u8,
    /// `+0x14f`: E.C.M./energy-bomb timeout.
    pub ecm_timeout: u8,
    /// `+0x150`: autopilot avoidance-check timeout.
    pub avoid_timeout: u8,
    /// `+0x151`: laser/equipment flags.
    pub laser_flags: u8,
}

impl AgentRecord {
    /// Read the agent block from a full 0x152-byte object record.
    pub fn from_object_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..OBJECT_BYTES)?;
        let name = b[offsets::NAME..offsets::NAME + NAME_BYTES]
            .try_into()
            .unwrap();
        Some(AgentRecord {
            target: b[offsets::TARGET],
            ai_mode: b[offsets::AI_MODE],
            nav_target: b[offsets::NAV_TARGET],
            retro: b[offsets::RETRO],
            target_offset: [
                i16_at(b, offsets::TARGET_OFFSET),
                i16_at(b, offsets::TARGET_OFFSET + 2),
                i16_at(b, offsets::TARGET_OFFSET + 4),
            ],
            target_direction: [
                i16_at(b, offsets::TARGET_DIRECTION),
                i16_at(b, offsets::TARGET_DIRECTION + 2),
                i16_at(b, offsets::TARGET_DIRECTION + 4),
            ],
            target_distance: u16_at(b, offsets::TARGET_DISTANCE),
            autopilot: [
                i16_at(b, offsets::AUTOPILOT),
                i16_at(b, offsets::AUTOPILOT + 2),
                i16_at(b, offsets::AUTOPILOT + 4),
            ],
            capacity: u16_at(b, offsets::CAPACITY),
            kind: b[offsets::KIND],
            cargo_fuel: b[offsets::CARGO_FUEL],
            bounty: u32_at(b, offsets::BOUNTY),
            fuel: u32_at(b, offsets::FUEL),
            name,
            radius: i64_at(b, offsets::RADIUS),
            collision_radius: i64_at(b, offsets::COLLISION_RADIUS),
            physics_timeout: u32_at(b, offsets::PHYSICS_TIMEOUT),
            flags: b[offsets::FLAGS],
            for_timeout: b[offsets::FOR_TIMEOUT],
            smoke_timeout: b[offsets::SMOKE_TIMEOUT],
            ecm_timeout: b[offsets::ECM_TIMEOUT],
            avoid_timeout: b[offsets::AVOID_TIMEOUT],
            laser_flags: b[offsets::LASER_FLAGS],
        })
    }

    /// Write the agent block into a full 0x152-byte object record. The two
    /// padding bytes (`+0x0f9`, and anything not in the table) are untouched.
    pub fn write_to_object_bytes(&self, bytes: &mut [u8]) {
        assert!(bytes.len() >= OBJECT_BYTES, "object record is 0x152 bytes");
        bytes[offsets::TARGET] = self.target;
        bytes[offsets::AI_MODE] = self.ai_mode;
        bytes[offsets::NAV_TARGET] = self.nav_target;
        bytes[offsets::RETRO] = self.retro;
        put_i16(bytes, offsets::TARGET_OFFSET, self.target_offset[0]);
        put_i16(bytes, offsets::TARGET_OFFSET + 2, self.target_offset[1]);
        put_i16(bytes, offsets::TARGET_OFFSET + 4, self.target_offset[2]);
        put_i16(bytes, offsets::TARGET_DIRECTION, self.target_direction[0]);
        put_i16(
            bytes,
            offsets::TARGET_DIRECTION + 2,
            self.target_direction[1],
        );
        put_i16(
            bytes,
            offsets::TARGET_DIRECTION + 4,
            self.target_direction[2],
        );
        put_u16(bytes, offsets::TARGET_DISTANCE, self.target_distance);
        put_i16(bytes, offsets::AUTOPILOT, self.autopilot[0]);
        put_i16(bytes, offsets::AUTOPILOT + 2, self.autopilot[1]);
        put_i16(bytes, offsets::AUTOPILOT + 4, self.autopilot[2]);
        put_u16(bytes, offsets::CAPACITY, self.capacity);
        bytes[offsets::KIND] = self.kind;
        bytes[offsets::CARGO_FUEL] = self.cargo_fuel;
        put_u32(bytes, offsets::BOUNTY, self.bounty);
        put_u32(bytes, offsets::FUEL, self.fuel);
        bytes[offsets::NAME..offsets::NAME + NAME_BYTES].copy_from_slice(&self.name);
        put_i64(bytes, offsets::RADIUS, self.radius);
        put_i64(bytes, offsets::COLLISION_RADIUS, self.collision_radius);
        put_u32(bytes, offsets::PHYSICS_TIMEOUT, self.physics_timeout);
        bytes[offsets::FLAGS] = self.flags;
        bytes[offsets::FOR_TIMEOUT] = self.for_timeout;
        bytes[offsets::SMOKE_TIMEOUT] = self.smoke_timeout;
        bytes[offsets::ECM_TIMEOUT] = self.ecm_timeout;
        bytes[offsets::AVOID_TIMEOUT] = self.avoid_timeout;
        bytes[offsets::LASER_FLAGS] = self.laser_flags;
    }

    /// `+0x124` up to the first NUL, as bytes (the original writes arbitrary
    /// bytes there, not necessarily UTF-8).
    pub fn name_bytes(&self) -> &[u8] {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name.len());
        &self.name[..end]
    }

    /// The name as a lossy string, for messages and tests.
    pub fn name_string(&self) -> String {
        String::from_utf8_lossy(self.name_bytes()).into_owned()
    }

    /// Copy a name into the fixed field, leaving a terminator when it fits.
    pub fn set_name(&mut self, name: &str) {
        self.name = [0; NAME_BYTES];
        let bytes = name.as_bytes();
        let count = bytes.len().min(NAME_BYTES - 1);
        self.name[..count].copy_from_slice(&bytes[..count]);
    }

    /// `+0x14c & 0x10`: the object can be docked at (station or starport).
    pub fn is_dockable(&self) -> bool {
        self.flags & FLAG_DOCKABLE != 0
    }

    /// `+0x14c & 0x20`: a starport rather than an orbital station.
    pub fn is_starport(&self) -> bool {
        self.flags & FLAG_STARPORT != 0
    }

    /// `+0x14c & 0x40`: open-air (a starport without a bay door).
    pub fn is_open_air(&self) -> bool {
        self.flags & FLAG_OPEN_AIR != 0
    }

    /// `+0x14c & 0x80`: a complex (terrain) collision shape.
    pub fn is_complex(&self) -> bool {
        self.flags & FLAG_COMPLEX != 0
    }

    /// `+0x14c & 7`: the light index.
    pub fn light_index(&self) -> u8 {
        self.flags & FLAG_LIGHT
    }
}

/// A typed view of the full 0x152-byte `PhysObj` record. The `tail` field is
/// the `+0xf2..+0xfd` union; use [`Self::caps`]/[`Self::hyperspace`] for its two
/// documented readings.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ObjectRecord {
    /// `+0x00`: relative orientation rows (right, up, forward), Q31 dwords.
    pub orientation: [[i32; 3]; 3],
    /// `+0x24`: drift counter (signed byte).
    pub drift: i8,
    /// `+0x25`: global position valid flag.
    pub position_valid: u8,
    /// `+0x26`: global position, three 64-bit words.
    pub global_position: [i64; 3],
    /// `+0x3e`: relative position, three 64-bit words.
    pub position: [i64; 3],
    /// `+0x56`: parent object index.
    pub parent: u8,
    /// `+0x57`: rotational frame-of-reference flag.
    pub rotating: u8,
    /// `+0x58`: global orientation valid flag.
    pub orientation_valid: u8,
    /// `+0x5a`: global orientation rows.
    pub global_orientation: [[i32; 3]; 3],
    /// `+0x7e`: exponential scale value.
    pub scale: u32,
    /// `+0x82`: model index.
    pub model: u32,
    /// `+0x86`: the object's own slot index.
    pub object: u8,
    /// `+0x87`: control byte ([`CONTROL_ORBITAL`], [`CONTROL_PLAYER`],
    /// [`CONTROL_AI_SHIP`]).
    pub control: u8,
    /// `+0x88`: exponential distance from the camera.
    pub distance: u8,
    /// `+0x8a`: exponential distance outside the view.
    pub distance_outside: u8,
    /// `+0x8b`: frames between updates.
    pub update_cadence: u8,
    /// `+0x8c`: velocity, three 32-bit words.
    pub velocity: [i32; 3],
    /// `+0x98`: screen x position (render-only).
    pub screen_x: u16,
    /// `+0x9a`: screen y position (render-only).
    pub screen_y: u16,
    /// `+0x9c`: animation parameter (`time >> 10`, F1536).
    pub animation: u16,
    /// `+0x9e`: the F1536/F1099 timer word.
    pub timer: u16,
    /// `+0xa0`: random seed.
    pub seed: u32,
    /// `+0xa4`: mass (planets only), an ffp word.
    pub mass: u32,
    /// `+0xa8`: creation time, tics.
    pub clock_tics: u32,
    /// `+0xac`: creation time, days.
    pub clock_days: u32,
    /// `+0xb0`: pitch, roll, yaw rates.
    pub rates: [i16; 3],
    /// `+0xb6`: thrust along right, up, forward.
    pub thrust: [i16; 3],
    /// `+0xbc`: main, retro, top, bottom, right, left acceleration words.
    pub acceleration: [i16; 6],
    /// `+0xc8`: general equipment flags.
    pub equipment: u32,
    /// `+0xcc`: damaged equipment flags.
    pub damaged: u32,
    /// `+0xd0`: drive type.
    pub drive: u8,
    /// `+0xd1`: number of gun mounts.
    pub gun_mounts: u8,
    /// `+0xd2`: front, rear, top and bottom laser codes.
    pub lasers: [u8; 4],
    /// `+0xd6`: the ten missile pylon codes (`+0xd6..+0xdf`).
    pub pylons: [u8; 10],
    /// `+0xe0`: total shields.
    pub shields_total: u16,
    /// `+0xe2`: current shields.
    pub shields: u16,
    /// `+0xe4`: hull (mass * 4 for ships).
    pub hull: u16,
    /// `+0xe6`: the hyperspace cloud's parent model, or F931's laser use.
    pub cloud_model: u16,
    /// `+0xe8`: pulse-laser refire time.
    pub refire: u16,
    /// `+0xea`: laser temperature.
    pub heat: u16,
    /// `+0xec`: fractional velocity accumulators.
    pub fraction: [u16; 3],
    /// `+0xf2..+0xfd`: the motion-cap / hyperspace union.
    pub tail: [u8; 12],
    /// `+0xfe..+0x151`: the agent block.
    pub agent: AgentRecord,
}

impl ObjectRecord {
    /// Read a record from the first 0x152 bytes of `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..OBJECT_BYTES)?;
        Some(ObjectRecord {
            orientation: [
                i32_row3(b, offsets::ORIENTATION),
                i32_row3(b, offsets::ORIENTATION + 12),
                i32_row3(b, offsets::ORIENTATION + 24),
            ],
            drift: b[offsets::DRIFT] as i8,
            position_valid: b[offsets::POSITION_VALID],
            global_position: i64_row(b, offsets::GLOBAL_POSITION),
            position: i64_row(b, offsets::POSITION),
            parent: b[offsets::PARENT],
            rotating: b[offsets::ROTATING],
            orientation_valid: b[offsets::ORIENTATION_VALID],
            global_orientation: [
                i32_row3(b, offsets::GLOBAL_ORIENTATION),
                i32_row3(b, offsets::GLOBAL_ORIENTATION + 12),
                i32_row3(b, offsets::GLOBAL_ORIENTATION + 24),
            ],
            scale: u32_at(b, offsets::SCALE),
            model: u32_at(b, offsets::MODEL),
            object: b[offsets::OBJECT],
            control: b[offsets::CONTROL],
            distance: b[offsets::DISTANCE],
            distance_outside: b[offsets::DISTANCE_OUTSIDE],
            update_cadence: b[offsets::UPDATE_CADENCE],
            velocity: i32_row3(b, offsets::VELOCITY),
            screen_x: u16_at(b, offsets::SCREEN_X),
            screen_y: u16_at(b, offsets::SCREEN_Y),
            animation: u16_at(b, offsets::ANIMATION),
            timer: u16_at(b, offsets::TIMER),
            seed: u32_at(b, offsets::SEED),
            mass: u32_at(b, offsets::MASS),
            clock_tics: u32_at(b, offsets::CLOCK_TICS),
            clock_days: u32_at(b, offsets::CLOCK_DAYS),
            rates: i16_row(b, offsets::RATES),
            thrust: i16_row(b, offsets::THRUST),
            acceleration: [
                i16_at(b, offsets::ACCELERATION),
                i16_at(b, offsets::ACCELERATION + 2),
                i16_at(b, offsets::ACCELERATION + 4),
                i16_at(b, offsets::ACCELERATION + 6),
                i16_at(b, offsets::ACCELERATION + 8),
                i16_at(b, offsets::ACCELERATION + 10),
            ],
            equipment: u32_at(b, offsets::EQUIPMENT),
            damaged: u32_at(b, offsets::DAMAGED),
            drive: b[offsets::DRIVE],
            gun_mounts: b[offsets::GUN_MOUNTS],
            lasers: b[offsets::LASERS..offsets::LASERS + 4].try_into().unwrap(),
            pylons: b[offsets::PYLONS..offsets::PYLONS + 10].try_into().unwrap(),
            shields_total: u16_at(b, offsets::SHIELDS_TOTAL),
            shields: u16_at(b, offsets::SHIELDS),
            hull: u16_at(b, offsets::HULL),
            cloud_model: u16_at(b, offsets::CLOUD_MODEL),
            refire: u16_at(b, offsets::REFIRE),
            heat: u16_at(b, offsets::HEAT),
            fraction: [
                u16_at(b, offsets::FRACTION),
                u16_at(b, offsets::FRACTION + 2),
                u16_at(b, offsets::FRACTION + 4),
            ],
            tail: b[offsets::TAIL..offsets::AGENT].try_into().unwrap(),
            agent: AgentRecord::from_object_bytes(b)?,
        })
    }

    /// Write every field into a 0x152-byte buffer. Padding bytes the source
    /// never names (`+0x59`, `+0x89`) are left as they are; `+0xf9` is part of
    /// the `+0xf2..+0xfd` union and is written back from `tail`.
    pub fn write(&self, bytes: &mut [u8]) {
        assert!(bytes.len() >= OBJECT_BYTES, "object record is 0x152 bytes");
        self.write_orientation(bytes, offsets::ORIENTATION, &self.orientation);
        bytes[offsets::DRIFT] = self.drift as u8;
        bytes[offsets::POSITION_VALID] = self.position_valid;
        self.write_i64_row(bytes, offsets::GLOBAL_POSITION, &self.global_position);
        self.write_i64_row(bytes, offsets::POSITION, &self.position);
        bytes[offsets::PARENT] = self.parent;
        bytes[offsets::ROTATING] = self.rotating;
        bytes[offsets::ORIENTATION_VALID] = self.orientation_valid;
        self.write_orientation(bytes, offsets::GLOBAL_ORIENTATION, &self.global_orientation);
        put_u32(bytes, offsets::SCALE, self.scale);
        put_u32(bytes, offsets::MODEL, self.model);
        bytes[offsets::OBJECT] = self.object;
        bytes[offsets::CONTROL] = self.control;
        bytes[offsets::DISTANCE] = self.distance;
        bytes[offsets::DISTANCE_OUTSIDE] = self.distance_outside;
        bytes[offsets::UPDATE_CADENCE] = self.update_cadence;
        self.write_i32_row(bytes, offsets::VELOCITY, &self.velocity);
        put_u16(bytes, offsets::SCREEN_X, self.screen_x);
        put_u16(bytes, offsets::SCREEN_Y, self.screen_y);
        put_u16(bytes, offsets::ANIMATION, self.animation);
        put_u16(bytes, offsets::TIMER, self.timer);
        put_u32(bytes, offsets::SEED, self.seed);
        put_u32(bytes, offsets::MASS, self.mass);
        put_u32(bytes, offsets::CLOCK_TICS, self.clock_tics);
        put_u32(bytes, offsets::CLOCK_DAYS, self.clock_days);
        self.write_i16_row(bytes, offsets::RATES, &self.rates);
        self.write_i16_row(bytes, offsets::THRUST, &self.thrust);
        for (index, word) in self.acceleration.iter().enumerate() {
            put_i16(bytes, offsets::ACCELERATION + 2 * index, *word);
        }
        put_u32(bytes, offsets::EQUIPMENT, self.equipment);
        put_u32(bytes, offsets::DAMAGED, self.damaged);
        bytes[offsets::DRIVE] = self.drive;
        bytes[offsets::GUN_MOUNTS] = self.gun_mounts;
        bytes[offsets::LASERS..offsets::LASERS + 4].copy_from_slice(&self.lasers);
        bytes[offsets::PYLONS..offsets::PYLONS + 10].copy_from_slice(&self.pylons);
        put_u16(bytes, offsets::SHIELDS_TOTAL, self.shields_total);
        put_u16(bytes, offsets::SHIELDS, self.shields);
        put_u16(bytes, offsets::HULL, self.hull);
        put_u16(bytes, offsets::CLOUD_MODEL, self.cloud_model);
        put_u16(bytes, offsets::REFIRE, self.refire);
        put_u16(bytes, offsets::HEAT, self.heat);
        for (index, word) in self.fraction.iter().enumerate() {
            put_u16(bytes, offsets::FRACTION + 2 * index, *word);
        }
        bytes[offsets::TAIL..offsets::AGENT].copy_from_slice(&self.tail);
        self.agent.write_to_object_bytes(bytes);
    }

    /// The ordinary-object reading of `+0xf2..+0xfd`: three unaligned dwords
    /// F601 uses as velocity-change caps.
    pub fn caps(&self) -> [i32; 3] {
        [
            i32::from_le_bytes(self.tail[0..4].try_into().unwrap()),
            i32::from_le_bytes(self.tail[4..8].try_into().unwrap()),
            i32::from_le_bytes(self.tail[8..12].try_into().unwrap()),
        ]
    }

    /// Set the three velocity-change caps (the ordinary-object reading).
    pub fn set_caps(&mut self, caps: [i32; 3]) {
        self.tail[0..4].copy_from_slice(&caps[0].to_le_bytes());
        self.tail[4..8].copy_from_slice(&caps[1].to_le_bytes());
        self.tail[8..12].copy_from_slice(&caps[2].to_le_bytes());
    }

    /// The AI ship's reading of the same words (F738/F740).
    pub fn hyperspace(&self) -> HyperspaceWords {
        HyperspaceWords {
            destination: u32::from_le_bytes(self.tail[0..4].try_into().unwrap()),
            max_range: u16::from_le_bytes(self.tail[4..6].try_into().unwrap()),
            post_mode: self.tail[6],
            jump_time: u32::from_le_bytes(self.tail[8..12].try_into().unwrap()),
        }
    }

    /// Write the jump words into the `+0xf2..+0xfd` union.
    pub fn set_hyperspace(&mut self, words: HyperspaceWords) {
        self.tail[0..4].copy_from_slice(&words.destination.to_le_bytes());
        self.tail[4..6].copy_from_slice(&words.max_range.to_le_bytes());
        self.tail[6] = words.post_mode;
        self.tail[8..12].copy_from_slice(&words.jump_time.to_le_bytes());
    }

    /// `+0x00..+0x24`: the three Q31 rows (right, up, forward) as floats. The
    /// source's 0x7fffffff is 1.0, as FFE's matrices use it.
    pub fn axes(&self) -> [[f64; 3]; 3] {
        const SCALE: f64 = 2_147_483_647.0;
        std::array::from_fn(|row| self.orientation[row].map(|value| f64::from(value) / SCALE))
    }

    /// Store unit axes in `+0x00..+0x24` as Q31 rows.
    pub fn set_axes(&mut self, axes: [[f64; 3]; 3]) {
        const SCALE: f64 = 2_147_483_647.0;
        for (row, values) in axes.iter().enumerate() {
            for (column, value) in values.iter().enumerate() {
                self.orientation[row][column] = (value * SCALE).round() as i32;
            }
        }
    }

    /// The orbital reading of this record (`+0xb0..+0xe0`, see
    /// [`OrbitalOverlay`]). The ship fields cover the same bytes, so use this
    /// rather than [`Self::rates`]/[`Self::thrust`] for an orbital object.
    pub fn orbit(&self) -> OrbitalOverlay {
        let mut bytes = [0u8; OBJECT_BYTES];
        self.write(&mut bytes);
        OrbitalOverlay::from_object_bytes(&bytes).unwrap_or_default()
    }

    /// Replace the orbital half of the record (it wins over the ship fields it
    /// overlaps, as the source's union does).
    pub fn set_orbit(&mut self, overlay: OrbitalOverlay) {
        let mut bytes = [0u8; OBJECT_BYTES];
        self.write(&mut bytes);
        overlay.write_to_object_bytes(&mut bytes);
        if let Some(read) = ObjectRecord::from_bytes(&bytes) {
            *self = read;
        }
    }

    fn write_orientation(&self, bytes: &mut [u8], at: usize, rows: &[[i32; 3]; 3]) {
        for (row, values) in rows.iter().enumerate() {
            self.write_i32_row(bytes, at + 12 * row, values);
        }
    }

    fn write_i32_row(&self, bytes: &mut [u8], at: usize, values: &[i32; 3]) {
        for (index, value) in values.iter().enumerate() {
            put_u32(bytes, at + 4 * index, *value as u32);
        }
    }

    fn write_i16_row(&self, bytes: &mut [u8], at: usize, values: &[i16; 3]) {
        for (index, value) in values.iter().enumerate() {
            put_i16(bytes, at + 2 * index, *value);
        }
    }

    fn write_i64_row(&self, bytes: &mut [u8], at: usize, values: &[i64; 3]) {
        for (index, value) in values.iter().enumerate() {
            put_i64(bytes, at + 8 * index, *value);
        }
    }
}

/// The source-shaped record of an F941 missile object. F941 allocates it with
/// F928 (`+0x87 = 0xb`, `+0x118 = 0xfd`), sets `+0xfe` to the target object and
/// `+0x100` to the launcher's. The caller resolves `target` and `owner` from
/// the same [`ShipSlots`] table (the player is [`PLAYER_OBJECT`]); the missile's
/// `velocity` is the port's world-units value, not the source's quantised
/// `+0x8c` word.
pub fn missile_record(
    missile: &crate::missile::Missile,
    slot: u8,
    target: u8,
    owner: u8,
) -> ObjectRecord {
    let mut record = ObjectRecord {
        model: missile.spec.model,
        object: slot,
        control: CONTROL_AI_SHIP,
        position: missile.position,
        velocity: missile.velocity.map(|value| value.round() as i32),
        agent: AgentRecord {
            target,
            ai_mode: missile.ai_state,
            nav_target: owner,
            kind: KIND_MISSILE,
            radius: missile.spec.radius,
            collision_radius: missile.spec.radius,
            ..AgentRecord::default()
        },
        ..ObjectRecord::default()
    };
    // `+0x00`: the missile's axes, so the frame can draw straight from the
    // record (the port's F941 object).
    record.set_axes(missile.axes);
    record
}

/// The orbital reading of the same `+0xb0..+0xe0` bytes: an orbital object
/// reuses the ship record's rates/thrust/acceleration area for its orbital
/// radius, phase, angular velocity, spin, tilt and its starport docking state
/// (`plstate.txt`, "Orbital object struct differences"). Read it with
/// [`ObjectRecord::orbit`] rather than the ship fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct OrbitalOverlay {
    /// `+0xb0`: orbital radius (an ffp dword).
    pub radius: u32,
    /// `+0xba`: the 64-bit angular position.
    pub phase: u64,
    /// `+0xc2`: angular velocity.
    pub rate: u32,
    /// `+0xc6`: eccentricity (the port's orbit longitude).
    pub longitude: i16,
    /// `+0xc8`: rotation speed.
    pub spin: u8,
    /// `+0xc9`: the starport's docking mode (F649-F657).
    pub mode: u8,
    /// `+0xca`: the docking object F649 is handling.
    pub current: u8,
    /// `+0xcb`: the sysobject value (`0x3f`).
    pub sys: u8,
    /// `+0xcc`: orientation (tilt).
    pub tilt: i16,
    /// `+0xce`: the highest bay (a starport).
    pub bays: u16,
    /// `+0xd0..+0xd7`: one byte a bay.
    pub pads: [u8; 8],
    /// `+0xd8`: the clearance timeout.
    pub timeout: u16,
    /// `+0xda`: the frame-of-reference weighting.
    pub for_weight: u16,
    /// `+0xdc`: the starport's remaining police (`bays * 8 + 8` at creation).
    pub response: i16,
    /// `+0xde`: the police clock F240 steps.
    pub heat: u16,
    /// `+0xe0`: the temperature word (below `0x116` is wind).
    pub temperature: u16,
}

impl OrbitalOverlay {
    /// The motion half of `orbit` (radius, phase, rate, longitude, spin, tilt).
    pub fn from_orbiter(orbit: &objects::Orbiter) -> Self {
        OrbitalOverlay {
            radius: orbit.radius,
            phase: orbit.phase,
            rate: orbit.rate,
            longitude: orbit.longitude,
            spin: orbit.spin as u8,
            tilt: orbit.tilt,
            ..OrbitalOverlay::default()
        }
    }

    /// Write the motion half back into a port [`objects::Orbiter`].
    pub fn apply_orbit(&self, orbit: &mut objects::Orbiter) {
        orbit.radius = self.radius;
        orbit.phase = self.phase;
        orbit.rate = self.rate;
        orbit.longitude = self.longitude;
        orbit.spin = self.spin as i8;
        orbit.tilt = self.tilt;
    }

    /// The starport half as the port's [`crate::traffic::Starport`]. The door
    /// word is at `+0x9e`, outside this block, so it is left at its default;
    /// [`starport_from_bytes`] reads it from the whole record.
    pub fn starport(&self) -> crate::traffic::Starport {
        crate::traffic::Starport {
            mode: self.mode,
            current: self.current,
            timeout: self.timeout,
            bays: self.bays,
            pads: self.pads,
            door: 0,
            response: Some(self.response),
            heat: self.heat,
        }
    }

    /// Store a station's traffic record in the overlay bytes.
    pub fn set_starport(&mut self, record: &crate::traffic::Starport) {
        self.mode = record.mode;
        self.current = record.current;
        self.timeout = record.timeout;
        self.bays = record.bays;
        self.pads = record.pads;
        self.response = record.response.unwrap_or_else(|| {
            (i16::try_from(record.bays).unwrap_or(0))
                .wrapping_mul(8)
                .wrapping_add(8)
        });
        self.heat = record.heat;
    }

    /// Read the overlay from a full object record.
    pub fn from_object_bytes(bytes: &[u8]) -> Option<Self> {
        let b = bytes.get(..OBJECT_BYTES)?;
        Some(OrbitalOverlay {
            radius: u32_at(b, offsets::RATES),
            phase: u64::from_le_bytes(b[0xba..0xc2].try_into().unwrap()),
            rate: u32_at(b, 0xc2),
            longitude: i16_at(b, 0xc6),
            spin: b[0xc8],
            mode: b[0xc9],
            current: b[0xca],
            sys: b[0xcb],
            tilt: i16_at(b, 0xcc),
            bays: u16_at(b, 0xce),
            pads: b[0xd0..0xd8].try_into().unwrap(),
            timeout: u16_at(b, 0xd8),
            for_weight: u16_at(b, 0xda),
            response: i16_at(b, 0xdc),
            heat: u16_at(b, 0xde),
            temperature: u16_at(b, 0xe0),
        })
    }

    /// Write the overlay into a full object record.
    pub fn write_to_object_bytes(&self, bytes: &mut [u8]) {
        assert!(bytes.len() >= OBJECT_BYTES, "object record is 0x152 bytes");
        bytes[0xb0..0xb4].copy_from_slice(&self.radius.to_le_bytes());
        bytes[0xba..0xc2].copy_from_slice(&self.phase.to_le_bytes());
        bytes[0xc2..0xc6].copy_from_slice(&self.rate.to_le_bytes());
        bytes[0xc6..0xc8].copy_from_slice(&self.longitude.to_le_bytes());
        bytes[0xc8] = self.spin;
        bytes[0xc9] = self.mode;
        bytes[0xca] = self.current;
        bytes[0xcb] = self.sys;
        bytes[0xcc..0xce].copy_from_slice(&self.tilt.to_le_bytes());
        bytes[0xce..0xd0].copy_from_slice(&self.bays.to_le_bytes());
        bytes[0xd0..0xd8].copy_from_slice(&self.pads);
        bytes[0xd8..0xda].copy_from_slice(&self.timeout.to_le_bytes());
        bytes[0xda..0xdc].copy_from_slice(&self.for_weight.to_le_bytes());
        bytes[0xdc..0xde].copy_from_slice(&self.response.to_le_bytes());
        bytes[0xde..0xe0].copy_from_slice(&self.heat.to_le_bytes());
        bytes[0xe0..0xe2].copy_from_slice(&self.temperature.to_le_bytes());
    }
}

/// F649-F657's station record at `+0xc9..+0xd8` plus the `+0x9e` door word,
/// read from a full object record.
pub fn starport_from_bytes(bytes: &[u8]) -> Option<crate::traffic::Starport> {
    let b = bytes.get(..OBJECT_BYTES)?;
    let mut record = OrbitalOverlay::from_object_bytes(b)?.starport();
    record.door = u16_at(b, offsets::TIMER);
    Some(record)
}

/// Write a [`crate::traffic::Starport`] into a full object record, including
/// the `+0x9e` door word.
pub fn set_starport_bytes(bytes: &mut [u8], record: &crate::traffic::Starport) {
    let overlay = OrbitalOverlay::from_object_bytes(bytes).unwrap_or_default();
    let mut overlay = overlay;
    overlay.set_starport(record);
    overlay.write_to_object_bytes(bytes);
    bytes[offsets::TIMER..offsets::TIMER + 2].copy_from_slice(&record.door.to_le_bytes());
}

/// F1054's chaff object: F291's F928 object with model 0xa4, control `0xb` and
/// the cargo kind `0xfa`. F1054 keeps the slot in `DATA_007253`; the caller
/// resolves it through [`ShipSlots::slot_of_decoy`] and F1057 retargets the
/// missiles to it.
pub fn decoy_record(slot: u8, position: [i64; 3], radius: i64) -> ObjectRecord {
    ObjectRecord {
        model: crate::missile::DECOY_MODEL,
        object: slot,
        control: CONTROL_AI_SHIP,
        position,
        agent: AgentRecord {
            kind: KIND_CARGO,
            radius,
            collision_radius: radius,
            ..AgentRecord::default()
        },
        ..ObjectRecord::default()
    }
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(b[at..at + 2].try_into().unwrap())
}

fn i16_at(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes(b[at..at + 2].try_into().unwrap())
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn i64_at(b: &[u8], at: usize) -> i64 {
    i64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn i16_row(b: &[u8], at: usize) -> [i16; 3] {
    [i16_at(b, at), i16_at(b, at + 2), i16_at(b, at + 4)]
}

fn i32_row3(b: &[u8], at: usize) -> [i32; 3] {
    [
        u32_at(b, at) as i32,
        u32_at(b, at + 4) as i32,
        u32_at(b, at + 8) as i32,
    ]
}

fn i64_row(b: &[u8], at: usize) -> [i64; 3] {
    [i64_at(b, at), i64_at(b, at + 8), i64_at(b, at + 16)]
}

fn put_u16(b: &mut [u8], at: usize, value: u16) {
    b[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_i16(b: &mut [u8], at: usize, value: i16) {
    b[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(b: &mut [u8], at: usize, value: u32) {
    b[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_i64(b: &mut [u8], at: usize, value: i64) {
    b[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record whose every field is distinctive, so a wrong offset shows up as
    /// a wrong value rather than an accidental zero.
    fn sample() -> ObjectRecord {
        let mut agent = AgentRecord {
            target: 0x11,
            ai_mode: 0x12,
            nav_target: 0x13,
            retro: 0x14,
            target_offset: [0x1515, 0x1616, 0x1717],
            target_direction: [0x1818, 0x1919, 0x1a1a],
            target_distance: 0x1b1b,
            autopilot: [0x1c1c, 0x1d1d, 0x1e1e],
            capacity: 0x1f1f,
            kind: KIND_PIRATE,
            cargo_fuel: 0x20,
            bounty: 0x2121_2121,
            fuel: 0x2222_2222,
            radius: 0x2323_2323_2323_2323,
            collision_radius: 0x2424_2424_2424_2424,
            physics_timeout: 0x2525_2525,
            flags: FLAG_DOCKABLE | FLAG_STARPORT | FLAG_OPEN_AIR | 5,
            for_timeout: 0x26,
            smoke_timeout: 0x27,
            ecm_timeout: 0x28,
            avoid_timeout: 0x29,
            laser_flags: LASER_SHIELDS_HIT | LASER_FIRED | LASER_FRONT,
            ..AgentRecord::default()
        };
        agent.set_name("Cobra Mk III 7");

        let mut record = ObjectRecord {
            orientation: [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
            drift: -2,
            position_valid: 1,
            global_position: [10, 11, 12],
            position: [13, 14, 15],
            parent: 16,
            rotating: 17,
            orientation_valid: 18,
            global_orientation: [[19, 20, 21], [22, 23, 24], [25, 26, 27]],
            scale: 0x2828_2828,
            model: 0x29,
            object: 0x2a,
            control: CONTROL_AI_SHIP,
            distance: 0x2b,
            distance_outside: 0x2c,
            update_cadence: 0x2d,
            velocity: [0x3030_3030, 0x3131_3131, 0x3232_3232],
            screen_x: 0x3333,
            screen_y: 0x3434,
            animation: 0x3535,
            timer: 0x3636,
            seed: 0x3737_3737,
            mass: 0x3838_3838,
            clock_tics: 0x3939_3939,
            clock_days: 0x3a3a_3a3a,
            rates: [0x3b3b, 0x3c3c, 0x3d3d],
            thrust: [0x3e3e, 0x3f3f, 0x4040],
            acceleration: [0x4141, 0x4242, 0x4343, 0x4444, 0x4545, 0x4646],
            equipment: 0x4747_4747,
            damaged: 0x4848_4848,
            drive: 0x49,
            gun_mounts: 0x4a,
            lasers: [0x4b, 0x4c, 0x4d, 0x4e],
            pylons: [0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58],
            shields_total: 0x5555,
            shields: 0x5656,
            hull: 0x5757,
            cloud_model: 0x5858,
            refire: 0x5959,
            heat: 0x5a5a,
            fraction: [0x5b5b, 0x5c5c, 0x5d5d],
            tail: [0; 12],
            agent,
        };
        record.set_caps([0x5e5e_5e5e, 0x5f5f_5f5f, 0x6060_6060]);
        record
    }

    #[test]
    fn every_documented_field_lands_at_its_plstate_offset() {
        let record = sample();
        let mut bytes = vec![0u8; OBJECT_BYTES];
        record.write(&mut bytes);

        assert_eq!(i32_row3(&bytes, 0x00), [1, 2, 3]);
        assert_eq!(i32_row3(&bytes, 0x0c), [4, 5, 6]);
        assert_eq!(i32_row3(&bytes, 0x18), [7, 8, 9]);
        assert_eq!(bytes[0x24] as i8, -2);
        assert_eq!(bytes[0x25], 1);
        assert_eq!(i64_row(&bytes, 0x26), [10, 11, 12]);
        assert_eq!(i64_row(&bytes, 0x3e), [13, 14, 15]);
        assert_eq!(bytes[0x56], 16);
        assert_eq!(bytes[0x57], 17);
        assert_eq!(bytes[0x58], 18);
        assert_eq!(i32_row3(&bytes, 0x5a), [19, 20, 21]);
        assert_eq!(u32_at(&bytes, 0x7e), 0x2828_2828);
        assert_eq!(u32_at(&bytes, 0x82), 0x29);
        assert_eq!(bytes[0x86], 0x2a);
        assert_eq!(bytes[0x87], CONTROL_AI_SHIP);
        assert_eq!(bytes[0x88], 0x2b);
        assert_eq!(bytes[0x8a], 0x2c);
        assert_eq!(bytes[0x8b], 0x2d);
        assert_eq!(
            i32_row3(&bytes, 0x8c),
            [0x3030_3030, 0x3131_3131, 0x3232_3232]
        );
        assert_eq!(u16_at(&bytes, 0x98), 0x3333);
        assert_eq!(u16_at(&bytes, 0x9a), 0x3434);
        assert_eq!(u16_at(&bytes, 0x9c), 0x3535);
        assert_eq!(u16_at(&bytes, 0x9e), 0x3636);
        assert_eq!(u32_at(&bytes, 0xa0), 0x3737_3737);
        assert_eq!(u32_at(&bytes, 0xa4), 0x3838_3838);
        assert_eq!(u32_at(&bytes, 0xa8), 0x3939_3939);
        assert_eq!(u32_at(&bytes, 0xac), 0x3a3a_3a3a);
        assert_eq!(i16_row(&bytes, 0xb0), [0x3b3b, 0x3c3c, 0x3d3d]);
        assert_eq!(i16_row(&bytes, 0xb6), [0x3e3e, 0x3f3f, 0x4040]);
        assert_eq!(i16_at(&bytes, 0xbc), 0x4141);
        assert_eq!(i16_at(&bytes, 0xc6), 0x4646);
        assert_eq!(u32_at(&bytes, 0xc8), 0x4747_4747);
        assert_eq!(u32_at(&bytes, 0xcc), 0x4848_4848);
        assert_eq!(bytes[0xd0], 0x49);
        assert_eq!(bytes[0xd1], 0x4a);
        assert_eq!(&bytes[0xd2..0xd6], &[0x4b, 0x4c, 0x4d, 0x4e]);
        assert_eq!(
            &bytes[0xd6..0xe0],
            &[0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58]
        );
        assert_eq!(u16_at(&bytes, 0xe0), 0x5555);
        assert_eq!(u16_at(&bytes, 0xe2), 0x5656);
        assert_eq!(u16_at(&bytes, 0xe4), 0x5757);
        assert_eq!(u16_at(&bytes, 0xe6), 0x5858);
        assert_eq!(u16_at(&bytes, 0xe8), 0x5959);
        assert_eq!(u16_at(&bytes, 0xea), 0x5a5a);
        assert_eq!(u16_at(&bytes, 0xec), 0x5b5b);
        assert_eq!(u16_at(&bytes, 0xee), 0x5c5c);
        assert_eq!(u16_at(&bytes, 0xf0), 0x5d5d);
        // The agent block starts exactly at +0xfe.
        assert_eq!(bytes[0xfe], 0x11);
        assert_eq!(bytes[0xff], 0x12);
        assert_eq!(bytes[0x100], 0x13);
        assert_eq!(bytes[0x101], 0x14);
        assert_eq!(i16_row(&bytes, 0x102), [0x1515, 0x1616, 0x1717]);
        assert_eq!(i16_row(&bytes, 0x108), [0x1818, 0x1919, 0x1a1a]);
        assert_eq!(u16_at(&bytes, 0x10e), 0x1b1b);
        assert_eq!(i16_row(&bytes, 0x110), [0x1c1c, 0x1d1d, 0x1e1e]);
        assert_eq!(u16_at(&bytes, 0x116), 0x1f1f);
        assert_eq!(bytes[0x118], KIND_PIRATE);
        assert_eq!(bytes[0x119], 0x20);
        assert_eq!(u32_at(&bytes, 0x11a), 0x2121_2121);
        assert_eq!(u32_at(&bytes, 0x11e), 0x2222_2222);
        assert_eq!(&bytes[0x124..0x132], b"Cobra Mk III 7");
        assert_eq!(i64_at(&bytes, 0x138), 0x2323_2323_2323_2323u64 as i64);
        assert_eq!(i64_at(&bytes, 0x140), 0x2424_2424_2424_2424u64 as i64);
        assert_eq!(u32_at(&bytes, 0x148), 0x2525_2525);
        assert_eq!(
            bytes[0x14c],
            FLAG_DOCKABLE | FLAG_STARPORT | FLAG_OPEN_AIR | 5
        );
        assert_eq!(bytes[0x14d], 0x26);
        assert_eq!(bytes[0x14e], 0x27);
        assert_eq!(bytes[0x14f], 0x28);
        assert_eq!(bytes[0x150], 0x29);
        assert_eq!(bytes[0x151], LASER_SHIELDS_HIT | LASER_FIRED | LASER_FRONT);
    }

    #[test]
    fn a_record_round_trips_and_leaves_the_padding_bytes_alone() {
        let record = sample();
        let mut bytes: Vec<u8> = (0..OBJECT_BYTES).map(|i| (i * 37 + 11) as u8).collect();
        let original = bytes.clone();
        record.write(&mut bytes);
        assert_eq!(&bytes[0x59..0x5a], &original[0x59..0x5a]);
        assert_eq!(&bytes[0x89..0x8a], &original[0x89..0x8a]);
        assert_eq!(ObjectRecord::from_bytes(&bytes), Some(record));
    }

    #[test]
    fn a_short_buffer_is_not_a_record() {
        assert_eq!(ObjectRecord::from_bytes(&[0; OBJECT_BYTES - 1]), None);
    }

    #[test]
    fn caps_and_hyperspace_are_the_two_readings_of_the_f2_union() {
        let mut record = ObjectRecord::default();
        record.set_caps([1, -2, 3]);
        assert_eq!(record.caps(), [1, -2, 3]);
        // The same bytes are F738's destination/range/post-mode/jump-time.
        assert_eq!(record.hyperspace().destination, 1);
        assert_eq!(record.hyperspace().max_range, 0xfffe);

        record.set_hyperspace(HyperspaceWords {
            destination: 0x1234_5678,
            max_range: 0x9abc,
            post_mode: 0x08,
            jump_time: 0x00ff_00ff,
        });
        assert_eq!(
            record.hyperspace(),
            HyperspaceWords {
                destination: 0x1234_5678,
                max_range: 0x9abc,
                post_mode: 0x08,
                jump_time: 0x00ff_00ff,
            }
        );
        assert_eq!(
            record.caps(),
            [0x1234_5678, 0xff08_9abc_u32 as i32, 0x00ff_00ff]
        );

        let mut bytes = vec![0u8; OBJECT_BYTES];
        record.write(&mut bytes);
        assert_eq!(u32_at(&bytes, 0xf2), 0x1234_5678);
        assert_eq!(u16_at(&bytes, 0xf6), 0x9abc);
        assert_eq!(bytes[0xf8], 0x08);
        assert_eq!(u32_at(&bytes, 0xfa), 0x00ff_00ff);
    }

    #[test]
    fn the_name_field_is_nul_padded_and_bounded() {
        let mut agent = AgentRecord::default();
        agent.set_name("Moray Starboat 3");
        assert_eq!(agent.name_string(), "Moray Starboat 3");
        assert_eq!(agent.name_bytes(), b"Moray Starboat 3");
        assert_eq!(agent.name[16], 0);

        agent.set_name("0123456789012345678901234");
        assert_eq!(agent.name_bytes().len(), NAME_BYTES - 1);
        assert_eq!(agent.name_string(), "0123456789012345678");

        // Bytes the game wrote are not necessarily UTF-8; they still round-trip.
        agent.name[0] = 0xff;
        assert!(agent.name_string().starts_with('\u{fffd}'));
    }

    #[test]
    fn class_and_laser_flags_decode_from_plstate() {
        let agent = AgentRecord {
            flags: FLAG_DOCKABLE | FLAG_STARPORT | FLAG_COMPLEX | 6,
            laser_flags: LASER_FRONT | LASER_TURRET | LASER_ECM | LASER_ENERGY_BOMB,
            ..AgentRecord::default()
        };
        assert!(agent.is_dockable());
        assert!(agent.is_starport());
        assert!(!agent.is_open_air());
        assert!(agent.is_complex());
        assert_eq!(agent.light_index(), 6);
    }

    #[test]
    fn phys_lists_store_the_same_bytes() {
        // The byte list and the typed record are one layout: object 3 starts at
        // 0x74 + 3 * 0x152, and the per-slot flag byte is outside the record.
        let mut list = crate::objects::PhysList::default();
        list.bytes[3] = 0x4b;
        let record = sample();
        list.set_record(3, &record);
        assert_eq!(list.record(3), record);
        assert_eq!(list.flag(3), 0x4b);
        let at = crate::objects::FIRST_OBJECT + 3 * crate::objects::OBJECT_BYTES;
        assert_eq!(u32_at(&list.bytes, at + 0x82), record.model);
        assert_eq!(list.bytes[at + 0x86], record.object);
        assert_eq!(i64_at(&list.bytes, at + 0x138), record.agent.radius);
        assert_eq!(list.bytes.len(), crate::objects::LIST_BYTES);
    }

    #[test]
    fn f927_hands_out_slots_from_the_top_down() {
        let mut slots = ShipSlots::new();
        assert_eq!(slots.allocate(10), Some(0x72));
        assert_eq!(slots.allocate(11), Some(0x71));
        assert_eq!(slots.allocate(12), Some(0x70));
        // Allocating an id that is already in the list is a no-op.
        assert_eq!(slots.allocate(10), Some(0x72));
        assert_eq!(slots.slot_of(11), Some(0x71));
        assert_eq!(slots.owner(0x72), Some(10));
        assert_eq!(
            slots.iter().collect::<Vec<_>>(),
            vec![(0x70, 12), (0x71, 11), (0x72, 10),]
        );
    }

    #[test]
    fn a_freed_slot_is_the_next_one_handed_out() {
        let mut slots = ShipSlots::new();
        for id in 0..4 {
            slots.allocate(100 + id);
        }
        assert_eq!(slots.free(100), Some(0x72));
        assert_eq!(slots.allocate(200), Some(0x72));
        assert_eq!(slots.free(102), Some(0x70));
        // 0x72 and 0x71 are taken again: the topmost free slot is 0x70.
        assert_eq!(slots.allocate(201), Some(0x70));
        assert_eq!(slots.slot_of(101), Some(0x71));
    }

    #[test]
    fn the_body_slots_below_0x14_are_never_handed_out() {
        let mut slots = ShipSlots::new();
        // One ship per slot from 0x14 to 0x72, in F927's order.
        let ships = u32::from(TOP_SLOT - FIRST_SHIP_SLOT) + 1;
        for id in 0..ships {
            let slot = slots.allocate(id).expect("a free ship slot");
            assert!(slot >= FIRST_SHIP_SLOT);
        }
        assert_eq!(slots.allocate(ships), None);
        assert_eq!(slots.slot_of(ships - 1), Some(FIRST_SHIP_SLOT));
        assert_eq!(slots.slot_of(0), Some(TOP_SLOT));
        assert_eq!(slots.free(ships - 1), Some(FIRST_SHIP_SLOT));
        assert_eq!(slots.allocate(ships), Some(FIRST_SHIP_SLOT));
    }

    #[test]
    fn syncing_the_live_list_frees_departed_ids_and_keeps_the_rest() {
        let mut slots = ShipSlots::new();
        slots.sync([1, 2, 3]);
        assert_eq!(slots.slot_of(1), Some(0x72));
        assert_eq!(slots.slot_of(2), Some(0x71));
        assert_eq!(slots.slot_of(3), Some(0x70));
        // Ship 1 leaves and ship 2 stays; the new ship 4 takes 0x72.
        slots.sync([2, 3, 4]);
        assert_eq!(slots.slot_of(1), None);
        assert_eq!(slots.slot_of(2), Some(0x71));
        assert_eq!(slots.slot_of(3), Some(0x70));
        assert_eq!(slots.slot_of(4), Some(0x72));
    }

    #[test]
    fn saved_slots_are_adopted_when_they_are_still_free() {
        let slots = ShipSlots::from_saved([(7, 0x70), (8, 0x72)]);
        assert_eq!(slots.slot_of(7), Some(0x70));
        assert_eq!(slots.slot_of(8), Some(0x72));
        // A duplicate or a body slot is not adopted; sync fills it from the top.
        let mut clash = ShipSlots::from_saved([(7, 0x70), (8, 0x70), (9, 1)]);
        assert_eq!(clash.slot_of(8), None);
        assert_eq!(clash.slot_of(9), None);
        clash.sync([7, 8, 9]);
        assert_eq!(clash.slot_of(7), Some(0x70));
        assert_eq!(clash.slot_of(8), Some(0x72));
        assert_eq!(clash.slot_of(9), Some(0x71));
    }

    #[test]
    fn the_player_object_takes_the_topmost_slot_and_the_ships_start_below_it() {
        let mut slots = ShipSlots::new();
        // F1074 allocates the player before F688's traffic.
        assert_eq!(slots.reserve_player(), Some(TOP_SLOT));
        assert_eq!(slots.player_slot(), Some(TOP_SLOT));
        slots.sync([3, 4]);
        assert_eq!(slots.slot_of(3), Some(0x71));
        assert_eq!(slots.slot_of(4), Some(0x70));
        // A ship leaving does not free the player's slot, and a new system
        // keeps it while the traffic is allocated afresh.
        slots.sync([4]);
        assert_eq!(slots.player_slot(), Some(TOP_SLOT));
        slots.clear();
        assert_eq!(slots.player_slot(), Some(TOP_SLOT));
        assert_eq!(slots.slot_of(4), None);
        slots.sync([5]);
        assert_eq!(slots.slot_of(5), Some(0x71));
        // The player key is not a ship owner.
        assert_eq!(slots.owner(TOP_SLOT), None);
    }

    #[test]
    fn a_saved_ship_in_the_players_slot_is_reallocated_but_a_fresh_one_keeps_its_own() {
        // `from_saved_with_player` reserves 0x72 first; an old save whose ship
        // held that slot leaves it for `sync`.
        let mut slots = ShipSlots::from_saved_with_player([(7, 0x72), (8, 0x71)]);
        assert_eq!(slots.player_slot(), Some(TOP_SLOT));
        assert_eq!(slots.slot_of(7), None);
        assert_eq!(slots.slot_of(8), Some(0x71));
        slots.sync([7, 8]);
        assert_eq!(slots.slot_of(7), Some(0x70));
        assert_eq!(slots.slot_of(8), Some(0x71));
    }

    #[test]
    fn missiles_share_the_slot_space_and_are_freed_when_they_die() {
        let mut slots = ShipSlots::new();
        slots.reserve_player();
        slots.sync([9]);
        assert_eq!(slots.allocate_missile(0), Some(0x70));
        assert_eq!(slots.allocate_missile(1), Some(0x6f));
        assert_eq!(slots.slot_of_missile(0), Some(0x70));
        assert_eq!(slots.slot_of_missile(1), Some(0x6f));
        // A missile id is not a ship id.
        assert_eq!(slots.slot_of(0), None);
        // F718/F758 remove the missile: its slot goes back to F927.
        slots.sync_missiles([1]);
        assert_eq!(slots.slot_of_missile(0), None);
        assert_eq!(slots.allocate_missile(2), Some(0x70));
        // The ship and player slots survive the missile sync.
        assert_eq!(slots.slot_of(9), Some(0x71));
        assert_eq!(slots.player_slot(), Some(TOP_SLOT));
    }

    #[test]
    fn a_missile_record_carries_its_target_and_launcher_indexes() {
        use crate::missile::{Missile, Spec, State, AI_MISSILE};
        let missile = Missile {
            spec: Spec {
                model: 6,
                damage: 200,
                forward: 0.0,
                reverse: 0.0,
                side: 0.0,
                turn: 0.0,
                radius: 2100,
            },
            ai_state: AI_MISSILE,
            id: 3,
            owner: PLAYER_OBJECT,
            target: Some(4),
            position: [10, 20, 30],
            velocity: [1.0, 2.0, 3.0],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            age: 0.0,
            guidance: 0.0,
            updates: 0,
            thrust: [0.0; 3],
            state: State::Flying,
        };
        let record = missile_record(&missile, 0x70, 0x71, 0x72);
        assert_eq!(record.object, 0x70);
        assert_eq!(record.control, CONTROL_AI_SHIP);
        assert_eq!(record.agent.kind, KIND_MISSILE);
        assert_eq!(record.agent.target, 0x71);
        assert_eq!(record.agent.nav_target, 0x72);
        assert_eq!(record.agent.ai_mode, AI_MISSILE);
        // `+0x00`: the axes the frame draws the missile with.
        assert_eq!(record.axes(), missile.axes);
        let mut bytes = [0u8; OBJECT_BYTES];
        record.write(&mut bytes);
        assert_eq!(bytes[offsets::OBJECT], 0x70);
        assert_eq!(bytes[offsets::TARGET], 0x71);
        assert_eq!(bytes[offsets::NAV_TARGET], 0x72);
        assert_eq!(bytes[offsets::KIND], KIND_MISSILE);
        assert_eq!(u32_at(&bytes, offsets::MODEL), 6);
    }

    #[test]
    fn chaff_objects_share_the_slot_space_and_have_a_record() {
        use crate::missile::DECOY_BASE;
        let mut slots = ShipSlots::new();
        slots.reserve_player();
        slots.sync([9]);
        let decoy = DECOY_BASE + 3;
        assert_eq!(slots.allocate_decoy(decoy), Some(0x70));
        assert_eq!(slots.slot_of_decoy(decoy), Some(0x70));
        // A decoy is not a missile; its key is its own id.
        assert_eq!(slots.slot_of_missile(3), None);
        // F1054's object: model 0xa4, control 0xb, the cargo kind.
        let record = decoy_record(0x70, [10, 20, 30], 2_000);
        assert_eq!(record.model, crate::missile::DECOY_MODEL);
        assert_eq!(record.object, 0x70);
        assert_eq!(record.control, CONTROL_AI_SHIP);
        assert_eq!(record.agent.kind, KIND_CARGO);
        assert_eq!(record.agent.radius, 2_000);
        assert_eq!(record.position, [10, 20, 30]);
        let mut bytes = [0u8; OBJECT_BYTES];
        record.write(&mut bytes);
        assert_eq!(u32_at(&bytes, offsets::MODEL), 0xa4);
        assert_eq!(bytes[offsets::TARGET], 0);
        // The ship sync leaves the chaff slot alone, and a departed decoy's
        // slot goes back to F927.
        slots.sync([9]);
        assert_eq!(slots.slot_of_decoy(decoy), Some(0x70));
        slots.sync_decoys([]);
        assert_eq!(slots.slot_of_decoy(decoy), None);
        assert_eq!(slots.allocate_decoy(DECOY_BASE + 4), Some(0x70));
    }

    #[test]
    fn the_orbital_overlay_round_trips_the_b0_block() {
        let overlay = OrbitalOverlay {
            radius: 0x5000_0000,
            phase: 0x0123_4567_89ab_cdef,
            rate: 0x0001_0000,
            longitude: -0x1234,
            spin: 3,
            mode: 0xff,
            current: 0x21,
            sys: 0x3f,
            tilt: 0x0456,
            bays: 5,
            pads: [1, 2, 3, 4, 5, 0, 0, 0],
            timeout: 0x7080,
            for_weight: 0x0102,
            response: 0x30,
            heat: 0x0304,
            temperature: 0x028a,
        };
        let mut bytes = [0u8; OBJECT_BYTES];
        overlay.write_to_object_bytes(&mut bytes);
        assert_eq!(u32_at(&bytes, 0xb0), 0x5000_0000);
        assert_eq!(
            u64::from_le_bytes(bytes[0xba..0xc2].try_into().unwrap()),
            0x0123_4567_89ab_cdef
        );
        assert_eq!(u32_at(&bytes, 0xc2), 0x0001_0000);
        assert_eq!(i16_at(&bytes, 0xc6), -0x1234);
        assert_eq!(bytes[0xc9], 0xff);
        assert_eq!(bytes[0xd0..0xd5], [1, 2, 3, 4, 5]);
        assert_eq!(u16_at(&bytes, 0xd8), 0x7080);
        assert_eq!(u16_at(&bytes, 0xe0), 0x028a);
        assert_eq!(OrbitalOverlay::from_object_bytes(&bytes), Some(overlay));
        // The ship reading of the same bytes is the overlapping field group.
        let record = ObjectRecord::from_bytes(&bytes).unwrap();
        assert_eq!(record.rates[0] as u32 as u16, 0);
        assert_eq!(record.orbit(), overlay);
    }

    #[test]
    fn an_orbiter_writes_its_motion_half_and_reads_it_back() {
        use crate::model_math::SineTable;
        use crate::objects::Orbiter;
        let table = SineTable::from_bytes(&vec![0; 16384 * 4]).unwrap();
        let orbit = Orbiter::new(
            &table,
            0x4000_0000,
            0,
            0x0001_0000,
            0x2000,
            0,
            0,
            0x10,
            [1234, 5],
        );
        let overlay = OrbitalOverlay::from_orbiter(&orbit);
        let mut record = ObjectRecord::default();
        record.set_orbit(overlay);
        let read = ObjectRecord::from_bytes(&{
            let mut bytes = [0u8; OBJECT_BYTES];
            record.write(&mut bytes);
            bytes
        })
        .unwrap()
        .orbit();
        assert_eq!(read, overlay);
        let mut back = orbit;
        read.apply_orbit(&mut back);
        assert_eq!(back, orbit);
        assert_eq!(read.radius, orbit.radius);
        assert_eq!(read.phase, orbit.phase);
        assert_eq!(read.rate, orbit.rate);
        assert_eq!(read.longitude, orbit.longitude);
        assert_eq!(read.tilt, orbit.tilt);
    }

    #[test]
    fn the_starport_traffic_record_bridges_to_the_orbital_bytes() {
        use crate::traffic::Starport;
        let mut station = Starport::new(5);
        station.mode = 0xff;
        station.current = 0x21;
        station.timeout = 0xc350;
        station.pads[0] = 7;
        station.pads[4] = 9;
        station.door = 0x1234;
        station.response = Some(0x20);
        station.heat = 0x0042;
        let mut bytes = [0u8; OBJECT_BYTES];
        set_starport_bytes(&mut bytes, &station);
        assert_eq!(bytes[0xc9], 0xff);
        assert_eq!(bytes[0xca], 0x21);
        assert_eq!(u16_at(&bytes, 0xce), 5);
        assert_eq!(bytes[0xd0], 7);
        assert_eq!(bytes[0xd4], 9);
        assert_eq!(u16_at(&bytes, 0xd8), 0xc350);
        assert_eq!(u16_at(&bytes, 0xdc), 0x20);
        assert_eq!(u16_at(&bytes, 0xde), 0x42);
        assert_eq!(u16_at(&bytes, offsets::TIMER), 0x1234);
        assert_eq!(starport_from_bytes(&bytes), Some(station));
    }
}
