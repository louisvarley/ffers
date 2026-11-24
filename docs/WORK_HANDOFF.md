# Work handoff

## Active — owner request 2026-11-23: galaxy-map bugs + enhanced map (toggle-gated)

**Scope (owner, new work; not an existing dashboard ID).** Two bugs on the
galaxy map / hyperjump, then an "enhanced galaxy map" feature set added as
launcher toggles, plus a `.md` that documents the toggle-addition workflow.

**Bugs to fix first.**

1. *"Fuel Required: 8t" but no jump.* **Fixed 2026-11-23.** The real gap was not a
   misleading fuel line: the original's galaxy is fully procedural and jumps to
   any in-range star, but the port only carried the fixed "on offer" systems and
   refused the rest as "unexplored". `hyperspace_to_destination_with` now
   generates a destination system on demand (`navigation::generated_entry`, the
   same F832 path the system map uses), `jump_possible` is the destination's
   range/fuel report (not entry presence), and `from_save` regenerates a saved
   system the fixed list does not carry. Verified by
   `a_jump_to_a_star_beyond_the_offer_generates_its_system` (jump + save/load
   round trip) plus the 36 cache-gated `navigation` and 7 `galaxy_map_view` tests.
2. *Map rotates while off-screen.* `GalaxyMap` is only live when
   `nav.view == NavView::Galaxy` (parked otherwise), and `map.rotate` is guarded
   by `galaxy.is_some()`; need the exact repro to confirm whether the drift is
   (a) `spin` persisting across park/restore, (b) the left-drag rotation also
   steering the cockpit look, or (c) `spin` i16 wrap. Record repro, then fix.

**Feature (each its own toggle, off by default, save-safe; docs + DEVIATIONS + test).**

- Multi-step jumps: `X` on an in-range system appends a waypoint (circle + line),
  the chain re-plans each leg from the previous waypoint; jumping runs the whole
  route with "Preparing hyperdrive…" and a ~5 s pause per leg, showing per-leg
  distance and fuel-remaining.
- Hover shows whether a system has a station/port.
- `Home`/`C` recentre the map on the current system.
- SOL direction + distance indicator.
- Middle-mouse zoom; right-drag pan.
- Rebuild the map screen to be easier to read and **drop the 3D projection**
  (owner: "remove need to have this map be 3d").

**Relevant files:** `crates/ffe-rs/src/{galaxy_map,map_keys,map_pages,system_map,
screen,settings,navigation,main}.rs`, `crates/ffe-launcher/src/main.rs`,
`docs/notes/{galaxy-map,hyperspace,enhancements}.md`, `docs/DEVIATIONS.md`,
`docs/CONTROLS.md`, `dashboard/burndown.js`, `docs/README.md`.

**Smallest verification:** `cargo test -p ffe-rs --lib galaxy_map` and
`cargo test -p ffe-rs --test galaxy_map_view -- --ignored` (needs the world cache).

**Status:** owner decisions in (bugs first; enhanced map = toggle replacement,
2D with a depth-size cue, "go wild on usefulness"; `H` stays the jump key, `Home`/
`C` recentre, `X` adds route waypoints). Bug 1 **fixed** 2026-11-23 (arbitrary
in-range jumps generate their system on demand, save/load included; the earlier
"Unexplored" label was wrong and reverted). `docs/notes/adding-enhancements.md`
written and linked. **Enhanced map (`enhancement-12`) delivered 2026-11-23**: the
`enhanced_map` module (flat top-down projection, depth-sized dots, selection,
route legs, Sol pointer, range radius), `Screen::draw_enhanced_galaxy`, the
`main.rs` open/input/draw/route wiring, the `~5s` "Preparing hyperdrive…"
multi-step `X`→`H` jump, middle-wheel zoom (added `session::Frame::wheel`),
right-drag pan, `Home`/`C` recentre and hover station/port notes. Toggle wired in
`settings.rs` + launcher "Galaxy map" group. Evidence: 329 lib tests (4 new
`enhanced_map`), 8/8 `galaxy_map_view` cache-gated tests (new
`the_enhanced_map_plots_stars_and_routes`), 36/36 `navigation`, `cargo fmt
--check` and both bins clean. **Bug 2 fixed 2026-11-23**: the 3D map's persisted
camera rotation was disorienting; on reopen its `spin` is levelled (centre, zoom
and selection still persist, so the map does not "rotate around" on return).
CONTROLS.md keys and the burndown/dashboard entry for `enhancement-12` are done.
**Map UI polish + edge indicator 2026-11-23**: dots are coloured by allegiance
(`enhanced_map::system_marker`: Federation blue, Empire red, Alliance green,
Independent amber; uninhabited white), inhabited + selected systems are name-
labelled, the selected star shows its stats (faction · population · F869
description), the Sol distance text is gone (arrow stays), the route flags a
stranded waypoint with an amber warning triangle (uninhabited, no fuel to
leave), `X` on a waypoint removes it and everything after
(`remove_waypoint_from`), and the enhancement-11 edge arrow is now a small
triangle. Evidence: 330 lib tests (targeter 14 green), 8/8 `galaxy_map_view`.

