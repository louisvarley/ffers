# Object system and agent records (`cstruct`, `gmstruct`)

Dashboard `flight-1`. Sources: JJFFE 2.8a7 `ffe.asm`/`ffedat.asm`, plus the
reverse-engineering notes in `jjffe-master/srctools/`: `cstruct.txt` (the
`Starport`/BBS/shipyard records), `gmstruct.txt` (the 0x556a-byte global block
at `DATA_008804`) and `plstate.txt` (the `PhysObj` field map). `cagent.txt` maps
the agent (NPC) routines F708-F790; the port's transcription of their dispatch
table is `ffe_core::npc::AI_STATES` (see `docs/notes/npc.md`).

## What the records are

- **`gmstruct`** is the single mutable game-state block, `DATA_008804`, 0x556a
  bytes, zeroed at startup (`_asmmain`). It holds the clock (`+0x000` tics,
  `+0x004` days), the seeds, the player/starport pointers and indexes, the
  player's flight state and flags, the cargo array and the rank/cash/fine
  words. The port splits this block across `ffe_core::player`, `galaxy::Details`,
  `npc::Legal`, `traffic`, etc.; it is not one struct.
- **`cstruct`** is the record-structure file: the 0x322-byte `Starport`
  record (`DATA_008999`, 18 slots, `+0x130` object index, `+0x131` flags),
  `StarportStock` (8 bytes) and `ShipyardEntry` (4 bytes). The port's
  `ffe_core::traffic::Starport`, `market` and `shipyard` records cover it.
- **The object record** is `PhysObj`: 0x152 bytes, 0x73 slots (`0..=0x72`),
  with a flag byte per slot first and object 0 at `+0x74`. The live list is
  reached through the `DATA_007758`/`DATA_009133` pointers (the storage follows
  at `DATA_009134`); `objects::PhysList` already uses this byte layout.
  `+0x87` is the control byte (1 orbital, 3 player, 0xb AI ship), `+0x82` the
  model, `+0x86` the object's own index, `+0xfe..+0x151` the agent block.
- **The agent block** is the AI half of the same record: `+0xfe` target,
  `+0xff` AI mode, `+0x100` navigation target, `+0x102..+0x10e` aim offsets and
  direction, `+0x110..+0x114` autopilot rates, `+0x116` capacity, `+0x118`
  ship-table kind, `+0x119` cargo fuel, `+0x11a` bounty, `+0x11e` fuel,
  `+0x124` name, `+0x138`/`+0x140` radii, `+0x148` physics timeout,
  `+0x14c` class flags, `+0x14d..+0x150` timeouts and `+0x151` laser flags.
  `DATA_006255..006258` are not records: they are the four-routine columns of
  the AI dispatch table (control, movement, notify, damage), already ported as
  `AI_STATES`.

`DATA_008561` is **not** an object array. It is 0x96 (150) 0x24-byte module
records used by `FUNC_000054`/`FUNC_000055` (the module event masks and
load/save callbacks). The per-object records are the 0x152-byte `PhysObj`
slots above.

## What the port already carries

| Source | Port |
|---|---|
| `+0x00` orientation, `+0x24` drift, `+0x56` parent, `+0x57` rotating, `+0x58` orientation valid, `+0x8c` velocity, `+0xec` fraction, `+0xf2..` caps, `+0xb0` rates, `+0xb6` thrust, `+0x110` autopilot, `+0xd1` mounts, `+0x148` timeout | `physics::Motion` |
| `+0x3e` position, `+0x82` model, `+0x86` craft | `NpcShip::{position,model,id}` (the craft byte is the stable id, not the slot; `+0x87` control is not stored) |
| `+0xe0/+0xe2` shields, `+0xe4` hull, `+0xe8` refire, `+0xea` heat, `+0x11e` energy | `laser::DamageState`, `laser::State` |
| `+0xc8` equipment, `+0xd2` lasers, `+0xd6..+0xdf` pylons | `NpcShip::{general,laser,pylons}` |
| `+0x11a` bounty, `+0x118` kind, `+0x124` name, `+0x138` radius | `NpcShip::{bounty,role,name,radius}` |
| `+0xfe` target, `+0xff` AI mode, `+0x100` nav target, `+0x102..` waypoint | `NpcShip::{npc_target,state/ai_code,station,waypoint,leader,formation}` |

## The typed record layer (2026-11-20)

`ffe_core::object_record` now has the full source-shaped view:

- `ObjectRecord`: every field at its documented `PhysObj` offset, with
  `from_bytes`/`write` over a 0x152-byte slice.
- `AgentRecord`: the `+0xfe..+0x151` agent block, read and written in place.
- `offsets`: the field table, and `flags`/`CONTROL_*`/`KIND_*`/`LASER_*`
  constants from `plstate.txt`.
- `ObjectRecord::caps`/`set_caps` and `hyperspace`/`set_hyperspace` expose the
  two source interpretations of the `+0xf2..+0xfd` union (motion caps for
  ordinary objects, destination/range/post-HS state/jump time for hyperspace
  agents).
- `objects::PhysList::{record,set_record}` read and write a slot's typed record.

## Live object-list wiring (2026-11-20)

- **Slots.** `object_record::ShipSlots` is F927: the topmost free slot from
  `0x72` down, never below `0x14`, with the freed slot reused first. It keys
  the port's stable ship ids to slots, so `+0x86` is the same index the source
  would allocate. `ShipSlots::assign` frees departed ships and stores each
  slot in `NpcShip::slot`; `sync` keeps a saved table, `from_saved` adopts the
  slots a save carries.
- **Player and missiles.** The player object (`DATA_008857`, F1074/F928) is
  reserved first with `ShipSlots::reserve_player`, so it holds `0x72` and the
  traffic starts at `0x71`, as the source allocates it. F941's missiles share
  the same slot space (`allocate_missile`/`sync_missiles`); a spent missile's
  slot goes back to F927. F1054's chaff objects share it too
  (`allocate_decoy`/`sync_decoys`). `Navigator::player_object_record` /
  `missile_object_record` / `missile_object_records` / `decoy_object_record`
  expose those records; the flight frame's missile list is built from
  `missile_object_records`, the first live renderer consumer.
- **Live list.** `Navigator` owns the table and calls `sync_object_slots`
  after every mutation of `ships` or `missiles`: the frame tick, F927's
  recycle, system entry (F688), contract/hyperspace/police spawns, script
  actions (`scripts::apply`), missile launches/ECM/ticks and save restore. A
  save's ships keep `slot`, and the table is rebuilt with the player's slot
  first.
- **Object indexes.** `NpcShip::object_record_in(&ObjectIndexes)` writes
  `+0x86` from the table, `+0x56`/`+0x3e` from the F606 frame, `+0xfe` from the
  ship's station while it is in the traffic cycle (F650/F747), else its ship
  target (`npc_target`, F695/F750), else the missile it is evading (F757), else
  the player object, and `+0x100` from its destination body's `Body::slot` (the
  source object index, not the `bodies` list index).
  `Navigator::ship_object_record`/`object_records` build them for the live list.
- **Jump words.** `spawn::jump_words` is F775/F776: two F857 picks one sector
  away, then two in the current sector, each filtered by F870's population
  (four or more) and checked with F901 (`hyperspace::ai_jump`) for the
  maximum range and jump time. `tick_ships` runs it on the frame a ship turns
  to flee, so `+0xf2`/`+0xf6`/`+0xfa` carry a real destination, range and
  time; `+0xf8` remains the post-jump mode F682 saves. A carried ship keeps
  `+0x119` fuel, and `npc::carried_jump` applies F680/F682 when it arrives.