**Controls audit + fix (2026-11-24, player report).** Classic routing verified
correct against the original (18 `input` unit tests + 3 cache-gated `controls`
integration tests pass). Two issues fixed: (1) the docs said Modern was the
default while the code/launcher default to Classic — `docs/CONTROLS.md` and
`CLAUDE.MD` now say **Classic (default)**, and the stale `controls.cfg` mention
was corrected to `config.cfg`; (2) Classic mouse steering now hides the cursor
and re-centres it each held frame (`GameWindow::set_cursor_visibility` +
`warp_cursor_to_center`, Windows `SetCursorPos`), so it no longer stops at the
window edge like the original's relative mickeys. 330 lib tests, `cargo fmt
--check`, bin check clean.

**Next (owner report 2026-11-23, not started):**
- *Mars/Sol takeoff*: `L` on Mars (Sol) does not take off — the surface-launch
  path (`begin_surface_launch` → `begin_takeoff`) reads correct; needs a repro
  to find why the landed state is not launching there.
- *Police never appear/attack* after offences — investigate F238/F239 police
  spawn (`Navigator::spawn_police`, `police_launches`).
- *Enhanced laser mode* (new toggle): first-person views (front/rear/turret)
  draw the laser as a fixed line from the bottom to mid-reticle (Saker look);
  third-person unchanged.
- *Lion transport* third-person laser comes from mid-ship — gun-mount origin bug.

## Closed 2026-11-21 — `release-1` full playthrough checkpoints (accepted ~90%)

**Delivered.** One checkpoint per playthrough stage, chained by
`tools/reference/verify-playthrough.py`; all nine stages pass (~70 s). The stage
inventory is `docs/notes/playthrough.md`. Three stages had no executed-original
trace and now do:

- `--trade 220 --start-key=0x04`: the real F555/F556 on the Lave docked game,
  fixed market entries, cash, hold and stock; ten cases including all five
  `0x98e1..0x98e5` refusals. `crates/ffe-rs/tests/market_trade.rs` replays them
  through `Market::{buy,sell}`; every cash, cargo, used, stock and code matches.
- `--save-roundtrip 260 --start-key=0x04`: F214/F170/F174 write a commander file,
  every saved field is changed, F176/F171 restore it. `save_roundtrip.rs` imports
  the file with `saves::legacy_import`: name, cash, hold, fuel, clock, pair,
  player object, ship and all 38 cargo words match.
- `--jump 2000 --start-key=0x39`: F908/F901/F909 from Gateway to Titican (442
  units, 189,322 time units, 1 t, 2.89 days). `jump_reference.rs` matches the
  plan, transit and the 7.4..11.3 AU arrival through `Navigator::hyperspace`.

**Evidence.** Rebuilt `build-smoke.ps1`; `verify-trade.py`,
`verify-save-roundtrip.py` and `verify-jump.py` pass (each runs the reference
twice, the test, a tampered trace and an empty trace);
`verify-playthrough.py --cache .cache/ui11/data.bin` prints 9 passed, 0 skipped,
0 failed; `cargo test -p ffe-rs --test {market_trade,save_roundtrip,jump_reference}
-- --ignored` passes; `cargo test -p ffe-core` and `cargo test -p ffe-rs --lib`
pass; `python ci/check_burndown.py` reports 242 items, 241 done, 1 open.

**Accepted gaps** (`docs/ACCEPTED_PARITY.md`): no executed in-play mission chain
(the station side is `--daily` plus the ten `mission_scripts` cases); F909's
arrival draw ledger (arrival checked on the F1507 7.4..11.3 AU orbit, not the
pair after); the galaxy map's destination cursor (the harness sets the selected
destination and answers F34/F35); a JJFFE-written save's string-section
`commander_name` is two bytes short in the importer (the player block name is
exact).

**Files.** `tools/reference/{headless.c,exports.inc,README.md,verify-all.py,
verify-playthrough.py,verify-trade.py,verify-save-roundtrip.py,verify-jump.py}`,
`crates/ffe-rs/tests/{market_trade,save_roundtrip,jump_reference}.rs`,
`docs/notes/{playthrough,hyperspace}.md`,
`docs/{ACCEPTED_PARITY,BURNDOWN,STATUS,PORT_LOG,WORK_HANDOFF}.md`,
`docs/README.md`, `dashboard/burndown.js`. Dashboard 242 total, 241 done, 1 open
(`release-7`, the owner's sign-off).

## Closed 2026-11-21 — `gameplay-8`, `parity-20`, `parity-22` (accepted ~90%)

**Delivered.** (1) `--combat <frames>` (`--start-key=0x39`): the player is put
2 km behind the Hope pirate leader (the harness default is 20 km;
`FFE_COMBAT_OFFSET`), the original
F750/F754 AI and F931 run, and one JSONL row per frame carries every hostile
ship's whole 0x152-byte record, the player and the effect objects.
`crates/ffe-core/tests/combat_replay.rs` checks the state changes/fire/damage
and replays F750's guidance + F727 physics: 398/399 thrust words, 399/399 caps
and positions, 396/399 velocities exact. (2) `--daily <frames>`
(`--start-key=0x02`): the Ross 154 pad at 10000x; frame 406 makes F633's 125
F922/F1530 draws (first twelve limits `[25,0,25,0,8,3,12,12,1,0,14,25]`, 105
F1530 steps). `crates/ffe-rs/tests/daily_reference.rs` replays every pair step
and `Navigator::tick_days` ends on the same pair
`1273017021/228009932 -> 1772616542/3616182582`, with the board/job state
changed. (3) `--missile-encounter hit|ecm|naval|mine|escape`: five whole
encounters; `crates/ffe-rs/tests/missile_encounter.rs` checks the model-6 hit
and F719, standard-vs-naval ECM (models 6/7), the 0xc1 model-5 state-0x14 mine
and F1098's model-16 Stowmaster with the old state-0x16 hull.
`Navigator::tick_missiles_inner` now spawns F719 on a hit/decoy/timeout and
F942/F926 on a missile-killed NPC.

**Evidence.** Rebuilt `build-smoke.ps1`; the three verifiers pass
(`verify-combat-replay.py`, `verify-daily-replay.py`,
`verify-missile-replay.py`; each reruns the reference twice), and all three new
tests pass with their traces and `.cache/ui11/data.bin`. `cargo test -p
ffe-core` 331 lib + every target; `cargo test -p ffe-rs --lib` 325 passed/34
ignored; `cargo test -p ffe-rs --bin ffe-rs` 3 passed; `cargo check -p ffe-rs
--bin ffe-rs`; rustfmt and clippy clean on the changed files (the crate-wide
clippy still trips other streams' `combat_computer.rs`/`outfitting.rs` test
regions and `screen.rs:3209`).

**Accepted gaps** (`docs/ACCEPTED_PARITY.md`): F754 state-1/2 combat pass;
EXP-006 per-station market streams; whole-encounter NPC-vs-NPC missile chains
and the legal witness trace; the rotating-frame rendezvous as one scene; no
pixel comparison of the F719 model.

**Files.** `tools/reference/{headless.c,exports.inc,README.md,verify-all.py,
verify-combat-replay.py,verify-daily-replay.py,verify-missile-replay.py}`,
`crates/ffe-core/tests/combat_replay.rs`,
`crates/ffe-rs/tests/{daily_reference.rs,missile_encounter.rs}`,
`crates/ffe-rs/src/navigation.rs`, `docs/audits/gameplay-parity-work-items.json`,
`docs/{ACCEPTED_PARITY,BURNDOWN,STATUS,WORK_HANDOFF}.md`,
`dashboard/burndown.js`, `docs/notes/{combat,time}.md`. Dashboard 242 total,
240 done, 2 open (`release-1`, `release-7`).

## Closed 2026-11-21 — `v0.48-14` whole-frame scene oracle, `persist-4` legacy commander replay

**Delivered.** (1) `tools/reference/headless.c --scene-frame` (and `exports.inc`):
a fixed station (model 74) and three ships (31, 23, 25) at fixed positions drawn
by the executed original's F1537/F1680/F1772, F1330/F1331 palette and F1789
depth-sorted flush into the 320x200 framebuffer; `scene-frame.bin` plus one JSONL
row (camera, objects' matrix/position/scale/64 globals, tics, light, shades).
`tools/reference/verify-frame.py` + `crates/ffe-rs/tests/scene_frame.rs` rebuild
the scene through `setup_object`/`ModelViewer` and compare every pixel:
63,946/64,000 exact; `Screen::scene_frame_clear`/`scene_frame_draw` are the new
test-support rasteriser entry points. (2) `--legacy-save <path>`: the real
F70/F66/F171 on `game/FIRSTENC._01`, one JSONL row with 14 commander fields and
the raw object array (0x984a) and player block; `verify-legacy-save.py` +
`tests/legacy_save.rs` match `saves::legacy_import` field for field and byte for
byte.

**Field map correction (`persist-1` gap closed).** The DOS player block is the
source `gmstruct` layout, not a +0x72-shifted one: `F170`'s scatter is 0x200
bytes = 0x40 every-other dwords (the old seven-dword count shifted the block and
its text, the source of the `+0x22E` observation), and the DOS player section is
0x5535 bytes (JJFFE's 0x556a over-reads 0x35 bytes of the string section). Map:
clock 0/4, RNG 0x24/0x28, player object 0xC0, credits 0x114, hold 0x118/0x120,
fuel 0x15A/0x15C/0x16E, name 0x1BC/0x1D2. Recorded in
`docs/notes/persistence.md` 1.2.1.

**Evidence.** Rebuilt the reference (`build-smoke.ps1`, NASM at
`C:\Program Files (x86)\NASM\nasm.exe`); both new modes run; both verifiers pass
(`legacy-save` 14 fields + two raw blocks, tamper cases rejected; `scene-frame`
63,946/64,000, `accepted 64`, tamper cases rejected). `cargo test -p ffe-rs --lib`
325 passed/34 ignored; `cargo test -p ffe-core --lib` 331 passed/1 ignored;
`cargo test -p ffe-rs --lib saves::tests -- --ignored` 1 passed;
`cargo test -p ffe-rs --test legacy_save/scene_frame -- --ignored` 1 each;
`cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt -p ffe-rs -- --check` clean;
lib and new-test clippy clean.

**Accepted gaps** (`docs/ACCEPTED_PARITY.md`): `v0.48-14`'s 54 pixels (the
unported F1747 name-board text and a two-pixel station-edge depth tie; synthetic
view, no cockpit/planet); `persist-4`'s read-only import and the DOS
0x5535/0x556a over-read. Dashboard 242 total, 237 done, 5 open.

**Files.** `tools/reference/{headless.c,exports.inc,verify-frame.py,
verify-legacy-save.py,verify-all.py,README.md}`,
`crates/ffe-rs/src/{saves.rs,screen.rs,model_viewer.rs}`,
`crates/ffe-rs/tests/{legacy_save.rs,scene_frame.rs}`,
`docs/notes/persistence.md`, `docs/{ACCEPTED_PARITY,BURNDOWN,STATUS,PORT_LOG,
WORK_HANDOFF}.md`, `dashboard/burndown.js`.

## Closed 2026-11-20 — `v0.48-8` planet-scale surface quad oracle (accepted ~90%)

**Delivered:** the planet-scale half of F1876's executed-original comparison is its
own gate. `surface_draw_reference::executed_reference_planet_surface_draw` runs the
saved `--surface-draw` table (`.cache/parity07/surface-draw-cases.jsonl`, 56 cases:
4 near-pad + 52 planet-scale) and requires 40 cases beyond 1.4 radii, so the
near-pad file alone cannot pass it; `tools/reference/verify-surface-planet.py`
checks the table's shape and runs it. On the local reference binary,
`--surface-near 0..3` reproduced `surface-near.jsonl` byte for byte.

**Evidence:** `python tools/reference/verify-surface-planet.py --vectors
.cache/parity07/surface-draw-cases.jsonl --cache .cache/parity07/data.bin` prints
`{"scenario":"planet-scale-surface-draw-v1","cases":56,"planet_scale_cases":47,...}`
and the test passes: 54 of 54 completing cases, 21,380 triangles, 54 queues
(369,811 words) word for word, 2 queue overflows, 47 beyond 1.4 radii (median 4.7,
up to about 70). The 4-case near file passes `executed_reference_near_surface_draw`
(168 triangles) and fails the planet test at its 40-case gate, as intended.

**Accepted gap:** the full 512-case `--surface-draw` table forks a child per case
and uses `open_memstream`, so it is Linux-only (Windows refuses `--surface-draw
CASE`); its 465-queue/207,689-triangle result is the recorded Linux run, not
re-run here.

**Files:** `crates/ffe-rs/tests/surface_draw_reference.rs`,
`tools/reference/verify-surface-planet.py`, `docs/notes/terrain.md`,
`docs/{ACCEPTED_PARITY,BURNDOWN,STATUS,PORT_LOG}.md`, `dashboard/burndown.js`.

**Remaining:** dashboard 242 total, 235 done, 7 open.

## Closed 2026-11-20 — `gameplay-6`, `gameplay-7`, `gameplay-18` (accepted ~90%)

**Delivered:** F659's launch tables (`police::launch_offset`, `DATA_006202`..`006209`,
used by `Navigator::make_police`); F233's event 1 (`Navigator::jump` clears wanted/alert
via `Legal::clear_alert`, F909's arrival broadcast); F770's trading approach
(`State::Trading` gives the 0x09 approach its own motion and no early fire; the
existing `scripts::approaches` keeps event 0x17 and the 0x98d0 hail); the missile/object rows
0x13-0x1b (`missile::AI_ROWS`, row-driven 0x13 guidance/lifetime, 0x14 contact-only
mine, 0x16 F767/F713 seeker); ground collisions (`NpcShip::hits_ground`,
`keep_ships_out_of_worlds` destroys an attacker as state 0x1c with F942's explosion).
A shared-checkout repair: `Navigator::station_record` falls back to `traffic` when
there is no physics list, which had broken `station_traffic` after the object-record
pass.

**Accepted gaps** (in `docs/ACCEPTED_PARITY.md`): F702's system-wide orbit, F597's
accelerated placement (`parity-11`), F589/F671's orientation/pad settle, EXP-023's
beam hit box, F770's approximate approach motion, F941's missile selection.

**Files:** `crates/ffe-core/src/{npc,police,missile}.rs`,
`crates/ffe-rs/src/navigation.rs`, `crates/ffe-rs/src/navigation_save_tests.rs`,
`docs/notes/{npc,legal,rank-and-fines,combat}.md`, `docs/{ACCEPTED_PARITY,BURNDOWN,
STATUS,DEVIATIONS,PORT_LOG,WORK_HANDOFF}.md`, `dashboard/burndown.js`.

**Evidence:** `cargo test -p ffe-core` 331 lib + every target; `cargo test -p ffe-rs
--lib` 325 passed/34 ignored; cache-backed `npc_ai` 8, `npc_fire_view` 1, `npc_view` 3,
`station_traffic` 1, `save_tests::f233s_arrival_event_clears_the_wanted_and_alert_bits`
and `save_tests::an_attacker_that_hits_a_world_is_destroyed_as_state_1c`;
`cargo check -p ffe-rs --bin ffe-rs`. Dashboard 242 total, 233 done, 9 open.

## Active — `flight-1` third pass: smoke, avoidance, FOR, carried jump, orbital record (2026-11-20)

**Contract:** the tractable remainders of `flight-1`, without a `main` flight-state
change: (1) F939's per-frame smoke particle from F725/F727/F729, drawing the
shared F1530 pair (a deliberate RNG-ledger change); (2) F802/F610's avoidance
steering at the `+0x150` cadence; (3) F606's frame-of-reference selection made
non-no-op for NPC records; (4) F680's `DATA_008916` to `+0xf2` and F682's second
F901 with the player's hyperspace distance; (5) F761's failed-last-attempt
`+0xf2` quirk; (6) a live caller of `OrbitalOverlay` through the system bodies'
object records; (7) the player record's `+0xff` and `+0x10e` from
`Navigator`'s autopilot/landed state (a `main`/`Flight` feed stays a follow-up).

**Acceptance:** each behaviour has a unit test; `cargo test -p ffe-core` and
`cargo test -p ffe-rs --lib` pass; the bin checks; rustfmt and clippy clean on
the changed files. Do not close `flight-1`.

**Delivered:** `NpcShip::smoke_spark` (F939 from the lent F1530 pair) pushed by
`tick_ships`; `npc::obstruction_along` + `NpcShip::steer_avoid` (F610/F802,
station/leader/anchor filtered out); `NpcShip::{take_frame_update,
update_frame_of_reference}` with the pure `frames::select_frame_of_reference`
(F606), reported through the record's `+0x56`/`+0x3e`; `spawn::jump_words_attempt`
and F761's short-circuit/stale-`+0xf2` quirk; `npc::carried_jump` (F680's
`DATA_008916`, F682's second F901 with the player's distance, F940 drop);
`Navigator::system_with_list`/`body_object_record`/`station_record` (the live
F880 `PhysList` + `OrbitalOverlay`, used by the NPC station cycle and
`object_records`); `player_object_record`'s `+0xff`/`+0x10e`.

**Evidence:** `cargo test -p ffe-core` 331 lib passed/1 ignored and every
integration target; `cargo test -p ffe-rs --lib` 325 passed/34 ignored; with
`.cache/parity07/data.bin` `save_tests -- --ignored` 19 (one new body-record
case), `npc_docking` 1, `station_traffic` 1, `npc_ai` 8, `npc_missiles` 1,
`npc_fire_view` 1, `ecm_view` 1, `npc_view` 3, `thargoid_view` 1,
`effects_view` 13; `cargo check -p ffe-rs --bin ffe-rs`; core clippy
`--lib --tests` clean; `ffe-rs --lib` clippy clean. `cargo fmt --check` still
lists only other streams' regions (`ffe-core/missile.rs`, `npc.rs`'s
`State::Trading`, `police.rs`, `navigation_save_tests.rs:528`).

**Remaining for `flight-1`:** NPC/chaff/player frames still draw from port
structs (only missiles read records); the renderer still reads `Body`/
`Starport` rather than `body_object_record`; the player `+0xff`/`+0x10e` are
derived from `Navigator`'s state, not `main`'s `Flight`; F761's first-failed
case with no words falls back to local trade (port traders do not keep their
spawn F776 pick); F802's scan is a centre/ray test over bodies and ships, not
F817's shape collision.

**Files:** `crates/ffe-core/src/{npc,frames,spawn}.rs`,
`crates/ffe-rs/src/{navigation,missile}.rs`,
`crates/ffe-rs/src/navigation_save_tests.rs`, `docs/notes/{object-system,npc}.md`.

**Smallest verification:** `cargo test -p ffe-core npc frames spawn` and
`cargo test -p ffe-rs --lib navigation`.

## Active — `flight-1` second pass: jump cycle, agent timeouts, orbital overlay (2026-11-20)

**Contract:** F738/F682's `+0xa8`/`+0xac` jump clock, F761's departure jump and
a carried ship's `+0xf2`/`+0xf8`; the `+0x14d`/`+0x14e`/`+0x150` behaviours;
the `+0xb0..` orbital/starport overlay as a typed record view; chaff slots and a
record-driven frame; the player's `+0xfe`/`+0x100`. Core plus additive
`navigation.rs`, no `main` flight-state change. Do not close `flight-1`.

**Delivered:** `npc::State::Jump` (0x11/0x12), `NpcShip::{jump_clock,
timer_clock, begin_jump, jump_out, smoking}`, F953's smoke on hull damage,
F732/F938's one-second pass, F610's avoidance cadence, `Persisted.jump` so a
carried pirate keeps `+0xf2` and returns in `+0xf8`. `object_record`: decoy
slots/`decoy_record`, `OrbitalOverlay` (`+0xb0..+0xe0`),
`ObjectRecord::{orbit,set_orbit,axes,set_axes}`, `starport_from_bytes`/
`set_starport_bytes`. `navigation`: `sync_object_slots` syncs chaff,
`object_slot_of_target` resolves a decoy slot, `missile_object_records` drives
`main`'s missile draw, `player_object_record` fills `+0xfe`/`+0x100`,
`begin_jump` on the flee frame and F761 in `ChooseStation`.

**Files:** `crates/ffe-core/src/{npc,object_record}.rs`,
`crates/ffe-rs/src/{navigation,main,missile}.rs`,
`crates/ffe-rs/src/navigation_save_tests.rs`, `docs/notes/{object-system,npc}.md`.

**Evidence:** `cargo test -p ffe-core` 316 lib + every target;
`cargo test -p ffe-rs --lib` 324 passed/31 ignored; cache-backed
`save_tests -- --ignored` 16 (two new), `npc_docking` 1, `station_traffic` 1,
`npc_ai` 8, `npc_missiles` 1, `npc_view` 3, `station_view` 15,
`thargoid_view` 1, `ecm_view` 1, `effects_view` 13, `npc_fire_view` 1;
`cargo check -p ffe-rs --bin ffe-rs`; rustfmt clean; core and `ffe-rs --lib`
clippy clean (`-A clippy::unusual_byte_groupings` / `unnecessary-cast`).

**Checkout note:** the tree did not compile before this pass: the other
stream's `bulletin_table.rs` declared 164 strings for 165 entries and their
`bulletin.rs`/`missions.rs` tests called `military` with two arguments and built
`Post` without `date`. Minimal test-only repairs were applied, and
`cargo fmt -p ffe-core -p ffe-rs` formatted their in-progress regions too.

**Remaining for `flight-1`:** the F939 smoke particle (shared-RNG ledger),
F802's avoidance steering, F606's real frame-of-reference update, F682's second
F901 call/`DATA_008916` (the port keeps the ship's F776 words), NPC/chaff
frames still drawn from port structs, the player's `+0xff`/`+0x10e`, and no
caller of `OrbitalOverlay` yet.

## 2026-11-20 — `gameplay-12` civilian removal conversation (F460), F503/F470

**Delivered:** a removal advertisement (0x981f..0x9822) now opens F460's
conversation instead of accepting directly. `missions::Briefing` serves it
(nothing new in `station.rs` beyond the back target): F331's 0x98d8 "Video
Link" title, F352's 0x99c4 description, the 0x9861..0x9864 + 0x983e/0x983f
rows (`REMOVAL_CODES`), F467's 0x99ca accept, F535's ad clear and F527's two
hang-ups (`BriefingStep::HangUp`). F460's gate is ported as
`missions::removal_rating`/`navigation::removal_briefing`: `(seed << 14) >>
16 + 0x1000` against `career.elite`, or no F466 destination place, gives F459's
0x99c5 page with only "Hang up.". The page uses F643's `Post.date`.

**Fidelity tightened:** `navigation::surface_place` is F503 (a destination
body with model 0x77..0x7a, first or last by `seed & 3`) for F496's
photograph/bombing; F466's `destination_place` is kept for F506/F460 packages.
`Navigator::system_entry_for` builds a destination outside the current
neighbourhood list (military strikes are picked around the enemy capital), so
those pages now generate the real destination instead of falling back to the
name. `npc::model_for_fraction` gives F470 the exact F658/F635 kind 0x10 ship
from the offer's seed (F460/F506's `DATA_009066`).

**Evidence:** `cargo test -p ffe-core --lib` 316 passed/1 ignored (every
integration target passed); `cargo test -p ffe-rs --lib` 324 passed/31 ignored;
cache-backed `cargo test -p ffe-rs --test missions -- --ignored` 7 passed (new
`a_removal_advert_opens_f460s_conversation` and
`a_photograph_briefing_uses_f503s_surface_record`); `cargo check -p ffe-rs
--bin ffe-rs`; `cargo fmt --check` clean; core and `ffe-rs --lib` clippy
clean. New core tests:
`f460s_removal_conversation_opens_its_six_rows_and_accepts`,
`f635s_fixed_fraction_indexes_the_kind_0x10_target_table`.

**Remaining for `gameplay-12`:** the port records the removal as F290's
20-byte contract with the `MAX_MISSIONS` cap instead of F467's 52-byte
six-entry list, and F291 still spawns the target at the place nearest the
player rather than F466's chosen starport. No matched whole-frame/state
comparison of the message pages. This chunk does not close the item by itself;
with the earlier military briefing pass it leaves `gameplay-12` at the
accepted ~90%.

## Closed 2026-11-20 — dashboard `gameplay-9` (BBS and missions)

**Delivered:** the four recorded remainders. (1) F460's removal conversation is
the `gameplay-12` section above (0x99c4/0x99c5, 0x9861..0x9864 + 0x983e/0x983f,
F467's 0x99ca, F535, F643's `Post.date`). (2) F291's non-player branch:
`scripts::Action::Jettison` now carries the anchor object, so statement 58 on
the player still spends one unit of `DATA_008896[mission]` and posts 0x9c2f,
while any other anchor emits F928(0x99) in F926's drifting state 0x16 just
behind it (the anchor's collision radius), carrying the anchor's velocity, with
the source's `+0x9e = DATA_005112[mission]` canister code. (3) F329's dynamic
row height: the entry wraps at `0x140 - 3 - x`; the next row advances by
`max(pitch, 10 * lines)`, and the same heights drive the drawing, the scroll
window and the button hot areas (`station::{wrap_words,row_heights,visible_rows,
click_with_heights}`, `DockState::step_visible`). (4) F642/F633's `+0x131`
service flags: a board holds at most one Federal (0x9827) and one Imperial
(0x9828) recruitment notice; F633 clears the replaced notice's bit before F621
draws (`bulletin::{FEDERAL_NOTICE,IMPERIAL_NOTICE,notice_flag}`; `generate` and
`daily` pass the board without the replaced entry).

**Files:** `crates/ffe-core/src/{bulletin,bulletin_table}.rs`,
`crates/ffe-rs/src/{scripts,station,screen,main}.rs`, `docs/notes/{missions,
station}.md`, `docs/BURNDOWN.md`, `dashboard/burndown.js`, `docs/STATUS.md`.

**Evidence:** `cargo test -p ffe-core` 316 lib + every integration target;
`cargo test -p ffe-rs --lib` 324 passed/31 ignored (new
`a_non_player_object_jettisons_its_own_canister`,
`a_wrapped_ten_pixel_row_moves_the_rows_below_it`); cache-backed
`bbs_reference` 3, `missions` 7, `mission_scripts` 10, `station_view` 15;
`cargo check -p ffe-rs --bin ffe-rs` and 3 bin tests; `rustfmt --check` clean on
the changed files (the checkout keeps other streams' unformatted `npc.rs`,
`object_record.rs`, `navigation_save_tests.rs`); core and `ffe-rs --lib` clippy
clean. New core tests for the notice flags, the daily flag clear, F643's date
and the F291/F329 geometry are in the same run.

**Recorded gaps:** F467's 52-byte six-entry removal contract (the port keeps
F290's contract under `MAX_MISSIONS`); F291's live NPC canister has an
interpreter test, not a cache-backed flight scene; and the F521/F460 pages have
no matched whole-frame capture. Dashboard 242 total, 230 done, 12 open
(`gameplay-9` and `gameplay-12` closed).

## Fixed 2026-11-20 — owner bug: `M` could not fire a missile

**Symptom:** every `M` launch detonated on the player instead of flying; with
two pylons the player took two 200-damage hits and saw no missile.

**Cause:** the live missile tick passed `ignore_launcher: false` whenever
`enhancement.missile_launch` was off (the default), and `Missile::contact_point`
honoured that flag. F941 separates the missile by the launcher's `+0x140`
collision radius (4,000 mm for the Saker) plus `0x2000` (9,116 mm total), while
`main` reports the player object with F1686's cull radius (17,600 mm); F718's
point contact test therefore hit the player on the first step. F575 skips the
missile's parent (`+0x56`), which the port stores as `owner`, so the skip is the
original behaviour — the enhancement had inverted it.

**Delivered:** `contact_point` always skips `owner` (source F575 parent skip;
this is now the off/original path). `TickOptions::ignore_launcher`
(`enhancement-2`) now guards only the extra `enhancement-6` swept ray, which is
the only test that can still meet the launcher. No change to `main.rs`,
`navigation.rs` or the launch impulse.

**Evidence:** `cargo test -p ffe-core` 309 lib + every integration target;
`cargo test -p ffe-rs --lib` 323 passed/29 ignored; new synthetic
`crates/ffe-rs/tests/player_missile.rs` 2 passed (M launch survives its launch
step; the ray toggle still distinguishes); cache-backed `missile_view`,
`npc_missiles`, `ecm_view` pass with `.cache/parity07/data.bin`;
`cargo check -p ffe-rs --bin ffe-rs`; rustfmt clean on the three changed Rust
files; core clippy `-D warnings` and `ffe-rs --lib` clippy
(`-A clippy::unnecessary-cast`) clean. Docs: `DEVIATIONS` EXP-036,
`notes/enhancements.md`, `notes/combat.md` §8.

## Active contract — `gameplay-9` closing pass (F460, F291, F329, `+0x131` flags)

**Gap:** the four recorded remainders. F460 (`ffe.asm:31896`): a civilian removal
advert (`0x981f..0x9822`) must print 0x99c4, refuse with 0x99c5 when the advert
seed's rating exceeds `DATA_008911` (or F466 finds no place), and offer
0x9861..0x9864 + 0x983e/0x983f (F467/F469/F470/F468); F467's accept prints
0x99ca and clears the advert. F291 (`ffe.asm:22692`): statement 58's non-player
branch spawns the jettisoned canister behind the anchor object, name
`0x8e00 + DATA_005112[mission]`, state 0x16. F329: a row's next top is
`max(pitch, 10 * wrapped lines)` and the hit areas follow. F642/F633: `+0x131`
bits 0x4/0x8 hold at most one Federal/Imperial recruitment notice; F633 clears
the replaced notice's bit before generating.

**Acceptance:** the removal advert opens its conversation page (not an accept),
the refusal/ship/time pages render; statement 58 emits a canister for an NPC
anchor and still only for a stock the player carries; a wrapped 10-pixel row
draws every line and the following rows/buttons move; a board never holds two
0x9827 or two 0x9828 notices and a replacement may re-add one. Regression in
`cargo test -p ffe-core` and `cargo test -p ffe-rs --lib`. Do not close on this
chunk without the verification gate.

**Files:** `crates/ffe-core/src/{bulletin,bulletin_table,missions}.rs`,
`crates/ffe-rs/src/{navigation,station,screen,main,scripts}.rs`.

**Smallest verification:** `cargo test -p ffe-core missions` and
`cargo test -p ffe-rs --lib scripts`.

## Closed 2026-11-20 — dashboard `gameplay-41` (military script rows, accepted ~90%)

**Delivered:** F326's screen 7 (F493's military office) now keeps its rows in
the scripts' station menu. Entering the page seeds the menu with the station's
offers as native entries (`native_entry` over the regenerated `DATA_009004`),
then pulses global 0 with 3; the scripts' F346 inserts land in that menu.
Mission 23's state 0 answers with `24 0xa9b9`, so the list prints the hidden
mission among the offers (F346 moves the row it displaces to the end), and
selecting it runs F360 and starts mission 25, whose F331 opens the mission's
conversation. The list (and the board) draws through the shared
`station::menu_rows` order model; native rows keep F493/F509's briefing path.

**Evidence:** `cargo test -p ffe-rs --lib` 323 passed/29 ignored (new
`station::f326s_rows_keep_the_scripts_entries_in_the_menus_order`);
cache-backed `--test mission_scripts -- --ignored` 10 passed (the extended
`the_other_station_screens_signal_the_scripts` starts mission 23 with a future
deadline, asserts the 0xa9b9 row after screen 7's signal 3, selects it and
asserts mission 25's 0x98d8 page with 0xa9bb and 0x983f);
`--test station_view -- --ignored` 15 passed; `cargo check -p ffe-rs --bin
ffe-rs`; rustfmt clean on the changed files; core and `ffe-rs --lib` clippy
clean. The core script/mission suites pass; the latest full `cargo test -p
ffe-core` is blocked only by another stream's new
`bulletin::tests::the_daily_replacement_clears_the_replaced_notices_flag`
failure in their uncommitted `bulletin.rs` (315 others pass). The bin clippy
keeps the pre-existing `main.rs:3425` redundant-closure and `:9679`
type-complexity lints; crate-wide `cargo fmt --check` still trips the other
streams' `missions.rs`, `npc.rs`, `tests/missions.rs` and
`tests/player_missile.rs`.

**Recorded accepted gaps:** F669's event 0x17 is raised after the run rather
than mid-statement (deliberate architecture change); the attackers'
time-accelerated closing (F597's branch) is `parity-11`; no headless in-play
playthrough check. Dashboard 242 total, 228 done, 14 open.

## Closed 2026-11-20 — dashboard `ui-11` (far/near transitions, clipping, planet approach)

**Gap found:** F1680/F1686 decided far class, dot class and the sphere cull
only for the planet discs (`Screen::draw_discs`). F1536/F1537's source loop sends
*every* object through F1680, but the port drew NPC ships and missiles as models
at any distance within the port-only `STATION_DRAW_RANGE`, and stations likewise,
so distant ships (sub-pixel geometry, often invisible) never became F1700's
glints and objects wholly outside the view still ran their models.

**Delivered:** `Screen::f1680_draws_model` is the shared decision (Far/Culled →
not drawn; Dot → F1700 glint via `object_dot_frame`; Model → the existing path).
Wired into `draw_system_ships` (ships and missiles; dead ships stay with F942's
effect) and `draw_system_station`. The unported F1700 lit shaded sphere is
unreachable for system bodies (largest dot 7 px over the 58 bodies of starts 0–3).

**Changed files:** `crates/ffe-rs/src/screen.rs` (helper + two call sites; two
`f1680_transition_tests`), `crates/ffe-rs/tests/far_objects.rs` (two new local
cases), `docs/NAVIGATION.md`, `docs/notes/animation.md`,
`docs/ACCEPTED_PARITY.md`, `docs/BURNDOWN.md`, `docs/STATUS.md`,
`dashboard/burndown.js`, this file, `docs/PORT_LOG.md`.

**Evidence:** `cargo test -p ffe-rs --lib` 321 passed/29 ignored;
`cargo test -p ffe-rs --test far_objects -- --ignored` 4 passed (station
2 km/600 km/140,000 km = 14,954/17/0 px; ship 9 km/5,000 km = 21/0 px; 58 bodies,
largest dot 7 px) with cache v35; cache-backed `planet_view` 26, `station_view` 15,
`npc_view` 3, `npc_ai` 8, `effects_view` 13, `ship_view`, `station_traffic`,
`npc_docking` all pass; `cargo check -p ffe-rs --bin ffe-rs`; rustfmt clean on
both files; `ffe-rs --lib` clippy clean; `--test far_objects` clippy clean with
`-A clippy::unusual_byte_groupings` (the other streams' `ffe-core/missions.rs`
lint) and `-A unreachable_patterns` (their in-progress `station.rs`).

**Cache note:** the documented `.cache/parity07/data.bin` is format v34 and has
no `base:world/object-dots`, so `far_objects` and the far/dot path cannot run on
it. Rebuilt from `game/` into `.cache/ui11/data.bin` (v35) with
`cargo run -p ffe-tools -- build-data -- game --out .cache/ui11/data.bin`.
Uncommitted work in `main.rs`/`station.rs` broke the bin target twice mid-session;
verification used the real tree once it compiled again.

**Remaining (recorded in the closure):** F1680's rotating-frame branch
(`world-13`, read-only, picture/key-equivalent), the textured surface fills and
the missing matched whole-frame approach capture. `ui-11` is done; no dashboard
totals other than remaining 18 of 242 changed.

## 2026-11-20 — `gameplay-12` military briefing (F508/F495/F496/F506)

**Delivered:** selecting a military mission row opens the offer's briefing
instead of accepting it. `missions::Briefing` holds F352's description, the
F508/F496/F506 response rows (`0x9858..0x9868`) and the
F469/F470/F468/F499-F502 answer pages (`0x990c..0x9911`, `0x9958..0x995b`,
`0x99c6..0x99c8`); `station::Screen::Briefing` draws them on F523's screen 1
and `navigation::military_briefing` supplies F466's destination place, F470's
ship and F791 kind 0x12. The accept is the `0x9858`/`0x9861` row
(F509/F498/F467): F498 fits the Military Cameras for a photograph and a
nuclear missile (0x89) in a free pylon for a bombing, printing 0x98f8 when no
pylon is free. Strike acceptance writes 0x99ca; the package keeps 0x992a. New
source strings are in `bulletin_table::STRINGS`; `bulletin::expand` handles
0x3010..0x3012 (time) and 0x3016/0x3017 (variable as text).

**Verified:** `cargo test -p ffe-core` 301 lib + all targets; `cargo test -p
ffe-rs --lib` 321 passed/29 ignored; `cargo test -p ffe-rs --test missions` 4
passed/5 ignored, and with `.cache/parity07/data.bin` `--ignored` 4 passed
(new `a_military_offer_opens_its_briefing_before_it_is_accepted`);
`cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt --check` clean; core and
`ffe-rs --lib` clippy clean.

**Remaining for `gameplay-12`:** the removal-advertisement conversation
(F460, 0x99c4 and 0x9861..0x9864) still accepts directly, and there is no
matched whole-frame/source-state comparison of the message pages. Do not
close the item on this chunk.

## Active progress — `gameplay-9`: F521's reply page and F524-F532

**Contract:** selecting a passenger/parcel BBS advert must open F331's screen 1
(F521's `0x991d` line + F522's thirteen responses `0x9833..0x983f`) and run the
F524-F532 handlers (passage/danger answers, permits, ten per cent, advance
terms), instead of accepting the row directly. F291's assassin spawn only if
tractable; a regression test where possible. Do not close `gameplay-9`.

**Delivered 2026-11-20 (one deliberate edit to the live click/tick paths):**
`ffe_core::missions::Negotiation` (renamed from `Reply` to avoid the other
stream's new military `Reply` enum in the same file) stores F453's entry fields
plus `DATA_009059`/`DATA_009060`, and implements `codes`/`rows` (F522, skipping
0x9834 for a parcel), `intro`/`refusal` (0x991d/0x991e), `respond` (F533,
F524-F528, F530-F532, F527/F526), F921 (`missions::f921`, including the source's
`basic_data`/seed mutation), `acceptance` (F534's halves/all-now), and
`acceptance_text`/`acceptance_line` (0x992a/b/c). F529's three permit
destinations are `PERMIT_SYSTEMS` with `DATA_009091` bits 13..15. The page is
`station::Screen::Reply` (pitch 10, x 140, one button, header "Suggested
Responses", title keeps the board's name) with `DockState::reply`; `main` opens
it from the board, routes rows to `StationAction::Reply`, posts the console
lines, pays the advance into `state.credits`, awards the permit through
`Navigator::career.award_medal`, clears the advert (F535) and, after F534,
leaves only "Hang up.".

`bulletin_table.rs` gained 0x997e..0x99ad (F524/F525's 48 answers) via
`tools/gen-bulletin-table.py`; the generator was extended with that range.
`State::accept_page` lets F534 record 0x992b/0x992c instead of 0x992a.

**F291:** not implemented. The decoded source facts are in
`docs/notes/missions.md` item 2 (opcode 58 → `JUMP_000442` → F291's player/non-
player branches; the port only has the player/hold branch and always spawns
behind the player). This needs a deliberate change to `scripts::Action` and is
a separate chunk.

**Shared checkout warning:** another stream is editing `missions.rs`,
`station.rs`, `main.rs`, `bulletin_table.rs` and `navigation.rs` at the same
time. Their `Screen::Briefing`/`missions::Briefing`/`Action::MilitaryReply`
work landed during this pass; `main.rs` briefly had no `MilitaryReply` arm and
a temporary `dock_before = dock` move error from dropping `DockState: Copy`
(fixed with `dock.clone()`). Re-read before editing any of those files. Their
0x9858.. military strings were hand-added to `bulletin_table.rs` and are **not**
in `tools/gen-bulletin-table.py`'s code list; re-running the generator would
drop them (my 0x997e..0x99ad range is in the script).

**Verified:** `cargo test -p ffe-core` (301 lib passed, 1 ignored, every
integration target passed); `cargo test -p ffe-rs --lib` 321 passed/29 ignored
(new `the_reply_page_is_f331s_screen_one`,
`f521_offers_the_thirteen_suggested_responses`,
`the_money_and_danger_questions_read_the_additive_index`,
`the_half_now_terms_pay_half_and_leave_half_on_arrival`,
`the_permit_question_grants_the_source_permit_bit`,
`accepting_records_the_negotiated_contract` and five more);
`cargo test -p ffe-rs --bin ffe-rs` 3 passed; cache-backed
`cargo test -p ffe-rs --test missions -- --ignored` 5 passed (new
`a_generated_passenger_post_runs_f521s_reply_page`);
`cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt --check` clean;
`cargo clippy -p ffe-core --lib -- -D warnings` clean;
`cargo clippy -p ffe-rs --lib -- -D warnings -A clippy::unnecessary-cast`
clean; the bin clippy reports only the pre-existing `main.rs` redundant-closure
and type-complexity warnings.

**Remaining for `gameplay-9`:** F460's removal-advertisement conversation
(0x99c4, 0x9861..0x9864) still accepts directly; the 10-pixel reply rows draw
one line (F329's dynamic two-line height is not modelled); F291's non-player
branch. Do not close on this chunk.

## Fixed 2026-11-20 — effects regression: F768 pod count ran on `Navigator::clock`

**Symptom:** `effects_view::hit_particle_explosion_and_frames` failed ("the
explosion did not expire"): 10 s after the kill a debris pod still had
`+0x14e` at 6 and threw `Kind::Hit` sparks every frame.

**Cause:** `Navigator::tick_effects` passed `clock_words(self.clock - seconds)[0]`
to `Effects::tick_clock`. `self.clock` is advanced only by `Navigator::advance`
(world time), so every caller that ticks effects without it (the tests, and any
paused-clock path) saw zero F152 boundaries for ever; F768 never counted the pod
down. The explosion itself did expire (F708 reached zero), so `free_dead` was
not at fault, and F927's slot logic did not change the debris count.

**Fix:** `effects::Effects` now owns the F152 clock low word (`clock`), advanced
by the same game tics the sweep consumes; `tick` passes it to
`EffectTable::sweep` and `tick_clock` is gone. `tick_effects` just calls
`self.effects.tick(seconds, tics, player, &mut self.random)`.
`EffectTable::sweep(tics, Some(clock), ..)` stays for `death_replay`. New core
tests: `a_pod_counts_the_next_f152_boundary_not_a_second_after_its_birth`,
`a_pod_stops_sparking_after_seven_f152_boundaries`.

**Death test:** already passes in the current tree — the uncommitted `death.rs`
expects the pods after F942's two F1530 draws and the drifted pod positions; the
"3 vs 0" report was stale. No test weakening.

**Verified:** `cargo test -p ffe-core` 281 lib + every integration target;
`cargo test -p ffe-rs --lib` 317 passed/29 ignored; cache-backed
`effects_view -- --ignored` 13 passed, `death` 9 passed, `death_view -- --ignored`
3 passed; `cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt --check` clean; core
and `ffe-rs --lib` clippy clean. `ffe-core --test death_replay -- --ignored`
still needs `FFE_DEATH_TRACE_FILE` (NotPresent), unrelated.

## Active contract — `gameplay-10` F302/F301 overdue pass (2026-11-20)

**Gap:** `missions::tick` settles an overdue contract at its destination (the
port-only 0x9c2c, -1, contract retained). The source runs F301 first: mark every
contract past `+0xc` with `basic_data` bit 0x8000 and post 0x9c36 for a
passenger passage (not a package, and not a seed-0 military contract). F302's
next pass tests that bit **before** `DATA_008885 == +0x8`, so an overdue
contract fails wherever the player is: `-2`, 0x9c37, record removed, F148 bong.
The one exception is a seed-0 package (`basic_data & 0x2000`) at its
destination: 0x9c38, no pay, no reputation step (`ffe.asm:23967-24029`,
`24149-24200`, `24081-24147`).

**Acceptance:** tick fails an overdue contract at any system with 0x9c37 and
`-2`, after F301's prompt with 0x9c36; a delivered military package overdue
gets 0x9c38 and no pay; reputation stays signed; regression covers the
overdue-at-destination case.

**Files:** `crates/ffe-core/src/missions.rs`, `crates/ffe-rs/tests/missions.rs`,
`crates/ffe-rs/src/main.rs` (rank-song decision), `docs/notes/missions.md`.
**Smallest verification:** `cargo test -p ffe-core missions`.

**Delivered 2026-11-20:** `Mission::overdue_noticed` (serde default) is F301's
0x8000 mark. `State::tick` now runs F301 (mark; 0x9c36 for a passage) and then
F302 (`expire`: destination-independent `-2` and 0x9c37, contract no longer
active; a seed-0 package at its destination gets 0x9c38 with no pay or step)
before any F303 settlement. The pass covers F290's passenger-list kinds;
F300's removal list keeps its previous 0x9c29 approximation in
`expire_removal`. `main` chooses the rank song from the outcome's
`PageKind::Rank`, not the contract kind, so a failed military contract rings
F349's bell. `settle`'s 0x9c2c late branch is kept for a direct settlement and
re-cited to F300.

**Evidence:** `cargo test -p ffe-core` 301 lib passed/1 ignored and every
integration target; `cargo test -p ffe-rs --lib` 321 passed/29 ignored;
`cargo test -p ffe-rs --test missions` 4 passed/3 ignored and, with an absolute
`FFE_WORLD_CACHE`, `--test missions -- --ignored` 4 passed (the other stream's
new briefing case included); `cargo check -p ffe-rs --bin ffe-rs`; core and
`-p ffe-rs --lib --no-deps` clippy clean. `cargo fmt --check` still reports
only the other stream's new `missions.rs` regions (983, 2393, 2427, 2461);
`crates/ffe-rs/tests/missions.rs` is rustfmt-clean. New tests:
`an_overdue_contract_fails_even_at_its_destination`,
`an_overdue_military_package_gets_f302s_late_arrival_page`; updated
`failed_contracts_take_reputation_below_zero`,
`expiry_writes_f301s_notice_then_f302s_failure` and the cache-gated
integration case.

**Can `gameplay-10` close?** The stated remaining gap is fixed. The remaining
documented deviations under the item are F300's removal-list overdue page, the
F304/F305 rank page on a late military package, and F485's abandon page/`-8`
(`missions.md` item 8); the journal archive/viewer was already ported. No
dashboard status changed in this pass (another stream is mid-edit in the same
checkout).

## Active contract — `gameplay-9` (BBS) / `gameplay-10` (journal and mission state)

**Remaining gap found:** `gameplay-9` still lacks F521's reply page (the 13
suggested responses 0x9833..0x983f) and F524-F532's negotiation pages; the
row click accepts directly. `gameplay-10`'s F302 overdue pass is delivered in
the section above, and the journal viewer/archive was already ported.

**Delivered 2026-11-20:** (1) F303's `(basic_data >> 8) & 0x1f == 0x14`
branch is ported (`missions::refuses_payment`): it is F638's passage additive
index 10, **not** a removal title (the old notes said title; 0x99b4 shifts to
0x19 under the mask). It pays nothing, makes no reputation step, posts
0x9c35 and closes the contract as `Status::Failed`. (2) F521's `F922(9)`
roll is now made when a 0x981a/0x981b row is selected
(`Navigator::offer_reply_draw`, `missions::f521_refuses`), posting F520's
0x991e page before F534's cabin check. (3) `missions::State::reputation` is a
signed `i32` with no floor (F302 -2, F485 -8), so the refusal is reachable;
`scripts::Leftover::store` no longer clamps it to 0. Docs: missions.md
items 2/3/5, station.md (F636/F637/F641 and the F639/F640 stock byte were
stale). Files: `crates/ffe-core/src/missions.rs`,
`crates/ffe-rs/src/{navigation,main,scripts}.rs`,
`crates/ffe-rs/tests/missions.rs`.

**Verified:** `cargo test -p ffe-core` (279 lib, all targets),
`cargo test -p ffe-rs --lib` 317 passed/29 ignored,
`cargo test -p ffe-rs --test missions` 4 passed/2 ignored and, with
`.cache/parity07/data.bin`, `--test missions -- --ignored` 3 passed (new
`a_negative_reputation_refuses_the_passenger_reply`);
`cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt --check` clean;
`cargo clippy -p ffe-core --lib -- -D warnings` clean. `ffe-rs --lib` clippy
clean with `-A clippy::too_many_arguments`: the reported lint is the other
stream's new `model_viewer.rs:1179 render_scene_with_instance_scale`.

**Next:** the F521 reply-page screen (and F524-F532's negotiation pages) still
needs a deliberate edit to the live click path. Do not close `gameplay-9` on
this chunk.

## `world-13` source model scale (2026-11-20)

**Gap:** `Screen::draw_discs` rendered a planet at the descriptor's scale and
passed only the descriptor's scale offset as F1680's `+0x130`, folding F1680's
precision shift into a port-only matrix shrink. F1680 lowers the operand scale
(`+0x148 = desc[+0x14] + e`) and raises the key scale (`+0x130 = desc[+0x18] -
e`) once `e = desc[+0x18] + obj[+0x7e] - shift < 0`, i.e. for a planet farther
than ~8,590 km at Hope's offset 4. The planet's F1777 depth key was then `2^-e`
times too small (sorted nearer than a ship/station at the same distance) and a
child such as Saturn's ring was sized through a compensating matrix shrink.

**Delivered:** `draw_discs` now takes the instance position, operand scale and
key scale from `ffe_core::model_animation::setup_object` (F1680) and renders
through new `ModelViewer::render_scene_with_instance_scale`; `Draw::Culled`
is honoured. The descriptor-less cache fallback keeps the old inference.
Near-surface terrain was already exact and is unchanged.

**Evidence:** `cargo test -p ffe-rs --lib` 317 passed/29 ignored (new
`screen::f1680_scale_tests` two cases); cache-backed `--test planet_view --
--ignored` 26 passed (rings, Hope launch, landed surface); `--test
surface_draw_reference executed_reference_near_surface_draw -- --ignored` 1
passed; `--test palette_frame -- --ignored` 2 passed;
`cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt -p ffe-rs -- --check` clean;
`cargo clippy -p ffe-rs --lib -- -D warnings -A clippy::unnecessary-cast`
clean.

**Remaining for `world-13`:** F1680's rotating-frame branch (`+0x57`, F1677/
F1665/F1679) is still read-only; the port's absolute-frame rendering is
picture- and key-equivalent but has no source frame trace. A matched
whole-frame planet capture is still missing. Do not close on this chunk.

## Closed 2026-11-20 — `gameplay-41` military script rows

See the closure at the top of this file. F493's screen 7 now prints the
scripts' F346 rows (mission 23's 0xa9b9) in menu order and runs F360 on them.

## Active contract — `gameplay-41` station-screen signals and `gameplay-12` repeat messages

**`gameplay-41` gap:** only the bulletin board calls F326's builder
(`scripts.open_screen(..., 2)`); the source sets `DATA_009054` to every
station screen and F326 dispatches on it. `Engine::build_screen` also
refills entries for screens 4 and 8, but F326's jump table sends those to
F385/F445, which only redraw (F326 rebuilds only 2, 3, 5, 6 and 7).

**`gameplay-41` acceptance:** entering the market (3), upgrades (4), ships
(5), military (7), repairs (8) and the roots (0) signals the scripts as
F327/F331/F342 do; a rebuild of a redraw-only screen leaves the entries
alone. Unit test for the dispatch plus a cache-backed real-script case.

**`gameplay-12` gap:** `Event::repeating` bypasses `Channel`'s dedupe but
`MessageLog::insert` still drops an identical newest entry, so a second
"WARNING: Incoming Missile" is never appended while the API reports
`Post::Added`. F271 always appends a discrete event.

**`gameplay-12` acceptance:** a repeat event appends one entry each time, a
repeated state line still dedupes, and the log length proves it.

**Files:** `crates/ffe-core/src/script/{engine,run}.rs`,
`crates/ffe-rs/src/{scripts,station,main,messages}.rs`,
`crates/ffe-rs/tests/{mission_scripts,prefs}.rs`.

**Smallest verification:** `cargo test -p ffe-rs --lib messages` and
`cargo test -p ffe-core script::engine`.

**Delivered 2026-11-20:** `StationScreen::script_screen` names the source
screen; main calls `open_screen` whenever the docked page changes (not only
the board), with the board's adverts as its native entries. `Engine` gained
`enter_screen` (F342 clear + station entries + F453/F493 signals) and
`build_screen` now rebuilds only 2/3/5/6/7 per `DATA_000025`; 0/1/4/8/11
only redraw. Messages: `MessageLog::record_repeat` keeps a discrete event
that matches the newest entry (`Channel::post` routes `Event::repeat`
there).

**Evidence:** `cargo test -p ffe-core` 279 lib passed/1 ignored and every
integration target; `cargo test -p ffe-rs --lib` 317 passed/29 ignored
(new `only_the_builder_screens_rebuild_the_scripts_menu`,
`the_military_office_signals_three_and_the_market_only_clears`,
`each_station_page_names_its_f326_screen`,
`a_discrete_event_appends_even_when_the_newest_entry_matches`);
`cargo test -p ffe-rs --test prefs a_repeating_event` 1 passed;
cache-backed `--test mission_scripts -- --ignored` 10 passed (new
`the_other_station_screens_signal_the_scripts`) and `station_view` 15
passed; `cargo check -p ffe-rs --bin ffe-rs`; `cargo fmt --check` clean;
core and `ffe-rs --lib` clippy clean (the bin keeps the pre-existing
`screen.rs:3118`, `main.rs:3415` and `main.rs:9160` warnings). Remaining
for both items is unchanged.

## Closed 2026-11-20 — dashboard `parity-24` (shared new-game setup)

**Contract:** every new-game entry and every resume entry must produce the same
source state a first boot does — no dead commander's ranks, slots, model, seat,
view or services, and no stale per-game UI state — before the first tick.

**Delivered (verified, not re-implemented):** the initial Enter boot, the
post-GAME-OVER RIP click/Enter restart, `--start N` and the intro's start keys
all call `Navigator::new_game` + `ffe_rs::boot::new_commander`; the restart arm
(`main.rs`'s `IntroExit::Start`) also resets `Commander` and the per-game UI
locals. Both load entries (the intro's `L`/key 4 and the Escape menu's Load)
share `apply_save` + `resume_model`.

**Evidence:** new cache-backed `source_boot::a_post_death_restart_builds_the_first_boots_commander_and_state`
(starts 0 and 1; a destroyed, mutated commander — name/ranks/missions, cash,
cargo/fuel, hull, damage, service days, loadout, lasers, position, view, clock,
fines — restarts to the first boot's whole `Flight`, whole `OriginalFlight`,
`Navigator::save_state`, `Commander::default()`, fuel and services).
`cargo test -p ffe-rs --test source_boot -- --ignored` 6 passed (the
`FFE_START_RECORDS` case skipped; it needs that variable), `cargo test -p
ffe-rs --lib` 313 passed/29 ignored, `cargo check -p ffe-rs --bin ffe-rs` and
`cargo clippy -p ffe-rs --lib -- -D warnings -A clippy::unnecessary-cast` clean,
`cargo clippy -p ffe-rs --test source_boot` clean, `rustfmt --check` clean on
`tests/source_boot.rs` (the crate-wide check still trips the other stream's
uncommitted `main.rs` formatting). Docs: `BURNDOWN.md` 19 open / 223 done,
`dashboard/burndown.js`, `STATUS.md`, the PAR-24 audit entry.

**Residual notes (recorded in the audit entry):** the window-bound intro/Escape
load handoff was verified by code path, not executed headlessly; no pixel HUD
capture; the restart's transient `message` says "New game." where a first boot
has none, but `Screen::draw` ignores `message` before the first tick.

## Closed 2026-11-20 — owner bug: skip the game-over RIP screen

**Contract:** while the game-over tombstone is showing, a fresh click or any new
key press must skip the remaining F917 loop and the intro and start the default
new game at Hope as a fresh commander. No input keeps the source timing;
`--capture-gameover` and replay evidence must not change.

**Delivered:** `gameover::GameOver::abort()` puts the state machine in its
existing `Phase::Finished` state (F917's longjmp equivalent). `main.rs`'s
game-over block now samples the frame's input before drawing:
`game_over_restart_edge(keys_pressed, mouse_down, mouse_was_down)` (a fresh key
press or a left-button *edge*; a button held when the screen appeared is not a
click). On abort the block sets `restart`, skips `run_intro`, and uses the
already-existing `IntroExit::Start(chosen_start)` arm, so the new-commander reset
is the one code path (no duplicated reset). The shortcut is disabled for
`--capture-gameover`, replays and an inactive window.

**Changed files:** `crates/ffe-rs/src/{gameover,main}.rs`,
`docs/notes/gameover.md`, `docs/DEVIATIONS.md` (EXP-047).

**Evidence:** `cargo check -p ffe-rs --bin ffe-rs`; `cargo test -p ffe-rs --lib`
308 passed/29 ignored (new `gameover::tests::
a_click_or_key_aborts_the_tombstone_to_the_front_end`); `cargo test -p ffe-rs
--bin ffe-rs` 3 passed (new `lifecycle_input_tests::
the_rip_screen_restarts_on_a_fresh_click_or_key_only` and
`the_rip_screen_restart_skips_the_intro`); `--capture-gameover
.cache/gameover-skip-check` exits 0 with its 35 saved frames; `death_view` 3 and
`gameover_view` 2 ignored (cache-gated) compile; rustfmt clean and clippy
shows only the pre-existing `main.rs`/`screen.rs` warnings. `tests/death.rs`'s
`f942s_explosion_and_debris_spawn_at_the_player` fails (3 pods vs 0 expected) on
the other stream's uncommitted effects/RNG work, not on this change.

## Closed 2026-11-20 — `enhancement-10` NPC targeting button

**Delivered:** a dedicated, discoverable NPC targeting button. The embedded
`npc-targeting.png` (compiled with `include_bytes!`) is drawn at (224, 133), just
above the console's targeting/combat icon at (224, 169) and wholly inside the
320x158 flight view; it exists only while `enhancement.npc_labels` is on and the
player is in flight. Clicking it opens the navigation-computer-style list of
visible NPC ships within 50 km, nearest first: a grey 10x10 toggle, ship name,
registration/serial and `(distance)`. Rows are red for pirates/hostiles, blue for
non-hostile police, green for neutrals. Clicking a row sets `nav.ship_lock` +
`nav.missile_lock` (`ship.id`), closes the list so the flight view's targeting
square shows, and keeps the lock. The list also closes on the button, any console
icon/screen action, leaving flight/a page, or changing the view it opened in;
only a lock made through the list is dropped. `l` keeps its original text-label
behaviour, and the toggle off draws nothing and changes no target.

**Changed files:** `crates/ffe-rs/src/{combat_computer,main,screen}.rs`,
`docs/notes/enhancements.md`.

**Evidence:** `cargo check -p ffe-rs --bin ffe-rs`; `cargo test -p ffe-rs --lib`
313 passed/29 ignored (ten `combat_computer::tests`, including the red/blue/green
classification, 50 km cap, row format, button hot area, row-select-closes-list and
embedded-icon decode); `cargo fmt -p ffe-rs -- --check` clean; `cargo clippy -p
ffe-rs --lib -- -D warnings -A clippy::unnecessary-cast` clean. The bin clippy
still reports only the pre-existing `main.rs:3415` redundant closure and `:9117`
type complexity. Registration uses F791 kind 0x12 from `ship.seed * 0x4a0d`
(`ffe_core::names`), falling back to `#id` without the cache's name table.

## Closed 2026-11-20 — dashboard `enhancement-7` (mission fixes)

**Delivered:** all seven runtime fixes behind `enhancement.mission_fixes`
(EXP-046). (1) A hyperspace jump no longer reports the left-behind ships as
destroyed, and script ship ids continue above the old systems' ids
(`mission_fixes::removed`, `Navigator::script_ship_id`, optional
`script_ship_serial` save word). (2) F1125's view-change pulse (`global 0 = 4`,
`Engine::camera_changed`) restores mission 69/78's `G0 == 4` distance tracking,
which the hitch-hiker run needs to avoid its "returned too late" message.
(3) F312's full medal list is drawn in two columns (`status::medal_position`);
the three-row cut hid awards at bits 8/9/12. (4) The Alioth flower briefing
(0xa837/0xa838) states the hidden 3252 deadline, UI only. (5) A `Full` F655
reply frees a bay and launches its occupant, then grants the player the bay.
(6) A docked mission dialogue holds time at 1x and ignores acceleration keys
and buttons. (7) Script departures reserve F650's bay (booting an occupant
when needed), so a target cannot be missing.

**Already correct (not re-implemented):** F307/statement-53 medal awards; the
`gameplay-42` script corrections (78, 55, 20, 83, 81) stay always-on. The
original's deep-space kill case has no port object to credit (the port has no
ships in deep space after a mis-jump); that half is documented, not exercised.