- **Stored source-only fields.** `NpcShip` now stores `+0x14c` `class_flags`
  (from `ModelInfo`, via `Spec::class`), `+0x140` `collision_radius`, `+0xd0`
  `drive`, `+0xcc` `damaged`, `+0x101` `retro` (updated from the guidance's
  backward thrust at F1533's two thresholds), the `+0x14d..+0x150` timeout
  bytes, and the `+0xf2..+0xfd` jump words (`HyperspaceWords`; F682's saved
  post-jump mode is stored when the ship flees). `+0x148` comes from
  `Motion::switch_delay`. The snapshot no longer reconstructs these from
  role, timers or `Spec`. `+0x151` stays derived from the live `flash`/`shot`
  frame state (the source sets and clears those bits per frame).
- Carried ships (`Persisted`, F675/F680) keep class, collision radius, drive
  and damaged flags across a jump.

## Fields the port still reconstructs (remaining `flight-1` work)

1. **Record-driven frames.** F1054's chaff now takes an F927 slot
   (`ShipSlots::allocate_decoy`/`sync_decoys`, `object_record::decoy_record`) and
   the flight frame draws F941's missiles from `Navigator::missile_object_records`
   (`record.axes()` from `+0x00`) instead of the missile list, so the draw and
   the object table cannot disagree. NPC ships, chaff and the player object are
   still drawn from their port structs; only the missile frame reads the record.
2. **Frame-of-reference state.** F732's 19-second countdown sets
   `NpcShip::frame_due`; `Navigator::tick_ships` then runs F606 over the live
   bodies and ships. `update_frame_of_reference` calls the pure
   `frames::select_frame_of_reference` (F606's `(shift - weight) * 4 + 0x20`
   parent test, `shift - weight + 0x28` grandparent test and the descending-slot
   sibling/cousin scan) and stores `+0x56`/`+0x3e`, so the record reports the
   source's relative position. The physics still integrates in the root frame
   (the port's documented frame model); F604/F605 only move the stored view.
3. **Timers.** F953 sets `+0x14e = 10` on hull damage, F938 counts it (and
   `+0x14f`) down a second at a time, and F610's `+0x150` cycles every 16
   navigation frames. F725/F727/F729's F939 smoke particle is emitted by
   `NpcShip::smoke_spark` from the frame's lent F1530 pair (two words a frame,
   a deliberate RNG-ledger change) and `tick_ships` pushes it into the effect
   list. F802's avoidance: `npc::obstruction_along` tests the bodies and live
   ships inside the braking distance, and `NpcShip::steer_avoid` aims the next
   F597 step at the collision normal plus a quarter of the heading, re-arming
   `+0x150` as F610 does. The ship's station, leader and anchor are filtered out
   (they are its destination, not obstructions).
4. **Jump state machine.** F738/F682 set `+0xa8`/`+0xac` from `+0xfa`
   (`NpcShip::begin_jump`), the `Flee` cloud lasts the jump time instead of the
   port's three seconds, and `Persisted` carries `+0xf2..+0xfd` through the
   player's jump: F680 writes the system just left (`DATA_008916`) to `+0xf2`
   and F682's second F901 call (`npc::carried_jump`) gives the ship its `+0xf6`
   range and `+0xfa` time from the *player's* hyperspace distance, spending its
   `+0x119` fuel; a ship that cannot make it is F940's kill and is dropped.
   `restore` puts a carried pirate in `State::Jump` (0x11/0x12) and it returns
   in its saved `+0xf8` mode. `DockRequest::ChooseStation` runs F761 as
   `F775 && F775`: the second call is skipped when the first fails, and a failed
   call leaves the last F857 candidate in `+0xf2` while `+0xf6`/`+0xfa` keep the
   last successful pick. The one port deviation: a ship with no F776 time words
   at all (the port's traders do not keep their spawn pick) falls back to F762's
   local choice instead of a zero-time cloud that would vanish at once.
5. **Orbital overlay.** `object_record::OrbitalOverlay` is the typed reading of
   `+0xb0..+0xe0` (radius, phase, rate, longitude, spin, tilt and the starport
   docking state), with `ObjectRecord::orbit`/`set_orbit`,
   `OrbitalOverlay::{from_orbiter,apply_orbit}` and
   `starport_from_bytes`/`set_starport_bytes` for `traffic::Starport` (including
   the `+0x9e` door word). `Navigator` now keeps the system's live F880/F866
   `PhysList` (`system_with_list`): `body_object_record` reads a body's real
   `+0xb0..` orbit block and writes the live `traffic::Starport` into `+0xc9..`
   and `+0x9e` on every read, so the two halves cannot disagree; the NPC station
   snapshot reads it (`station_record`) and `object_records` includes the
   bodies. The renderer still draws `Body`/`Starport`.
6. **Player object's full block.** `player_object_record` fills `+0xfe` from the
   locked ship's slot (else the navigation body), `+0x100` from the
   autopilot's destination body, `+0xff` from the autopilot (`8`, F1007's basic
   navigation) or the landed state (`0x20`; zero free-flying), and `+0x10e` from
   F610's exponential distance to the autopilot's target. The `main` `Flight`
   mode is not consulted; `Navigator`'s autopilot/landed state stands in.

The record layer is deliberately independent of the uncommitted NPC changes;
the live wiring above is the call-out into `navigation.rs`/`scripts.rs`.

## The agent timeouts and the jump clock (2026-11-20, third pass)

- `+0x14d` FOR timeout: F732 (`cagent.txt:1184`) runs once a second, subtracts
  the elapsed seconds from the byte and, when it goes negative, calls F606 and
  resets it to `0x13`. `NpcShip::tick` runs the one-second pass and sets
  `frame_due`; `tick_ships` applies F606 with `update_frame_of_reference`.
- `+0x14e` smoke: F953's hull branch sets it to `0xa` (`ffe.asm:71290`), F938
  subtracts the elapsed seconds (`ffe.asm:70185`), and F725/F727/F729 call
  F939 at the object while it is non-zero. `NpcShip::smoke_spark` draws the two
  shared F1530 words on the frame's lent pair; `tick_ships` pushes the particle
  after the movement step (the source's F1512/F730/F939 order). The RNG ledger
  changes for a smoking ship, deliberately.
- `+0x14f` E.C.M./bomb: F937 sets 7 and F938 counts it; the port's ECM burst is
  instant, so only the byte counts down.
- `+0x150` avoidance: F610 (`cphys.txt:842`) subtracts the time-acceleration
  step and, when it drops below, resets to `(step + 1) << 4` and runs the F802
  collision check. `tick_ships` runs `npc::obstruction_along` at the cadence and
  `steer_avoid` replaces the aim with the collision normal plus a quarter of the
  heading, setting `+0x150 = 0` to re-check the next frame.
- `+0xa8`/`+0xac` and `+0xfa`: F738 (`cagent.txt:1282`) adds the jump time
  (`+0xa8 += +0xfa << 16`, `+0xac += +0xfa >> 16`); F764/F737/F740 wait it out.
  `NpcShip::begin_jump`/`jump_out` add the deadline and set the cloud's
  duration, and `object_record_in` writes `clock_tics`/`clock_days` from it.
- The jump cycle: F761 (`cagent.txt:1732`) tries F775 twice (two successes
  mean F762's local target, otherwise `+0xff = 0x1d` and F738); F680/F682
  (`cagent.txt:240`) re-enter a carried ship as the 0x11 delayed exit cloud,
  which F737 turns into 0x12 and F740 returns in `+0xf8`'s mode.