**Evidence:** `cargo test -p ffe-core` 273 lib + all integration targets
(the new `traffic` bay test included), `cargo test -p ffe-rs --lib` 302
passed/28 ignored; cache-backed `save_tests` 14 passed (new
`the_mission_fix_ship_serial_resumes_and_off_saves_keep_the_original_words`),
`mission_scripts -- --ignored` 9 passed, `station_view -- --ignored` 15
passed; `cargo check -p ffe-rs --bin ffe-rs`, `cargo fmt --check` on both
crates, core clippy clean, `ffe-rs --lib` clippy clean (the bin still has the
pre-existing `main.rs:3349` redundant-closure and `:8997` type-complexity
warnings from the other stream).

**Dashboard:** 242 total, 218 done, 24 open.

## Active contract — under-attack feedback (owner-reported bug)

**Gap:** pirates fire and damage the player, but nothing warns the player:
`F750`'s alert is only handled when time was accelerated, NPC laser hits play
no `F953` sample, and `F753`'s combat lamp (console record `0x37`,
`DATA_009046` bit 2) is never set.

**Source:** `jjffe-master/ffe.asm:49569-49587` (`F750`: `F217`, message
`0x98c3` = `ffedat.asm:8535` "WARNING: Ship is Under Attack", sample `0x1e`,
song 14); `ffe.asm:49696-49699` and `ffe.asm:49960-49964` (`F753`/`F751`:
`DATA_009046 |= 2`, `ConsoleSetButtonImage(0x37, 1)`); `ffe.asm:15132-15136`
(`F151`: lamp 0 and bit 2 cleared).

**Acceptance:** a hostile entering combat with the player posts the alert and
the siren, and the flight console lights 0x37 while it attacks; NPC laser hits
play `F930`/`F953` samples. Regression test for event + lamp + siren.

**Files:** `crates/ffe-rs/src/{navigation,console,sound,main}.rs`,
`crates/ffe-rs/tests/npc_ai.rs`, `docs/notes/combat.md`.

**Smallest verification:** `cargo test -p ffe-rs --lib console` and the
cache-backed `npc_ai` case with `.cache/parity07/data.bin`.

**Delivered 2026-11-20:** `Navigator::combat_mode` (alive hostile in Attack/Pass,
player as implicit target, `in_combat`, player not inside a station);
`ShipSystems::combat` lights console record 0x37 (F753/F151) on the flight and
equipment pages; main posts `0x98c3` and plays the siren (`0x1e`, new
`GameAudio::attack_alert`, always even if the battle song is already on) on
every `ShipEvent::UnderAttack`; NPC laser hits play `Cue::Laser` (bank from
`(code & 0x78) >> 3`) plus `Cue::ShieldHit`/`Cue::HullDamage`. The F217
"only when accelerated" gate is dropped (owner report), recorded in
`docs/DEVIATIONS.md`.

**Evidence:** `cargo test -p ffe-rs --lib console::tests` 5 passed (new
`f753_lights_the_0x37_lamp_while_a_hostile_attacks`), `--lib sound::tests` 14
passed (new `f750s_attack_alert_always_sounds_the_siren`), cache-backed
`--test npc_ai -- --ignored` 8 passed (new
`a_pirate_attack_alerts_the_player_and_lights_the_combat_lamp`),
`cargo test -p ffe-rs --lib` 299 passed, 0 failed, 28 ignored,
`cargo test -p ffe-core` 270 passed, 1 ignored, every integration target passed,
`cargo check -p ffe-rs --bin ffe-rs` passes and
`cargo clippy -p ffe-rs --lib -- -D warnings -A clippy::unnecessary-cast` is
clean. `cargo clippy --lib --tests` still trips pre-existing lints in
`tests/prefs.rs` and `main.rs`'s test module (from the other stream).
`rustfmt --check` is clean for `console.rs`, `sound.rs` and `tests/npc_ai.rs`;
`git diff --check` is clean. The mid-session whole-crate failures came from the
other stream's in-progress `navigation.rs`/`screen.rs` edits and cleared once
they landed.

## Parked contract — dashboard `flight-1` (object system and agent records)

**ID:** `flight-1` (still `todo`; foundational and multi-step).

**Gap:** the source's `PhysObj` (0x152-byte object record) and its `+0xfe..+0x151`
agent block were only implicit in `NpcShip`, `physics::Motion` and
`laser::DamageState`; there was no source-shaped record type or byte bridge.

**Acceptance for this chunk:** a typed object/agent record in `ffe-core` whose
field offsets match `jjffe-master/srctools/plstate.txt`, readable from and
writable to the 0x152-byte slot layout that `objects::PhysList` already stores;
a snapshot from `NpcShip` of the fields the port carries; unit tests for every
documented offset, the `+0xf2` union and the flags.

**Delivered 2026-11-20 (record layer):** `crates/ffe-core/src/object_record.rs`:
`ObjectRecord`, `AgentRecord`, `offsets`, control/kind/class/laser constants,
`caps`/`hyperspace` for the `+0xf2..+0xfd` union, name helpers.
`objects::PhysList::{record,set_record}`; `NpcShip::object_record` (then taking
the slot as an argument; see the live wiring below).
Scope/field map: `docs/notes/object-system.md`.

**Delivered 2026-11-20 (live wiring):** `object_record::ShipSlots` is F927's
allocator (0x72 down, floor 0x14, freed slot reused first) keyed by ship id.
`NpcShip` stores `slot` (`+0x86`), `class_flags` (`+0x14c`), `collision_radius`
(`+0x140`), `drive` (`+0xd0`), `damaged` (`+0xcc`), `retro` (`+0x101`, from the
guidance's backward thrust), the `+0x14d..+0x150` timeout bytes and the jump
words; `Persisted` carries class/radius/drive/damage across a jump.
`NpcShip::object_record_in(&ObjectIndexes)` writes `+0x86`, `+0xfe` (station
while docking, else ship target, else the player index) and `+0x100` (the
destination body's `Body::slot`). `Navigator` owns the table, syncs it after
every `ships` mutation (tick, recycle, system entry, contract/hyperspace/police
spawns, `scripts::apply`, save restore) and exposes
`ship_object_record`/`object_records`; save resume keeps the slots.
`Spec` gained `class` and `collision_radius` from `ModelInfo`.

**Evidence:** `cargo test -p ffe-core` (263 lib passed, 1 ignored; every
integration target passed), `cargo test -p ffe-rs --lib` (283 passed, 28
ignored), all 13 cache-backed `navigation::save_tests` with
`.cache/parity07/data.bin` including the new
`live_ship_slots_and_object_records_resume`, `cargo check -p ffe-rs --bin
ffe-rs`, `cargo fmt --check -p ffe-core -p ffe-rs`, `cargo clippy -p ffe-core
--lib -- -D warnings` and `cargo clippy -p ffe-rs --lib -- -D warnings -A
clippy::unnecessary-cast`. `cargo clippy -p ffe-core --lib --tests` still fails
on a pre-existing `system_pick.rs:177` precedence lint from another
developer's uncommitted file. New tests: five `ShipSlots` cases, three `NpcShip`
record cases, one cache-backed live-list case.

**Remaining for `flight-1`:** the player object and F941 missiles now have
object slots (below), but no frame renders from their records, chaff has no
slot, the frame-of-reference (`+0x14d`), smoke (`+0x14e`) and avoidance
(`+0x150`) timeouts still have no port behaviour, the source's AI jump states
7/8 and the `+0xa8`/`+0xac` jump clock remain approximations, and the orbital
overlay (`+0xb0..`) still splits between `objects::Orbiter` and
`traffic::Starport`. Do not mark the dashboard item done.

**Delivered 2026-11-20 (player, missiles, F682 jump words):**
`ShipSlots` now reserves the player object (`DATA_008857`) at `0x72` before
traffic (`reserve_player`, `from_saved_with_player`), so ships start at `0x71`;
F941 missiles share the slot space (`allocate_missile`/`sync_missiles`,
`Missile.velocity`/`target`/`owner` to `+0x86`/`+0xfe`/`+0x100` via
`object_record::missile_record`). `NpcShip` stores `+0x119` `cargo_fuel`
(F773's outfit draw, persisted by `Persisted`); `object_record_in` reports the
evaded missile's slot for `+0xfe` and the player index for a hostile hunting
the player. `Navigator::{player_object_slot, player_object_record,
missile_object_record, missile_object_records}` expose them; `sync_object_slots`
covers launches, ticks, ECM and resume; `tick_ships` runs F775/F776
(`spawn::jump_words`, `hyperspace::ai_jump`) on the frame a ship turns to flee,
filling `+0xf2`/`+0xf6`/`+0xfa` from the galaxy tables while keeping `+0xf8`.

**Files:** `crates/ffe-core/src/{object_record,npc,hyperspace,spawn}.rs`,
`crates/ffe-rs/src/{navigation,missile}.rs`,
`crates/ffe-rs/src/navigation_save_tests.rs`, `docs/notes/object-system.md`.

**Evidence:** `cargo test -p ffe-core` (300 lib passed, 1 ignored; every
integration target passed), `cargo test -p ffe-rs --lib` (321 passed, 29
ignored), cache-backed
`navigation::save_tests::live_ship_slots_and_object_records_resume -- --ignored`
1 passed with `FFE_WORLD_CACHE=.cache/parity07/data.bin`,
`cargo check -p ffe-rs --bin ffe-rs`, core and `ffe-rs --lib` clippy clean
(core needs `-A clippy::unusual_byte_groupings` for the other stream's
`missions.rs`), rustfmt clean on every changed file. `cargo fmt --all --check`
still trips the other stream's unformatted `missions.rs`/`combat_computer.rs`.

**Next for `flight-1`:** the F939 smoke particle (a shared-RNG ledger change),
F802's avoidance steering, F606's real frame-of-reference update, F682's second
F901 call (`DATA_008916` to `+0xf2`), NPC/chaff frames drawn from the records
rather than the port structs, and the player object's `+0xff`/`+0x10e`; see the
section at the top of this file for the second pass.


## Closed 2026-11-20 — dashboard `flight-5` (input remapping)

**Delivered:** `config.cfg`'s `bind.*` lines drive the input mapping end to end
in two forms: a minifb game-key name (`bind.A=Up`) or a preset-independent action
name (`bind.pitch_up=NumPad8`, `input::Action`, 33 names covering the flight
controls in `docs/CONTROLS.md`). `session::bindings` validates and expands an
action to the key it uses in each preset; `Frame::remap` (already wired into
`main.rs` before this turn) delivers it every frame. Unknown key/action names and
self-bindings are ignored, not fatal. Input processing and key accumulation were
already replay-verified; the in-game remap screen remains out of scope.

**Verification:** in an isolated HEAD worktree (the in-place tree has an
unrelated uncommitted `navigation.rs` build error): `cargo test -p ffe-rs --lib
input` 18 passed, `--lib session` 4 passed, `--test controls` 3 passed,
`cargo check -p ffe-rs --bin ffe-rs` and `cargo fmt -p ffe-rs -- --check` pass.
`cargo clippy -p ffe-rs --lib -- -D warnings -A clippy::unnecessary-cast` passes
(the allowed lint is a pre-existing `screen.rs` cast; `--tests` still trips
pre-existing `main.rs` type_complexity).

**Changed files:** `crates/ffe-rs/src/{input,session,settings}.rs`,
`docs/CONTROLS.md`, `docs/notes/persistence.md`, `docs/DEVIATIONS.md`,
`docs/ACCEPTED_PARITY.md`, `docs/BURNDOWN.md`, `dashboard/burndown.js`.
Dashboard: 231 total, 203 done, 28 open (`gameplay-5` closed in the same batch).

## Session 2026-11-20 (parity-21 follow-up) — effect slots wired into the live game

**Contract:** wire `ffe_core::effects::EffectTable` into `Navigator::effects`
so the live game allocates F939/F942/F291 objects with F928 and F926 debris with
F927, frees them with F924/F767, and preserves every existing reader
(renderer, scripts, save/resume). Add a regression that a freed slot is reused.

**Delivered:** new `ffe_core::effects::Effects` list facade over `EffectTable`
(`push` = F928, `push_pod` = F927, `push_burst` = F928 explosion + F927 pods,
`tick` = table pass with F924/F767, positional `get`/`last_mut`/`iter` for the
existing callers, `from_saved`/`to_vec` for the unchanged save format).
`Navigator::effects` is now `effects::Effects`; `scoop` frees dead slots by
F924, `tick_effects` delegates to the table, and the death camera follows the
explosion's slot instead of a Vec index. Save files keep `Vec<Effect>` in
`NavigatorRuntimeSave` (reverse spawn restores the exact slots).

**Evidence:** `cargo test -p ffe-core` (244 lib + all targets), `cargo test -p
ffe-rs --lib` (283 passed, 27 ignored), the 12 cache-backed
`navigation::save_tests` (including effects resume) and all 13
`effects_view` ignored cases with `.cache/parity07/data.bin`, plus
`death_view`/`ecm_view`/`laser_view`/`thargoid_view` and the navigation jump
test. New tests: `the_effects_facade_hands_a_freed_slot_to_the_next_push`,
`a_burst_takes_f928s_slot_and_its_pods_f927s`,
`saved_effects_return_to_their_source_slots`, and cache-backed
`effects_view::f924_a_freed_effect_slot_is_reused`. `rustfmt --check` clean on
changed files; core clippy `-D warnings` clean; ffe-rs lib/test clippy clean
for the changed files (one pre-existing `screen.rs` warning and a pre-existing
`tests/prefs.rs` clippy error remain unrelated). `cargo check -p ffe-rs --bin
ffe-rs` passes.

**Remaining for parity-21:** the frame-by-frame effect object trace against
the source (expected), so the dashboard ID stays open.

## Session 2026-11-20 (coordinator batch) — five items closed

Closed: `release-6` (MIT licence), `v0.48-17` (model census 448/466 + CD 1.01), `v0.48-7` (quad emission), `persist-1` (legacy commander import read-only) and `gameplay-42` (broken missions: 5 fixed always-on, 5 orphaned documented-not-patched). Progressed, still open: `parity-21` (F1512 movement + F928/F927/F924 slot table in `ffe-core`, now wired into `Navigator::effects`; the frame trace remains) and `world-8` (F1347 map templates; lit-sphere/hyperspace scene remain). Dashboard: **231 total, 201 done, 30 open**. Build clean, `Cargo.lock` unchanged, `ci/check_burndown.py` passes. The police/NPC items `gameplay-5/6/7/18` are still the other developer's uncommitted work — do not touch until they close them.

## 2026-11-20 — dashboard `gameplay-42` (broken-mission fixes)

**Contract:** correct the source-verified broken mission data in the port's
decoder, always-on, with a DEVIATIONS row and a regression per fix; document
any starter the source does not determine instead of inventing it.

**Delivered:** `ffe_core::script::Image::new` now applies a small `CORRECTIONS`
table to the in-memory region (the cached asset and `FIRSTENC.EXE` stay
byte-identical). Fixed: mission 78's `0x217`/`0x197` article test (EXP-030),
mission 55's missing `0xa963` response (EXP-031), mission 20's tenfold journal
debts (EXP-032), mission 81's menu-cleared reward trigger (EXP-033) and mission
83's swallowed `goto 7` via the one-operand `0x2b` reading (EXP-034).
Documented, not patched: 37/38/39 and 91 have no `1 n` starter in the region or
in F52's call sites, and their intended starter could not be determined
(EXP-035).

**Verification:** `cargo test -p ffe-core` (all), `cargo test -p ffe-rs --lib`;
the nine ignored `mission_scripts` cases pass with
`FFE_WORLD_CACHE=.cache/parity07/data.bin`, including the new
`the_broken_mission_data_is_corrected_in_memory` and the repinned
`(334, 667)` state/condition totals with an empty quirk list. See
`docs/notes/broken-missions.md` (`## Fixes`).

**Remaining:** the 37/38/39 chain and 90/91's hand-over need owner/reference
evidence (a 1.02 native-code disassembly or play capture) for their intended
starters; do not close `gameplay-42` fully until that is decided.

## Closed 2026-11-18 — dashboard `gameplay-dust`

Closed on the executed-original comparison in `docs/ACCEPTED_PARITY.md` (153 port dust pixels against 151). The contract below is kept as history; the three gaps left (Milky Way band, day-tick ageing, crosshair pixels) are recorded there.

## Former contract — dashboard `gameplay-dust`

**Gap:** owner sees a dense grey dust cluster after climbing away from a planet, absent in the original comparison. Recent absolute-chain sizing may expose a separate dust detail/lifecycle mismatch.

**Acceptance:** reproduce ascent, identify DOS-backed cause, correct cloud placement/size/count or lifecycle without arbitrary density suppression, preserve dust preference and motion; verify matched original/port behavior before closure.

**Files:** `crates/ffe-core/src/{dust,model_execute,model_commands}.rs`, `crates/ffe-rs/src/{main,screen,navigation}.rs`, `crates/ffe-rs/tests/effects_view.rs`.

**Smallest verification:** core dust tests and local-cache `effects_view` dust regressions, then ascent capture and DOS comparison. Dashboard status/totals unchanged. Background parity remains parked and unverified.
**Progress:** removed main.rs's port-only 75 km dust visibility cutoff (including spherical near-body check). F1035 already creates clouds above 20 km; hiding them until 75 km can reveal accumulated clouds together. DOS 1.02 file 0x3277f compares altitude at 0x642218 to 0x1312d00; JJFFE F1035 agrees. Preference still controls visibility. This is a source-backed correction, not a reproduced explanation of every grey pixel in the owner's image. Owner confirms onset after climbing away. Twelve core dust tests and five ignored cache-backed effects_view dust tests pass; cargo check for game binary passes. Strict binary clippy fails on unrelated main.rs type_complexity near line 8122; rustfmt --check reports existing unrelated formatting near line 507. Logs: `.cache/parity07/dust-{core,view,build,clippy,format}.log`. No dashboard status/totals changed. Next: matched ascent capture with dust on/off, confirm whether cluster persists after removing cutoff, and audit F1680 hidden/cull feedback (particles currently retain hidden=false).


## Parked contract — dashboard `parity-07` (owner flight-background correction)

**Gap:** model 315 currently uses unverified depth/orientation/placement/light; geometry reaching a frame and the unmatched final capture do not establish parity.

**Acceptance:** DOS FFE 1.02-derived call arguments and bitmap 53–58 masking; reliable front-view nebula and source star chains; matched state/view/options/logical-resolution captures establishing orientation, extent, palette, brightness, star count and starburst shape; preserve motion, visibility settings and haze. Keep open until every visual criterion is verified.

**Relevant files:** `crates/ffe-rs/src/{screen,model_viewer}.rs`, renderer texture paths, existing reference harness under `tools/reference`, `.cache/reference-jjffe/ffe.asm`, `game/FIRSTENC.EXE`, `.cache/parity07`.

**Smallest verification:** `cargo test --locked -p ffe-rs flight_background_model315_reaches_frame` is geometry regression only; completion additionally requires DOS source comparison and matched original/port flight captures.

**Verified correction:** F1766 absolute star-chain sizes now bypass depth division and the instance-depth gate. DOS 1.02 file 0x51773 masks with 0x7fff and jumps directly to 0x517cc, bypassing F1666 at 0x517c1; JJFFE agrees. The port previously divided these absolute sizes, collapsing model 316's packed 0x0140 (two pixels) to zero at distance. Changed `crates/ffe-core/src/model_execute.rs` and its existing integration test. All 33 model_execute tests pass; strict clippy for that test passes; ignored flight_background_model315_reaches_frame passes with local data.bin. The latter proves frame integration only, not visual parity. Latest synthetic capture: `.cache/parity07/flight-background-source-transform.ppm` (unmatched).

**Second verified correction:** face emission now honors F1762's normal-0 colour write. Model 315 resets normal 0 to black, but faces previously kept F1480's initial shade, adding the system tint to the nebula. DOS file 0x51475 tests bit 0x80 and 0x51489 loads normal-table base 0x653088 for F1691 at 0x5148f; JJFFE F1762 agrees. Added emission-order regression; all 34 model_execute tests, ignored flight background test, strict core test clippy and formatting pass. Same synthetic port scene before/after saved as `.cache/parity07/nebula-colour-{before,after}.{ppm,png}`: visible shift from pale pink to dark blue-violet. These are port-to-port frames, not matched DOS parity evidence. No depth/orientation change in this step.

**Unresolved:** nebula transform/light/palette and matched original/port UI capture. DOS F1775 at 0x51cac, transpose F1665 at 0x4ac60, F1682 at 0x4c140 and F1683 at 0x4c164 confirm direct z=0x200. Prior direct-transform experiment was reverted; production still uses unverified 0x2000/star orientation. Investigation trace `.cache/parity07/execution.log` shows model scale 10 and child 316 scale 8; missing geometry was not proof that the DOS depth was wrong. Next inspect model 315's camera parent frame and texture 53–58 descriptors/masking, then extend the existing capture path. No temporary tracing remains in the renderer. Dashboard unchanged; parity-07 stays open.

## Current authorization

Work through the existing police/pirate parity items in dependency order, one dashboard ID at a time. Preserve mixed-checkout edits; do not add dashboard tasks, commit, push or spawn bots.

## Parked contract — dashboard `parity-17`

**Gap:** the shared RNG pair is implemented and existing source-backed cases match F622 plus F688 for default and Ross 154 starts. The full draw ledger remains incomplete for F633 daily draws and later consumers, including object-event and rendering dependencies.

**Acceptance:** record a baseline draw ledger including object-event and rendering dependencies; match both source words and downstream outcomes. Optional port presentation must not silently choose a different gameplay seed.

**Relevant files:** `crates/ffe-core/src/{npc,spawn}.rs`, `crates/ffe-rs/src/navigation.rs`, `tools/reference/{headless.c,README.md}`, `docs/audits/gameplay-parity-work-items.json`.

**Smallest verification:** use the existing `bbs_reference` cases for F622/F688, then compare a same-seed executed-source `FFE_RNG_TRACE` against the port through F633 and downstream consumers. The current checkout lacks the complete JJFFE smoke-build inputs and NASM executable for new source traces.

**Progress:** `parity-15` depends on this item. Existing ignored `bbs_reference` cases record the default F622+F688 1,876-draw trace and Ross 154 entry/exit states; they supersede the older claim that F688 diverges at draw 1712. `parity-16` is already done on the dashboard; its remaining F702 placement/F238 police-response work is carried by `parity-15`. This turn corrected the contract to match current evidence; no source code changed.

## Parked contract — dashboard `parity-15`

**Gap:** source-like physics is implemented for ships with statistics, but per-state NPC traces remain unmatched; the stats-less speed-cap fallback remains. Police response is currently reconstructed from wanted/alert bits, and F702 system-wide placement remains carried from `parity-16`.

**Acceptance:** trace traders, escorts, pirates, police and heavy freighters under the same objects/seed; match per-state target, thrust, turn, velocity and event time at 1x and accelerated time. Keep the full PAR-15 acceptance unchanged.

**Relevant files:** `crates/ffe-core/src/npc.rs`, `crates/ffe-rs/src/navigation.rs`, `tools/reference/{headless.c,verify-npc.py}`, `.cache/reference-jjffe/ffe.asm`, `docs/audits/gameplay-parity-work-items.json`.

**Smallest verification:** same-seed source/port `--npc-trace` comparison at 1x and accelerated time, including police response and F702 placements.

**Progress:** source F238 uses the player's law-response/count fields, calls F239 to allocate and construct police objects, and consumes shared RNG; the port instead provokes all existing police/military from `Legal::wanted`. The exact F238 path is a multi-state behavior change. No source edit started; preserve PAR-15 as in progress until its trace gate is available and full acceptance passes.

## Latest completion — dashboard `ui-7`

**Delivered:** closed the existing Intro and new-game flow item. Direct `L` and the load-picker flow are outside this dashboard item's scope.

**Evidence:** existing source intro checks cover 76 cases; nine `front_end` tests and the intro-reference compile are recorded as passing. The owner authorized closure of the remaining UI QA gate. Live key delivery and a matched-frame capture were not verified in this session; no direct-load behavior is claimed.

**Dashboard:** 224 total, 154 done, 70 open (58 in progress, 12 todo).

## Latest completion — dashboard `content-1`

**Delivered:** verified and closed the existing ship, equipment and economy table import item. Mission-cargo handling and F641 board generation remain in their gameplay items.

**Evidence:** `cargo test --locked -p ffe-core --test market` (14 passed); absolute-cache `ship_names -- --ignored` (2 passed, all 466 model slots and 58 hulls); `cargo test --locked -p ffe-core --lib equipment::tests` (16 passed); `cargo test --locked -p ffe-rs --test fitting` (6 passed). Source records document the DOS equipment-table byte comparison and 8,192 executed-original market cases.

**Changed files:** `dashboard/burndown.js`, `docs/BURNDOWN.md`, `docs/STATUS.md`, `docs/WORK_HANDOFF.md`, `docs/notes/market.md`.

**Dashboard:** 224 total, 153 done, 71 open (59 in progress, 12 todo).

## Parked contract — existing `audio-3` launcher music bundle extension

**Gap:** audio is rendered by the game process into per-track WAV cache files. The owner wants generation moved to launcher first-run after a valid game folder is set, with one `music.bin` bundle gating regeneration.

**Acceptance:** launcher generates all 17 GeneralUser GS renders only when root `music.bin` is missing; writes it atomically; the game loads its songs from that bundle; changing the selected game folder invalidates it; existing bundle is reused; original game files and effects stay unchanged.

**Relevant files:** `crates/ffe-launcher/Cargo.toml`, `crates/ffe-launcher/src/{main,checks,config}.rs`, `crates/ffe-audio/src/{render,audio}.rs`, `crates/ffe-rs/src/sound.rs`, `crates/ffe-audio/tests/hmp_assets.rs`, `.gitignore`, `Cargo.lock`.

**Smallest verification:** audio bundle round-trip/cache-gate tests, launcher check tests, `cargo check --locked -p ffe-launcher` and `cargo check --locked -p ffe-rs --bin ffe-rs`.

**Progress:** implemented. 29 audio unit tests pass; the optimized local 17-song integration renders and packs `music.bin`, reloads every track and confirms reuse; launcher source-folder invalidation tests pass (2). Launcher/game release build passes in `target/music-bundle-test`. Linux device output remains unverified on this Windows host; OxiSynth release notices/relinking compliance remain before distribution.

## Parked contract — dashboard `content-2`

**Gap:** message arrays, journal archive/stories, viewer and the 256-byte `DATA_007648` width asset are present. F1392 model text uses F1398 width `0x23`; subscription status, F76 `Sl`/`Oo`/`Re`/`Ri`/`Rf`, and font control anchors are implemented. Other F76 macro groups and text source-oracle coverage remain open.

**Acceptance:** complete the existing message-string, journal and font scope against the source; verify the archive, viewer/macros, subscription messages and remaining font behavior with focused tests and applicable source comparisons. Keep open while any of those scoped behaviors remain unresolved.

**Relevant files:** `crates/ffe-core/src/{journal,journal_library,message_strings,script/brackets}.rs`, `crates/ffe-rs/src/{journal_page,model_viewer,scripts}.rs`, `crates/ffe-rs/tests/{message_strings,font_reference,station_view}.rs`, `docs/notes/{text,journal}.md`.

**Smallest verification:** run `cargo test --locked -p ffe-core script::brackets::` and `cargo test --locked -p ffe-rs --lib journal_page::`; then use the local content cache for ship-library and text/font source-vector checks when available.

**Current task contract:** `Cs` now copies the current-system F860 name after F75 consumes its following source byte; the archive regression shows `Gatewayand`, preserving the source spacing behavior. Remaining `Sy`/`Ci` enter F76's F858 generated-system path, try up to five generated codes, fall back to the current system, and use F860 naming plus F461/F462 in the alternate path. F858 has no equivalent in the current Rust runtime, whose `Texts.world` carries only the current system and known destinations. Do not substitute a nearby destination or start a new system-generation subsystem under this item without revising scope/estimate. `Sy` appears 67 times in the imported archive. Relevant files: `crates/ffe-core/src/{script/brackets,names}.rs`, `crates/ffe-rs/src/{navigation,scripts}.rs`, `crates/ffe-rs/tests/station_view.rs`, `docs/notes/journal.md`. Smallest verification: focused bracket tests and ignored archive census using `.cache/content-census.bin`. Keep full `content-2` acceptance unchanged. `Si` still reads `DATA_008861+0x124`; `Flight` stores only the ship model index, and native `Commander` saves have no separate ship-object string. Do not substitute `Sh`'s model display name.

**Progress:** archived JJFFE `ffe.asm` confirms F1392 calls F1398 with width `0x23` (35) for `0x3022`. `ModelViewer` reads the imported signed-width table and applies the existing F1398 wrapper. The archive draws F76 subscription strings `0xa605`/`0xa606` at x=200, colour 0xf, with the source period/expiry count. F76 `Sl`/`Oo` use F75/F921 to choose alternatives (with `Oo` recursively expanding its choice); `Re`/`Ri`/`Rf` map source score thresholds to imported rank words; `Sh` resolves the player's ship model through its statistics name code. Cache-backed tests confirm subscription strings, all Elite/Federal rank words and all 13 Imperial words; first issues across five journals expand without bracket fallbacks. The ignored cache-backed census covers all 671 imported journal articles and counts 616 bracket prefixes. It confirms archive uses of `Ci` (10), `Cr` (13), `Cs` (11), `Sy` (67) and `Zz` (8). Its sole `[Da` occurrence is not an F76 macro: JJFFE's `DATA_102307` table contains no `Da` entry (`ffedat.asm:3033-3034`). Source trace in `FUNC_000076` confirms `Cr` and `Co` both call F791 with company-name kind 12, matching the existing code; a regression now covers `Cr`. `Zz` is source-ported: it resets its F74-selected lazy-random-word slots and emits no text. `Cs` is source-ported with a cache-backed real-article regression; `Sy`/`Ci` remain unresolved because F858's generated-system search path uses F921/F853 and has no equivalent in current runtime data. Preserve the unchanged acceptance scope. The three font newline anchors import as 58/38/20 and F1747 uses them for control-code line moves; cached and synthetic tests pass. `Si`'s source object string has no equivalent in the current `Flight` or native save model; implementing it requires identifying and persisting the source value, so it remains open. This turn's full-archive prefix census passed with the local cache; it also exposed and fixed a `screen.rs` borrow mismatch that prevented compiling the test target. Model-text source-oracle and other macro behavior remain open. Dashboard unchanged: 224 total, 153 done, 71 open.

## Session 2026-11-20 (content-2, Sy/Ci)

**Contract:** port F858's generated-system search and F76's `Sy`/`Ci` argument
filters into `ffe-core`, wire them into `brackets.rs`, and cover them with
synthetic tests plus a cache-backed archive regression. `Pl`/`Sc`/`Sr`/`Si`
stay open; do not close `content-2`.

**Delivered:** `ffe_core::system_pick` (`resolve`, `system_name`,
`object_name`, the `DATA_002289/90/91` lists); `sector::Tables::
generated_system` (F858) and `search_generated` (F76's five tries + current
system fallback); `sysgen::params_for`/`seed` (F832/F869 for an arbitrary
code); `galaxy::Tables::allegiance`; `brackets::Macros`'s `select_system`,
`generated_system_name`, `system_object_name` and the `Sy`/`Ci` handler;
`scripts::Texts` gains the galaxy/system/string tables. `Sy` names via F860;
`Ci` runs F878 through `sysgen` and names an object with kind `>= 0x20`.
The archive's Sy/Ci arguments are all the `(argument)` form (none use F858);
`Imp riot`/`F edge`/`Imp edge` match the DOS argument-buffer compares (JJFFE's
decompilation wrongly compares the two-letter code there).

**Changed files:** `crates/ffe-core/src/{sector,sysgen,galaxy,lib,system_pick}.rs`,
`crates/ffe-core/src/script/brackets.rs`, `crates/ffe-rs/src/scripts.rs`,
`crates/ffe-rs/tests/station_view.rs`, `docs/notes/journal.md`.

**Verified:** `cargo test -p ffe-core` (lib 263 passed, 1 ignored; every
integration target passed), `cargo test -p ffe-rs --lib` (283 passed, 28
ignored), and `cargo clippy` with `-D warnings -A clippy::unnecessary-cast` on
`ffe-core --lib`, `ffe-rs --lib` and `ffe-rs --test station_view`. Rustfmt
`--check` and `git diff --check` are clean on the changed files. New unit tests:
`sector`/`system_pick::tests::{f858_picks_another_star_of_the_sector,
the_arguments_pick_the_sources_systems}` and the extended
`script::brackets::tests::the_script_texts_macros_expand`. Cache-backed
(`.cache/content-census.bin`): `the_ship_library_expands_sy_and_ci_systems`
and `every_archive_sy_and_ci_macro_expands` pass, as do the three
`the_ship_library*` tests and the archive census (671 articles; 67 `Sy`, 10
`Ci`). The census still counts raw prefixes, so it does not change when a macro
becomes ported; the new archive test is what proves Sy/Ci now expand.

**Remaining for content-2:** `Pl`, `Sc`, `Sr`, `Si`, and the source-oracle
gates. `Si` reads `DATA_008861+0x124`; no current runtime value exists for it.

## Parked contract — dashboard `ui-5`

**Gap:** the owner reports Save/Load/Delete appear grey. Preference tests and a captured frame show those rows enabled, but the live menu path has not been verified. This dashboard ID covers every screen's menus and text.

**Acceptance:** trace the real Escape/menu input path, verify Save/Load/Delete/New Game render active and route in applicable states, and retain the broad every-screen scope before closure.

**Relevant files:** `crates/ffe-rs/src/{main,prefs,screen}.rs`, `crates/ffe-rs/tests/prefs.rs`, launcher startup in `crates/ffe-launcher/src/main.rs`.

**Smallest verification:** `cargo test --locked -p ffe-rs --test prefs`, followed by a configured-launcher in-game check.

**Progress:** `cargo test --locked -p ffe-rs --test prefs` passes (13 tests). A bounded launcher capture and replayed Escape verify the main preference branch and active Load/Save/Delete rows; screenshot: `.cache/ui5-replay-route-bounded-20261116/start-0040.png`. Physical keyboard delivery and broad every-screen menu/text coverage remain unresolved. Resume after `content-2` or another item; do not close on the preference screen alone.

## Audio bundle implementation progress — `audio-3` scope extension

Launcher generation and game loading use a single root `music.bin`. `cargo test --locked -p ffe-audio --lib` passes (29); the local 17-track integration renders, packs, reloads and confirms reuse; launcher pack-invalidation tests pass (2); `cargo check --locked -p ffe-launcher` passes. The owner will verify the tracks in-game. Linux audio output still needs a Linux/ALSA build and device check. OxiSynth is LGPL-2.1; distribution notices/relinking compliance remain before shipping binaries. GeneralUser GS is bundled with its license; its author notes uncertain provenance for some samples.

## Parked contract — dashboard `parity-07`

**Gap:** owner-supplied in-game comparisons show that flight model 315 is often missing or off-axis, with too few visible starbursts and a redder nebula than the original. The flight adapter must preserve F1775's system orientation while mapping its camera-local placement into the renderer's view space. The intro also retains a separate nebula appearance gap in model 176's F959 path.

**Acceptance:** use model 315's source star chains and F1775 transform/light in flight; recover the original starbursts and nebula visibility/colour shown in owner captures. Compare source and port screenshots and preserve camera/world motion and palette behavior. Keep `parity-07` open until its broader Hope start/cockpit image scope is complete.

**Relevant files:** `crates/ffe-rs/src/screen.rs`, `crates/ffe-rs/src/model_viewer.rs`, `crates/ffe-rs/src/camera.rs`, `crates/ffe-rs/tests/intro_view.rs`, `crates/ffe-rs/tests/effects_view.rs`, `docs/notes/intro.md`, `docs/notes/dust.md`.

**Smallest verification:** inspect existing model 315/star-chain tests and the supplied original/reference capture path; then run focused intro and flight renderer tests with `.cache/parity07/data.bin`.

**Progress (2026-11-16):** JJFFE F1775 copies the active matrix, writes local z `0x200`, and calls F1682 for model `0x13b`. With the real new-game station orientation, the old raw `0x200` view-space placement yielded no star spans; a view-space projection depth of `0x2000`, the system star orientation, and camera-forward placement now produce a centered vertical blue-violet nebula and model 315 star-chain spans in the captured 320x200 flight frame (`.cache/parity07/flight-background-final.png`). The background uses the viewer's default light instead of local-star lighting, correcting the red tint. `effects_view::flight_background_model315_reaches_frame` passes with `.cache/parity07/data.bin`; it checks model textures and star spans reach the live flight frame. Owner screenshots still show a denser, more visible starburst field than the port capture, so visual parity is not complete. The intro model 176 nebula and broader Hope start/cockpit image scope also remain; keep `parity-07` open.

## Verified baseline — 2026-11-16

- Dashboard after latest completion: 224 total, 153 done, **71 remaining** (59 in progress, 12 not started). PAR-23, `ui-24`, `gameplay-26`, `gameplay-38`, `gameplay-40`, `world-9`, and `content-1` are closed; `release-4` remains in progress for audit scoping.
- Mixed checkout remains uncommitted. Preserve unrelated edits. `cargo clippy --locked -p ffe-rs --bin ffe-rs -- -D warnings` passes.
- PAR-23 evidence: six `source_boot` cases and five source-start comparisons pass; malformed cache/ship records and unsupported saves error; default debug/release three-frame startup smoke exits 0. Details are in its audit entry and dashboard history.


## Latest completion — dashboard `gameplay-26`

**Delivered:** completed the F998 start-record, Saker/Ross 154 hold, fitted-loadout and F2 review.

**Evidence:** existing executed-source comparisons cover start types 0/1/2/3/5. With isolated `.cache/parity07/data.bin`, the default Hope start, Ross 154 start, and F2/services hold tests each pass. Dashboard totals: 224 total, 151 done, 73 remaining; `gameplay-26` done.

**Changed files:** `dashboard/burndown.js`, `docs/BURNDOWN.md`, `docs/STATUS.md`, `docs/WORK_HANDOFF.md`.

**Next:** correct the stale space-dust deviation text and retain the source-renderer gate under `gameplay-dust`.

## Progress notes — existing open items in this batch

- `ui-7`: closed under the owner's QA authorization; live input delivery was inconclusive. Do not treat the port-only `L` shortcut or load-picker flow as part of this dashboard item.
- `ui-5`: focused Display Options corrections verified (`cargo test --locked -p ffe-rs --lib prefs`: 8 passed). Display Options menu row and page heading are white; regression coverage asserts no preference menu row uses yellow. `git diff --check` passes. Broad `ui-5` remains open; preserve those uncommitted changes.
- `flight-5`: closed 2026-11-20 (see the entry at the top). `bind.*` is applied;
  the in-game remap screen is the only part still unported.
- `gameplay-12`: with `.cache/parity07/data.bin`, `station_view::mission_offer_journal_and_comms_frames` passes. The wider item remains in progress because mission notes still identify military offer constructors and message behavior needing source-state coverage.
- `gameplay-41`: all 8 ignored `mission_scripts` tests pass with the absolute `.cache/parity07/data.bin` path. The full scope remains open for F669 event ordering and unresolved journal controls/timing documented in `docs/notes/mission-scripts.md`.
- `ui-7`: the original intro capture and port captures remain at different animation moments; they were retained locally as QA artifacts, not claimed as a source-frame comparison.
- `gameplay-dust`: 12 core dust tests and five cache-backed `effects_view` render regressions pass. The required source-backed view-space/culling comparison remains unavailable; leave in progress.
- `gameplay-34`: its three non-fixture player-event tests pass; the 96-case source comparison requires missing `FFE_SERVICE_RECORDS`. Source audit still lists missing combat-score state and F952/F953/F950 routes; stays in progress.
- `release-4`: audit scoping started; no release workflow or package outputs were found, and packaging acceptance is not defined. Dashboard now marks it in progress; no completion.
- `gameplay-39`: two autopilot control tests, four cache-backed runtime tests and the focused cancel test pass; keep in progress because linked PAR-11 still lacks avoidance, moving-target pursuit, rotating-parent guidance and event-cadence replay.

## Partial progress — dashboard `ui-5`

The Escape menu's Load/Save/Delete/New Game rows now route to active workflows. Focused menu routing and save deletion pass; `cargo test --locked -p ffe-rs --test prefs` passes all 13 tests after correcting the stale intensity click and yellow-pixel assertion. A configured release-launcher run reached the landed game, but injected Escape presses after explicitly focusing the game window did not visibly pause or open preferences. This does not establish whether the application or automation input delivery is at fault; live menu verification remains incomplete. Broad `ui-5` stays open. Preserve the separate Display Options behavior in `prefs.rs`, `tests/prefs.rs`, and `docs/notes/display.md`.

## Partial progress — dashboard `flight-6`

**Gap:** the port had distance caps that switched the console acceleration step near planets, stations and starports. Original JJFFE F208 applies the selected step with no distance check, and F1013 changes takeoff mode by altitude without changing time acceleration. The caps are removed (owner, 2026-11-18); time returns to 1x only through the source's F217 events (docking completion, station/pad state, event messages, jump arrival, death).

**Acceptance:** the selected meter step is never limited by distance, on takeoff, in manual flight or under the autopilot; the F217 resets match the source; record the source comparison.

**Relevant files:** `crates/ffe-rs/src/main.rs`, `crates/ffe-rs/src/console.rs`, `docs/DEVIATIONS.md`, `docs/NAVIGATION.md`.

**Smallest verification:** `cargo test --locked -p ffe-rs --lib console`.

**Verified:** JJFFE source comparison recorded; player-selected acceleration is no longer forced to 1x or distance-capped anywhere (EXP-021 withdrawn, 2026-11-18). `cargo test --locked -p ffe-rs --lib console`, `cargo check --locked -p ffe-rs --bin ffe-rs`, `cargo clippy --locked -p ffe-rs --bin ffe-rs -- -D warnings`, `rustfmt --check` for `console.rs`, and `git diff --check` pass. `flight-6` remains open for its wider time-acceleration/RNG scope.

## Partial progress — dashboard `gameplay-39`

The two autopilot control tests, four cache-backed runtime tests and focused cancel test pass. Leave in progress: linked PAR-11 still lacks avoidance, moving-target pursuit, rotating-parent guidance and event-cadence replay.

## Partial progress — dashboard `gameplay-12`

The focused communication/messages frame test passes with the local content cache. Military offer constructors and remaining message behavior are still noted as gaps; leave in progress.

## Partial progress — dashboard `ui-7`

The 9 front-end tests pass and the intro-reference target compiles. With Skip Intro disabled, the configured release app reaches the animated intro splash/credits, but live key `1` did not visibly leave the intro. Original key `4` maps to Commander Load; direct `L` is a port-only shortcut and not an acceptance requirement. The key-4 handoff still needs verification. DOSBox-X ran `game/FIRSTENC.EXE` from a read-only mount and saved `.cache/ui7-original-20261116/dosbox-intro.png`; `--capture-intro` produced 13 port frames in `.cache/ui7-port-20261116` (including a model-176 undefined child-basis warning). The captures are from different intro moments, so a matched source-frame comparison remains outstanding. Leave in progress.

## Partial progress — dashboard `gameplay-34`

The three non-fixture player-event tests pass. The 96-case source comparison cannot run without `FFE_SERVICE_RECORDS`; the source audit still lists combat-score state and F952/F953/F950 routing gaps. Leave in progress.

## Partial progress — dashboard `gameplay-dust`

**Gap:** dust now spawns, ages and reaches the renderer, but `EXP-020` still says it is not drawn. Full source lifecycle/view-space culling parity has no focused render comparison.

**Acceptance:** correct the stale deviation text to reflect the current render path, and keep `gameplay-dust` open until a source-backed view-space/culling comparison passes.

**Relevant files:** `crates/ffe-rs/src/main.rs`, `crates/ffe-rs/src/screen.rs`, `crates/ffe-core/src/dust.rs`, `docs/DEVIATIONS.md`, `docs/notes/dust.md`.

**Smallest verification:** source/core dust unit tests are already recorded as passing; `git diff --check` for the documentation correction.

**Progress:** corrected the stale `EXP-020` statement: the renderer submits model `0x6d`, while source-equivalent view-space/culling behavior remains unverified. `git diff --check -- docs/DEVIATIONS.md` passes. `gameplay-dust` remains in progress pending the source-backed renderer comparison.

## Partial progress — dashboard `parity-01`

**Gap:** F938/F929 reserve consumption and repair cadence need full executed-source replay across idle, thrust, firing, and acceleration rates; the audit identifies whole-second event timing and wrapping arithmetic as the key parity gates.

**Acceptance:** compare event counts, reserve, warning time, restored thrust limits, shields and repair across the audit's 1x/10x/10000x cases; preserve source arithmetic and close only if the full trace evidence exists.

**Relevant files:** `crates/ffe-rs/src/main.rs`, `crates/ffe-rs/src/flight.rs`, `crates/ffe-rs/src/combat.rs`, `crates/ffe-rs/tests/damage.rs`, `docs/audits/gameplay-parity-2026-10-26.md`.

**Smallest verification:** `cargo test --locked -p ffe-rs --test damage`.

**Progress:** `cargo test --locked -p ffe-rs --test damage` now passes 18 tests after correcting the service-roll fixture under `gameplay-34`; no F938/F929 logic changed. PAR-01 still needs the source-backed multi-rate event trace, so remains open.

## Closed 2026-11-20 — dashboard `v0.48-8`

See the closure at the top of this file. The saved `--surface-draw` table (56 cases,
54 compared, 47 beyond 1.4 radii) is the Windows-runnable planet-scale oracle,
reached through `surface_draw_reference::executed_reference_planet_surface_draw` and
`tools/reference/verify-surface-planet.py`. The full 512-case table is Linux-only
and recorded as the accepted gap in `docs/ACCEPTED_PARITY.md`.

## Historical UI-5 check (before configured-launcher capture)

Preference tests passed 13 cases and `--capture-prefs` produced a main-page frame with active Load/Save/Delete/New Game labels. The configured-launcher and scripted `--capture-start` results are recorded in the active contract above.

## Partial progress — dashboard `parity-07`

**Gap:** the Eagle's real muzzle origin is behind the front camera and model 156 expands into a screen-filling fan; the Saker currently shows one narrow ray. The owner requests the same single-beam presentation for all ships, with appearance still driven by the equipped laser.

**Acceptance:** every ship's front view shows one narrow beam, never a screen-wide fan or two fixed rays. Preserve model 156, the active mount's laser code/type appearance and the real simulation muzzle/hit origin; any camera-depth adjustment is render-only. Keep `parity-07` open for its broader Hope start/cockpit-image scope.

**Relevant files:** `crates/ffe-rs/src/screen.rs`, `crates/ffe-rs/tests/combat_view.rs`, `docs/notes/combat.md`.

**Smallest verification:** with the local cache, run `combat_view::f1776_model_156_draws_in_front_and_external_cameras` and `cargo test --locked -p ffe-rs --lib beam_view_tests`.

**Progress:** `draw_beam` moves only any behind-eye rendered origin to 0.33 flight units ahead of the camera; beam code/length and simulation muzzle remain unchanged. The ignored Eagle/Saker/external comparison passes, including an Eagle-vs-Saker pixel-footprint bound and laser-code variation checks. `beam_view_tests` (1) and the ignored 60 Hz firing/damage sequence (1) pass with `.cache/parity07/data.bin`; `rustfmt --check` and `git diff --check` pass. `--locked` cannot run because the existing lockfile omits 16 audio dependencies; unlocked resolution was used for checks and `Cargo.lock` was restored unchanged. The JJFFE source shows model 156 at the muzzle with the active laser code/length, but does not confirm a pixel-exact cockpit width; no original reference binary is available for image comparison. `parity-07` stays in progress.

## Partial progress — dashboard `gameplay-34`

The F934 regression fixture now uses one day remaining before the decrement and a fitted drive; its assertions accept the first deterministic equipment-or-drive failure and match the resulting state. `cargo test --locked -p ffe-rs --test damage` passes all 18 tests. No service logic changed. Keep `gameplay-34` in progress: the 96-case source trace and remaining F952/F953/F950 routes are still incomplete.

## Partial progress — dashboard `world-8`

**Gap:** system/galaxy map navigation still lacks the original details page (F842), picture mode (F844-F849), string templates, hover-pick, lit-sphere branch and drawn hyperspace transit scene.

**Acceptance:** finish the remaining source behaviors and compare the affected map state/frames against the executed original; keep the broad ID open until those gates pass.

**Relevant files:** `crates/ffe-rs/src/galaxy_map.rs`, `crates/ffe-rs/src/map_keys.rs`, `crates/ffe-rs/src/screen.rs`, `crates/ffe-rs/tests/galaxy_map_view.rs`, `docs/notes/galaxy-map.md`.

**Smallest verification:** inspect existing map frame tests and available reference/cache artifacts before choosing a focused change.

**Progress:** all five ignored `galaxy_map_view` tests pass with `.cache/parity07/data.bin`. They cover the map render, console-page states, key script, destination/jump frames and nearest-star selection. The notes still list the original details and picture modes, map hover pick and lit-sphere branch as unfinished; no source reference executable is available here. `world-8` stays in progress.

## Parked contract — dashboard `flight-6`

**Gap:** the manual takeoff acceleration correction is verified, but the broader time-acceleration/RNG item lacks parity traces across rate boundaries, frame rates, pauses and focus changes. PAR-12 defines the unresolved scheduling and event-ordering gate.

**Acceptance:** compare identical scripted inputs and RNG consumption at 20/50/60/144 fps, pauses, focus loss and acceleration boundaries; match source timer domains and event ordering. Keep the item open until the complete trace passes.

**Relevant files:** `crates/ffe-rs/src/main.rs`, `crates/ffe-rs/src/original_flight.rs`, `crates/ffe-core/src/player.rs`, `crates/ffe-rs/tests/flight_reference.rs`, `tools/reference/headless.c`, `tools/reference/verify-flight.py`, `docs/audits/gameplay-parity-2026-10-26.md`.

**Smallest verification:** run the focused `flight_reference` comparison with the executed-source vectors and world cache.

**Progress:** the F208/F1013 manual takeoff portion is source-compared and verified; the distance caps are removed entirely. The checkout has the original source (`jjffe-master`, JJFFE 2.8a7 assembly) and a built reference at `.cache/jjffe-reference/jjffe-smoke.exe`; the broader `flight_reference` gate was not run for this change. `flight-6` stays in progress. `gameplay-12` remains in progress for the separate military-offer/message gaps documented above.

## Readiness queue — not additional active work

| Existing ID | Readiness / unresolved gate |
|---|---|
| `gameplay-26` | Source starts 0/1/2/3/5 matched original fields. Review F2/hold behavior and affected old tests after correcting Ross 154 capacity. |
| `ui-7` | Intro oracle passed 76 cases. New source start choices and native load picker need UI/flow validation. |
| `flight-5` | Closed 2026-11-20: `bind.*` remap applied; only the in-game remap screen remains out of scope. |
| `gameplay-34` | Servicing matches 96 original cases. F952 cargo gate and F953/F950 RNG/hit routing remain unfinished: NOT ready to close. |
| `gameplay-dust` | Needs source lifecycle/view-space culling audit; do not treat existing rendering tests as full source parity. |
| `gameplay-12` | Military offer constructors need target-ship and military-base state absent from the port; this expands the subsystem and requires source-state comparisons. |
| `release-5` | Clean-machine player-data acceptance is not defined by the bounded project evidence; keep not started until its actual import path is identified. |
| `release-4` | Audit scoping started; package outputs and acceptance criteria remain undefined. License and owner sign-off are separate open items. |

## Unfinished work to preserve

- Startup changes include source starts 2/3, corrected type-1 hold/general bits,
  source key 4 load picker, source key 5 start, supporting frames on pad loads,
  and per-commander UI/script reset. These extend beyond PAR-23; do not bundle
  their completion claims into that item.
- Events changes add persisted F934 servicing countdown and exact RNG/counter
  behavior, correct F933 to 29 attempts, and move shared player events outside
  the flight-only branch. `tests/player_events.rs`: four cases passed, including
  96 original F934 dispatches. Full old-suite compatibility is unverified.
- Earlier persistence work added `persist-6` through `persist-25` as completed
  subtasks. Those additions did not reduce the original backlog. Do not count
  them again or rewrite history to conceal the scope mistake.
- Detailed local investigation: `.cache/backlog-78-to-68.md` (historical and
  partially stale). Original captures: `.cache/start-records.jsonl` and
  `.cache/service-records.jsonl`; reference binary under `.cache/jjffe-reference`.
  Keep originals/captures/binaries out of commits and external services.

## Handoff discipline

Keep one active contract. Replace its progress notes after each meaningful
result. Do not expand this document into another port log. When switching chats,
provide this file and the selected ID rather than the entire conversation.

## Parked investigation — dashboard `gameplay-41` (bounded F669 event-order investigation)

**Gap:** F669 attacker-creation event 0x17 is currently raised after the script run; source notes identify this as a remaining ordering difference. Full item scope also includes mission-script import, journal behavior and scripted missions.

**Acceptance:** establish event 0x17's source statement timing and, if the runtime architecture supports an exact fix, raise it at the matching action point with a regression. Do not close `gameplay-41` unless the journal controls/timing and full scripted-mission acceptance are also verified.

**Relevant files:** `crates/ffe-rs/src/main.rs`, `crates/ffe-rs/src/scripts.rs`, `crates/ffe-rs/tests/mission_scripts.rs`, `docs/notes/mission-scripts.md`.

**Smallest verification:** `cargo test --locked -p ffe-rs --test mission_scripts -- --ignored` with absolute `FFE_WORLD_CACHE` set.

**Progress:** investigating whether the existing script hook can emit 0x17 synchronously during attacker creation. No code or dashboard status changed yet.

**Investigation result:** synchronous F669 event delivery would require materializing attacker objects during interpreter execution; the current host only queues abstract ship actions, and `apply` materializes them after the run. This is a runtime architecture change, not a small ordering patch. No code changed; keep `gameplay-41` in progress and retain the known journal control/timing gaps above.


## Latest completion — dashboard `world-9`

**Gap resolved:** the dashboard note predated source shade-table loading and F1535 class selection. The current renderer uses the local star vector on each required object path and the planet terminator follows the same system light.



**Relevant files:** `crates/ffe-rs/src/main.rs`, `crates/ffe-rs/src/screen.rs`, `crates/ffe-rs/src/model_viewer.rs`, `crates/ffe-rs/tests/planet_view.rs`, `crates/ffe-rs/tests/intro_view.rs`, `crates/ffe-core/src/model_lighting.rs`, `docs/notes/palette.md`.

**Smallest verification:** run `planet_view::local_star_light_changes_the_scene` and `planet_view::planet_surface_has_a_day_night_terminator` with the absolute local `FFE_WORLD_CACHE`; inspect the flight render routes for ship/station/lift use of `scene_light`.

**Delivered:** source-signed local-star lighting for ships, stations and lifts; F1535's six imported shade tables selected from the active star class; planet day/night shading.

**Evidence:** with `.cache/parity07/data.bin`, `planet_view::local_star_light_changes_the_scene` and `planet_view::planet_surface_has_a_day_night_terminator` pass; `ffe-core`'s `model_lighting` test passes. The extended `intro_view::returning_to_intro_drops_the_flight_star_tint` test verifies all six imported shade rows and Ross 154's flagged class `0x81`. Main routes the same F1481-signed light through ship, station and lift draw paths. Dashboard totals: 224 total, 152 done, **72 remaining**; `world-9` done.

**Changed files:** `dashboard/burndown.js`, `docs/BURNDOWN.md`, `docs/WORK_HANDOFF.md`, `crates/ffe-rs/tests/intro_view.rs`.

**Note:** classes 6 and 7 index past the six defined source shade tables; the port retains its documented default-shade fallback for those out-of-table values.

## Session 2026-11-20 - enhancement-3/4/5 (optional autopilot toggles)

**Scope:** implement the three autopilot enhancements in `docs/notes/enhancements.md`; do not change dashboard statuses or `Cargo.lock`.

**Delivered:** `Navigator::source_command_with(cache, flight, Enhancements)` (the existing `source_command` stays the default/off wrapper). Enhancement-3 gates the port's existing detour (`detour_waypoint`/`waypoint_detour`) behind `autopilot_waypoint`; it was always on before, so off restores F741/F742's straight line. Enhancement-4 adds `ffe_core::autopilot::{thrust_acceleration,retro_thrust,braking_distance}` and `navigation::braking_guidance`: when the range to a planet/moon's collision radius is inside `v^2 / 2(a_retro - g)` (F601 continuous per-word acceleration, F603 pull from the body's mass), the event commands full retro thrust with a permissive F601 cap. Enhancement-5 adds `Navigator::every_body_parent`, which tests every starport/station/planet/moon in range and keeps the deepest penetration instead of F569's preference order; `main.rs` selects it at the flight scan. All toggles are read from `stored_config.enhancements` in `main.rs`; off is the previous source path. DEVIATIONS: EXP-043/044/045.

**Evidence:** `cargo test -p ffe-core` (270 lib + every integration target passed), `cargo test -p ffe-rs --lib` (299 passed, 28 ignored), new synthetic cases `navigation::tests::{the_waypoint_detour_only_applies_with_the_enhancement, the_braking_enhancement_brakes_from_retro_thrust, the_local_gravity_points_toward_the_body, every_body_collision_scan_finds_the_deepest_body}` and `autopilot::tests` x4. `cargo check -p ffe-rs --lib --bin ffe-rs`, core/ffe-rs lib clippy `-D warnings` and `rustfmt --check` on `autopilot.rs`/`navigation.rs` pass. `tests/autopilot.rs` ignored cache cases (4) pass. `cargo fmt --check` reports only other developers' unformatted `missile.rs`/`main.rs` hunks (ECM/NPC-speed work).

**Limits:** the braking override covers body targets, not stations or ship targets; F603 gravity is the target body's own mass. Enhancement-5 still runs one F569 scan per frame and chooses the deepest intersecting body; testing every model stream simultaneously would need multiple integration steps.
