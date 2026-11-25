# Port log

## 2026-11-21 - release-1 closed: nine-stage playthrough checkpoints

`release-1` is `done` as accepted at about 90% (242 total, 241 done, 1 remaining:
the owner's sign-off). Three new reference modes, three Rust tests, three
verifiers and one chained runner.

- **`--trade 220 --start-key=0x04`**: the real Lave orbital-station game to frame
  200, then F555 (buy) and F556 (sell) on `DATA_008804` with fixed market entries
  (the pointer at `+0x48f8`), cash, hold and stock. Ten cases: a three-tonne
  purchase, a two-tonne sale, and `0x98e1` cash short, `0x98e2` negative price,
  `0x98e3` hold full, `0x98e4` no stock, `0x98e5` nothing to sell, plus the two
  partial runs. `crates/ffe-rs/tests/market_trade.rs` rebuilds the market and
  hold and replays every case through `Market::{buy,sell}`; all ten match,
  including F556's quirk that `0x98e5` is both its initial value and its refusal.
- **`--save-roundtrip 260 --start-key=0x04`**: F214, F170 and F174 write a
  commander file (7,346 packed bytes, 14,672 with F69/F65) at frame 240; every
  saved field is changed; F176 and F171 must restore it. The reference's
  in-process check passes, and `crates/ffe-rs/tests/save_roundtrip.rs` imports
  the same file with `saves::legacy_import`: name, 1,000 CR, 49 t capacity, 4 t
  used, hydrogen 4, clock, pair, player object, the ship (38) and registration,
  and all 38 cargo words match the original's before row. The importer's
  string-section `commander_name` is two bytes short on this JJFFE-written file
  (accepted gap; the player block's name copy is exact).
- **`--jump 2000 --start-key=0x39`**: Tab launches from the Hope/Gateway pad; in
  free flight at frame 400 the real F908 runs with Titican selected (code
  `0x12a53716`) and the F34/F35 dialog answered yes, F1530 returning `0x1000` so
  the jump is normal. F901's plan: mass 28, class 2, distance 442, 189,322 time
  units, 1 t of hydrogen. `crates/ffe-rs/tests/jump_reference.rs` checks the
  same distance, fuel, jump time, 2.89 days and transit through
  `Navigator::hyperspace`, and the 7.4..11.3 AU arrival orbit. The arrival draw
  ledger and the map cursor are accepted gaps.
- **`tools/reference/verify-playthrough.py`** chains nine stages — import/start,
  intro, launch/fly/dock, trade, save/load, jump, combat, missions/BBS, render —
  each through the existing or new verifier and its port test. All nine pass
  (about 70 s). `verify-all.py` now has 66 oracles, including the three new
  ones; each new verifier runs the reference twice (byte-equal traces), the port
  test, a tampered trace that must fail and an empty trace that must fail.

**Evidence**: `verify-trade.py`, `verify-save-roundtrip.py`, `verify-jump.py`
and `verify-playthrough.py` all exit 0; the three tests pass with
`.cache/ui11/data.bin`; `cargo test -p ffe-core` and `cargo test -p ffe-rs
--lib` pass; `python ci/check_burndown.py` reports 242 items, 241 done, 1 open.
Docs: `docs/notes/playthrough.md`, `docs/notes/hyperspace.md`,
`docs/ACCEPTED_PARITY.md`, `docs/{BURNDOWN,STATUS}.md`, `dashboard/burndown.js`.

## 2026-11-21 - gameplay-8, parity-20 and parity-22 closed: combat, day and missile replays

`gameplay-8`, `parity-20` and `parity-22` are `done` as accepted at about 90%
(242 total, 240 done, 2 remaining). Three new harness modes and three Rust
tests, plus `verify-*-replay.py` in `verify-all.py`.

**`gameplay-8`.** `--combat 900 --start-key=0x39` starts the Hope new game, puts
the player on the pirate leader's forward axis (`FFE_COMBAT_OFFSET`, 20 km by
default and 2 km in the verifier) and lets the original F750/F754 AI and F931
run. One JSONL row per frame carries every hostile ship's whole 0x152-byte
record, the player and the effect objects. `crates/ffe-core/tests/combat_replay.rs`
checks that the hostiles change state (4, 1, 2, 11), the player fires, hostile
pulse hits take the player's hull from 112 to 80 and F939/F942 effects appear;
it replays the leader's F750 guidance and F727 physics and reproduces 398 of 399
thrust words, 399 of 399 caps and positions and 396 of 399 velocities.

**`parity-20`.** `--daily 2000 --start-key=0x02` runs the Ross 154 pad at 10000x
and logs every F922/F1530 draw. Frame 406 makes F633's 125 draws: the first
twelve limits `[25,0,25,0,8,3,12,12,1,0,14,25]`, 105 F1530 steps (99 market
steps plus six news/ship-queue steps), pair `1273017021/228009932 ->
1772616542/3616182582`. `crates/ffe-rs/tests/daily_reference.rs` replays every
pair step and runs `Navigator::tick_days` from the same pair; the final pair,
the changed board and the job list match. The per-station market streams remain
EXP-006.

**`parity-22`.** `--missile-encounter hit|ecm|naval|mine|escape` traces the
player, hostiles, missiles and F719/F942 effects per frame.
`crates/ffe-rs/tests/missile_encounter.rs` checks the model-6 hit (target hull
300 -> 100) and F719, standard-vs-naval ECM (model 6 destroyed at 505; model 7
survives the standard burst and dies at the naval burst), the 0xc1 model-5
state-0x14 mine, and F1098's model-16 Stowmaster with the old model-31 hull in
state 0x16. `Navigator::tick_missiles_inner` now spawns F719 on a hit, decoy
contact or timeout and F942/F926 on a missile-killed NPC; the port-side
hit/timeout test passes. Accepted gaps: F754 state-1/2 combat, whole-encounter
NPC-vs-NPC missile chains, the rotating-frame rendezvous, the legal witness
trace and no F719 pixel comparison.

Evidence: the reference was rebuilt; all three verifiers pass with two
byte-equal reference runs; `ffe-core` 331 lib + every target, `ffe-rs --lib` 325
passed/34 ignored, `ffe-rs --bin` 3 passed, `cargo check -p ffe-rs --bin`,
rustfmt and clippy clean on the changed files. `check_burndown.py` reports 242
items, 240 done, 2 open.

## 2026-11-21 - v0.48-14 and persist-4 closed: a whole-frame scene oracle and the legacy commander replay

`v0.48-14` and `persist-4` are `done` (242 total, 237 done, 5 remaining).

**`v0.48-14`.** `tools/reference/headless.c` gained `--scene-frame`: a fixed station (model 74) and three ships (models 31, 23 and 25, the Saker, Eagle and a third hull) at fixed positions are drawn by the executed original's real F1537 object loop, F1680/F1772 instance and model execution, F1330/F1331 palette build and F1789 depth-sorted flush into the 320x200 framebuffer. The mode writes `scene-frame.bin` (64,000 indexed pixels + 768 palette bytes) and one JSONL row with the exact camera, each object's matrix/position/scale/64 globals, the tics, the light and the shade table. `tools/reference/verify-frame.py` runs it twice (frames and rows byte-equal) and runs `crates/ffe-rs/tests/scene_frame.rs`: the port rebuilds the same scene through `model_animation::setup_object` (F1680) and `ModelViewer::render_scene_with_instance_scale` (F1772), applies the same F1788 keys and draws through `Screen::scene_frame_draw` (the flight view's rasterisers, added as test support). 63,946 of 64,000 RGB pixels are identical (97 shapes); the accepted 54 pixels are the original's one F1747 name-board text record (the viewer counts it in `Frame::texts` but does not draw text) and a two-pixel depth tie on the station's lower edge. The verifier also rejects a moved object, a shifted frame, a blank frame and an empty trace.

**`persist-4`.** `--legacy-save <path>` loads `game/FIRSTENC._01` through the real F70 (F69 loose globals), F66 (the F65 stream) and F171 (magic, F169 checksum, F166 zero-run expansion) and dumps one JSONL row: the container offsets, 14 commander fields and the raw 0x984a object array and player block as hex. `tools/reference/verify-legacy-save.py` requires the original twice, compares `saves::legacy_import` field for field and byte for byte, and rejects altered fields/raw bytes/offsets and an empty trace. The established field map (recorded in `docs/notes/persistence.md` 1.2.1) is the source `gmstruct` layout: clock `+0x000/+0x004`, the RNG pair `+0x024/+0x028`, the player object `+0x0c0`, credits `+0x114` (10,000), hold `+0x118/+0x120` (7/1), fuel `+0x15a/+0x15c/+0x16e` (1/0/0) and the name `+0x1bc/+0x1d2` (Jameson/JAMESON). The player object (87) is model 31, the Saker Mk III, registration `TY-198`. Two decode fixes: `F170`'s scatter is 0x200 bytes = 0x40 every-other dwords, not seven (the old count shifted the player block and its text, which is where the `+0x22e` name observation came from), and the DOS player section is 0x5535 bytes, not JJFFE's 0x556a (JJFFE's F171 over-reads 0x35 bytes of the following string section; the import decodes the file's 0x5535-byte block and then the 135-byte string block). Evidence: `ffe-rs --lib` 325 passed/34 ignored, `ffe-core --lib` 331 passed/1 ignored, the FIRSTENC ignored unit test and the new `legacy_save`/`scene_frame` tests, `cargo check -p ffe-rs --bin ffe-rs`, rustfmt and clippy clean; both verifiers are in `verify-all.py`.

## 2026-11-20 - gameplay-6/7/18 closed: agents, encounters and the NPC flight AI

`gameplay-6`, `gameplay-7` and `gameplay-18` are `done` as accepted at about 90% (242 total, 233 done, 9 remaining). They share one evidence base: the hunting AI (F750/F753/F755/F756/F749) already replays the executed original (`npc_pirate_replay`, `police_replay`), F724 fires every frame, F953 damages the player and F674/F675/F680 carry a close pirate across a jump. This pass closed the tractable remainders. **F659:** `police::OPEN_AIR_LAUNCH`/`STATION_LAUNCH`/`OTHER_LAUNCH` are `DATA_006202`..`006209`; `launch_offset` selects the open-air table by the starport's bay count (`+0xce`), applies F1669's axis rotation and the `<< 10`, and `Navigator::make_police` adds the body's position, replacing the fixed far-side 270 m. **F233:** F909 broadcasts event 1 on arrival and F233 clears `DATA_008647`/`008646` bit 0, so `Navigator::jump` calls `Legal::clear_alert`. **F770:** state 0x09 is `State::Trading`; it flies at `+0xfe` without hunting or firing, and the existing `scripts::approaches` keeps F770's class-0xa transition, event 0x17 and the `0x98d0` hail. **Missile rows:** `missile::AI_ROWS` transcribes 0x13-0x1b with each row's control/movement routine; `Missile::tick_with` runs 0x13's F758 guidance/fuse/60-update lifetime, 0x14's F716 contact-only mine and 0x16's F767/F713 seeker with removal at `2^26` mm. **Ground collisions:** `NpcShip::hits_ground` plus `keep_ships_out_of_worlds` destroy an attacking ship that reaches a world as state 0x1c with F942's explosion, while ordinary traffic keeps the lift-clear stand-in. Accepted gaps are in `ACCEPTED_PARITY.md`: F702's orbit, F597's accelerated placement (`parity-11`), F589/F671's orientation/pad settle, EXP-023's beam box, F770's approximate approach motion and F941's missile selection. Verified: `ffe-core` 331 lib + every target, `ffe-rs --lib` 325 passed/34 ignored, cache-backed `npc_ai` 8, `npc_fire_view` 1, `npc_view` 3, `station_traffic` 1 and the new `save_tests`. A test-only repair was needed in the shared checkout: `station_record` now falls back to the live traffic half when a navigator has no physics list (a synthetic/test start), which the object-record pass had made return no record.

## 2026-11-20 - ui-11 closed: F1680's far/dot/cull rule for every object of the flight view

`ui-11` is `done` as accepted at about 90%. F1680 decides an object's drawing from its model descriptor — at or past the far class (`+0x20`) nothing, from the dot class (`+0x28`) an F1700 dot, otherwise the model — and F1686 culls the bounding sphere. The port had that only for the planet discs; the source's F1536/F1537 loop sends every object through it. `Screen::f1680_draws_model` is now the shared check and `draw_system_ships` (ships and missiles) and `draw_system_station` use it: a ship is a model close up, a glint from its dot class and absent past its far class. `far_objects` (4, local) adds the station/ship boundaries (14,954/17/0 pixels for station 74 at 2/600/140,000 km; 21/0 for Saker 31 at 9/5,000 km) and walks the 58 bodies of the four start systems at their dot classes: the largest dot is 7 pixels, so F1700's unported lit shaded sphere (9+) is unreachable and the model stand-in there is unobservable. The 26 `planet_view` approach cases, the executed-original near-pad comparisons and every other cache-backed view test pass. Remaining: F1680's rotating-frame branch (`world-13`), the textured surface fills, and no matched whole-frame approach capture.

Note: the local `.cache/parity07/data.bin` is cache v34 and has no `base:world/object-dots`, so the far/dot tests cannot run on it; a v35 cache was rebuilt from `game/` into `.cache/ui11/data.bin`.

## 2026-11-20 - gameplay-42 closed: broken missions fixed or documented

`gameplay-42` is `done`. A static audit (`docs/notes/broken-missions.md`) of the 105 mission scripts flagged 9 high-confidence broken missions and 1 medium. Five are now fixed as always-on bug fixes, applied in memory by `ffe_core::script` `CORRECTIONS` (original data untouched): mission 78's `0x217`/`0x197` article test (EXP-030), mission 55's missing `0xa963` response (EXP-031), mission 20's tenfold journal debits (EXP-032), mission 81's menu-cleared reward trigger (EXP-033), and mission 83's swallowed `goto 7` via the one-operand `0x2b` reading (EXP-034). Five are documented not-patched because their intended starter is absent from the source (37/38/39 and 90/91; EXP-035) — inventing a trigger would be new gameplay. `ffe-core` 441 tests, `ffe-rs --lib` 280, and the 9 ignored `mission_scripts` cache-backed tests pass.

## 2026-11-20 - persist-1 closed: legacy commander import

`persist-1` is `done`: `game/FIRSTENC._01` (the supplied original commander file) is parsed read-only. It is a DOS-era FFE file in F175's layout (F69 loose block, F65 stream, F170 body), not a plain JJFFE F65 file. `saves::legacy_import` validates the F170 magic/checksum (`0x81756ACA`) and decompresses all five sections exactly to EOF, recovers the commander name (`Jameson`), system (`Gateway`) and save identity, and refuses malformed/truncated input. The full gameplay-state field map (credits/ship/cargo/clock/RNG) is deferred to `persist-4`: the DOS player-block offsets differ from `ffe.asm` (name at `+0x22E`, not `+0x115`) and must be established against the reference build.

## 2026-11-20 - v0.48-17 closed; parity-21 and world-8 progressed

`v0.48-17` is `done`: the model census executes 448 of 466 slots (floppy 1.02 and CD 1.01); the remaining 18 are 16 child-only programs (verified to run via their real parent call), model 0x59 (truncated identically in both releases), and null slot 149. The CD 1.01 census (extracted locally from the supplied ISO) matches floppy per slot. `parity-21` advanced in `ffe-core`: F1512/F1511 integer movement now advances every effect and the F928/F927/F924 slot table is implemented and unit-tested (F926 pod drift corrected to source F1530 signed words); the EffectTable is not yet wired into `Navigator` (the live game) and the frame-by-frame trace remains, so it stays in progress. `world-8` gained F1347/F830 template expansion for the map's fuel/sector/distance/description strings (including the "1t" tonne suffix); the hover-pick is a port-only follow-up in `main.rs`, the lit-sphere branch is blocked on `model_dot.rs`, and the drawn hyperspace scene remains, so it stays in progress.

## 2026-11-20 - MIT licence chosen; GeneralUser GS bundling confirmed

`release-6` is `done`: the project's Rust code is now MIT (owner decision). `LICENSE` replaces the placeholder, `license = "MIT"` is set on the workspace and inherited by all seven crates, and `docs/LEGAL.md` now states the licence scope (the new Rust code only — not the game, its assets, the JJFFE/FFED3D reference, the bundled GeneralUser GS SoundFont, or third-party dependencies). The owner also confirmed the GeneralUser GS SoundFont stays bundled with its licence (`crates/ffe-audio/assets/soundfont/`), accepting the author's note that some sample provenance is uncertain. The remaining release decision is the LGPL-2.1 `oxisynth` static-linking relinking arrangement.

## 2026-11-19 - Four more open items closed (owner request)

`gameplay-2`, `gameplay-11`, `flight-8` and `world-6` are `done` as accepted at about 90%: 228 total, 192 done, 36 remaining. Evidence and gaps are in
`docs/ACCEPTED_PARITY.md` ("Third batch"). Two stale notes were corrected (`equipment.md` on mining deployment and passenger cabins, `station.md` on F636/F637/F641).

## 2026-11-18 - Ten more open items closed (owner request)

`world-4`, `-5`, `-7`, `gameplay-1`, `-3`, `-13`, `-25`, `-34`, `release-4` and `release-5` are `done`: 228 total, 187 done, 41 remaining.
Eight rest on oracles and tests re-run today (list and gaps in `docs/ACCEPTED_PARITY.md`, "Second batch"). New: `tools/clean-machine-check.ps1`
(release-5), `tools/third-party.py` with `docs/THIRD_PARTY.md` and a CI check, and the package now carries the third-party
notices (release-4). The LGPL relinking question and the sound bank's provenance are left as owner decisions. Also fixed:
`ui-26` to `ui-28` had the status `not-started`; the dashboard's word is `todo`.

## 2026-11-18 - F633 on the shared pair; PAR-17 and PAR-18 closed as accepted

- `ffe_core::bbs::daily_jobs` and `daily_place` are F633's job and board halves (F922(25) with F623, the placed board's replaced
  entry and flag clearing, the leaving ship, F634 for the patrol, trade and starport rows); `Navigator::pair_daily` runs them per
  new day with the place's 99 market steps between, on the shared pair, from `tick_days`. The `DATA_009001` threshold is `maximum
  control * 8 + 8`, not the maximum. Each system's boards also live in `pair_boards` (saved) so the draws keep their counts after a
  station is visited.
- Verified against the executed original: `tests/rng_ledger.rs` replays the day boundary's 125 logged draws (the harness's `--flight`
  runs now log F922 and F1530 with `FFE_RNG_TRACE`) from the pair before the day, and the port ends at the original's pair
  `69a7fb5e d78a9136`; the replay of the log itself reproduces the original's end state.
- `Navigator::day()` now changes at midnight (twelve hours into a new game, which starts at noon), as `DATA_008807` does; it changed
  twenty-four hours in, so every daily event fired twelve hours late.
- Market values from the day's 99 steps are still consumed, not read (EXP-006); gaps are in ACCEPTED_PARITY.md.
- PAR-17 and PAR-18 are closed on the dashboard as accepted at about 90% (owner decision).

## 2026-11-18 - Ten open items closed (owner request)

`flight-4`, `-6`, `-9`, `-10`, `-11`, `-12`, `-13`, `-14`, `gameplay-39` and `gameplay-dust` are `done` on the dashboard (accepted at about
90%, or source-verified for the dust); the board is 224 total, 174 done, 50 remaining. The evidence and the gaps left on purpose are in
`docs/ACCEPTED_PARITY.md` ("Linked flight and gameplay items closed 2026-11-18"). Every replay was re-run today against a fresh
executed original and passes at the figures recorded there. An earlier attempt to add ten new done tasks instead was reverted: the
board rule is to close existing items, not add scope.

## 2026-11-18 - Ten small non-parity fixes (tooling, text and docs)

Chosen from the open work items as low effort and independent of any parity gate. They are not board items (an attempt to
add them as ten done tasks was reverted at the owner's direction; the board total stays 224).

1. `ffe-rs --version` prints `ffe-rs <crate version>` (the playtest contract's build ID).
2. `ffe-rs --help` (also `-h`, `/?`) prints the options. Both are answered before any window, cache or game folder is read
   (`crates/ffe-rs/src/cli.rs`, 4 tests).
3. The unknown-argument error and `--help` share one usage string, and a test keeps the parser's flag names in it.
4. `ffe-tools export-external` no longer says MISSION.DAT decoding and HMP conversion are "not implemented" (the cache imports
   the journal; the launcher renders the music).
5. `ffe-tools --help` dropped its stale "other 16 songs / bounded native command execution" text.
6. `ci/check_burndown.py` (with `ci/test_check_burndown.py`, run in CI) fails when `dashboard/burndown.js`, `docs/BURNDOWN.md`
   and `docs/STATUS.md` disagree on counts, when the last history point is not the open count, or when a listed item's status
   differs.
7. `tools/package.ps1` stages a playtest zip (executables, LICENSE, README, CONTROLS, LEGAL, `SHA256SUMS.txt`, `PACKAGE.txt`),
   refuses to continue if player data or generated files are in the folder, and `/dist/` is ignored. It is the playtest package,
   not the `release-4` distribution audit (third-party notices for the synthesiser and sound bank are still to do).
8. `crates/ffe-rs/src/filters/xbrz.rs` says plainly that it is an xBRZ-style approximation, not the library, and drops the TODO.
9. The three display-filter documents at the repository root moved to `docs/notes/` (unreferenced elsewhere) and are linked from
   `docs/README.md`.
10. `docs/KNOWN_ISSUES.md` section 4 is marked historical with what is still true, and `laser.rs`'s comment no longer says the
    caller does not play the laser sounds (it does; see `docs/notes/sound-cues.md`).

Checks: `cargo test -p ffe-rs --lib cli::` (4), `cargo test -p ffe-tools` (all pass), `python -m unittest ci.test_check_burndown`
(5), `python ci/check_burndown.py`, a run of `tools/package.ps1 -NoBuild`, and `ffe-rs --version/--help/--bogus` on a debug build.

## 2026-11-18 - PAR-12, 18 and 17: frame-rate and resume tests, and the original's draw ledger

- **Harness bug fixed.** `FFE_ACCEL_AT` pulled F152's marker back one event by writing state offset `0x24`, which is the shared
  random word `DATA_008818`, not the marker (`DATA_008808`, offset 8). The earlier `event_replay.rs` therefore compensated for a
  pull-back the original never made and could not have failed on it. The write is now at offset 8; the test counts the dispatch
  first and the F208 write at the end of the frame, as the original does, and passes on the corrected trace (358 frames; events
  per step 1x, 10x, 100x, 1000x, 10000x: 43,203, 13, 127, 1,261, 12,601). The PAR-01 evidence about F208's pulled-back event is
  now real.
- PAR-12 is closed on the dashboard as accepted at about 90% (owner decision), gaps in docs/ACCEPTED_PARITY.md.
- PAR-12: `ffe-core` `frame_timer_tests` run one scripted 1x/10x/100x/1000x/10000x/1x schedule at 20, 50, 60 and 144 fps: the
  same 60,000 ms and 3,000 Timer50Hz callbacks, and the F152 events within 2 of the ideal; a pause adds nothing (the timer is
  not called) and a stalled window is one 0.1 s frame. Open: the prototype accumulator, event-6 controllers on Timer50Hz and
  accelerated collision work.
- PAR-18: the integer flight words never survived a save (rates, sub-unit fractions, caps, thrust words, drift, scanner). The
  save now carries `Flight::resume` (`original_flight::ResumeState`), written by `OriginalFlight::apply` and used by
  `from_flight` only while the position, velocity and rows still match what it says. `a_resumed_free_flight_matches_the_uninterrupted_one_exactly`
  resumes eight times in 400 frames of steering and gets the identical `Motion` every frame; without the words the next frame
  already differs (the control). Not carried: the pad world and autopilot guidance (no resume state while they are live), the
  steering-key hold counters (a save is made from a menu), the sub-millisecond clock carry.
- PAR-17: `--flight` runs now take `FFE_RNG_TRACE` and log every F922 and F1530 draw. The new-game pad with 10000x from frame 200:
  boot 291 draws at frame 161 and two F1530 at 162, then nothing for 244 frames, then 125 draws at the day boundary (frame 406):
  F922(25) with F623's draws, 99 F1530 for the market drift (three words a commodity), the board and ship-queue draws
  (`tests/rng_ledger.rs`). The port's `Navigator::tick_days` draws none of them on the shared pair (its markets and boards keep
  per-station streams, EXP-006), so its pair stays where it was and every later draw differs. Porting F633 on the shared pair
  (F621/F623/F634 are already ported draw for draw; F633's own loop is not) is the next step.

## 2026-11-18 - PAR-01, 05, 10, 11 replayed against the original and closed as accepted

- New harness hooks in `tools/reference/headless.c`: `FFE_FIRE_FROM`/`FFE_FIRE_TO` (the real F931 on the front mount every
  frame in the range), `FFE_ACCEL_AT=<frame>:<step>,...` (F208's own writes for 1x to 10000x, the marker pull-back included),
  the `--flight paddock` scenario (a planet start: Tab launch, Return held for frames 260-299, then F1020 engaged on the
  starport at frame 770), and `refire`/`laser_heat` in the `FFE_RESERVE_DUMP` rows.
- PAR-01: `takeoff_replay.rs` `port_firing_follows_the_original` (21 shots over 728 frames: refire and laser heat exact on
  every frame, reserve within 6; Hope within 68, one thrust event) and `event_replay.rs` (F152's events exact on 358 frames
  through 10x, 100x, 1000x, 10000x and back to 1x on the pad).
- PAR-05: `port_long_flight_follows_the_original_to_the_descent` (`--flight takeoff 4000`): the original climbs to about 6,700 km
  and falls back, never leaving the planet's frame, and strikes the ground at frame 3120; the port keeps every mode and the set
  speed exact and the speed within 0.5% to frame 2800, then falls more steeply and touches about 25 frames early.
- PAR-10: the Lave Station launch and exit flight re-run and pass (`launch_replay.rs`); no other start begins in a station.
- PAR-11: `port_autopilot_flies_the_descent_to_a_pad` replays the whole 14,934-frame autopilot descent to Ross 154's starport
  (`FFE_AUTOPILOT_AT=770`, `FFE_PAD_POSES` = the scenario's `FFE_ROWS_DUMP` stderr): 5% of the 8 km descent off in the first
  150 frames, 1.2% (50 to 65 m) after frame 1570. `replay_with` now engages the autopilot, runs F741..F746 on F152's events and
  advances the world clock once it flies.
- The four items are closed on the dashboard as accepted at about 90 to 95% (owner decision); the gaps left on purpose are in
  docs/ACCEPTED_PARITY.md. Remaining open parity items: 08, 12, 15, 17, 18, 20, 21, 22, 24.

## 2026-11-17 - Far bodies are dots (Gateway's other planets), cache v35

- Gateway's other planets (Gateway 1, 2, 4, 5, 6) did not show from Hope: they were drawn as models a hundredth of a pixel across.
  The original's F1680 draws a body as a dot from its model's dot class and not at all past its far class; the port now does the
  same. Read from `jjffe-master/ffe.asm` (F1680 at 101877, F1700 at 103407, F1801/F1802, F1609): the object flag recomputes a finer
  size, maps it through `DATA_007825` to a sprite record of -7..0 and draws the halo and core sprites.
- Cache v35 adds `base:world/object-dots` (sprites, size map and every model's far class, dot class and colour). The floppy 1.02
  bytes were checked against the executable and the source; `ffe-tools build-data` refuses a floppy cache without them.
- Assumptions: a system body's object scale (`+0x7e`) is 0 (only dust and effects set it); CD 1.01 and shareware 1.06 take the
  floppy's distances from `DATA_007835` and drop the asset if it does not validate; a lit body nine pixels or more across at its dot
  class (the original's shaded sphere) still draws its model. Pixel parity with the executed original is not measured: only the
  data, the decision (verified through `setup_object`, 32 original dots) and the glint's shape and colour against a screenshot.
- Tests: `ffe-core/tests/model_dot.rs` (object dots), `ffe-rs/tests/far_objects.rs` (descriptor values and the Gateway frame,
  local-only). The unsupported cache version in the format tests moved from 35 to 36.

## 2026-11-17 - Station docking cap, launch orientation, start locations

- Manual flight at high time acceleration no longer docks instantly: the station cap of EXP-021 now applies without the autopilot (not on a surface takeoff).
- A station launch aligns the ship to the station's spun orientation (`Navigator::spun_rows`, `align_to`), so it leaves straight with the door.
- The launcher's start location is a dropdown of the four implemented F998 starts (Gateway/Hope, Ross 154, Sol/Mars, Lave Station). `--start` no longer maps 2..=5 to Ross, and the chosen start also applies when the intro is skipped or the game restarts. An Earth start is not implemented (needs its own start spec from the source).
- The Lave Station start (type 3) no longer begins with a leftover docking sequence, so it can launch (`tests/lave_start.rs`, local-only).
- Free flight near a planet or starport kept the ship's rows at 0.985 of unit length after the first frame (a per-frame F1520 normalises to 0.992), so thrust was 1.5% low from the clear of a takeoff on; `step_scan` now divides its landing margin out instead. Takeoff replay speed error 1.25% -> 0.18%.
- The Eagle's laser beam was drawn at the wrong render scale (a red fan); `draw_beam` now uses the firing ship's scale. A one-pixel ray the original does not draw remains (docs/KNOWN_ISSUES.md).

## 2026-11-15 - Second ten native-resume tasks (persist-16 through persist-25)

Completed ten scoped follow-ups under PAR-18/PAR-20: mid-jump timing and progress;
F675/F680 pursuers and player anchor; changed shipyard stock; changed bulletin
posts; unvisited F622 boards and entry marker; BBS jobs/destination memory;
calendar marker and undelivered days; pending/queued mission deadlines;
autopilot angle/drift/transfer count/clearance retry; and navigation frame.
The dashboard records each task. Total 224, done 146, remaining 78; neither
broad parity item is closed by these ten additions.

The optional `runtime.continuation` record stores the live state. Station maps
are sorted before encoding and retain their exact RNG/generation context.
Earlier saves seed their calendar at the saved clock, avoiding a replay of all
prior days. New saves retain unprocessed boundaries and queued deliveries.
Validation rejects malformed records and references before simulation resumes.

Twelve cache-backed cases cover the ten behaviors, earlier-save migration and
malformed-state rejection. Checks compare resumed transit completion, pursuer
spawn/equipment, daily station changes, event delivery and clearance retry
across its two-second boundary. All 38 navigation integration cases, seven save
integration cases, the previous 12 resume cases, 246 library unit tests and 200
core unit tests pass. Strict clippy, formatting checks and the locked release
build pass. Full reference-compatible simulation persistence remains open;
physical player state and dust are not completed here.

## 2026-11-15 - Ten native-resume follow-ups (persist-6 through persist-15)

Completed ten scoped PAR-18 tasks: career points/medals; live missiles and their
next id; chaff and next decoy id; effects, cargo pods and clouds with their
current timers; ship autopilot target; launch permission; landing-warning latch;
fractional clearance seconds; mission-target spawn records; and an initialized
empty NPC population. They are individually listed in the dashboard under saves.
These are narrower follow-ups discovered under PAR-18, not ten closed parity
milestones. The total becomes 214 with 136 done and 78 remaining.

`NavigatorSave::runtime` stores existing simulation records rather than
reconstructing objects with fresh RNG. Version-2 files without it retain their
migration path; script canisters are restored only once in either path. Input
validation covers the new collections, motion, ids, timers and references.

Twelve cache-backed regressions pass: ten behavior/resume cases, older-save
migration, and malformed state. Missile guidance, subsequent object allocation,
ECM, effect evolution and RNG agree with uninterrupted play. All seven existing
save integration cases, 24 combat/effects integration cases, 246 library tests
and 200 core unit tests also pass. Strict clippy passes after simplifying three
constant index expressions in pre-existing scanline tests. The tracker check
also found a pre-existing duplicate `gameplay-41` id: the completed crash task
is now `gameplay-crash`, leaving the script-engine id and history intact.
The optimized game build passes. Stale ship targets are dropped from snapshots
if the target disappeared before the next autopilot update. Broader PAR-18 remains
open; these changes do not claim full reference save compatibility.

## 2026-11-15 - Daily servicing and save/resume timers (gameplay-3, PAR-18)

`Flight::service_day` used `unwrap_or(day)` without recording the first day.
A fresh or loaded flight therefore kept `None` and skipped daily wear forever.
It now records the initial day without drawing RNG or applying retroactive wear;
later days run the existing F933/F934 checks once each. Repeated or earlier days
do not consume RNG or change counters.

Native version-2 saves now carry `last_service_day` and `scoop_cooldown` rather
than resetting both on load. Missing fields retain the old-save defaults;
non-finite, negative or excessive cooldowns fail save validation. Pending script
actions remain transient. Complete PAR-18 parity remains open.

Validation: reproduced initialization and lost-cooldown failures before fixing
them. Regressions cover day boundaries, older saves, invalid cooldowns and
uninterrupted versus resumed wear, RNG, drive failures and cargo. All seven save
integration tests and the cockpit laser pixel-change replay pass with the local
cache. Library tests: 246 passed, one ignored.

## 2026-11-11 — CD videos play at the station screens (content-7)

The clip selection is in JJFFE 2.8a7 after all: F333 (services root, once
per arrival), F382 (Shipyard), F384 (Upgrades) and F442 (a ship bought) set
the clip list, and F120 plays one name from it at (1, 17).
`ffe_core::fmv_select` ports the four choices with their F922 draws in the
original's order (made whether or not videos exist); `ffe_rs::fmv::Player`
shows the 120 x 120 picture at 15 frames a second with its sound through a
new stream voice in the mixer, holds the game clock while it plays, and ends
on any key or click (the input still acts) or when the screen changes. Only
CD 1.01 content with the `FMVs` folder plays them. 135 of the 323 clips are
reachable; JJFFE never selects the bulletin-board adverts, `p` or `st`.
`DATA_005268..5272` were documented as ship-name lists; they are these clip
lists (`ship-names.md`, `station.md` corrected). A floppy start capture now
differs after the launch because F333's draws advance the shared stream, as
in the original. The other session's blocking `play` API, which slept
between frames, was replaced by the frame-driven player.

## 2026-11-11 — Shareware 1.06 loads: variable galaxy table

Elite Club shareware 1.06 ("English Release 1.1 (Remastered Version)") has
339 galaxy records. Its system block is floppy 1.02's with ten records
appended; everything after the table is the same once moved down 540 bytes,
apart from relocated pointers the port does not read. `galaxy::Tables` now
counts the records up to the zero code and maps every offset from the
special codes on past the longer table; `decode_system_tables` reads the
block to that length. The shareware imports and plays, and its Hope start
frame matches the floppy's.

JJFFE 2.8a7's `DATA_007110` also has 339 records and the same version and
model-name strings, so the gameplay reference's data is this 1.1/1.06 data,
not floppy 1.02's. 1.06 also differs from 1.02 in 35 models, the ship
statistics and the mission scripts. Galaxy oracles (world, market, sectors,
hyperspace, system names, map stars, galaxy map, takeoff) pass unchanged.

## 2026-11-11 — CD 1.01 imports; shareware assessed; CD videos load

**CD 1.01 said "no system tables".** The importer had CD locations only for
the models, bitmaps and font; every other table (system, galaxy, market,
messages, scripts, journal, names, lighting, terrain and the rest) was mapped
for floppy 1.02 alone, so a CD cache had none and the game stopped.
`tools/reference/map-layouts.py` found each floppy table in the CD executable
by content (its bytes with pointer dwords as wildcards, or a pointer table's
pattern of differences), each match unique; the decoders then check every
table. The script statics sit in zeroed memory, where the content match was
false, and were taken from the scripts' own pointers. The CD cache matches
the floppy one in 1,816 of 1,820 assets; the other four differ only in the
version string and code addresses the port does not read, and the CD start
frame is pixel-identical (content-5 done).

**Elite Club shareware (floppy 1.06).** Its locations map the same way, and
`FIRSTENC.DAT` is accepted wherever the executable is read. Every table
decodes except the system tables: 1.06 lists 339 star systems against 329, so
it is refused with that reason. It is not a drop-in for the JJFFE 2.8a7
baseline.

**CD videos.** `ffe_convert::avi` decodes the CD's 8-bit DIB/PCM AVIs and
`ffe_rs::fmv::Library` loads them from `FMVs` for CD content only; all 323
decode. No event plays one yet: the clip selection is CD-only code
(content-7, `docs/notes/cd-media.md`).

## 2026-11-11 — PAR-07: the Hope start frame's text matches the original

The port's `--capture-start` frame, diffed against the executed original's
Hope start (`--game-frame`, 320x200 indexed): 2187 differing pixels before,
1003 after, none of them text.

- F1099's four Set/Act/Alt strings (`0x8620`..`0x8623`) start with position
  codes: Set (2,139) and Act (2,149); Alt (266,139) and the parent (266,149).
  F1589 steps 10 rows per line (`push 0xa`). The port drew from (2,148) with
  8-row lines. `PlayerHud::speed_layout` places both columns ending on row
  149; the turret lines use the same 10-row step.
- F1399 draws every HUD line through F1588: colour 0 one pixel right and down,
  then the glyph. The port drew no shadow.
- F272's message window draws colour 0 at (0xc4, y) first, then the text at
  (0xc3, y - 1) in its colours. The port had the passes the other way round,
  with colour 4 as the shadow, so the date sat a pixel right and down.
- The Hull line shows only while `DATA_008886` (`player+0x10e`) is non-zero.
  F953 broadcasts message 11 (event 1) on every hit; the flight module's F1103
  hands it to F1102, which sets 30 seconds for the player's own ship, and
  F1007 counts it down with the whole-second events. `Flight::hull_display`
  ports it; a new game shows no Hull line, as the original's frame does.
- Still different: an orange tower's stem (drawn in front of a dark building
  in the original), a small red-roofed object at about (52-65, 76-82), and the
  scanner contacts. Object placement and depth, recorded on PAR-07.
- Update 2026-11-12: the "orange tower" is the setting star, and its depth
  against the buildings is right (the star queue, above). What was left in its
  top outline (rows 44-52) was the corona ring's colour: F1877 fades the ring's
  seven colours from the material to the sky colour it reads from
  `DATA_008815..8817` on entry, and that colour persists from the last call
  that set it. The port started every surface with a zero sky colour, so the
  ring faded to black and drew a dark outline where the original blends into
  the sky (palette entries 145-184). `ModelViewer::sky_colour` now carries it
  between calls and frames. Against the executed original (`jjffe-smoke
  --start-key=0x39 --game-frame 180 0xff`), the star's rim pixels now match
  index for index; seven pixels on the disc's right edge (rows 49-52,
  x 167-169) still differ by a vertex rounding.

## 2026-11-10 — PAR-06 done: F1037's module gate and the recorded key chain

F1037 starts `cmp byte [DATA_008630],0x0; jnl` and returns: the set-speed keys
and F1038's steering run only while the flight module is active. F1003 sets
`DATA_008630` to 0xff; F1006 (the flight module's leave entry in
`DATA_007254`) sets 0x7f when another module takes over: the map (module 2),
the F2 status screen (module 0x0e), comms. The port fed flight keys under the
F2 status screen and the map pages, so Return or Shift changed the set speed
there and an up-key frame cleared the slow-down latch.

- `OriginalFlight::set_flight_view` sets `Player::active`; the game loop sets
  it each frame from `input::flight_module_active(status_page, map view)`. The
  navigation computer (`n`) and the F5 engines page are drawn by the flight
  module and keep it active. The keyboard accumulators keep counting either
  way, as the original's input read and `Timer50Hz` do.
- `tests/controls.rs` replays recorded input lines (`session::Frame::parse`,
  now public) through `HeldControls` (EXP-003), the Classic mapping, the gate,
  F1037/F1121 and the thrust: intro-held and F2-held keys need a fresh press,
  a fresh Return under F2 does nothing, right Shift stops at zero and a new
  press reverses, left Shift does nothing, and the speed survives save/reload.
- Engines off (`+0xe9` bit 0) skips the set-speed keys but still steers: unit
  test. With the 2026-11-02 autopilot cancel and the executed takeoff and
  keyboard replays, PAR-06's acceptance list is covered.
- Burndown reconciled with the audit file: parity-02 done (2026-11-04),
  parity-09 and parity-10 in progress. 80 of 202 open.

## 2026-11-10 — Launch after an autopilot docking at an orbital station

Owner report: the station's Launch Request did nothing. A replay of the
owner's recording showed every request answering "Already launching." after
the autopilot had docked at Wicca Town. F746's accelerated arrival starts the
docking stages from inside the flight step; the stages' first step (F653)
resets the port's record, and the station-region touch the flight model
reported on the next frame then looked uncleared, so `region_touch` set
F1021's `0x24` climb under the docking ship (with a spurious F1022 warning).
That takeoff record survived the dock and blocked `begin_launch`.

- `Navigator::region_touch` returns `RegionTouch::Ignored` while a docking or
  launch sequence runs: the stage handlers (F1025-F1030) replace the flight
  frame, so F1019/F1021/F1024 do not run.
- `Navigator::dock` clears the takeoff record (F1025's mode `0x2a`).
- `begin_launch` from an orbital station drops a stale takeoff record, so a
  save made in an affected session launches again.
- Regression test `a_station_docked_by_the_autopilot_can_launch_again`
  (local, content cache); it fails without the guard. Owner-tested in the game.

## 2026-11-10 — PAR-06: F1121's held-key scale and the right Shift key

F1121 scales the set-speed step by `DATA_007690 / 16` once that dword's low
byte passes 0x10. `DATA_007690` is the last-key counter: `GuiReadAllInput`
starts it at 1 when the platform reports a key other than the one it tracks
(`DATA_009181`), `Timer50Hz` adds one every 20 ms, and it clears when that key
is released. The platform layer (`win32/w32input.cpp`) reports a key on its
initial press and on auto-repeat (300 ms delay, 30 ms rate in `ffewin.cfg`),
the highest scancode winning a read. The port never set it, so holding Return
or Shift for more than about a third of a second changed the set speed more
slowly than the original. The existing takeoff reference could not show this:
the adapter never reported Return as a last key.

- `ffe_core::keyboard::Keyboard` ports the three pieces: the 50 Hz ticks on
  20 ms boundaries, the last-key counter and the per-key accumulators, and the
  platform's press/repeat report. `OriginalFlight` reads every held key by its
  original scancode (`input::scancode`, the `pKeybConvTable` order), advances
  the clock by the frame's whole milliseconds, sets `speed_boost` and takes
  F1038's steering counts from the same accumulators (it had used wall-clock
  seconds times 50 plus one). Input and world time are now `ms * 0x3e2 / 20`
  and `ms * acceleration / 20`, truncated per frame as `TimerAdvance` does.
- The Classic slow-down key is **right Shift only**: F1037 reads
  `DATA_007694`, the state of scancode 0x36; left Shift (`DATA_007693`) only
  selects the shifted half of the action table. The port accepted either Shift.
- `jjffe-smoke --flight keyboard 900` (new) is the takeoff with Return held
  260-399, right Shift 450-599 (down through zero, where flag 0x80 holds it),
  right Shift again 620-689 (below zero by the input time over 256 a frame),
  Return 700-799 (a negative speed snaps to zero first) and A pressed 740-745,
  reported through an emulation of the platform layer read at `tmLastTime`.
  It also prints each frame's `tmLastTime`, `tmNext50Hz` and `DATA_007690`.
  `port_set_speed_follows_the_original_keyboard` aligns the port's input clock
  with the trace and matches the set speed on all 730 frames; with the scale
  removed it fails at frame 290. The plain takeoff replay is unchanged.

Remaining for PAR-06: the menu, reload and autopilot-cancel key paths and the
port's release-to-rearm gate in a recorded input replay.

## 2026-11-10 — F1014 in flight: atmospheric drag and heating (PAR-04, PAR-14)

The takeoff replay showed the port accelerating ever harder after F1031's
clear, while the original's per-frame gain fell steadily with speed (47 units a
frame at 7,000, 35 at 20,000). The source: F1021 (mode 0) and F1019 (mode 2,
the autopilot) call F1014 after F1033's scan whenever the contact's bit 0 is
set (F1021 only outside the `0x20` region and without the `0x40` touch).
F1014, below 20 km (`DATA_008867 <= 0x1312d00`), adds JJFFE's speed heating
(`speed * min(tics, 0xffff) >> 0x13`, or `0x16` with atmospheric shielding,
capped at `0x1000`; past `0xffff` the ship burns up, crossing `0x8000` posts
"You're going to die soon!") and, from a speed word (`+0xd8`, F1099's length
of the velocity from the frame before) of `0xbb8`, JJFFE's drag:
`F1521(v, min(tics, 0xffff) << 11)` off each velocity component, about
`v * tics / 2^20`. F1016 and F1017 are empty. The port's notes had F1014 only
in the landed handlers.

`Player::atmospheric_drag` (core) is the drag; `OriginalFlight` applies it
after the scan under F1021's and F1019's conditions and reports it, and the
game loop applies `damage::atmosphere_heat` beside the star heating. With it
the replay's per-frame speed gain matches the original's within one unit and
the speed is within 1.25% to frame 899 (it had been 23% fast).

Flying fast below 20 km in an atmosphere now heats the hull, as in the
original; F1007's whole-second cooling still applies.

## 2026-11-10 — PAR-04: frame-by-frame takeoff replay; F650's seat in the port's own frame

`tests/takeoff_replay.rs` (local-only) replays the executed original's
`--flight takeoff 900` trace (Ross 154, Tab at frame 170, faster held from
260, 21 ms frames) through `Navigator`/`OriginalFlight` with the game loop's
own hand-off, and compares every frame's mode, F1033 altitude, set speed,
position and velocity in the planet's rotating frame.

- Mode `0x24` at 171, `0x26` at 261 (F1013's set-speed hand-off) and every
  set-speed value match. Neither moves the ship on the frame the launch is
  processed (the game loop builds the pad model after that frame's step).
- Fixed: the pad seat. F650 multiplies the pad by the starport's rows in its
  own frame (Ross 154 A's or Hope's rotating frame) and shifts the product
  left by 10; F994 adds F1521's radius product there, and F1677 turns the
  whole into the system frame. The port used the system-frame rows, so a one
  unit rounding difference became 2,048 mm (Ross 154) or about 1.8 m (Hope).
  `Body::parent_frame` keeps the port's own rows and its planet's rows, and
  `Body::pad_offset` serves the new-game seat, the parked seat, the takeoff
  seat, the pad points and the autopilot's final-leg pad (F745's words are
  now taken in the port's frame too). Both starts' seats match the original's
  `--geometry` within one unit; the time-warp test still holds the parked
  seat to about 1 mm.
- Climb drift fixed 2026-11-17: the pad world now integrates in the planet's
  rotating frame as the original does (see docs/KNOWN_ISSUES.md); the climb's
  velocity matches within 1 unit and the clear is 1 frame early. Free flight
  after the clear matches too (velocity within 1 unit to frame 899): the 1.25%
  gap was `step_scan`'s per-frame F1520, not the frame (docs/KNOWN_ISSUES.md).

The playable's default start now parks 1.8 m from where it did (the altitude
readout on the pad goes from 19 m to 20 m). The scripted `--capture-start`
frames change accordingly; on this machine that capture is not repeatable
when its window loses focus (the port pauses without focus), so the replay and
seat tests are the regression evidence.

## 2026-11-10 — Classic split cleanup (classic-split-2)

`model_viewer` no longer builds the data only the removed Modern engine read:
`Solid` faces (views, textures, sprites, plumes, outlines, curves), atmosphere
`Shell`s, surface poses and view corners, shape owners, render children and the
frame's light/origin. `merge_surface_shapes` keeps its exact ordering on keys
and shapes alone. `ffe-core`'s `terrain`, `terrain_draw` and `terrain_emit`
are restored to their committed pre-split versions, which never had the
Modern visual/radial detail, planet-frame rotation, shoreline capture or
fade; those were all behind flags only the Modern engine set.

Every scripted capture (intro 13, display 7, start 150, traffic 7, game-over
35, planet 1) is byte-identical to the build before the split. 867 workspace
tests pass (three Modern-terrain unit tests went with the code).

## 2026-11-11 — PAR-16: F688's ships are the original's

`ffe_core::spawn` ports F688's own draws call for call: F635/F658 (the model),
F772/F773/F774 (the hull's outfit: drive, fuel, laser, shield, pylons, gadgets),
F689/F690 (traders and Thargoids with F775/F776's destination and fuel check,
F857/F870/F852/F903), F691 (bay ships), F699/F701 (the aux carrier) and F694-F697
(pirate groups). With `bbs` (F622) the shared pair now follows the executed
original from the F147 seed through the whole new-game frame.

- Default start (Old Blackelk): 2 traders (one survives: its destination search
  fails four times for the first, once-and-done for the second), 6 bay ships,
  1 aux carrier, 3 pirate groups; the 17 models are the original's object table
  slot for slot (trader 65; bays 65, 44, 44, 45, 64, 59; aux 71; pirates 36,
  28, 48, 22, 43 and four 23). The pair leaves F688 in `654ec13e 08573e56`.
- Ross 154 (start type 1): enters F688 in `c1ff509f 1f7a567d` and leaves in
  `8eb9e749 d2cc2b66`, both the original's.
- `npc::spawn_plan` builds the port's `NpcShip`s from the plan (models, roles,
  places, bounty rows); positions and flight state are still the port's. The
  scripted capture (`respawn_traffic`) keeps the old guaranteed-trader spawn.
- The bays are counted `0..=bays` per place (F650), the player's own taken.
- Markets are no longer created for every place at system entry:
  `Navigator::peek_market` reads a station's stock for F641 without recording it
  (saves and market tests expect only visited markets).
- Tests: `bbs_reference.rs` (default start twice, Ross 154). Old ignored tests
  that assumed police at the new game (`firing_at_hope...`) now describe a start
  with none; the F783 part uses the capture spawn.
- Still open on PAR-16: F702's system-wide placement (F1506/F1508 put ships on
  a sphere near 1e15 units; the port's AI cannot fly at those distances yet,
  PAR-15), F238's police response (the port's patrol outside the new game is a
  reconstruction), and start types 2 and 3 (the port maps them to Ross 154).

## 2026-11-11 — Play-through bugs: ship change, drives and station time

Found by replaying the owner's recording (`--replay` with `FFE_TRACE=1`).

- **Exploded on docking after a ship change.** Not the new hull: the autopilot
  reached Wicca Town at 10000x time acceleration (one frame is about 167 s), the
  ship ended inside the station and took crash damage until the hull went from
  5088 to 0. The port held time at 100x only near planets and moons. It now also
  holds it near stations and starports (EXP-021), by distance and by last
  frame's travel.
- **No hyperdrive after selling the ship or the drive.** F1533 and F441 give a
  bought hull the drive its statistics word names (`ship+0xd0`, the low byte),
  and the hold is the cargo statistic less that drive's mass (an integral drive,
  a negative word, is already out of `cargo_space`). `buy_ship` cleared the
  loadout and fitted nothing, so a small hull had no room for any drive and the
  fit rows (stepped through one by one) bought the Interplanetary drive, which
  then blocked every hyperdrive. A bought hull now carries its stock drive
  (`Navigator::hull_drive`, `trade::capacity`), selling it frees its tonnes and
  the Class 1 fits again.
- **Services wanted to launch after a ship change.** `buy_ship` cleared
  `nav.new_game`, which `on_pad()` needs, so the start pad was forgotten and F4
  said "No station services here." (four times in the recording). The record now
  stays with a `bought` flag (`NewGameState::bought`) that switches only the hold
  and the delivered equipment to the new hull's own.
- Tests: `crates/ffe-rs/tests/ship_change.rs` (ignored, needs
  `FFE_WORLD_CACHE`). Not played by hand; the replay no longer explodes and no
  longer prints "No station services here.".

## 2026-11-11 — PAR-17/20: F622's draws before F688 match the original

With NASM installed the reference traces every F922 draw (`FFE_RNG_TRACE`).
The executed original's default start makes 1,711 draws before F688, from the
F147 seed `12345678/456789ab`: for each of the 12 places 99 F1530 steps (the
market over 33 goods), `population * 4 + 1` board attempts (F621) and the F634
ship queue, then F623's news rounds (`maximum control * 8 + 8`). `ffe_core::bbs`
now ports F621, F636-F646, F857 (the destination picker: `RND(12)` twice, then
`RND(stars)`), F634/F635 and F623-F632 draw for draw, and
`Navigator::generate_boards` runs it from the shared pair on entering a system.
All 1,711 draws and the final state `a3e751f9/1df3aaed` match.

- Differences from the earlier generator: the board keeps drawing after 18
  entries; F639/F640/F636/F637/F641 exist (the port had left three out);
  F645's notice is always `0x982c`; F644/F645 take F1530 words, not F922 draws;
  the starport's ship queue is the shipyard's hull stock.
- Data caveat: JJFFE's galaxy table has ten more (empty) systems than the
  floppy's 329, so the comparison sets `FFE_REFERENCE_GALAXY=1`
  (`bbs::REFERENCE_EMPTY`). The game itself uses the player's table.
- Regression test: `crates/ffe-rs/tests/bbs_reference.rs` (ignored; needs
  `FFE_WORLD_CACHE`) checks the state entering F688.
- Open: F688's spawn bodies (F689/F691/F698/F699/F702/F772/F775, about 14 draws
  per ship); the port diverges at draw 1712. F633's daily half and the player's
  ranks for the news gate are not ported.

## 2026-11-11 — PAR-16: the new game's traffic follows F688 (started)

The executed original's default start (`--npc-trace 260 0x39`, frame 200) holds
six F691 bay ships at three ports, an F699 aux trader, one F689 trader, two
pirate-group ships and no police ship. The port's new game had three bay ships,
no aux trader and a police ship 3 km off. The new game's F688 pass
(`System::start`) now spawns no police, and the port's traffic caps are lifted
(`MAX_SHIPS` 9 to 40, `MAX_BAY_SHIPS` 3 to F691's own 25, no trader, pirate or
aux limits): F688's rolls stand. Two unit tests pin both. Workspace tests pass.

Follow-up (same day, NASM installed): the reference adapter now logs every F922
draw (`FFE_RNG_TRACE=<file>` with any state mode; row = frame, sequence, state
before, limit, result, caller). Findings: the default start's F688 is draws
1711..1874 of 1876 in the new-game frame, entered with state
`a3e751f9 1df3aaed` after 1,711 earlier F922 draws; the port enters with the
untouched F147 seed. Every start key runs F688 once with the first-pass bit
**clear**, so the port's `System::start` for the new game was wrong and is no
longer set (a port-only `new_game` flag omits the police). F692 is an empty stub.

Not done: the draw ledger. The port's new game rolls 0 traders, 13 bay ships and
0 aux traders over 12 places; the original rolled 1, 6 and 1, so the same ships
do not appear until every draw before F688 matches (PAR-17). Tracing F922 in the
reference needs `DATA_007217` exported and a NASM rebuild (NASM is not installed
here). I could not reproduce the earlier start-frame pixel count, so no frame
figure is claimed.

## 2026-11-10 — PAR-01: F152's whole-second events from the day fraction

F938's events (shield recharge, hull repair, thrust energy, laser and hull
cooling, through F151's event 6 and F1007) were counted from a floating
carry of accelerated seconds at 49,700 tics each. The source's F152
(`ffe.asm:15176`) divides the day fraction's advance since its marker,
`DATA_008804 - DATA_008808`, by `0xc22e` and moves the marker by whole
events. `ffe_core::second_events::SecondEvents` ports that arithmetic, and
`Combat::pending_events` keeps the undispatched advance (saved with the ship
state, as the marker is part of the player block F166/F167 write).

- The marker follows the clock in every module (pad, stations, services,
  hyperspace's clock step), not only in flight. The flight step applies the
  events; elsewhere they are consumed, as before.
- At 1x a real minute is now 59 events (2,982,000 units), not 60. The
  10x..10000x words are exact multiples of `0xc22e` per second.
- The new-game set-up leaves fraction `0x80000000` against marker 0, so the
  first F152 dispatches 43,200 events at once, as the original does.
- F208's 10x..10000x selection pulls the marker back by one event.
- Old saves load with nothing pending.

Evidence: unit tests for the arithmetic; `damage.rs` runs F1418's frame
timer at 50/64 fps and 1x/10x/10000x and requires `total / 0xc22e` events.
The `--state` adapter now prints `second_marker` (state `+8`) and
`verify-reference` checks it against `SecondEvents` on every in-game frame,
with tamper tests. Classic captures (start 150, traffic 7, game-over 35)
are byte-identical to the previous build.

F1007's mode-2 branch (`ffe.asm:74576`) is ported too: F1020 sets the
player's `+0xe8` to 2 when the autopilot engages, and in that mode the
per-second event runs the autopilot's arrival checks instead of F938. While
the autopilot flies there is no shield recharge, hull repair, laser cooling or
thrust-energy charge; the hull temperature still cools
(`combat::f938_events`).

The executed original confirms the marker: at the new-game frame it is 0
against fraction `0x80000000`, on the next frame `0x7fffd280` (43,200 events),
and all 238 in-game frames match `SecondEvents`. The full oracle suite gives 55
passed, 1 skipped and 1 failed; the failure (`child-models` at depth
`0x50000`, models 273 and 287) reproduces on the pre-split build.

Open (recorded on PAR-01): F1007's `+0x10e` countdown and its mode-2 arrival
checks (PAR-11); NPC event-6 handling (PAR-15); a full reserve trace from
boot through climb, thrust, firing and 1x..10000x.

## 2026-11-10 — Classic-only split

Owner decision: this checkout is a like-for-like Classic reimplementation. The
Modern GPU engine (`modern*.rs`, `modern3d`, `ffe-render`/wgpu), the FFED3D
art, model and panel integration (`ffed3d.rs`, `art_source`, `ffe-convert`'s
`.X` reader, `ffe-tools inspect-ffed3d`/`build-assets`, `ffe-content::art_pack`),
the `ffe-launcher` crate and `launcher.bat` were removed, with the Graphics
Classic/Modern row and the Home/Ctrl+G switch. `--ffed3d` and
`saves/launcher.cfg` are no longer read; an old `graphics=` or `font=` line in
`config.cfg` is ignored. The Classic/Modern control presets are input only and
remain.

Classic output is unchanged: the pre-split and post-split release builds wrote
byte-identical frames for `--capture-intro` (13), `--capture-display` (7),
`--capture-start` (150), `--capture-traffic` (7), `--capture-gameover` (35) and
`--capture-planet` (1); `--capture-prefs` differs only on the Port Options page,
which lost the Graphics row. 863 workspace tests pass; the two ignored
`missile_view`/`scanner_view` failures reproduce on the pre-split build.
The burndown dropped the packs and enhanced milestones (17 open items) and
added `classic-split` (83 open of 202). See [BURNDOWN.md](BURNDOWN.md).


## 2026-11-08 — Modern world rendering audit

The Modern worlds roadmap (`docs/MODERN_WORLDS.md`, removed with the Modern engine) separates original game data
and collision queries from Classic draw execution. Modern already streams
indexed ground but still obtains shells and rings from model execution.
FFED3D facility replacements also depend on executed root and child models.
The plan moves both through a shared scene snapshot, then consolidates
climate, biome, water and weather systems and removes the obsolete adapters.
The locked release build and Hope departure replay passed. The replay
compared 393 frames across Classic, Modern and four Ctrl+G switches.

## 2026-11-05 — external-camera floor clearance in both engines

The shared external-camera input step now rejects yaw, pitch or zoom changes
that would put the eye into terrain or a pad floor. The original orbit,
position and axes are used directly; the automatic lift/retarget correction
was removed at the owner's request. Valid components can still move while
another component is blocked. Tests cover unchanged clear movement, holding
at the boundary without accumulating pitch, immediate reversal, and Hope's
actual pad. Both renderers receive the same unmodified orbit camera.

## 2026-11-04 — match the FFED3D cockpit reference

Corrected the cockpit projection to the full viewport, 35-degree vertical
field of view and aspect 2 from `doPerspectiveNear`/`ViewPort(false)`. The
flight view's centre (160,79) had placed it too high; the cockpit uses
(160,100). Removed the added rectangular control menu, cyan scanner,
lever labels, horizontal gauges and message window. The scanner, indicator
lamps, lever and gauges now draw FFED3D replacement PNGs using original
placement metadata only. Clock/log text overlays the panel directly.
Panel material colours and normals are retained, with selected-button
green point lights using the source's positions, range and specular power.
The configured HUD font stays selected. Classic rendering is unchanged.
Regression coverage checks projection, material retention, light range and
normals, UI picking at three sizes, and absence of classic framebuffer pixels.

## 2026-11-04 — interactive modern dashboard

The owner requested the modern console's buttons after the decorative mesh was
disabled for overlapping the original dashboard. Modern now replaces rows
158..199 with the FFED3D panel mesh and composites its four icon pages,
equipment-dependent blanks and green selected states. Drawing and picking
share a cached perspective-correct UV map. Actions still use the existing
console dispatcher, with scanner/log regions blocked from hidden old row hits.
The first layout compressed the panel and reused classic instrument pixels;
the owner rejected it. The revised panel retains its original projection and
proportions, with semantic modern systems/time controls, gauges, scanner and
log. No classic dashboard bitmap, widget or framebuffer crop is drawn by
the modern console pass. A poisoned-frame test checks that changing every
old console pixel cannot change the modern result.
Classic retains its original frame and input coordinates. Missing panel assets
use a plain modern console, never the classic dashboard. No simulation or keyboard mappings change.
The battle/navigation inset artwork remains deferred. Unit coverage exercises
page/state changes, unavailable equipment, ghost hits and relocated ship/time
actions; local asset-backed captures check flight/status at three window sizes.

## 2026-10-26 — F938 fuel timing correction (PAR-01/02 in progress)

Live thrust/recharge now consume whole-second events shared with laser cooling,
not physics tics. Laser refire keeps physics tics. Thrust cost follows the source
signed IMUL/SAR low-word arithmetic. Military reserve refuelling produces waste.
27 fuel/combat tests, workspace tests, two cache-backed idle/pad regressions and
Clippy pass; root release rebuilt for play.bat. Full source event traces and
remaining scheduler/recharge boundaries are outstanding; neither item is closed.


## 2026-10-26 — owner source-gameplay parity audit and backlog

Reviewed launch/docking/set speed, fuel/hyperjumps, the Hope starting view,
player destruction and the main gameplay adapters. Confirmed F938 consumes
whole-second event deltas in the original but per-frame physics tics in the
live port; this can drain the internal reserve about 49,700 times too fast.
Recorded additional source-order/integrator, contact, camera, RNG, NPC,
persistence, new-game and jump differences without claiming them fixed.
Added 24 work items with source/port references, priorities, dependencies and
acceptance traces; reopened 14 done labels. Remaining 64 -> 102. Current
acceptance is original gameplay apart from Port Options; old experimental
deviations are not exemptions. Full report: docs/audits/gameplay-parity-2026-10-26.md.

## 2026-10-26 — Hope start and launch ground

Fixed model-to-world terrain scaling from source scale offsets; restored the
original F1868/F1869 culling and F1877 atmosphere record. Near-pad original
oracle matches 168 triangles and all four queues; the Saker rises and ground
stays visible through 0/1/3 s with orbital updates. The starting seat is unchanged.
See [launch-ground](archive/launch-ground-2026-10-26.md).

## 2026-10-26 — source autopilot guidance audit

Replaced the live prototype law/long-leg transfers with F597 requests/caps,
F587/F589 turn words, F599 limits and F727 motion. Corrected F1020 state/equipment
gates, F741 offsets, F745 fines/retries and F650 plane versus F671 seat. Added
an executed-original arithmetic oracle (512 cases) and live controller tests.
The oracle exposed F589 sign errors in the recovered notes. Accelerated F746
near-pad completion passes. The Old Blackelk 1x route now lands with a real
contact flag after putting ship and port as siblings under the planet. Full
source parity remains open. Details and limitations:
[autopilot-source-audit.md](notes/autopilot-source-audit.md).


## 2026-10-25 — launch orbital-frame regression (partial fix)

New moving-clock test reproduced the one-second jump (port 10.8 km away at
frame 50). Use absolute positions at navigation/autopilot/range/contact
boundaries; carry the pad world's origin and climb reference with orbital drift;
resolve current motion before the pad-world hand-off. Capture-start is now a
fixed 20 ms step. Near-port range and no-teleport hand-off assertions pass.
The empty Hope terrain/horizon is still unresolved: a terrain-only view emits
no planet shapes; previous tests counted buildings. No guessed core culling or
radius changes retained. Details in archive/TAKEOVER_ISSUES.md.


## 2026-10-25 — services return-to-ship icon

Docked rendering skipped all console buttons and the input collector rejected
all docked console clicks. Restore F1439's common screen buttons/time meter on
station pages with F331's station selection, and route the F1 console click to
the pad board's existing return action. The Old Blackelk regression failed
before the fix and now passes with all four local station-view tests.


## 2026-10-25 — fix immediate first-frame death

F938 recharge ran before ensure_damage and changed an uninitialised hull from
[0,0] to [0,full], which the late initialiser preserved as destroyed. Initialise
both words to full; leave existing damage intact. Regression reproduced red
before the fix, green after it; 13 damage and 8 death tests pass. Earlier boot
safety checks skipped this live-loop call order. See archive/TAKEOVER_ISSUES.md.


## 2026-10-25 — takeover issue audit

Laser input now uses HeldControls, disarmed at entry; release then press fires
normally. Object text no longer substitutes a catalogue name for an empty
registration. Removed the invented destruction sentence and PAUSED overlay.
Existing intro/boot/planet/death checks pass. Dust remains missing and game-over
pixel fidelity remains partial. See [archive/TAKEOVER_ISSUES.md](archive/TAKEOVER_ISSUES.md).


Running record of decisions, discrepancies and open questions. Newest entries at the top; append rather than rewrite, unless a decision supersedes an earlier one (mark it).

---

---

## 2026-10-26 - Game-over tombstone spin follows F917

- **Fixed the tombstone's rotation.** `F917` calls
  `F1675_MatBuildYZT(matrix, 0, counter >> 1)` with the second argument 0 and
  the third the yaw; the port had passed `(angle, 0)`, turning the slab in the
  screen plane (a roll) instead of about its vertical axis. It also now draws
  the object at F917's `z = max(counter, 0x88b8)` (the slab approaches for
  215,000 tics, then holds) and uses F917's light `DATA_007201..3`
  `(0x5a82, 0, 0x5a82)`; the 30°/-20° camera and the `angle & 0x8000`
  face-selection workaround are gone because the text records' own normals pick
  the visible face. The 320 frames now match the reference `death-frame-800`
  (obituary) and `death-frame-990` (GAME OVER) geometry; regenerated the
  evidence images including a fresh live `--capture-gameover` frame. Details in
  [notes/gameover.md](notes/gameover.md).


## 2026-10-26 - Batch 3c: runtime, autopilot, autoland and crash

- **Takeover work preserved.** A concurrent takeover stream had a source-arithmetic
  autopilot (`ffe_core::autopilot` F587/F589/F592/F597, `source_command`, new tests)
  in the working tree. It did not compile against the committed navigation API, so it
  was committed verbatim on `backup/takeover-autopilot` (`3f8825d`) and `main` was
  returned to the clean certified tip. It is a competing/duplicate autopilot and stays
  backup only; the pushed `autopilot` branch is the one live implementation. No unique
  fix from it was promoted (it was incomplete).
- Merged, one at a time, each green and pushed: `runtime` (ef55331), `autopilot`
  (4da64a3), `autoland` (f18e0b0) and `crash` (7057d6b).
- **runtime (v1.11).** F3 is the source map again (the preset lives only in Port
  Options), the port's PAUSED overlay is gone while source Escape/pause stays, F4
  services overlays the landed view without wiping the console/HUD (F343/F1003), the
  clock advances in every module scaled by TIME_ACCELERATION (F152), pad/landed
  set-speed and thrust are live (F1037/F1041), and the HUD Set line needs a model.
  Conflicts: pad model setup, message window/pad board, launch path, clock advance and
  the mission tick; the duplicate `nav.advance` was removed (one frame clock).
  Owner decisions: pad-speed carry, 10000x pad rotation, P key, save clock.
- **autopilot (v1.12).** Intercept-time bisection, F741/F745/F746 slot law and hull
  detour, 4,096 km station/6,144 km starport hand-off, arrival slop, starport
  `final_leg`, Backspace cancel applying F1018 set-speed 0, 1x capture and evidence
  logs. The old transfer block was dropped in favour of the intercept solver.
  Owner decisions: transfer model, Backspace cancel, 1x.
- **autoland (v1.13).** F741/F742/F745/F746/F744/F652 pad descent, F599/F650 bay and
  F746 orientation, the independent 0x59 geometry import (cache v29 now **1,815
  assets / 1,816 entries**, 465 geometries/464 programs), `step_starport_landing`'s
  three-body frame and manual pad contact. Conflicts: the autopilot command body
  (autoland pad target kept; station hull detour restored), original-flight helpers
  (kept `set_pad_bodies` and added the collision/thrust accessors), navigation tests
  and docs. `verify-assets.py` updated 1813/1814 -> 1815/1816. Owner decisions:
  lateral capture, F575 0x40 flags.
- **crash (v1.14).** F575's contact tail applies F953 damage (shields -> hull ->
  F942/F1104), removes the invented crash message, keeps only the source bounce and
  separates landing 0x40 from crash 0x80; 28 non-lethal/181 lethal GAME OVER cases and
  the launch trace. Conflicts: `Flight` helpers (deduplicated `destroyed`), the landing
  block (autoland's starport path kept with crash flags, damage moved to the shared
  tail). Owner decisions: silent bounce, launch UX, capsule-at-zero, Contact.damage sum,
  EXP-016.
- Checks: workspace tests, clippy `-D warnings`, fmt, each branch's cached/ignored
  suites and affected oracles. Cache v29 was rebuilt from the merged tools; the old
  1813-asset file is kept as ignored `data-v29-1813.bin`.

## 2026-10-25 - Batch 3b: station shuttles, NPC missiles, saves, galaxy console and intro stars

- Merged, one at a time, each green and pushed: `shuttles` (4108683),
  `npcmissiles` (09bb3cb), `saves` (eb10f78), `galaxy-console` (6db3224) and the
  newer `intro-fix2` (e43b45e).
- **shuttles (v1.06).** F691's station-bay ships (`DATA_008997/8999`,
  `RND(4 * places)`, F650 bay, state 0x24 countdown), bay release, F699's Lynx
  hold, F751's escort hand-off and the F1530 split, with `MAX_SHIPS=9` and
  `MAX_BAY_SHIPS=3`. The `station_traffic` fixture now isolates its F691 ship
  from other F688 traffic sharing the station's single docking record. Owner
  decisions: caps, F699, F686/F687, closed-bay tables, surface traffic.
- **npcmissiles (v1.07).** F754 launches, F773 pylon fit, F757/F935 ECM and
  evasion, the player's chaff (cargo 32), F1057 decoy retargeting, C chaff and
  the HUD lamp; `npc_missiles` frames. Conflicts: `Spec` gained both the
  status `rating` and the missile `missiles` word; kept both. The `station_traffic`
  fixture's model-table expectation was widened to any F635 table (the F691
  model now comes through the allegiance shift). Owner decisions: chaff
  consumption, F937/F935 visual, cadence, F773 budget.
- **saves (v1.08).** Native versioned JSON v2, F5 quick / Ctrl+F5 numbered /
  Shift+F5 load, intro L, `--load` and `--capture-save`, full
  Flight+Navigator+legal+rank+missions state, atomic bounded writes, legacy
  read-only recognition and `saves/config.cfg` migration. Conflicts: README
  player-build wording (kept cleanup's), `Legal` (kept the jurisdiction doc and
  added serde), `main.rs` options/usage (combined dev with load/capture), and
  `DEVIATIONS` EXP-002/003. Added in-memory drive migration for v1 and
  pre-`Loadout::drive` saves. Owner decisions: legacy specimen, RNG, object
  array, mission/rank model, remap UI/dialog.
- **galaxy-console (v1.09).** F834/F840's map console page rebuilt against the
  current main: F5 controls, F6 select, F7 details, F8 picture, F9/F10
  next/previous, F11/F12 zoom; map-local ownership only for the 3D map; row
  clicks routed. Conflicts: `main.rs` dev block (kept cleanup's `--dev` chords
  and gated them off while the map owns keys), F12 capsule (map owns F12), view
  commands (suppressed while the map is open) and F5 save/load (native saves
  kept; map suppresses them while open). Owner decisions: map-local keys,
  details/picture still unported.
- **intro-fix2 (v1.10).** The newer head adds the nebula-behind-planet key fix
  and F1766/F1814 star chains; the earlier text/ship/ring fixes are retained
  (the merge brought only the new commit's diff). Conflicts: none. Two
  global key-sort assertions were relaxed to landmark ordering because the
  port's F1840 surface merge is not part of `verify-order`'s queue; the
  ship/planet and nebula ordering checks remain. Owner decisions: remaining
  chain-star density/cull branches.
- Checks: every merge ran workspace tests, clippy `-D warnings`, fmt, its
  cached/ignored suites and the affected oracles; `verify-intro`, `order`,
  `fills`, `textures`, `surface`, `combat`, `npc`, `station`, `prefs`,
  `galaxy-map` and `hyperspace` were exercised across the batch. Cache stays v29
  (1,813 assets).

---

## 2026-10-25 - Batch 3a: ship names, fitting, market, repairs, status and missions

- Merged, one at a time, each green and pushed: `shipnames` (cbc6837),
  `fitting` (a573f71), `market` (30a762e), `repair` (debf94d), `status`
  (ea58545) and `missions` (c5edb37).
- **shipnames (v1.00).** `ShipStats` carries `+0xc` shield and `+0xe` name code;
  names resolve through `DATA_004682` (58 hulls, Saker `0x1f`, Eagle `0x17`,
  `LAST_SHIP 0x47`). Conflict-free; `ship_names` covers 466 slots/58 hulls.
  Owner decisions: DEFAULT_SHIP/Eagle comment, cache v12, Unknown names.
- **fitting (v1.01).** All seven equipment categories fit/remove with the
  F409/F410/F413/F415/F423-F427/F394 rules, trade capacity, `Combat.drive` sync
  and old-save drive migration. Conflict-free. Owner decisions: drive-service
  date, mining deployment, passenger cabins, bought-hull drive, shield clamp.
- **market (v1.02).** Black market/illegal mask, F640's fake-ad trap (offence 5,
  500 CR), F233's scan, exact F555/F556 strings, generic F236 recording and the
  black-market keyboard/Enter path. Conflicts: `docs/notes/market.md` (kept the
  timed-events daily path and added the black-market cases) and the market_view
  draw signature after the cleanup. Owner decisions: smuggling trigger, fine
  screen, RNG stream, mission cargo.
- **repair (v1.03).** +0xcc/six thruster words, F950/F952/F953 subsystem rolls,
  F938 recharge/auto-repair, F929 energy/fuel, F1033 scoop, F444-F452 repair
  rows/costs and crash damage. Conflicts: `views.rs` (kept shipnames' shield/name
  asserts) and `docs/notes/equipment.md` (kept fitting's decisions and the
  DATA_005303 note). The auto-merged `ships.rs` duplicated `price_mass`; removed
  the duplicate, then added the new `fuel_scoop_mounting`/`name_code` fields to
  the repair fixtures. A stale `ffe-core` incremental made verify-prefs falsely
  fail; `cargo clean -p ffe-core` cleared it. Owner decisions: cargo split,
  afford wording, 500-day roll, scoop heat.
- **status (v1.04).** Rank/reputation/Elite/medals, F2 F6/F7 profile, legal
  bands, the DATA_008996 jurisdiction correction, the police fines office and
  F942 rating. Conflicts: `npc.rs` (kept legal's generic `record`), `station.rs`
  (kept Repair and added PayFine), `main.rs` (view commands now respect the
  status page's F6/F7; kept Bulletin + Police; combined PayFine and Post; added
  Repair back), and `status_view`/damage fixtures. Renamed the status accessor to
  `offence_bits` to avoid colliding with the generic `record`, and made `record`
  set `DATA_009093` bits. Owner decisions: police spawns, mission rewards,
  page-0 text, officer name.
- **missions (v1.05).** Mission state/kinds, BBS accept/complete/expire/abandon,
  passenger/parcel/removal/military packages, journal/comms/military pages,
  F306 song 7 and save round-trip. Conflicts: `navigation.rs` (kept the
  timed-events deadline API and added the offer/clear methods), `station.rs`
  (combined Screen/Action/Layout/title/row variants), `main.rs` (Post/ReadPage/
  MilitaryPost plus Police and Repair; Post falls back through
  `open_bulletin_post` so 0x9829 still opens the black market), `station_view.rs`
  (combined the repair and mission frames), and `station.md`. Owner decisions:
  F625-F628, removal combat, cabins, rank and journal archive.
- Checks: every merge ran workspace tests, clippy `-D warnings`, fmt, its
  cached/ignored suites and the affected oracles; the exhaustive navigation test
  was rebuilt in release for the navigation-touching merges (96 s after the
  missions merge). Cache stays v29 (1,813 assets).

---

## 2026-10-25 - Once-a-day time tick

- Merged `timed-events` (`b011b9d`) as `0594f0e`, conflict-free. The branch was
  based on the final-batch `main`, so the market/BBS/shipyard and
  hyperspace/landing-audit paths were already common.
- `ffe_core::calendar::Calendar` carries `DATA_008808`'s day marker;
  `Navigator::tick_days` runs F633's market drift, BBS replacement and shipyard
  stock once per crossed day, through `advance`/`transfer` and hyperspace
  (a direct `jump` adds no time). Per-station day markers keep each offer set
  stable, and `schedule_deadline`/`cancel_deadline`/`take_expired` give the
  missions stream a typed bridge without inventing a mission format.
- Owner decisions: the clock and navigator are not in `Flight` today, so saving
  the date/market days needs a serialized navigator section and a save-version
  bump; the `DATA_009004` news buffer and F636/F637/F641 generators need state
  and a visible screen; `F48(0x18)`/module 21 need the script VM; paused/docked
  time does not advance; F59's dated service reminders wait on the repairs
  actions (`docs/notes/time.md`).
- Checks: workspace tests, clippy `-D warnings`, fmt; `timed_events` (6
  ignored), `market_view` (2), `station_view` (2), `npc_docking` (1),
  `npc_ai` (4); navigation debug and the release exhaustive case (244 s);
  `verify-market`, `verify-station` (including queue/models) and
  `verify-galaxy-map` pass.

---

## 2026-10-25 - Player-build cleanup

- Merged `cleanup` (`fcc2ec9` as `80b7507`) conflict-free: the port-only
  full-screen help page, scenario selector and debug labels are gone; F11, F12
  and O moved behind `--dev`/`FFE_DEV=1` (Ctrl+F11 screenshot, Ctrl+F12 model
  viewer, Ctrl+Shift+F12 lab); F1 is the source view cycle again, F11 shows the
  source combat-computer note (view 6 is not ported) and F12 is the escape
  capsule/Stowmaster action. `input::view_command`/`dev_command` tests, the
  views/playable coverage and `docs/evidence/cleanup/` are in.
- No conflicts: the branch was based on the current `main` and the
  start-flow/events/hyperspace/prefs paths were already on both sides.
- Checks: workspace tests, clippy `-D warnings`, fmt; `views` (13), `playable`
  (13), input unit tests (11); `--smoke-frames 30`, `--dev --smoke-frames 10`,
  `--capture-start` and `--capture-prefs` all completed. Affected oracles
  `prefs/takeoff/surface/galaxy-map` pass.
- Owner decisions: F3 still toggles Modern/Classic although source F3 is the
  galaxy map (candidate to move to the Port Options page), F5 snapshots remain,
  `--start sol` stays CLI-only, and `tools/test-playable.ps1` still carries the
  old help/scenario assumptions.

---

## 2026-10-25 - Final batch: Thargoids, hyperspace, shipyard, ECM, game-over and events

- Merged, one at a time, each green and pushed: `thargoids` (04b65d1),
  `hyperspace` (2bf8592), `shipyard` (f7c83bb), `ecm` (5b0f6de), `gameover`
  (81a405b) and `events` (e7518ef).
- **thargoids (v0.92).** F690 draws `RND(F35(0x16))` and spawns the
  `DATA_006197` models with their statistics/hull/shields/drive; F774 selects
  the weapon, and the 0x11 role and 0xd5 equipment record are wired. Conflict
  resolution kept spawn-safety's `SpawnOptions`/normal-entry semantics and added
  the Thargoid limit to the spawner (`System::thargoids`,
  `Navigator::spawn_thargoid_traffic`). Owner decisions: no mission trigger,
  zero starting laser energy, hostile entry state, no ECM/missiles/surrender,
  original model names; `docs/evidence/thargoids/`.
- **hyperspace (v0.93).** Persistent map destination from the nearest-star rule,
  `H` jumps anywhere with help moved to `?`, console 0x0f, F908's
  altitude/fuel/range gates, F901's burn, F909's arrival reset and the
  Out of Range/Insufficient Fuel strings. Conflicts: `main.rs` (kept start-flow's
  F4 services block and added the Slash help key; combined F4 + help paths),
  `navigation.rs` (kept legal's illegal-fire reset and added the destination/
  altitude resets), `CONTROLS.md` (Escape/F4/help rows). Owner decisions: no
  drawn jump/misjump, only 16 systems enterable, destination clearing and
  altitude HUD remain; `docs/notes/hyperspace.md`.
- **shipyard (v0.94).** Prices/trade-in derive from statistics (`+0xa * 1000`,
  `950 * mass difference`), with the 58-model table, 13-row stock,
  buy/part-exchange and the BBS (20 templates, 18 posts, pagination), generated
  tables and station frames. Conflict: `navigation.rs` imports (kept both
  `Strings` and `shipyard`). Owner decisions: the model-name vs F437 name source
  and the `+0xe` shield/name confusion in `ships.rs` are documented without
  changing flight stats; RNG stream, availability byte and negative
  part-exchange remain; `docs/notes/station.md`.
- **ecm (v0.95).** ECM/naval ECM (F935, F718 table 5/6 and naval 7/9/10), F719's
  effect, F1057 decoy retargeting, F989/F1061 auto-targeter arming, the F10
  missile camera/HUD and console 0x14. Conflicts: `main.rs` (kept the lock
  message on `name`/`range_label`), `screen.rs` help text (locked target,
  targeter, ECM and `?` help). Owner decisions: chaff ammo source, NPC launches
  absent, ECM/F719 rendering, T fallback, E/roll conflict and the target split;
  `docs/evidence/ecm/`.
- **gameover (v0.96).** The real F1104 -> F1107 -> F917 tombstone (model 172,
  rare 175), no menu, a timed spin back to the front end, `--capture-gameover`
  and `--death-probe`; death's placeholder strings/options are gone. Conflicts:
  `main.rs` (combined all capture options, fixed the post-game-over
  `run_intro` call to the presenter signature and default start), `exports.inc`
  (kept the legal probe exports and added `_ref_damage`). Owner decisions:
  commander name source, load UI, the model-175 trigger and obituary text
  scale/face selection; `docs/notes/gameover.md`, `docs/evidence/gameover/`.
- **events (v0.97).** One source-tagged log writer (12 `Source` variants); the
  port-only entries are removed or rerouted (unported clicks, view names,
  pause/accel/HUD/save/takeoff notices), F271's replace/NULL-clear/ring/scroll
  semantics are in, with a `message_log` scan and `messages_view` evidence.
  Conflicts: `main.rs` (kept `pad_board` and the transient-note comment; routed
  the lock text through `pending_log.replace(Source::Target, ...)`) and
  `docs/notes/cockpit-dashboard.md` (F4 services, F5 transient note). Escape/pause
  messages stay off the log and the places list still wins. Owner decisions:
  DOS start-line inference, target-lock wording, unported-click feedback and
  DATA_008810/scroll actions; `docs/notes/message-log.md`.
- **`6a9dbf5` verdict.** The preserved galaxy-map console commit is **not
  superseded** by `places` or `hyperspace`: it adds F834's 0x1c..0x24 map
  console page and F840's F5-F12 re-imaging, which are still absent from the
  merged port. It is not merged here because it predates start-flow/hyperspace/
  events and its F5/F9/F10 bindings collide with the port's save/view/missile
  keys; it should be rebased as a focused follow-up with owner input on the
  key map. It remains on `backup/galaxy-map-6a9dbf5`.
- Checks: every merge ran workspace tests, clippy `-D warnings`, fmt, its
  cached/ignored suites and the affected oracles; the final full suite is
  56 entries (55 PASS / 1 SKIP / 0 FAIL on Windows). Cache stays v29
  (1,813 assets / 1,814 entries).

---

## 2026-10-25 - Music, display, intro depth, Saker and start-flow round

- Merged, one at a time, each green and pushed: `music-all` (5b7c986),
  `resolution` (3e9759d), `intro-fix` (28c1a7e), `intro-fix2` (bf7c997),
  `saker` (bf645be) and `start-flow` (952f729).
- **music-all (v0.86, cache v29).** All 17 `MUSIC/*.HMP` songs are catalogued
  and cached (1,813 assets / 1,814 ZIP entries); F1902's contexts are wired and
  `music_sequence` logs each call. Conflict: `main.rs`'s death branch (kept the
  F1104 death path and discarded the old starport recovery). After the merge the
  live `data.bin` was rebuilt from scratch with the merged `ffe-tools`
  (`build-data --game .\game --out data.bin`); the previous v28 file is kept
  as the ignored `data-v28.bin`, and v28 remains loadable. `verify-assets` and
  the full 56-entry suite pass. Owner decisions: timbre/system synthesis and
  F306/F307 are exposed but unwired; `docs/notes/music.md`.
- **resolution (v0.87).** Presentation-only `display::compose` (1x-6x
  pixel-perfect/fill, nearest/linear, 4:3/square, fullscreen), Display Options,
  Alt+Enter, `saves/display.cfg` and `--capture-display`; captures are
  byte-identical across runs. Conflicts: `README.md` (kept the current test
  count), `lib.rs` (kept both `death` and `display`), `main.rs` (kept the F1104
  block and added Alt+Enter, plus both usage strings). `verify-prefs` needed its
  test count 7 -> 8 for the new Display page test. Owner decisions: 640x480
  capture format, Windows-first fullscreen, reopen-on-scale and the page are
  port-only; `docs/evidence/display/`.
- **intro-fix (v0.88).** F1740/F1747's text-frame row negation removes mirrored
  GAME!TEK/PRESENTS/DAVID BRABEN'S text, the shape-detail patch follows
  `DATA_008812`, and the F1330/F1329 median-cut heap plus per-frame
  registration stops the 31,31,31 speckle. Conflict: the visuals evidence README
  (kept the branch's handedness wording and the child-model oracle note). Owner
  decisions: model 456->321 and the asteroid cluster stay skipped; 16 palette
  representatives remain the port's.
- **intro-fix2 (v0.89).** `model_math::text_frame` for all intro texts,
  F1840/F1841 surfaces merged into F1778's keyed depth list (ship in front of
  the planet), and F1879's atmosphere ring through the F1805 texture path
  (71 distinct colours, was 330). Conflict: the evidence README (kept the new
  depth/ring sections and the oracle note). `intro/fills/textures/surface`
  oracles pass.
- **saker (v0.90).** F998 type 0 is the Saker Mk III (`0x1f`, hull 112, 0/0
  shields, front `0x88`, two `0x82`, drive 2, 10,000 CR, 1 t hold, general
  `0x40240020`, index 87); `--start 0..5` and the Hope parent name are kept.
  Conflicts: `front_end.rs` (start docs/default constant), `main.rs` (boot,
  usage, HUD registration/source-speed), `navigation.rs` (start constants,
  `new_game`, cargo), `tests/front_end.rs`, `tests/navigation.rs` (added the
  Saker test, kept the Ross start-record-1 wording) and `docs/notes/intro.md`.
  The saker branch's pre-landed-view float HUD fixtures were converted back to
  integer speed words; `planet_view` now allows F1879's ring bitmap `0x43`.
  `verify-combat/takeoff/prefs/intro` and the exhaustive navigation test pass.
  Owner decisions: type-1 hold 1 t vs 7 t, model-name table shift, start types
  2-5; `docs/evidence/saker/`.
- **start-flow (v0.91).** F224 Pause->Open->Close (closing leaves paused), F4
  opens services over the 0x30 pad with F1/Backspace return, the pad world is
  ship + planet (F880 mass) + starport sibling so F603 gravity/F1033
  altitude/fall-back work, with a 120-frame pad grace and Tab/L launch;
  `--capture-start` and the prefs/takeoff updates. Conflicts: `main.rs`
  (capture-start plus resolution's presenter; combined capture options and
  usage), `tests/prefs.rs` imports, and `DEVIATIONS.md` EXP-010 (took the
  three-body/grace revision). `verify-prefs/takeoff/intro/surface/fills/textures`
  and the exhaustive navigation test pass. Owner decisions: manual pad takeoff,
  elevation-vs-AGL, the grace, Escape UX, Tab launch, altitude HUD;
  `docs/evidence/start-flow/`.
- Checks: each merge ran workspace tests, clippy `-D warnings`, fmt, its
  cached/ignored suites and the affected oracles; the exhaustive
  `autopilot_after_every_jump` used the release build (~60-105 s). The final
  full suite stays 56 entries (55 PASS / 1 SKIP / 0 FAIL on Windows).

---

## 2026-10-25 - Visual, start, landing, spawn, legal and death round

- Merged, one at a time, each green and pushed: `visuals` (33d00a9),
  `landed-view` (9ed9f89), `landing-audit` (b0fda6d), `spawn-safety` (b55be74),
  `legal` (30ffe51) and `death` (67a7b8e).
- **visuals (v0.82 preview).** A failed child subtree no longer discards its
  siblings and child curved surfaces are collected, so the intro's planet is
  back; surface colours are registered before the palette build with a
  per-frame cube clear, and the GAME!TEK text material/glyph colour is applied.
  Conflicts: none. Owner decisions: model 456 -> 321 stays skipped (stale
  basis), some logo rotations are a shade darker (normal/lighting), message-log
  stamps unchanged.
- **landed-view.** The default start is F829 record 0: Gateway [-1,4], planet
  Hope, starport Old Blackelk, Saker Mk III (0x1f), 10,000 CR, state 0x30, index
  87; state 0x30 draws the landed surface view. HUD parent `Hope`, F1393 speed
  columns at source positions, Set/Act and Alt. Conflicts: `hud.rs` and
  `main.rs` speed fields/messages. Resolution: kept landed-view's original
  speed-unit HUD (`PlayerHud::speed_columns`, F1393 `km{`/`km|`) over log-ui's
  m/s conversion, and adapted the places/log tests to integer speed words; the
  autopilot range messages stay log-ui's range-free text. The start-record-1
  test is explicitly not the default. Owner decisions: the Saker loadout still
  uses the Eagle/type-1 list, open-air re-docking does not enter services 0x2a,
  start keys 2-5 still Ross 154, the fine message trigger is not invented and
  duplicate log stamps remain; `docs/evidence/landed/`.
- **landing-audit (v0.82).** One F1100 formatter (`targeter::range_number` /
  `range_spaced`: 2 dp km below 17,592,186.04 km, 3 dp AU above; the raw
  fixed-point bug is fixed), F741's 4,096 km station hand-off and 6,144 km
  starport hand-off, `Navigator::land_at_starport` `0x2e`->`0x30` + F381/F654
  launch, F1903 stop at a starport landing, and `sequence_end_music` replacing
  the stale `docking_music` helper. Conflicts: `laser.rs` (combined both range
  tests), `main.rs` (launch climb, range messages, imports), `navigation.rs`
  (range/tests), `screen.rs` (range HUD), `NAVIGATION.md` (kept the resolved
  terrain paragraph and the new starport gaps). The `combat` oracle and the
  exhaustive navigation test pass. Owner decisions: final pad descent needs
  F741/F742/F745/F746 + F599/F600, pad contact needs the F576 decode or
  F1019/F1021 0x20/0x40, instant long-leg transfer vs F596/F606, crash/bounce
  fidelity, planet auto-land standoff.
- **spawn-safety (v0.83).** `npc::spawn` takes `SpawnOptions`; a normal entry
  uses F688's counts and the anchor orbit, while the capture-only guarantee and
  near-player pirate framing moved to `SpawnOptions::CAPTURE` and
  `respawn_traffic`. `Navigator::on_pad`/`landed_view` name F994's 0x30 and
  F1021/F1047, and main skips the NPC AI/fire/missile step while landed.
  Conflicts: `hud.rs` (the branch's pre-landed-view F64 HUD; restored main's
  F1393 block) and `main.rs` speed fields/message; mechanical integer speed
  fixtures in `boot_safety`/`planet_view`, and the landed HUD test now checks
  both columns. Owner decisions: anchor-orbit stand-in vs F702's system-wide
  point, per-kind count caps, landed freeze (source traffic still moves).
- **legal (v0.84).** F236 row 3 prints `$600` and records `6000`; the 4500 in
  the owner note is not in `DATA_005039` (the only 4500 is the Saker's ship
  value). Starport witness F234/F235 with the 0x13 exponential byte, no anarchy
  gate, wanted/alert, F783 clear, per-starport fines and F655's >=10,000 refusal
  are ported. Conflict: `navigation.rs` tests (combined range/starport tests
  with the distance-byte/witness tests). Owner decisions: 4500 vs 600 (keep the
  DOS table; an override would be explicit data), fines office/payment window
  and police spawns not ported, F2 LEGAL STATUS absent; `docs/notes/legal.md`.
- **death (v0.85).** The starport-recovery substitute is removed. F1104's
  external view and 5.001 s hold, F1098's capsule/Stowmaster with an exact
  loadout reset, F12 capsule launch, death cues/song stop, save rejection and
  the GAME OVER/COMMANDER DIED strings are in; the full game-over screen is the
  later `gameover` stream. Conflicts: none (semantic review confirmed the
  spawn-safety landed guard and legal police path still run). Owner decisions:
  commander name placeholder, derelict AI not ported, 5 s timer consumer open;
  `docs/notes/death.md`.
- Checks: per merge workspace tests, clippy `-D warnings`, fmt, the branch's
  cached/ignored suites and the affected oracles; the exhaustive
  `autopilot_after_every_jump` passes in release (59-105 s) because the debug
  run takes hours. Final suite: 56 entries (55 PASS / 1 SKIP / 0 FAIL on
  Windows); cache stays v28 (1,797 assets).

---

## 2026-10-25 - Six-stream integration round: controls, lasers, lock-on, log, places, terrain

- Merged, one at a time, each green and pushed: `terrain` (365bd36), `places`
  (ce2f35e), `log-ui` (03bae77), `lockon` (640ccf2), `lasers` (9307746) and
  `controls` (44f74f9).
- **terrain (v0.76).** `navigation::surface_radius(body, landed)` uses the inner
  `+0x140` base on approach and the outer `+0x138` collision radius once landed;
  `terrain_draw` culls the far shell for an eye inside it and suppresses the
  haze/sky, so F575 lands on the drawn sphere. EXP-016 records the per-state
  radius and inside-shell culling; `verify-surface.py` stays green (coverage
  0.6074). Conflict: `DEVIATIONS.md` (kept the npc-dock EXP-015 wording and
  added the branch's EXP-016). Owner decisions: per-state radius vs one outer
  radius, no depth buffer for far-side starports, textured fills now available.
- **places (v0.77).** F1099's list geometry (action 0x11, marker cells x10/230,
  names x25/245, 9 rows/column, 15-px rows, bitmap 0x31, colour 0x11, no
  panel), click targets/autopilot engagement and flight-only overlays; Gateway
  evidence frame. Owner decisions: the Gateway start (Old Blackelk/Hope) is not
  reachable (start type 0), paging beyond 18 is port-only, 0x11 is always
  available, and Alt/Hope belongs to the dashboard stream.
- **log-ui (v0.78).** F271/F272/F273: 120/124 wrap, header plus newest undated
  and older dated lines, five top-anchored rows at x196/y159 with clip
  158..200 and 0x18 scroll, `messages_visible`, F1393 km/h (km/s above the
  threshold), separate Set/Act lines, no autopilot range spam, PgUp/PgDn; the
  places list wins when open. Conflict fix: the places tests' `PlayerHud` speed
  literals became floats (mechanical). Owner decisions: colours approximate,
  the 0.342 km/h constant differs from the port unit by ~2%, the log draws after
  the console per source.
- **lockon (v0.79).** F1100's navigation green 0x11 and combat red 0x10,
  two-decimal AU/km (`5.40AU`), on-view `(x-5,y+5)`, off-view arrows/Behind/Out-
  of-Range, F1111 cursor radius 6/colour 0x10, and combat tunnels only in the
  missile view. Owner decisions: green is the navigation target and red the
  combat target per F1099, target-computer arming and the `+0x8a` gate remain,
  two simultaneous targets are collapsed.
- **lasers (v0.80).** F1776's raw muzzle origin with the front-view cull (no
  invented flash), interaction-box entry so boxes containing the gun no longer
  swallow shots, F930 laser1/laser2 bank by shot index, and `frame_tics` (994 at
  50 Hz, 828 at 60 Hz); `laser_view`/`combat_view` and `verify-combat.py`.
  Conflict: `verify-combat.py`'s test-name set (kept the lock-on colour test and
  the renamed front-only beam test). Owner decisions: AABB vs target axes, NPC
  gun origin, source-accurate no-front-muzzle, missile view/upper palette
  unchanged.
- **controls (v0.81).** Classic holds RMB to steer (F1038 in the original
  flight, 50-px stick in the prototype), Enter/Shift sets speed,
  preset-specific turret/external routing with no leaks, immediate switching
  and `saves/controls.cfg` persistence (`docs/CONTROLS.md`); Modern unchanged.
  Owner decisions: sensitivity option, prototype roll missing, Classic default
  vs Modern, only controls persist.
- Checks per merge: workspace, clippy, fmt and the affected cached suites;
  `verify-surface.py` after terrain, `verify-combat.py`/`verify-labels.py` where
  affected; the final suite is 56 entries (55 PASS / 1 SKIP / 0 FAIL on
  Windows). Cache stays v28 (1,797 assets).

---

## 2026-10-25 - Ship-label pen and clipping merged (v0.75)

- Merged `labels` (653f539, 236b4a8): ship labels now use F1108's projected
  point plus (4,−4), with F1584/F1598's signed top clipping and F1587's 0x13c
  right limit through the shared clipped-text path; the label mask and pen now
  match the original's walk and pixels.
- New `--labels` reference adapter, `verify-labels.py` (13 cases, trace
  SHA-256 `b08a9b62…`) and `labels_reference.rs`; `build-smoke.ps1`'s manifest
  records `labels_adapter = 'labels-v1'`. `verify-all.py` now has **56
  entries** (labels included; the existing suite is unchanged), so the suite is
  55 PASS / 1 SKIP / 0 FAIL on Windows. Cached `npc_view` label frames and
  `console_view`/`views` assertions accompany the images in
  `docs/evidence/labels/`.
- Merge was conflict-free (`ort`). The reference binary was rebuilt for the new
  adapter before running the oracle.
- **Owner decisions/remaining:** the original shows type registrations (for
  example `AB-123`) rather than the port's model name + serial — port
  F703/F791 or keep readable names; 3D hull text/decal scaling is separate; and
  near-target suppression, overlap order and mission labels remain unported.
- Tests: workspace, clippy, fmt; `verify-labels.py` (13 cases), cached
  `npc_view` (3), `console_view` (4), `npc_ai` (3) and `views` on the v28
  cache; `docs/notes/labels.md`.

---

## 2026-10-25 - Textured fills and terrain textures merged (v0.74)

- Merged `fills` (210d749): `Shape::TexturedUv` carries F1645/F1648's explicit
  per-vertex u/v words into the F1644/F1647 affine span rasteriser
  (`ModelViewer::rasterise_uv`), and `terrain_texture` reads the real
  F1805/F1807/F1806/F1808 queue layouts and `DATA_009252` slot base instead of
  the flat/gouraud stand-in. `Screen::draw_shapes` draws the new shape and the
  cached `planet_view`/`effects_view` assertions cover it, with frames in
  `docs/evidence/fills/`.
- **Palette reconciliation:** fills call `Textures::rgb`/`ModelViewer::rgb`,
  which resolve a colour through the palette branch's runtime entries first
  (F1326/F1327) and fall back to the fixed low map only for words not yet built
  (then the pre-palette nibble approximation). EXP-013 is untouched: the space
  colour remains `#0c0c4c` (Black `#000000`). The API names/semantics the
  `terrain` branch was told to use (`Shape::Flat`/`Polygon`/`TexturedUv`) are
  unchanged and documented.
- Evidence: F1876's Mars disc at two radii has 27 background pixels of 27,516
  inside it (surface colour 0 is legitimate), and the starport fill covers
  50,191 pixels with a 320-pixel horizontal run; `verify-fills.py` passes
  (28.6 s).
- Merge was conflict-free (`ort`); notes in `fills.md`, `textures.md`,
  `surfaces.md`, `combat.md` and `npc.md` now say fills/textures are in and the
  old "wireframe" statements are corrected.
- **Owner decisions/remaining:** model 155's fragment placement and `+0x7e`
  (the blue cloud is model 153 debris lines, not unfilled polygons); the
  upper-palette approximation inside fills; shading fidelity; there is no pixel
  oracle for F1806/F1808 yet; `EXPLOSION_MASS` and effect lifetimes as before.
- Tests: workspace, clippy, fmt; `verify-fills.py`; cached `planet_view` (19),
  `effects_view` (3), `scanner_view` (2), `dashboard_view` (4), `combat_view`
  (6), `intro_view` (2) and `palette_reference` (5) on the v28 cache.

---

## 2026-10-25 - Starport tech level, Upgrades and repairs merged (v0.73)

- Merged `station-services` (13ae64c): F869/F622's starport tech level reaches
  `Navigator` (`galaxy::Details/Tables::tech_level`, `Navigator::tech_level`),
  and the Upgrades screen builds real rows from `equipment::outfitting` with
  Fit/Remove across general equipment, cabins, lasers, shields and missiles at
  F387/F388 prices/refunds. F394's fuel-scoop gate, F329's Remove button/hot
  area and F444's repair rows with dates are in; the shipyard and BBS keep
  their faithful empty states. The `station_view` covered test now checks the
  Upgrades/Ships/Repairs frames too.
- Conflict: `lib.rs` only (kept `palette` and `outfitting`); `station.rs`,
  `main.rs`, `screen.rs` and `navigation.rs` auto-merged with the npc-dock,
  effects, missiles and palette work, and the workspace is green.
- **Owner decisions:** drive/mining rows are visible-but-disabled rather than
  hidden; ship prices/BBS tables wait for their imports; repair actions are
  listed before thruster damage exists; the price multiplier follows the
  current F383 table.
- Tests: workspace, clippy, fmt; `station_view --ignored` (1, including the new
  Upgrades/Ships/Repairs frames), `market_view`, `dashboard_view` and
  `console_view` cached suites; the v28 cache is unchanged.

---

## 2026-10-25 - NPC docking cycle merged (v0.72)

- Merged `npc-dock` (f42b9b2): the `DATA_006255` AI table rows were offset and
  are now correct; NPCs fly Approach, Docked, Launch and Depart, with
  `DockRequest`, `tick_with_station`, bay occupancy through the station's
  traffic record, F747's door-close wait and F733/F734/F735/F736/F761/F762.
  Docked ships are hidden, traders receive destinations, and the cycle is
  covered by `npc_docking` and its `docs/evidence/npc-docking/` frames.
- Merge was conflict-free (`ort`); the palette/npc.rs/navigation.rs changes
  auto-merged and the playable still passes.
- **Owner decisions/remaining:** the original's 2-3 day docked interval is
  shortened for observation; a ship that cannot dock gives up after 30 s and
  tries the next station rather than retrying endlessly; the cruise-speed
  floor and the despawn range are the port's. Missiles, police
  spawns/fines/HUD badge and Thargoids remain outside this cycle.
- Tests: workspace, clippy, fmt; `npc_docking` (1 ignored), `npc_ai` (3),
  `npc_view` (1), `missile_view` (1) and `effects_view` (2) cached suites pass
  against the v28 cache.

---

## 2026-10-25 - Faithful runtime palette merged (v0.71)

- Merged `palette` (fe4aa1d): `ffe_rs::palette` implements the original's
  runtime upper palette (F1326/F1327/F1328/F1329/F1330/F1331): the 64-box
  median cut with odd representatives, the two banks, the fixed 128-entry low
  palette and its fixed map, and the bank toggle. `textures.rs` and
  `model_viewer.rs` register and resolve upper-palette entries, so textured and
  model colours now go through it; the intro uses its own F1575 start colours
  (`(0,0,0)` for model 177, `(0,0,4)` for model 176) and reproduces the
  executed switch frame's single upper entry (six-bit `(1,1,17)`, `#040444`).
- **EXP-013 preserved:** the drawn space colour stays the captured `#0c0c4c`
  and the Black option `#000000`; the palette path does not override the
  prefs sky.
- Conflict: `docs/notes/intro.md` (the old "dynamic upper palette not ported"
  text vs the new section); combined, keeping the music/model-176 notes.
- Tests: `palette_reference --ignored` (5), intro (2), scanner (2),
  galaxy-map (2) and `planet_view` (19) cached suites, plus the workspace,
  clippy and fmt. Cache v28 unchanged.
- **Owner decisions/remaining:** terrain queue colours are not pre-registered
  (the queue's colours resolve at draw time), one palette build per model
  render, and the intro backdrop's missing planet/asteroid scene makes the
  upper palette ramp more slowly than the original's (`docs/notes/palette.md`
  §4).

---

## 2026-10-25 - Software mixer, WinMM sink, intro theme and cache v28 (v0.70)

- Merged `audiomix` (638f32a): `ffe-audio::mixer` decodes the cached WAVs,
  keeps the 40-entry sample table and 16 voices with F1908's reuse,
  fixed-voice and laser1-steal rules, bakes loops, pitch-resamples and applies
  F1907's exact centibel volume curve; `sink`/`waveout` add a WinMM `waveOut`
  sink with a headless fallback. `GameAudio::from_cache`/`system_with_effects`
  connect it to the playable, so the cues wired by `soundfx` are audible. The
  intro plays song 0 through F919/F1904 and stops on `leave_front_end`
  (`music_on` respected).
- **Cache v28** adds `base:music/qqfront`: 1,797 assets / 1,798 ZIP entries.
  The reader still loads v27 (v28 is preferred); `verify-assets.py` expects the
  v28 counts. The local `data.bin` was rebuilt to v28 and the full oracle suite
  ran against it.
- Merge was conflict-free (`ort`). One transient full-suite failure was
  environmental: `target/release/ffe-rs.exe` was briefly locked (cargo could not
  remove it, failing `depth`/`textures`/`lines`/`fills`); after rebuilding the
  release binary the same four passed and the re-run gave **54 PASS, 1 SKIP,
  0 FAIL of 55** (the SKIP is `world-linux` on Windows).
- **Owner/remaining:** continuous station/engine beds (F1910/F1908), positional
  audio (F1759 is unused by DOS), `.mid` overrides, caching the other 16 songs
  (owner decision) and a device picker. The mixer's rules and the v28 fallback
  are in `docs/notes/sound.md`/`sound-cues.md`.
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `cargo test -p ffe-audio` (3), `--test sound_mixer` (3),
  `--test sound_cues` (2), `--test front_end` (9); every cached view suite
  (dashboard, combat, console, station, lift, market, npc, npc_ai, effects,
  scanner, missile, intro, planet) passes against v28.

---

## 2026-10-25 - Deterministic reference gate and the new oracles merged (v0.69)

- Merged `oracles` (f812f2b): the Windows `--worldgen` repeatability failure is
  fixed at its root. F877/F878 build part of each object's name from an
  uninitialised byte, so two runs can differ; the two-run comparison now masks
  only the object-name ranges `0x26..0x3f` and `0x124..0x138`, keeps the raw
  first run for the Rust oracle and compares every other byte exactly. The
  world scenarios are split into `world` (Windows-safe) and `world-linux`
  (`--sysgen` needs a working SIGALRM watchdog, `--surface-draw` needs POSIX
  fork/`open_memstream`), so the Windows gap is an explicit SKIP rather than a
  failure.
- `verify-all.py` was rewritten to print one PASS/FAIL/SKIP line per oracle,
  map exit 77 to SKIP, enforce per-oracle timeouts, and support `--list`,
  `--only` and exit codes (0 clean, 1 failure, 2 missing input). The merge kept
  the surface branch's `verify-surface.py` entry and the v27 fixes
  (`verify-assets.py`'s 1,797 ZIP entries, `verify-intro.py`'s no-text-tamper),
  and the final list has **55 entries**.
- New oracles: `verify-combat.py` (executed F998 loadout/hitscan/pixels),
  `verify-ship-types.py` (F635's nineteen records from `ffedat.asm`),
  `verify-npc.py`, `verify-dashboard.py`, `verify-takeoff.py` and
  `verify-prefs.py`, plus the `npc_reference.rs` Rust side. The `ship-types`
  oracle found **F635 table slot 0x3** carried `DATA_006181`'s list instead of
  `DATA_006183`'s (and the Federal/Imperial labels were swapped); fixed. Ross
  154 uses slot 5, so the playable is unchanged. Its trace SHA-256 is
  `ae421da6e844b377033ca95d094458ea0894575fc0a0f1ca7acc350e8391d2cd`.
- **Full suite (v27 cache, 1,796 assets / 1,797 ZIP entries): 54 PASS, 1 SKIP,
  0 FAIL in 708 s.** The only SKIP is `world-linux` on Windows. `verify-prefs`
  expected five unit tests; it now expects six (the soundfx rows added one) —
  a mechanical count fix.
- Remaining oracle gaps: F688's spawn sequence and F635's `population < 4`
  branch are not covered; `hud`, `npc`, `takeoff` and `prefs` are pinned from
  `ffe.asm`/the cache rather than frame captures; `galaxy-map` is the only
  frame-level comparison.
- Checks: full `verify-all.py` (above), `cargo test --workspace --locked`,
  clippy `-D warnings`, `cargo fmt --check`. No game behaviour changed for
  oracle convenience.

---

## 2026-10-25 - Surface landing and the landed view merged (v0.68)

- Merged `surface` (f05c403, 2bbfa20): F1021's rough touch drives 0x32 and
  `landing_frame` picks F1047's 0x34; the starport 0x2e/0x30 states,
  `Takeoff::from_landed`, `OriginalFlight::settle_landing`, the F1050/F1054
  landed view and F1877's haze sky (with F1775's stars suppressed while it is
  set) are ported. `L` climbs again through 0x24 -> 0x26 -> 0x00. The core
  `takeoff` tests grew the landed sequence and `planet_view` the full
  landing-to-launch frames.
- Merge was conflict-free (`ort`, main.rs/navigation.rs/screen.rs auto-merged),
  but one stale test was fixed: `planet_view`'s landed background check only
  drew when `FFE_SHOT_DIR` was set, so it read an undrawn frame in a normal run;
  it now draws explicitly and passes with shots off.
- **`verify-surface.py` is now part of `verify-all.py`.** It runs the executed
  original's landed frame, drives `planet_view`'s `new_game_landed_surface_view`
  and reports `sky_matches: true`, `header_close: 0.9934`, `view_inside:
  0.9969`, `view_coverage: 0.589`, `ours_view_pixels: 25086` vs the real
  42594, and rejects tampering. The full suite is now **48 entries; 47 pass**,
  the one failure being the known Windows `--worldgen` ASLR case. Two stale
  expectations were fixed: `verify-assets.py` expects the v27 ZIP's 1,797
  entries, and `verify-intro.py` no longer tampers the plain `text` field (the
  port compares the cached DOS bytes' codes/colour/position/wrap, not the
  trace's transcription).
- **Owner decisions:** the `+0x138` collision radius vs the `+0x140` drawn
  radius mismatch can show space where the original shows ground; landing on a
  starport pad from flight is still not routed to 0x2e/0x30; 0x30 shows the
  station services (EXP-014) rather than F1050's landed view; the landed state
  is not saved; surface object depth/fills remain. The local v27 cache rebuild
  is required (the branch predates it).
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `cargo test -p ffe-core --test takeoff` (6),
  `--test planet_view -- --ignored` (19), `verify-surface.py` and
  `verify-all.py --only surface`; all cached view suites still pass.

---

## 2026-10-25 - Missiles, lock-on and pylon lamps merged (v0.67)

- Merged `missiles` (ef41062): `ffe_core::missile` (the ten pylon codes and
  damage table, F941's launch impulse and separation, F758's 20-frame guidance,
  the 60-update/24 s lifetime and 131 m fuse, F718/F721 contact and F953
  damage), `ffe_rs::missile`, ten pylon words in `Combat`, and
  `Navigator::{missile_lock, missiles, launch_missile, tick_missiles}`. `M`
  launches the first pylon only after a `Shift+T` lock; missile objects draw
  through the model renderer, the combat target draws F1114's circle and the
  F1120 pylon lamps follow `Combat::missiles`.
- Conflicts: `main.rs` (the NPC event block vs the missile tick; the lock/lamp
  `ShipSystems` fields), `navigation.rs` (spawn clear), `screen.rs`
  (`SystemEffect` vs `SystemMissile`, and the targeter/gunsight interleave),
  `docs/notes/combat.md`. Resolution keeps the shipai/effects tick and event
  loop, then ticks missiles and handles `MissileEvent`; both `SystemEffect` and
  `SystemMissile` are drawn; the equipment `Loadout` remains the saved pylon
  mirror while `Combat::missiles` is the live F941 word set used by the lamps
  (the new game sets both).
- `--missile-probe` in `tools/reference/headless.c` (with its export alias)
  runs the executed original's new game: the pylon `[130,130]` becomes
  `[0,130]`, a model-6 object appears with owner 107/target 114, and its
  guidance runs every 20 frames. That is the calibration for the port's 0.4 s
  guidance interval, 24 s lifetime and per-thrust acceleration.
- **Owner decisions:** the missile view (F10), NPC launches (F754), ECM, the
  F1113 navigation box and the remaining F1116 targeter lamps are not wired;
  `Shift+T` is the port's lock gesture; the F597 lead and F585 turn
  quantisation are approximated in floating point.
- Cache: the branch predates v27; the local v27 `data.bin` (1,796 assets,
  ignored) served every cached test.
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `--test missile_view -- --ignored` (1), `--test combat`
  (9) and `--test npc_ai -- --ignored` (3).

---

## 2026-10-25 - Hit particles, explosions and debris merged (v0.66)

- Merged `effects` (9974338): `ffe_core::effects` spawns F939's model-158 hit
  particle at F817's hit point, F942's model-155 destruction object and F926's
  model-0x99 debris pods (the pod count from the ship's `+0x116` cargo,
  `min(4, cargo >> 1)`); `Navigator::effects` holds them and `tick_ships` ages
  them; `Screen::SystemEffect` draws them. `--capture-traffic` and the traffic
  capture timing were brought over, and `NpcShip`/`Spec`/`Persisted` carry the
  cargo word so a restored ship still makes debris.
- Conflicts: `lib.rs`, `npc.rs`, `main.rs`, `navigation.rs`, `ships.rs`,
  `npc_view.rs`, `combat.md`, `npc.md`. The effects branch's `damage_ship_at`
  was reconciled with the shipai `player_attack`: `damage_ship_at` now returns
  `Option<Attack>` and calls `player_attack` (legal/bounty/police) after
  spawning F939/F942/F926 at the pre-hit position, axes and velocity;
  `damage_ship` remains the `Damage`-only wrapper, and `tick_ships` keeps the
  shipai signature (events + player radius) while ticking the effects even when
  the traffic list is empty. `Spec`/`NpcShip`/`Persisted` carry both branches'
  fields (thrust/drive/mounts and cargo).
- **F942's `+0xa4` gate:** model 155's twelve fragment calls read object global
  word 4; F942 writes `0xffffffff` (all bits set) which selects none. The port
  deliberately passes FFP 1.0 (`EXPLOSION_MASS` 0x4000) so the twelve fragments
  draw, and they are wireframe until F1560/F1613's polygon fill lands. This is
  a recorded deviation/owner decision.
- **Owner decisions:** effect lifetimes are the port's per-effect seconds
  rather than the original's AI timers (F764/F769/F927); keep `EXPLOSION_MASS`
  0x4000 or follow F942 literally (no fragments); the fills dependency
  (F1560/F1613) before the explosion is solid. Sound cues from these paths were
  already wired.
- Cache: the branch predates v27; all cached tests ran against the local v27
  `data.bin` (ignored).
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `--test effects_view -- --ignored` (2), `npc_view` (1)
  and `npc_ai` (3), plus dashboard/combat/console/station/lift/market/scanner/
  intro cached suites.

---

## 2026-10-25 - Scanner contacts and the combat targeter merged (v0.65)

- Merged `scanner` (7a82070, eda3790): `ffe_rs::scanner` implements F275's
  contacts (live bodies and NPC ships, parent-frame relative positions, the
  19-colour `DATA_008683` table indexed by the mass word, stalk geometry and the
  8-frame orientation overlay gated on `ship+0xca & 4`); `ffe_rs::targeter`
  implements F1113's four-box tunnels, F1114's edges, F1612's circle and F1100's
  size rule. `main` feeds scanner input from `Navigator.bodies` and live ships;
  the targeter tunnels are gated by the preference and the target flags, and the
  screen draw order is fixed to the source order.
- Conflicts: `console.rs` (`ShipSystems`), `combat.md` and `dashboard.md`.
  `ShipSystems` now keeps the equipment slice's `status`/`general` as canonical
  and adds only `ca` (F272's scanner gate, `& 4`; default 0x04 for every flyable
  ship) rather than the scanner branch's overlapping `c6..cb` copies. Docs were
  combined: contact colours are done, the targeter is drawn, and the remaining
  owner decision is the auto-targeter *arming* (target-computer bit and
  F1010/F1060/F1062; `T` still cycles locks).
- Cache: the branch predates v27, so the local `data.bin` (v27, 1,796 assets,
  ignored/untracked) was rebuilt and all cached suites ran against it.
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `--test scanner_view -- --ignored` (2); cached
  dashboard/combat/console/station/lift/market/npc/intro/npc_ai suites all
  pass.
- Left: auto-targeter arming/equipment, F1100's per-target text colour (nav
  0x11/combat 0x10/missile 0x5f) and its arrow forms; two simultaneous targets
  (`+0xec`/`+0xed`) are collapsed to one display (JJFFE-vs-DOS candidate recorded
  in `eda3790`).

---

## 2026-10-25 - Intro playback merged and the cache moves to v27 (v0.64)

- Merged `intro` (b4cd65e): the default boot plays F961/F962 from the DOS
  `FIRSTENC.EXE` (models 177/176, primary bitmap 0x42, the 18 credits with their
  `0x0d`/`0xfd`/`0xfe`/`0x01` layout codes, the `0x400` GameTics clock, the
  `0x7e7b76` reset), V's version line after `0x719668`, keys 1-5 and
  `--capture-intro`. `--skip-intro` and the scripted capture/smoke modes bypass
  the front end; `--start sol` is unchanged.
- **Cache v27** adds `base:text/intro-credits` (19 NUL-terminated strings) and
  `base:text/char-widths` (256 signed bytes): 1,796 assets, no other change.
  Old v26 caches still load for the game, but the intro (and `--capture-intro`)
  report the missing assets; the local data.bin was rebuilt for testing and is
  ignored/untracked. `docs/FORMATS.md` records the versions.
- Merge was conflict-free (`ort`). One stale local frame test needed a fixture
  fix: `console_view`'s navigation-button check now fits F991's navigation
  computer bit (general bit 5) before drawing, because the equipment merge made
  F991 hide `0x11` without it. No code behaviour changed.
- **Not reproduced:** the dynamic upper palette (`F1575`/`F1330_DPalBuild`), so
  F961's start colours do not build the original's runtime palette; model 176's
  late animation calls child 456 through a basis F1750 leaves on stale stack
  (reported once, the credits keep drawing, those frames have no ship); and no
  intro music backend exists, so song 0 (`QQFRONT`) is still silent.
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `cargo test -p ffe-rs --test front_end` (9); the cached
  `intro_view` with `FFE_INTRO_CACHE=data.bin` (2); `--capture-intro` wrote the
  nine sample frames; and the cached dashboard/combat/console/station/lift/
  market/npc view suites pass with the v27 cache.
- **Owner decisions:** start types 2-5 and the DOS type-0 arm are mapped to type
  1 with a notice (needs the real F998 records); the dynamic upper palette;
  which music backend should play song 0; and the first in-game frame's exact
  hand-off (the intro leaves into the new game directly today).

---

## 2026-10-25 - NPC flight/combat AI, fire and player damage merged (v0.63)

- Merged `shipai` (dd573c2, 543f660): the encoded `DATA_006258` AI state table
  and the transit/hold/attack/escort/pass/flee/dead/escaped states, F585's turn
  limits, the statistics-based thrust/braking speeds (the constant-speed
  placeholder is gone), NPC fire through F931/F930 with F953 player damage,
  police/wanted (`Navigator::{legal, player_attack, provoke}`, F697 bounties)
  and F674/F675/F680 jump persistence with F927's recycle.
- **DOS vs JJFFE:** JJFFE's "Laser invulnerability" fix replaces F724's F931
  call with just the fired flag, so JJFFE's AI ships do no damage. The port
  follows DOS FFE and fires; recorded in `docs/notes/npc.md`/`combat.md`.
- Conflicts: `npc.rs` module doc (kept shipai's state-machine text),
  `flight.rs` and `tests/combat.rs`. **One damage API** reconciled:
  `Flight::take_damage(damage, mass)` is the F953 path (fills the hull/shield
  words on first use); `ensure_damage(hull_full)` still refreshes them each
  frame from the statistics/Loadout; `hit(damage)` is kept for the equipment
  slice's callers/tests and shares the same internal apply; `destroyed()` is
  the single F942 test. Both branches' tests are kept (equipment save/loadout
  and `ensure_damage`/`hit` cases, plus `take_damage` and its save test).
  `main.rs` needed a hand fix: the player-hit message/sound block was merged
  against `Navigator::player_attack`'s older `Option<Damage>` shape; it now
  matches `Attack { damage, crime, bounty, police, destroyed }`.
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `cargo test -p ffe-core npc` (16);
  `-p ffe-rs --test npc_ai -- --ignored` with the cache (3); `--test combat`
  (9); cached `dashboard_view` (4), `combat_view` (6), `npc_view` (1) and
  `station_view` (1) still pass.
- Owner decisions raised by the agent and left open in `docs/notes/npc.md`
  §5: the new-game Eagle has no shield generator (a 1MW pulse kills it in two
  hits), the 25% flee trigger vs F737's position rule, `MAX_SHIPS = 5`, and
  the full docking AI. Also left: missiles, police spawns/fines/HUD wanted
  badge, Thargoids, F942's model-0x9b explosion and AI sounds.

---

## 2026-10-25 - Equipment table, fitted loadout and player damage merged (v0.62)

- Merged `equipment` (da4eb8c, f5a1d7d): `ffe-core::equipment` with the
  66-record `DATA_005289` table (generated by `tools/gen-equipment-table.py`),
  F393's general-bit formula and mask, F383's availability and prices, and
  `Loadout` (general bits, ten missile pylons, cabins). `Flight.loadout` is
  serde-defaulted so old saves load; `Flight::{ensure_damage, hit, destroyed}`
  apply F953's shields-then-hull order over the existing `laser::apply_damage`.
  Console F991's view-row icons, F1120's missile icons/lamps and the engines-off
  page now read the real state; the new game's two `0x82` missiles draw their
  icons. No cache change (still v26).
- Merge was conflict-free (`ort`, `main.rs` auto-merged); the `Flight` fields
  and the `Loadout` API stay public and documented for the `shipai`/`missiles`
  branches to rebase onto.
- **Record 53 (Class 3 Military Drive):** the DOS `FIRSTENC.EXE` bytes at file
  offset `0xc1674` differ from the JJFFE listing (tech `0xeb` vs `0xd7`); 1,451
  of 1,452 bytes match otherwise. The port follows DOS. F383's second-hand
  price random is not reproduced (deterministic tech multiplier), recorded in
  `docs/notes/equipment.md`.
- **Flagged for the combat owner:** the equipment table's laser category codes
  name `0x9a` 30MW mining, `0xc6` small plasma, `0xcf` large plasma and `0xd5`
  Thargoid, while `ffe_core::laser` names `0xc6` 30MW mining and `0xcf`
  Thargoid. The table/`DATA_006265` are the data; this merge did not rename the
  combat slice's constants (the F930 dispatch is unaffected, but the message
  names in `main` should be reconciled there).
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `cargo test -p ffe-core --lib equipment` (8),
  `-p ffe-rs --test combat` (7), and the cached `dashboard_view` equipment
  frame test (4).
- Still open: nothing fires at the player, so `Flight::hit` has no call site;
  F938 recharge/auto-repair, F1104 death/F1098 capsule, `player+0xef`, and the
  station Upgrades rows. The last needs the starport tech level `DATA_008995`
  from F869 — **do not invent it**; `equipment::outfitting` is ready for when it
  exists (`docs/notes/station.md`).

---

## 2026-10-25 - Sound cues and scene music merged (v0.61)

- Merged `soundfx` (f985ea1, d88036f): `ffe_rs::sound::{GameAudio, Cue}` with
  F1906/F1907/F1908 dispatch and the per-sample gates, F1902-F1904's music
  rules (explicit/random/done/continuous), the cue-to-sample table from
  `w32sound.cpp`, and the live event wiring: preference pages and toggles,
  console/station beeps, message bong, ship lock, lasers, shield/hull hits,
  explosions, battle siren and song 14/stop, docking/launch doors and samples,
  engine/station beds, camera, hyperspace and song 15, docking music, and the
  world tick for MIDI/continuous music. The Sound Effects page is applied at
  boot and on change.
- Merge was conflict-free (`ort`); the prefs and main.rs additions sit beside
  the HUD/combat/NPC/takeoff wiring and all of it still passes.
- **The effect mixer/device is still missing:** `AudioEngine`'s device engine
  owns a `NullSampleSink`, so cue calls are recorded/no-op on a real run; music
  is audible through the HMP/WinMM player. Song 0 (front end) is not reachable
  until the intro is drawn. ECM, missile and wind cues exist as API + tests
  only; `docs/notes/sound-cues.md` records the table, call sites and gaps.
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `--test sound_cues` (2 + unit tests) and `--test prefs`
  (10). The first full test run hit a full `C:` drive (`Os error 112`) in
  `ffe-tools/tests/inspect.rs`; after clearing `target/debug/incremental` and a
  stale 691 MB temp mirror the suite passed. That was environmental, not code.

---

## 2026-10-25 - NPC traffic, ship damage and targeting merged (v0.60)

- Merged `npcs` (81569a2): `ffe_core::npc` (F635's nineteen-record model table
  with the allegiance shift, F922's bounded RNG, F688's counts and cap,
  placement around the station/player, the three-state flight AI, F953/F942
  damage and the F1536 shield flash), `Navigator::{ships, spawn_traffic,
  tick_ships, lock_ship, damage_ship}`, `Screen::{SystemShip,
  draw_system_ships}` (ship models, labels, F1100 range and F1099 hull/shield
  readout), click/lock/`T` cycling and `--capture-traffic`.
- Conflicts: `navigation.rs` (the new-game seat block — kept the takeoff
  branch's `models`/`pads.seat` computation and added the spawner's
  `launch_axis`, dropping the older duplicate seat/cargo code) and `screen.rs`
  (kept the retired `MAP_BUTTONS` comment and added `SystemShip`). No stream's
  work was dropped.
- The target hull/shield readout and the model-157 shield flash are now
  reachable in a generated system; the notes' "unreachable" wording is
  corrected in `docs/notes/cockpit-dashboard.md`. Full AI, police response, ship fire,
  missiles, F942's own explosion, station-bay ships and persistence remain
  (`docs/notes/npc.md` §4).
- Checks: `cargo test --workspace --locked`, clippy `-D warnings`,
  `cargo fmt --check`; `cargo test -p ffe-core --lib npc` (6);
  `cargo test -p ffe-rs --test npc_view -- --ignored` with the cache (1); the
  cached `dashboard_view`, `combat_view`, `console_view`, `station_view` and
  navigation takeoff suites still pass.
- Deviation: the spawn placement is the port's station/player-based rule, not
  F702's raw system-wide orbit; F691's station-bay ships are not spawned
  because the ship-data table is not loaded. To be recorded as its own
  deviation entry if the owner keeps this placement.

---

## 2026-10-25 - Open-air starport takeoff merged (v0.59)

- Merged `takeoff` (74da8d8): F994/F650's seat (the pad offset plus the ship's
  `+0x140` collision radius along its up row, `Pads::seat`), F381/F654
  clearance, F1013's `0x24` climb, F1031's starport `0x20` effect-region test
  and hand-off to `0x26`, F1032/F651's bay release, F1042's zero motion and
  F573/F1677's return to the system frame. The pad contact runs F575/F576's
  complex-shape scan in a persistent two-body world
  (`OriginalFlight::{begin_pad_takeoff, step_pad, end_pad_takeoff}`), so the
  old invented 20 km teleport is not involved. `ffe_core::traffic::Takeoff`
  holds the mode word; `Navigator::{begin_launch, takeoff_step, takeoff_seat}`
  drive it. The default new game starts landed at Ross 154's Sirocco Station.
- **The refusal is gone.** `begin_launch` no longer rejects an open-air
  starport. The only remaining deviation in EXP-010 is F1013's altitude test:
  a starport's complex shape leaves F1033's altitude at `i32::MAX`, so the port
  measures the climb from the pad seat along its up row instead.
- Merge was conflict-free (git `ort` auto-merged `main.rs`); the HUD round's F2
  status page and console table, the combat HUD and the station/prefs screens
  are untouched. Checks: `cargo test --workspace --locked`, clippy
  `-D warnings`, `cargo fmt --check`; `cargo test -p ffe-core --test takeoff
  --test pads` (4 + 5); the two cached navigation tests
  (`new_game_starts_at_ross_154`, `new_game_takes_off_from_the_pad_into_free_flight`)
  pass.
- Docs: EXP-010 rewritten (takeoff ported, refusal removed); `STATUS.md`,
  `NAVIGATION.md`, `docs/notes/cockpit-dashboard.md` and `docs/notes/intro.md` no
  longer call takeoff blocked; the 2026-10-24 PORT_LOG bullet is marked
  superseded.

---

## 2026-10-25 - Original HUD, message log and table-driven console merged (v0.58)

- Merged `hud` (2cccccd, 4d4f2be): `ffe_rs::messages` (ten-entry dated log,
  124-pixel wrap, 8-pixel lines, (195/196) shadow/text, y 158 clip, F273 scroll
  and auto-scroll), `ffe_rs::hud::PlayerHud` (shields (2,1), hull (2,11),
  Autopilot (2,139), Set/Act/Alt (2,148), turret Decn/Rotn (5,76)),
  `console::ShipSystems` with F1116/F991/F1117/F1120 (lamps and the engines-off
  page, toggled by F2 or console record 0x0c), and `console_table.rs`, all 82
  `DATA_005100` records generated byte-exactly by
  `tools/gen-console-table.py`, with F261's draw order, F262's
  set-image-with-radio-reset and F263/264's visibility.
- The branch retired the port's top bar, controls button, SPEED line and the
  119..157 status/map/autopilot block. That region contained the lift,
  station-view and navigation-list guards added in earlier merges; the branch
  replaces it with the original HUD and message window, so those guards have no
  block left to guard and were not reintroduced. The lift, station, prefs and
  combat work itself is untouched (their local frame suites pass).
- Merge was conflict-free (git `ort`), but the integration checks were run:
  `cargo test --workspace --locked`, clippy `-D warnings`, `cargo fmt --check`,
  and the local cached-frame suites `dashboard_view` (3), `combat_view` (6),
  `console_view` (4), `lift_view` (2) and `station_view` (1). An untracked
  stale copy of `console_table.rs` from the earlier aborted attempt was removed
  (the branch's tracked version is authoritative).
- Two stale port messages fixed while integrating: the F2 status page said
  "F2 or F1 returns" (only F2 toggles it) and the pause overlay still offered
  "F2 SCENARIOS" after the branch removed the scenario selector.
- Docs: `docs/notes/cockpit-dashboard.md` now ticks the message log/clock, HUD readouts,
  lamps/engines page and the 82-record table; the Set/Act position is corrected
  to x=2 (not 15); the audit records what remains (player damage and equipment
  words, `F991` 0x10-0x14, altitude/F1393 speed units, scanner contacts, the F3
  key mapping and the unverified DOS F2 mapping). `docs/notes/console.md`
  records the table/state-machine port.

---

## 2026-10-25 - Crosshair, laser firing and the combat HUD merged (v0.57)

- Merged `combat` (0c28c87): `ffe_core::laser` (F930's table, F931's gates,
  F929's energy, F730's refire, F938's cooling, F953/F942 damage, F1100's
  range words, F270's gauge heights, the F817/F819 hit test),
  `ffe_rs::combat::Combat` on `Flight` (F998's loadout, serde default so old
  saves load with no lasers), `Screen::{draw_gunsight, draw_velocity_marker,
  draw_gauges, draw_target_hud}`, `IndexedImage::blit_clipped`, and Space firing
  the current view's mount. Hits are hitscan and only the model-156 muzzle cone
  is drawn; there is no laser projectile or beam line. `docs/notes/combat.md`
  records the routines, the F998 loadout and what remains.
- Conflicts resolved by combining both intents: `indexed.rs` kept `blit_rgb`
  and added combat's `indices`/`size` accessors and clipped blit; `main.rs`
  kept the space colour/stars block and added F1100's target HUD; `screen.rs`
  kept the dock, prefs and marker icons and added the gauge bitmaps plus
  `lock_external` and the gauge draw call. No stream's work was dropped.
- **Corrected the dashboard audit** (`docs/notes/cockpit-dashboard.md`): bitmap 98 is
  the velocity/prograde marker (`F1112`), not a target X; bitmap 97 is the
  gunsight at (129,48), drawn per mount and never in views 4/6; the gauges and
  F1100 range are now drawn; the target hull/shield readout and model 157's
  flash are implemented but unreachable because the generated systems have no
  NPC ships; the combat targeter boxes, missiles, explosions and sounds remain.
  Start type 1 is the **Eagle long range fighter (model 23)**, not a
  Sidewinder; `intro.md`, `intro-todo.md`, `STATUS.md`, `NAVIGATION.md`,
  `DEVIATIONS.md` (EXP-010) and the source comments now say so.
- **Fixed the reported lock/range bug:** `ship_position` returned `[0,0,0]`
  whenever the original flight model was not running, so the lock line and
  every navigation distance were measured from the system origin in the
  sandbox. It now falls back to the prototype state's position scaled to system
  millimetres, the same fallback the new target range uses.
- Local checks: `cargo test -p ffe-core laser` (8), `-p ffe-rs --test combat`
  (4 + 5 unit), `--test combat_view -- --ignored` with the cache (6), whole
  workspace, clippy and fmt green. The new `--state-combat` reference mode
  prints F998's loadout from the executed original.

---

## 2026-10-25 - Space colour uses the captured original default (#0c0c4c)

Owner decision: the default flight background is the original's, not the old
`#000044`.

- `ffe_rs::prefs::space_colour_rgb` now draws the captured reference frame's
  default: palette entry 182, six-bit `(3,3,19)` = `#0c0c4c`. The original stores
  `(0,0,index)` in `DATA_008815..817` and clears with the nearest entry of its
  runtime upper palette (`FUNC_001575`/`FUNC_001618`); the port has no runtime
  palette, so it keeps that captured entry.
- The other four settings are derived from the same entry (each six-bit
  component scaled by index/4, rounded, then `v * 255 / 63`): `Black #000000`,
  `Very Dark Blue #040414`, `Dark Blue #080828`, `Medium Blue #080838`,
  `Light Blue #0c0c4c`. The intermediate ramp is an interpolation, not a capture;
  recorded as EXP-013 with the quantisation.
- "Black" stays the existing Graphic Detail row and renders `#000000`; the
  original's stars are still drawn over it. Tests: the default sky pixels are
  `#0c0c4c`, Black renders black with stars (a draw with stars on differs from
  one with them off), and a Black selection survives closing and re-opening the
  pages in the session. Frames regenerated in `docs/evidence/prefs/`
  (`space-light-blue.png`, `space-black.png`).

## 2026-10-25 - Escape preference pages merged (v0.48p, EXP-012/013)

- Merged `prefs` (d37cb84, bf80d08) after `station-ui`; conflicts resolved in
  `indexed.rs` (kept the navigation marker's `blit_colours` over the one
  `blit_rgb`), `main.rs` (Escape block before the controls-button edge; kept
  `--start`/`--skip-intro` with `--capture-prefs`), `screen.rs` (navigation-list
  hiding, the prefs icons and the docked/status-block guards all kept) and
  `STATUS.md` (both milestone rows). The branch's `shots-prefs/` captures moved
  to `docs/evidence/prefs/` (the repo does not commit scratch frames), and the
  duplicated EXP numbers were renumbered: EXP-012 (pages), EXP-013 (space
  colour), EXP-014 (station screens).
- `--capture-prefs` renders every page plus the two space colours against the
  local cache; `cargo test --workspace --locked`, clippy `-D warnings` and
  `cargo fmt --check` pass.

## 2026-10-25 - Station services and stockmarket merged (v0.56, EXP-014)

- Merged `station-ui` (f3cb3d8, d93b1ad): the docked screen is now the original's
  services root, shipyard, stockmarket, upgrades and ships screens (F328/F330
  buttons and arrows, F558's status line) with the market as one of them
  (`crates/ffe-rs/src/station.rs`, `docs/notes/station.md`). The merge kept the
  planets/lift work: the port's status block is skipped while docked or in the
  station view, the navigation list still hides the top bar, the labels/places
  icons and marker stay, and the starport message still distinguishes an
  open-air landing. `tests/station_view.rs` (local, cache) draws the original
  layout and passes; whole workspace green.
- Equipment, hull and repair lists are empty shells until their tables and
  generators are imported; the BBS is empty for the same reason.

## 2026-10-25 - Launch lift fix merged (v0.55, EXP-011)

- Merged `lift-fix` (204d717): the launch uses F1088's seat (the station's own
  position) and F1089's `+0x9e` door word, stays in the lift until the door
  opens, and skips the station hull until the ship is clear. `SystemStation.door`
  is initialised by every caller, including the planet tests (0xffff, closed).
- Conflicts: `main.rs` (kept the planets starport message and added the launch
  grace), `screen.rs` (hides the status block while the lift or the station view
  is drawn *and* while the navigation list is open), `DEVIATIONS.md` (the launch
  entry was renumbered to EXP-011 because the new-game/starport entry already
  held EXP-010). `tests/lift_view.rs` passes both synthetic tests and the two
  local cached-frame tests.

## 2026-10-25 - Planets merged (v0.54): surfaces, surface starports and landing

- Merged `planets` (1bd1b71, a1e96f6): the shaded-disc fallback no longer draws
  a planet at or inside the eye (the owner's vanishing planet), F1876 surfaces
  reach the horizon, starports and buildings are drawn on the surface, and
  F575's contact drives the landing parent. `navigation::Body` gained
  `inner_radius` and `parent`; one test initialiser on main was updated.
- `crates/ffe-rs/tests/planet_view.rs` (surface horizon, starport frame, landing
  parent) passes; whole workspace, clippy and fmt green.

## 2026-10-25 - Intro implementation plan merged (docs only)

- Merged `intro-plan` (b6c0e16): `docs/archive/intro-todo.md` records the complete
  remaining intro/front-end plan. No code or dashboard item changes.

---

## 2026-10-24 — F575's contact tail: soft landing, crash and bounce (v0.48o)

Notes: `docs/notes/collision.md`. `crates/ffe-core/src/landing.rs`,
`crates/ffe-core/src/collision.rs`.

- **The tail (F575 `JUMP_002410`).** A pair that touches now runs the decision:
  flag 2, `speed = |vx|+|vy|+|vz|`, the surface's up (terrain: F1658 cut to
  0x1c then F1503 to Q31; sphere: F1519 of the low dwords) and `dot = up . row1`
  through F1521. A soft landing needs `speed <= 0x258`, the word at `+0x9e`
  equal to `0xffff` and `dot >= 0x78000000`; it sets 0x40, clears 0x80, puts the
  body on the surface and settles its orientation. Otherwise
  `DATA_008699 = DATA_008700` and flag 0x80 is set; the player's body bounces
  (`2v/3 - 10(up.v)up/12`, the direction becomes row 1, F1520), and F953's
  damage amount is returned. A missile or escape capsule (`+0x87` 0x0b/0x0e) is
  turned to the surface and stopped instead. Flag 1 (the planet's two radii
  differ) and the `DATA_008698` object record close the tail.
- **F578** (touching a body that is not the parent) ported too: flag 0x80, the
  heavier/lighter ordering by the statistics' mass word, the F1519 direction and
  the momentum exchange with F953's amount, unchanged from the supplied
  `FIX: Missile retardation` block.
- The crash mask is threaded as an argument: 0xff for the player's scan (F569,
  `Player::tick_with`, `Bodies::collide`), zero for F568/F574. The bounce
  compares the body's `+0x86` with `DATA_008857`, now `Bodies::player` (zero
  until a game sets it, as in the reference process); the damage needs F1538's
  record, so `Shapes` grew a `model` method and a scan without the cache
  declines a touch.
- Found by the oracle: `DATA_008705` (the altitude) is not reset by F570. When a
  shape pair finds the body inside a planet's inner radius, F575's terrain
  placement uses the altitude left by the *previous* terrain record, even from
  an earlier case. It is now `Bodies::altitude`, carried by the tests like the
  stale `DATA_008704`.
- Re-enabled the grown oracles: **`--collide` 4,096 of 4,096 match, including
  950 contacts** (`verify-collide.py`, trace SHA-256
  `29144e66671d1f214e6fa3f7fe47b5c62710ee3a3e8431fc2a3ae0ff4ec5c57d`), and
  **`--world-tick` 4,096 of 4,096, including 584 contacts**
  (`verify-world-tick.py`, SHA-256
  `4258a0b2756c09c53c102a805a365cce9e6b7c51770894d801875603fa43d6ac`). The older
  notes' 993/620 contact counts predate the current generator; the traces now
  report 950 and 584 and nothing is declined. Both scripts run the oracle twice,
  the Rust test once, then reject ten altered fields each and an empty trace.
- Synthetic tests: `crates/ffe-core/tests/landing.rs` (4) pin each boundary of
  the landing test (`0x258`, `0x9e`, `0x78000000`, at and either side), the
  bounce velocity formula and F953's amount, the three crash branches and flag 1.
- Still open: a complex shape whose stream ends between the planet's radii
  (`DATA_008707` 0xff) takes F575's sphere placement from F577's squares, which
  the F576 path never set; the port declines that (the original reads
  uninitialised stack). The playable still uses its own station touch test.

## 2026-10-24 - Sound documentation and the HMP/MIDI player (v0.48q)

- `docs/notes/sound.md`: all 40 effect slots (file, loop/pitch/voice, call
  sites and conditions), the 17 HMP songs with their start/stop routines, the
  sound globals and ESC action numbers, the HMP format (192 division,
  `0x188000` tempo, `hmp.cpp` semantics - the 480-PPQN note in FORMATS.md is
  wrong), and what can be verified locally.
- `ffe-core::hmp` parses HMP into absolute-tick events, duration and SMF;
  `ffe-audio` adds the runtime `MUSIC/*.HMP` library, a tick-driven player with
  original timing/loop/volume semantics, a WinMM system-MIDI backend and
  null/recording sinks, `AudioSettings`/`SoundFlags` and the `Audio` trait.
  All 17 tracks parse (track counts, durations, tempo, division asserted).
- Deviations recorded in the note: WinMM instead of DirectMusic/MCI, no `.mid`
  override, player-owned deterministic RNG. Effects playback is still a no-op;
  the 23 cached WAVs are mixed by nothing yet.
- The prefs page can drive `AudioSettings`/`SoundFlags` (serde) through the
  `Audio` trait; no prefs files were touched.

## 2026-10-24 - The original new game and the intro timeline (v0.48p)

- **Default boot is now F998's start type 1** (the owner: the DOS original is the
  source of truth): Ross 154 at absolute clock `0x121cf7.5` days, Sidewinder
  (model 23), 1000 CR, an 8 t hold, player state `0x30` on the open-air
  starport's F650 pad, calendar `1-Jan-3250 12:00:00`. `--start sol` keeps the
  Sol/Mars High sandbox for development and the existing tests.
- `navigation::Body` keeps its physics slot, raw Q31 rows, class byte and
  open-air flag; `Navigator::new_game` uses F650's pad maths for the seat.
  `START_SYSTEMS` holds `DATA_006820`; only record 1 is verified (2-5 map to 1).
- `ffe_rs::front_end` models the F961/F962 front end: models 177 then 176,
  bitmap `0x42` from tics `0x795cc`, the 18 credits with their exact windows,
  positions and wraps, and the V version line. **No JJFFE legal splash** (it is
  the mod's, not the DOS game's). The timeline is not yet drawn by the boot loop.
- Oracle: `--intro` hooks F962's text and model draws; `verify-intro.py` compares
  76 cases (all tamper fields and an empty trace rejected); trace `a9489bbe…`.
  `verify-all.py`: 46 of 47 (only the known Windows `--worldgen` ASLR case fails).
- **Open-air starport takeoff is blocked** with a clear message (player states
  `0x24`/`0x26` are not ported) instead of inventing a teleport (EXP-010).
  Indoor stations keep the verified lift launch.
  *(Superseded 2026-10-25: F1013/F1031/F1032 are ported; see the takeoff entry
  above. The refusal is gone.)*

## 2026-10-24 - F650 pad tables (cache v26)

- Imported `DATA_006175..006179` (16 pads, 192 bytes) plus the six dwords of
  `DATA_006180` their bays 6/7 read past into, 216 bytes at file `0xc7e0c`
  (RVA `0xde20c`, floppy 1.02), located by matching the declared bytes - the
  derived RVA in the plan was wrong. Verified byte-for-byte against the
  executable and the declarations. `ffe_core::pads` maps model to table exactly
  as F650, applies F1669 and the wrapping `<< 10`, and keeps the indoor
  `(0, 0, -1)`.
- `--pads` oracle: 8,192 cases with tampering; trace `1ee017c0…`. Cache v26
  (1,794 assets).

## 2026-10-24 - Black wedges on a planet approach (v0.48n)

Owner report: "weird black issues when I try to go to a planet".

- **Reproduced.** A new scripted run, `--capture-planet DIR`, puts the ship
  three radii out from the nearest planet on the station's side, locks it and
  engages the real autopilot and original flight model, presenting a fixed 0.1 s
  per frame and saving every 100th frame. Its frames (and the local test
  `planet_approach_fills_the_disc`) show it: at 3 and 2 radii a large wedge of
  the planet is unpainted and shows the black sky. Marking the view's untouched
  background separately (a temporary debug fill) proved the wedge was **not**
  painted black but never drawn.
- **Cause.** F1876's terrain triangles queue through F1778, whose buffer holds
  50,000 words. A planet at close range needs **59,190 words at 3 radii** and
  **63,125 at 2 radii**; the original exits when one more is asked for, while the
  port's `ModelViewer` ignored `Fault::QueueFull` and drew the entries queued so
  far. The missing wedge is the rest of the surface. At the standoff (1.5-1.6
  radii) the queue is about 45,000 and the view always drew completely, which is
  why the planet looked right in the earlier tests and at arrival.
- **Fix.** `Queue::with_limit` lets the flight view give the surface queue
  0x2_0000 (131,072) words; `Queue::default` and every reference oracle keep the
  original 50,000-word limit and its `QueueFull` fault, so the queued words and
  pixels that the oracles compare are unchanged. Recorded as EXP-009. A queue
  failure is now also reported on stderr instead of silently truncating.
- **Not a bug:** the remaining dark regions on a planet are its night side
  (shading by the local star) and the port's own status block drawn over the
  view's bottom rows; with the queue fixed, no background pixel remains inside
  the disc in the clear part of the view (< 100 of ~15,000-34,000 sampled pixels
  at 3, 2 and 1.6 radii; 672 before the fix at 3 radii).
- **Also checked, no defect found:** the `surface_extent` probe at z=1e6 (it
  still resolves the axes), the 1.5e9 "too far" limit and the power-of-two
  matrix shrink (the shrunk rows shorten the light, and `draw_surfaces`
  renormalises it before shading), and `ModelViewer` surface culling.
- Regression: `planet_approach_fills_the_disc` (local, ignored, cache needed)
  renders Mars at 3, 2 and 1.6 radii through the flight view and fails with 672
  background pixels at 3 radii without the larger queue.

## 2026-10-24 — Object light sign and one frame-wide light (v0.48m)

Read from F1534/F1535/F1481 (see the analysis below); **not yet pixel-verified**.

- The original's light vector is the **observer's global position, normalised**
  (the star is assumed at the origin): light travels *from the star towards the
  observer*. F1481 lights a face when `dot(object light, normal) < 0`.
- The port's `Screen::sun` points **towards** the star and was passed through
  unnegated, so ships and stations were lit on the wrong side. `scene_light`
  now negates it, and `draw_system_station` uses that one frame-wide light
  instead of a per-station direction.
- Still to do (world-9): the star's own shade table (`DATA_007759`, selected by
  the star model descriptor `+0x24 >> 4 & 7`) is imported but never used — every
  object uses the default F960 shades. The planet path (`Disc.light` is
  "towards star" and F1840 tests `dot > 0`) needs its sign checked separately.

## 2026-10-24 — The black band in the lift (v0.48l)

- The lift filled the view correctly; `Screen::draw` then cleared rows 119..157
  with the port's own status block (speed/range/cargo line and the map/autopilot
  buttons), which is not the original's. During the door stages that erased the
  lift's lower 39 rows, so the band appeared exactly as the door closed or
  opened.
- The status block is now skipped while a lift scene is drawn (`self.lift` is
  `Some`). The lift keeps its fixed default light (the interior is not lit by the
  local star).
- Regression: `lift_view.rs` asserts row 119 is not cleared on the closing-door
  stage (0x1c); `--capture` frames confirm the wall reaches the console.

## 2026-10-24 — Local-star lighting for ships, stations and lifts (v0.48k)

- `Screen::sun` carries the direction toward the system's star (computed each
  frame from the star's position and the ship's); `scene_light` uses it for the
  player's ship, the stations and the lift objects instead of the original's
  fixed default light (F960). Planet discs already had their own light.
- Local test `local_star_light_changes_the_scene`: the picture changes with a sun
  and differs between opposite suns.
- **Day/night on planet surfaces is already there and now checked**: the terrain
  emitter shades triangles by the light it is given, so Earth rendered from
  opposite sides has the same 955 triangles with different colours (local test
  `planet_surface_has_a_day_night_terminator`, Earth at three surface extents).
  Model 125 (Mars) culls at the distances tried and is not used in that check.
- Not done: the original's own light tables for ships/stations/lifts; the fixed
  F960 light remains the fallback outside a system.

## 2026-10-24 — A planet in the scenario (v0.48j)

Owner request: "we need a planet on our scenario".

- `Navigator::new` now docks the player at a station **in low orbit of a planet
  or moon** (closest orbit under ten radii), falling back to the first station.
  In Sol that is **Mars High, 1.61 Mars radii**, so a launch faces Mars; the old
  start was Daedalus, a deep-space station with no world in view. This is a
  starting-scenario choice, not a ported original behaviour; `docs/NAVIGATION.md`
  says so.
- The planet surface path already existed (terrain emitter, `screen::Disc`); the
  new local test `boot_station_has_a_planet_in_view` checks the orbit distance
  and that Mars's surface draws in the flight view (extent 13,238,272, 1,584
  colours on screen).
- Landing on the surface, local-star lighting and the whole-frame pixel oracle
  remain.

## 2026-10-24 — The galaxy map (F840, F862, F1699-F1701, F1611), cache v25

Owner request: "Galaxy map as per original game". Notes: `docs/notes/galaxy-map.md`; deviation EXP-008.

- **Dots** (`ffe_core::model_dot`): F1611/F1610 (the particle rasteriser), F1699's far and near paths and F1700/F1701's dot
  branches for unlit points (stars). **Verified against the executed original** pixel for pixel: `--particle` (6,000
  centres and sizes) and `--dots` (6,000 points through the real F1699 and the queue flush, far and near path, 3,285 two-tone
  and 1,838 single dots); `verify-dots.py`, `dots_reference.rs`. Cache v25 imports the two size tables.
- **The scene** (`ffe_rs::galaxy_map`): camera matrix, nine sectors, grid, sphere, marker, selection, console text, F828's
  corner text; `ModelViewer::render_matrix` runs a model with an exact matrix and star records. Lines are projected as
  F1703 does (they were not: a latent bug that left lines with stale screen points).
- **Verified against the executed original's map frame** (`jjffe-smoke --game-frame 400 0x3d`, a new game's F3 screen):
  `verify-galaxy-map.py` finds 8,249 of the original's 8,270 overlay pixels where ours are (8,269); 85% of those in the same
  colour (the rest: the original's runtime-palette approximations of dim colours, and the textured "CORE").
- Text is lit by the font's own normals; the playable's G key and SYSTEMS button open the map.

---

## 2026-10-24 — Click-to-target and the autopilot buttons

Owner request: click a station or space port and have the autopilot dock; autopilot buttons.

- `Navigator::locked` (a lock, apart from the autopilot's `target`), `lock`, `cycle_lock`, `engage_locked`; `Screen::pick` and
  `body_screen` find the body under a click from the camera's projection; TARGET / AUTOPILOT / STOP buttons beside the map
  buttons, corner brackets and a `TARGET:` / `AUTOPILOT:` line. Keys T (Shift+T), K, Backspace. The autopilot itself
  (`navigation::autopilot`, the two-leg approach, clearance, the docking stages) was already there.
- Tests: `locking_and_engaging_a_target` (local cache), `the_autopilot_buttons_sit_beside_the_map_buttons`.
- Not done: landing at open-air starports (F650's pad tables, `DATA_006175..6179`); the autopilot flies to a starport's
  standoff only.

---

## 2026-10-24 — Lift posters, launch world and text placement (cache v24)

Owner report: black frames in the lift scenes when leaving a station, and the posters of the real game missing.

- Posters: the lift and bay models call child model `0xeb`, whose boards are text records (and textured faces chosen by the
  station's seed). The viewer ran child texts but never drew them, and drew all text at the font model's scale instead of
  the record's size nibble (F1747's `+0x148`), which left glyphs a few units wide. Both fixed; newlines (control codes)
  now work through the font descriptor's `+0x3c` vertex (cache v24 `newline_vertex`). Lift objects now get the station's
  seed and global 4.
- Black frames: the launching scenes drew only the lift, so a lift model's open door showed nothing. They now draw the
  world from the slot (the docking sequence already did).
- Not verified against the original's pixels (no oracle for text, and the lift cannot be driven in the headless harness
  without a long flight); the posters' wording and layout are the original's data.

---

## 2026-10-24 — The galaxy map's stars (F1769, cache v23)

Notes: `docs/notes/galaxy-map.md`.

- Cache v23 imports the four star light-point tables (`DATA_007820/7821/7822/7828`). `execute_live` runs opcode 30
  `0x801e` (F1769): per star a stalk line, a point and, within the type's depth limit of **vertex slot 0**, a system-name
  label (`Primitive::SystemLabel`); `ModelViewer::execute_stars` runs model `0xab` over a sector's F841 records.
- The point and line handlers became helpers (`emit_point`, `emit_line`) shared with opcodes 1 and 2; behaviour unchanged.
- **Verified against the executed original**: `--map-stars` (240 sectors, 1,589 stars, 831 stalks, 214 queued labels with
  their keys) match. The oracle's point probe now resolves its vertex like F1699 (slot 0 is read by F1769 afterwards).
- The point's pixels (F1700/F1701) and the scene assembly (F840) are next.

---

## 2026-10-24 — System names (F830), cache v22

- Cache v22 imports the 267 system strings (`DATA_006796`). `sector::system_name` and `sector_label` port F830; the
  `--system-names` oracle matches 8,292 cases (all hand-made stars, table strings, 8,000 generated codes). The harness
  also gained `--game-frame`, which captures the real game's 320x200 frame and palette for the galaxy map's pixel oracle.

---

## 2026-10-24 — The galaxy's stars and the hyperdrive (`ffe_core::sector`, `hyperspace`), cache v21

Notes: `docs/notes/hyperspace.md`; deviation EXP-007.

- Cache v21 imports the galaxy generator's tables and the hyperdrive records (18,916 bytes, matching JJFFE's declarations
  and the executable). `sector.rs` ports F853 (star counts), F841 with F861 (a sector's stars), F856 (positions) and F833
  (distances); `hyperspace.rs` ports F903 (range, fuel, jump time; the test and the jump).
- **Verified against the executed original**: `--sectors` (16,049 cases: every hand-made sector, 3,000 others, 4,000 counts at
  seven detail levels, 3,000 positions, 6,000 distances including full 32-bit values) and `--hyperspace` (8,192 cases for
  non-player ships and the player). All match; tampers and empty traces rejected. Corrections to the `srctools` notes: F861's
  second rotation is 5 bits; F853's grid cell and neighbours read the 128 by 129 table at `+1`, `+128`, `+129`.
- Distances are hundredths of a light year: Sol to Alpha Centauri 4.35, Sirius 8.68, 61 Cygni 11.2, matching the real stars.
- The playable's galaxy list shows distance and fuel; the jump burns fuel (hydrogen; military fuel with waste for class 10 and
  up) and advances the game clock. The four test systems now have real sector codes. The jump scene (F908-F910) and the galaxy
  map screen are not done.

---

## 2026-10-24 — The lift scenes of docking and launching (`ffe_core::lift`)

Notes: `docs/notes/docking.md` ("The lift scenes").

- F1070 draws the cage, interior or bay model (`0xea`, `0xec`, `0xe9`) in the player's view at a depth in the ship's
  frame less the eye offset, turned about the vertical, with the door word; F1071-F1080, F1086 and F1092 are the per-stage
  calls. `ffe_core::lift::{objects, view_position}` is that arithmetic; `traffic::Stages::drawn` records which stage drew
  with what remaining timeout each frame (zero on the frame a stage ends).
- **Verified against the executed original**: `--lift` runs the stage functions with the draw call (`DATA_007807`) captured on
  8,192 random cases; every object's model, position dwords, MatBuildYZT orientation, animation and time words, and the stage
  state and timeout after the call match (trace `08f72014103b...`); nine tampered fields and an empty trace are rejected.
- The playable draws them (`Screen::lift`): over the outside view when docking, alone when launching. The frames show the
  original's cage, shaft and bay. The interior model with word 0 (the two waiting stages) draws almost nothing, so those
  frames are dark; the arguments are the original's, so this is expected but worth comparing against the real game.

---

## 2026-10-24 — The reference harness builds on Windows again; traffic and market oracles run

`headless.c` had become Linux-only (POSIX headers, `sigsetjmp`, `fork`, `setitimer`, `personality`, `open_memstream`).
`tools/reference/win-compat.h` (forced include) and `winstub/` (empty headers) shim them for MSVC and `build-smoke.ps1`
uses both; the scenarios that need a forked child stay Linux-only. Both reference executables rebuilt.

- `verify-traffic.py`: **8,192 cases match** (F650-F655, F657, F867); trace `b3d95748de13...`; tampers and empty rejected.
- `verify-market.py`: **8,192 cases match** (F619, F620 with the game's tables and generator); trace `edf4725858dab...`.
  This replaces the "written but not run" notes in the two entries below, which stay as the record of what was true then.
- `verify-all.py` now lists both and runs **35 of 36** oracles to a pass on Windows in 251 s; `verify-world.py` fails only on
  `--worldgen`'s run-to-run comparison (uninitialised stack words that hold addresses; Linux pins them with `personality`).

---

## 2026-10-24 — Station markets and trading, cache v20 (`ffe_core::market`)

Notes: `docs/notes/market.md`; deviation EXP-006.

- Cache v20 imports the market tables (`DATA_006164..6169`) and the 38 commodity names; `verify-assets.py`
  matches them to JJFFE's declarations and the executable (run). F619, F620, F622, F633's market half and F555/F556 are
  ported from the assembly (the `srctools` notes have F619's cap and F620's two draws right but are easy to misread).
- The docked screen is now the market (25 legal goods at Sol, price, stock, held), with Up/Down, B and N, cash in the
  original's units (1000 = 100.0 credits) and the ship model's hold (5 t for the Eagle); the hold is saved.
- All 329 table systems generate a market. Negative prices (radioactives, rubbish) pay the player to take them, as the
  original's buy code allows.
- **Not run against the original**: `--market` in `headless.c`, `verify-market.py`, `market_reference.rs` are written
  (the harness builds only on Linux); the C was syntax-checked and the test plumbing exercised on a self-made trace.

---

## 2026-10-24 — Traffic control and the docking and launch stages (`ffe_core::traffic`)

Notes: `docs/notes/docking.md`. Replaces the always-granted clearance and the instant dock of the entry below.

- F649-F657 (the station record: clearance, bays, touch, release, reset), F867's clearance countdown and the player's
  timed stages (F1025, F1067/F1068, F1069-F1088: align, lift door, into the lift, turn, close, wait, inner door, into the
  bay; and the launch stages) are ported from the assembly. Docking takes about 26 s of game time, launching about 20 s;
  D answers busy, full or granted with a bay; the autopilot asks at its approach point.
- The original's lift scenes (a lift object built by F1070-F1080), the velocity matching the rotating station and the sounds
  are not ported; the status line names each stage instead.
- **Not run against the original.** `--traffic` in `headless.c` (8,192 records), `verify-traffic.py` and
  `traffic_reference.rs` are written, but the harness now builds only on Linux and this checkout is Windows without WSL.
  The C was syntax-checked and the Rust test's plumbing exercised on a self-generated trace. Run it on Linux before calling
  the port verified.

---

## 2026-10-24 — Stations in system flight and docking at them (ffe-rs rules)

Notes: `docs/NAVIGATION.md` ("Docking"); deviation EXP-005.

- The flight view draws every station of the system from its own model at its generated
  place, turned by F866 at the station's speed byte (`+0xc8`, now `Body::spin`). Before
  this, launching into a system left no station to come back to and D said docking was
  not in the build.
- All generated station models (72, 73, 74, 80 looked at) have the slot on the +z face;
  the face distance is the model's largest vertex z (`ViewerModel::front`), except the
  orbital city, whose root vertices stop short of its hub (a child model) and which takes a
  quarter of its radius.
- `ffe_rs::docking::Port`: clearance from the nearest station within 100 km, docking on
  crossing the face inside the corridor under 250 m/s, a push clear otherwise. The
  autopilot to a station flies to the docking axis, takes clearance and flies in; Sol's
  Daedalus, Abraham Lincoln and Galileo dock from the start (local test). Docking at a
  station makes it the one the docked screen names and the one the next launch leaves.
- The navigator now carries the ship with the body it is near between refreshes (was: the
  engaged target only), so a station orbiting at km/s stays put for a ship that has
  stopped near it. The every-jump autopilot test still arrives at 50 of 52 bodies.
- Not ported: the original's traffic control F654/F655/F650, the pad hit in the collision
  stream and the docking states 0xa..0x18.

---

## v0.53 — Planet surfaces in the viewer and the flight view, the atmosphere (F1877-F1879)

- `ModelViewer::draw_surfaces` runs the ported F1876 and emitter for curved surfaces; the
  flight view draws planets from their models (`Screen::discs`), shrinking the matrix by a
  power of two so the terrain's 32-bit arithmetic stays in the range the original runs
  it in (radii near 2^29 overflowed into slivers). Detail entries blend their ramp
  colours; the navigator's bodies carry model, seed and orientation.
- F1877/F1878/F1879 and the order queue's groups (F1780) ported (`terrain_haze.rs`) and
  verified on 361 haze cases of the executed original (queue word for word, haze state,
  sky colour), with F1876's scale shift (`scale << (instance scale - 8)`) and the
  original's `SIGFPE` cases.

## 2026-10-23 — Physics objects from system objects (v0.50c)

Notes: `docs/notes/objects.md`. Cache v16 adds model records and gameplay tables.

- `ffe_core::objects`: F927, F1533, F880, F879 and the orbit helpers
  F1506-F1510, F1674, F1675. 1,024 systems / 17,762 physics objects match byte for
  byte (starports were masked here until the terrain query was ported, below).
- The harness lessons: `DATA_008807` is `DATA_008804 + 4` (label granularity);
  JJFFE passes the clock to F1510 with the low word in the high position (kept).
- Model records now cover every slot (`models 89` etc. are not imported as
  models but their descriptors are needed).

---

## 2026-10-23 — System generation (v0.50b)

Notes: `docs/notes/sysgen.md`.

- `ffe_core::sysgen::Generator` ports F872-F897 (generated and hand-coded
  systems) on `ffe_core::ffp` (F1463, F1482, F1483, F1486, F1489, F1491, F1496)
  and the F875 data (`handcoded.rs`, generated from the assembly). 4,156
  systems / 63,810 objects match byte for byte, names aside.
- The csysgen notes are wrong in several places (F894's rotate, F887's mass
  exponent, F882's stray shifts, `0x42`'s parent term, the polarity of F884's
  last test); the assembly was the source throughout. Every divergence the
  oracle found was a transcription error of mine, not a surprise in the data.
- Names wait for the message-string engine; F880 (turning objects into physics
  objects) and F879 (starport placement) are next.

---

## 2026-10-23 — Galaxy table and system parameters, cache v15 (v0.50a)

Notes: `docs/notes/galaxy-table.md`. Start of the world milestone (system generation).

- Cache v15 adds `base:world/system-tables`: the galaxy table and the system
  generator's tables, imported raw from the executable (RVA `0xe71d8`, floppy
  1.02 only). `ffe_core::galaxy::Tables` ports F868-F871; `--galaxy` matches on
  5,835 lookups.
- JJFFE's galaxy table is longer than the original's (extra systems after the
  329th); the oracle skips codes only the reference knows.
- System generation (F872-F899) is next; names (F791) need the message-string
  engine, which is not imported yet, so generated objects will be compared
  without their names for now.

---

## 2026-10-23 — The player's whole tick with the collision scan (v0.49n)

Notes: `docs/notes/collision.md`.

- `Player::tick_in`: F1041 (gravity, auto-roll, F606), F601/F602, F1512, then
  F569/F570 with the F572/F573 switch and the altitude. `--world-tick` (4096
  ticks) matches in 3,476 contact-free cases; the 620 contacts are declined.
- Found by the oracle: a body that is its own parent aliases the matrix in
  F572/F573 (the velocity is turned by the updated orientation); ported.
- Harness lessons: the player's `+0x86` must equal its list index or the scan
  collides it with itself, and the contact code crashes on models without
  collision data (the player uses ship models 14-70 here).
- Still missing: the contact paths (landing, crash, bounce), complex shapes, a
  station's velocity, F1033's remaining effects, a planet in the flight
  scenario, the surface drawing.

---

## 2026-10-23 — Collision distance and frame switching (v0.49m)

Notes: `docs/notes/collision.md`.

- `ffe_core::collision`: F569/F570 with F575/F577 for the contact-free path,
  the altitude (`Contact::altitude`) and the linear/rotating frame switch
  (F572/F573, with F1669/F1672/F1517 and the rotating branch of F1677).
  3,103 oracle cases without contact match (892 with a distance, 962 switches);
  the 993 contact cases are the ones the port declines.
- F577's far branch tests the outer radii (`+0x138`) while its near branch
  tests the collision radii (`+0x140`): a JJFFE "FIX" applied to one branch.
  A planet's atmosphere therefore counts as contact at long range. Kept as is
  (`docs/KNOWN_ISSUES.md`).
- A harness bug found on the way: the physical flags printed after the call,
  when F578 had already cleared some. They are captured before now.
- Still missing: the contact paths (landing, crash, bounce), complex shapes,
  F1033's flag handling, wiring altitude into the player's tick, a planet in the
  flight scenario, the surface drawing.

---

## 2026-10-23 — Auto-roll F609 and the whole tick in a world (v0.49l)

Notes: `docs/notes/physics.md`.

- `Motion::auto_roll` (F609) and `Player::orient_and_thrust_in` / `step_in`:
  gravity, auto-roll, and the F606 countdown now run inside the player's tick
  with a world of bodies. `--world-step` (4096 whole ticks, 482 frame changes,
  3,183 pulls) matches the original, every body's cache included.
- `Player::orient_and_thrust` (free space) no longer rejects a low altitude; it
  still rejects a parent frame, which needs a world.
- Still missing for planet flight: the altitude itself (F569/F1033 collisions
  compute `DATA_008867`), a planet in the flight scenario, the surface drawing.

---

## 2026-10-23 — Frames of reference ported and verified (v0.49k)

Notes: `docs/notes/frames.md`.

- `ffe_core::frames::Bodies` ports F590, F595 (non-station), F604, F605, F606
  and F1677/F1678 over an object list with the original's frame-stamp cache.
  4096 oracle cases (a seven-body tree, each routine 1,024 times) match,
  including every cache and stamp byte.
- Rotating frames (`+0x57`, stations) and a station's own velocity are not
  ported: those paths return `None`. They belong with station docking.
- Still missing for planet flight: auto-roll F609, collisions F569/F1033, wiring
  gravity and frame selection into `Player`, and the surface drawing.

---

## 2026-10-23 — Gravity F603 ported and verified (v0.49j)

Next step toward flying near a planet. Notes: `docs/notes/physics.md`.

- `Motion::gravity` ports F603 with the ffp helpers F1496/F1482/F1483 and the
  F1658 truncation (F1657/F1660 and F1466 already existed). 4096 oracle cases,
  1818 of which move the velocity, match exactly, including the truncated
  vector left in `DATA_008708..DATA_008710`.
- The cube step keeps the square's exponent for the cubed number, as the
  original does; the divide by the cube is then what makes the units work out.
- Still missing for planet flight: the frame-of-reference transfers (F604/F605/
  F606, F590), auto-roll F609, collisions F569/F1033 and the surface drawing.

---

## 2026-10-23 — Surface plumbing: entries, dispatch and queued words (v0.48e)

Milestone v0.48 step 2, continuing. Notes: `docs/notes/curved-surfaces.md`.

- **F1823** builds a small hash map from the 16-byte entries (key word +0,
  FindMSB normalisation, `>> 11` bucket, two 16-byte records per bucket). It is
  plumbing; the port should build the map from `Surface::entries` instead of the
  legacy arena.
- **F1833** is a mode dispatch: mode 0 (always used by F1874) selects the flat
  emitters F1715/F1716/F1847, mode 1 the textured F1831/F1832/F1848, then calls
  the table entry `DATA_007843 + 48*mode`.
- **Queued words.** The probe records F1876's order-queue additions (events
  60/61). In the synthetic station context they are always empty: these surfaces
  are planet-scale (axis vectors ~1e8 units) and the instance sits at z 2000, so
  the patch is clipped. Exercising F1876 needs a distance-appropriate instance.
- The whole-table oracle is unchanged (448 models, 0 mismatched, trace
  `4431b861…`); events 60/61 are documentation only and excluded from it, and
  model 118's tamper now alters a surface event rather than dropping a
  documentation one.
- **`--station-model N Z`** sets the instance depth. At z `0x20000000`, model
  118 finally queues words (1,236), which is the capture the F1876 port will be
  built against. At the default 2000 every surface is clipped.

## 2026-10-23 — Surface helper ports (v0.48d)

Milestone v0.48 step 2, continuing. Notes: `docs/notes/curved-surfaces.md`.

- **F1875, F1824 and F1825 ported** (`ffe_core::model_surface`):
  `cross_sign` (sign of the 64-bit 2D cross product, 0 or -1), `mix32` (the
  shift/or/add mixer) and `max_abs_msb` (F1656 MSB of the largest absolute
  coordinate; `i32::MIN` -> 31).
- **Probe modes:** `--surface-sign` (12 vector pairs) and `--surface-helpers`
  (8 words, 9 points), using the new `_ref_surface_sign`/`_ref_surface_mix`/
  `_ref_surface_vector_msb` aliases. The unit tests pin the original's values.
- **F1816/F1817/F1820 are arena allocators**, not geometry: a 1499-block 0x48
  arena and a 2999-block 0x10 arena, with a longjmp after 0xbb8 allocations.
  The port will use Rust collections instead; recorded rather than reproduced.
- Next helpers: F1823 (entry decode), F1833, then F1834/F1835 (quad emission).

## 2026-10-23 — Curved surface resolution (v0.48c)

Milestone v0.48 step 2, first part. Notes: `docs/notes/curved-surfaces.md`.

- **F1874's geometry is ported.** The executor resolves the surface's origin
  through F1470 (view) and each axis through F1472 (object), and records the
  three `axis - origin` vectors F1874 passes to F1876, in
  `SurfaceEmission::{origin, axes}`. A synthetic test checks the resolved origin
  and vectors (including the Q31 identity's one-unit truncation).
- **The census:** 40 opcode 31 records in 39 models, 224 entries.
- **Verified against the original.** `make-probe.py` now redirects F1874's F1876
  call to `trace_surface`, which records events 50-54 (word0/6/10, the origin's
  view position, the three axis vectors) and then calls the original F1876.
  `verify-station-models.py` compares them over the whole table: 448 models,
  **0 mismatched, 40 surface resolutions** (trace `2036461a…`); tampering model
  118's nested events is rejected. The test summary now reports the count.
- **F1876 is not ported.** It is about 1,580 lines with nine helper routines and
  two 480/240-byte topology tables (`DATA_007893`/`DATA_007894`). The note
  records the twelve-argument call contract, the helper call counts and a
  bottom-up port plan with scoped probe hooks (`trace_spline_*` covers opcode 22,
  not surfaces) and a `verify-surfaces.py` oracle. No approximate patch
  rendering was invented.

## 2026-10-23 — 3D text: the glyph programs (v0.48b)

Milestone v0.48 step 1, second half. Notes: `docs/notes/text.md`.

- **F1747 executes a program per character.** The descriptor's table at +0x40
  points at small command programs inside the font model's command block. F1747
  calls F1772 on each one in a nested frame, then reads the word at the returned
  cursor - 2: the program's return record, whose bits 6.. name the font vertex
  that becomes the next pen. The table holds 95 entries (0x20..=0x7e); font 1
  defines 52, and lowercase letters alias the uppercase programs (the starts are
  therefore not sorted).
- **Cache v14** imports all three sets (`base:model-glyphs/000..002`, starts
  plus block): 95/52/95 programs, 3,008/874/624 bytes. Each program must decode
  over the font model's vertices and name an in-range pen vertex, or the build
  fails. 1,781 assets.
- **Rust** (`ModelViewer::draw_texts`): the text vertex (F1470) and the record's
  frame word set the frame (F1740 rules); each character runs through
  `execute_live` with a fresh font vertex cache, and its primitives go through
  the same drawing path as the model's. `0x3016`/`0x3022` expand the drawn
  object's string (+0x124); the viewer defaults it to the model's name.
- **Results:** models 15 and 30 went from 5/7 shapes to 24/26 with nothing
  skipped (their registrations now draw); model 66 from 59 to 74. All six
  labels still draw.
- **Not done:** control codes below 0x20 (the font's +0x3c anchors, used by the
  logo/station multi-line strings), `0x3022`'s 35-column wrap (`DATA_007648`),
  `0x9c00`-`0x9c03` (`DATA_005104`), depth interleaving of glyph geometry, and
  the font model's own normal evaluation in the text frame. No reference oracle
  for text yet; the synthetic test and the census are the current checks.

The previous v13 cache is retained at `.cache/data-v13-backup.bin`.

## 2026-10-23 — Labels, and the string table the text commands index (v0.48a)

Milestone v0.48 step 1, first half. Notes: `docs/notes/text.md`; census:
`crates/ffe-rs/tests/text_census.rs` (local-only, `FFE_TEXT_CACHE`).

- **The model string table was cut short.** `DATA_004682` is declared with 209
  pointers in JJFFE 2.8a7, and cache v11 imported exactly those. The release
  executable's table continues to **219** entries (`police`, the First
  Encounters logo, the station's `DOCK OTHER SIDE` artwork); the model text
  and label commands index the same table as `0x4000 | n`, so six codes above
  `0x40d0` could not resolve. `decode_model_names` now scans pointers while
  they resolve to a NUL-terminated printable string inside the data section,
  rejects a table shorter than the declaration (a misplaced RVA), and records
  the scanned count. Cache **v13** (1778 assets) carries all 219.
- **Labels (opcode 18) are drawn.** `execute_live` already queued F1812; the
  viewer now resolves the string as a model string and draws it with the
  original bitmap font at the projected point in colour 15, clipped to the
  view, which is what F1812 -> F1583 does. All six labels (models 66 and 453)
  resolve. Unsupported codes are counted as skipped, not drawn as something
  else.
- **The census** settles the scope of the remaining 3D text: 236 text records
  in 76 models, 130 distinct model-string codes (all resolvable), and only six
  codes needing handlers (`0x3016` in 50 models, `0x3022` in 10, `0x9c00`-
  `0x9c03` in models 172/175). The glyph sets are model slots 0/1/2, already
  imported.
- **F1344's `0x3016`/`0x3022` read one field.** F1747 fills the StrVars it
  passes with only `vars+0xc` = object+0x124, and both codes expand that field
  (the ship's registration); `0x3022` additionally wraps at 35 columns via
  F1398, which needs the `DATA_007648` width table. No other game state is
  read by the codes the models use.
- **Not ported:** F1747's per-character glyph placement (the glyph programs lie
  outside the font model's 10-byte top-level extent and are not imported), so
  text records are reported in the viewer status rather than drawn. The
  mechanism read from the assembly is recorded in `docs/notes/text.md`.

Cache v13 was rebuilt and inspected; the previous v12 cache is retained at
`.cache/data-v12-backup.bin`. Tests: 204 runnable synthetic tests on Windows
(+1 embedded scan test, +1 viewer label test), clippy clean.

## 2026-10-22 — Console view clicks and flickering plumes (v0.47)

- **Console view buttons now take clicks.** `main.rs` scaled minifb's
  `get_mouse_pos` (already divided by the creation scale, 2) by the client
  size from `get_size` (not divided), so clicks landed at half their position
  and never reached the icons; it also ignored resizing and the aspect-ratio
  letterbox. `Screen::logical_point` maps client pixels
  (`get_unscaled_mouse_pos`) through minifb's own AspectRatioStretch
  placement to the 320x200 canvas, then `view_button` applies the original hot
  areas as before. Test: initial size, resized, wide and tall windows.
- **Thruster plumes flicker.** The plume's far end is a jittered vertex
  (opcode 7, F1476), but every render started from `JITTER_SEED`, so the plume
  was frozen. `AnimationInputs::jitter` now seeds the vertex cache and
  `Frame::jitter` returns the state after the frame. Flight chains one stream
  through the drawn objects, advancing once per 50 Hz flight step (a paused
  picture holds still); the F6 viewer advances it with its clock. The stream
  stays presentation-only (KNOWN_ISSUES, F1476); how far the original's
  stream moves per frame depends on its other readers, which are not modelled.
- **Viewer engines.** Nothing set the viewer's thrust, and test inputs 5/9 put
  bit 15 in global 15, which the ships' preamble treats as reverse thrust, so
  glows could not be seen in the viewer. E now steps the engines off / 2048 /
  4095.
- Checked: every ship's forward thrust limit (cache v12) is 1635 or more, so
  full throttle always shows a plume; reverse limits are stored negative,
  giving a negative global 15 and no glow, as in the original.

`docs/STATUS.md` was refreshed and its "Next task" list replaced by the v0.48
milestone plan ("Every model fully drawn").

## 2026-10-22 — Engine glows, moving parts and the spinning station

Two parallel tasks, merged with the ships/views work; details in
`docs/notes/thrusters.md` and `docs/notes/animation.md`.

- **Thruster glows (opcode 9, F1738)**, `model_thruster`: nothing, a line (width
  <= 4) or one/two glow quads (core 0x123, shaft 0x65) keyed by vertex A, with
  six colour slots fading the material in sevenths; B is resolved only when A
  is deeper than 0x200. Thrust is **global 15** (object word +0xba, the forward
  thrust); most ship programs skip glows at or below zero and lengthen the plume
  up to 4095. `verify-thrusters.py`: 12,288 direct cases, 0 mismatched
  (`5a8eddfe…`); whole-table oracles carry the thruster events. The earlier
  "screen position" question does not reproduce. The viewer now passes the model
  scale as +0x148, as F1683 does.
- **Animation inputs**, `model_animation`: globals are the object's own words
  (global 0 = `GameTics >> 10` written by F1534 before drawing; global 1 the
  gear/door word, F865 moving it by half the frame's tic delta); top-level
  locals are 0 apart from the shades. **Station rotation** (F866/F1675) spins a
  station about its docking axis at `(tics << (speed - 1)) >> 16`; model 74 is
  generated with speed 10 (about 169 s per turn). `verify-animation.py`: 2,640
  rotation and 4,096 orbital-update cases, and the real F1537/F1680/F1683
  set-ups on synthetic objects, 0 mismatched (`10ed4039…`, `2ab63f4d…`).

In flight the station spins at its original rate and animates its time-driven
parts; in the external view the player's ship is drawn with the original inputs
over flight time and its engines glow with the commanded thrust (full thrust =
4095, a diagnostic mapping until ship thrust limits are modelled; `Flight::thrust`
is saved). The F6 viewer animates over the original clock: C clock, 0/5/9 test
input, G gear/door, T time rate (x1..x1000, diagnostic). All 29 oracles pass.

## 2026-10-22 — Ships and the original's views

The scenarios now fly a real ship. **Cache v11** imports the original model name
table (`DATA_004682`, RVA 0xd1444, 209 entries, identical to JJFFE); slots
14..=70 are the craft, and `ffe_rs::ships::ShipList` keeps those with a real
name and a drawable model (entries 0..110 match JJFFE; from 111 floppy 1.02
inserts ten digit entries and differs in decal texts, which `verify-assets.py`
reports as a data difference). Page Up/Down changes ship; the ship is saved
(`Flight::ship`, older saves load the Eagle long range fighter, slot 20).

Views follow the original (research in `docs/notes/views.md`): F1 cycles front,
rear, external (turret views skipped, as for a ship with <= 2 gun mounts);
F6/F7/F9 select directly; the console shows the original view icons
(secondary bitmaps 63..81) with the current one lit, and their hot areas take
mouse clicks. The external camera keeps the original's yaw/pitch/distance
units, start values, held-key ramp, opposite-key resets and zoom limits, and
FUNC_001054's two rotations of the ship's axes. In the external view the
player's ship is drawn (at the station's model scale: true relative size), the
crosshair hidden, and the arrows move the camera, so Modern steering uses the
Classic keys there. Docking returns to the front view. ffe-rs's own keys moved
off the original's: help H, model viewer M, load Shift+F5. Flight drawing now
goes through `camera::Camera` (one projection for stars, station and ships).
Not ported: turret/missile/combat views, eye offsets. Seven tests added. All 26
local oracles pass (`verify-all.py`; on Linux, link the reference binaries to
the `.exe` names it expects).

## 2026-10-22 — Thruster glow analysis (not shipped)

Opcode 9's queueing was read out of F1738/F1704 and attempted, then withdrawn
rather than shipped unverified: the differential check exposed differences in
the port's screen positions for the vertices involved, and no oracle could yet
say the glow arithmetic was right. The full reading - the three queue paths,
the entry layouts, the six-step colour fade, why only 7 of 466 slots reach the
opcode in the synthetic scenes, and the hook/event-number recipe - is in
`docs/notes/thrusters.md`. The operand event remains as v0.23 left it.

## 2026-10-22 — The last census blockers: zero normals and the return record (v0.43)

Two small pieces of the original's own behaviour were still rejected by the
port, and both held three models back.

**A declared normal count of zero.** F1480 writes the implicit pair 0 and 1
from the shades and computes the rest lazily, so a model that declares no
normals needs no records at all; the importer now accepts a count of zero the
way it already accepted a count of two with a null pointer. Models 0, 1 and 2
execute.

**The return record.** F1754 is the return: it clears the dispatcher's flag
whatever the material bits hold, so *any* word whose opcode is zero ends the
program. The decoder accepted only the zero word, and its search for the
program's end (the blocks that carry data after the terminator) looked for the
same. Both now use the opcode.

**Census 446 -> 449 of 466**; `verify-station-models.py` 445 -> **448 models
compared, 0 mismatched**; `verify-child-models.py` 437 -> **440 models compared,
0 mismatched**, with only five left that the port does not execute: 89 (the
release data differs from the JJFFE declarations), 94 (the original's own
undefined child state), 148 (a child scale of 32) and 229/319 (top-level
external references, which the original resolves through null pointers). The
station-queue, texture and vertex-program traces are unchanged.

The local cache was rebuilt again: 1776 assets, 462 of 466 models. The F6
viewer now draws **449 of the 466 models** (424 of 456 in v0.41).

**The assets oracle was stale and is repaired.** `verify-assets.py` still
expected the v6 cache (385 assets, two models), so it had not run since v7.
It now checks the whole v10 cache: every payload against the executable's own
bytes, the descriptor and table pointer provenance for every model, the
bitmaps, fonts, palette, cabins and effects, the repeated build and the
corruption rejection. It also reads the ship object table slot by slot (the
null entry at 149 is four zero bytes, not a pointer, which shifted every later
slot).

That repair exposed a difference worth recording: **the JJFFE 2.8a7 source's
model data is not the floppy 1.02 release's data**. 34 of the 462 imported
models differ from the declarations by a few bytes each (model 361's vertices
differ, as v0.29 already recorded). The port imports the *player's
executable*, which is the original game's own data; the local reference build
embeds the mod's data, so the differential oracles can only compare the models
whose bytes agree - and they do: the whole-table station and child oracles
still report no mismatch, because the differing bytes are in the trailing
data and in fields the compared events do not carry. The oracle now reports the
differing model slots instead of failing on them.

## 2026-10-22 — Screen-space midpoint vertices and the remaining external references (v0.42)

`model::ObjectCache` now runs the original's two remaining vertex opcodes.
**3/4** are F1478's case 11 and **13/14** its case 6: a *screen-space* midpoint
of two vertices. The original resolves both sources through **F1472** (the object
path), which writes no screen position, so the midpoint averages what its
sources already had - zero in a fresh arena, the projected position when a
drawing command projected the source first. It writes no object coordinates
either, and keeps the nearer of the two depths; case 6 also keeps the first
source's view x/y, while case 11 leaves them as the arena holds them. The port
mirrors all of that: the object stays zero, the screen and the view come from
the sources' entries, and `model_instance::Vertices::resolve` does not project
these vertices again. `VertexContext` gained the instance position because the
sources' own evaluation needs it.

**Census 443 -> 446 of 466** (models 315, 316 and 444 now execute) and the
child oracle **417 -> 437 models compared, 0 mismatched** (models 346-360,
448, 450 and 452 now run with their children). The three models that still do
not execute are unchanged: 0/1/2 (a declared normal count of zero), 89 (the
release data differs from the JJFFE declarations), 94 (the original's own
undefined child state), 148 (a child scale of 32), 229/319 (top-level external
references) and the 15 slots the original itself cannot run standalone.

Two smaller decoder changes came with it: the thruster (F1738), beam (F1761)
and chain (F1766) records read their vertex bytes sign-extended, so 0x80..0xff
are external references there too, exactly as for faces, lines, points, text
and child calls. Model 316's chain uses them.

Two oracle changes: the child oracle no longer compares the child's base
handedness (+0xf8), which the original leaves as stack memory for a child whose
own handedness is undefined - it is reported apart as event 26 - and the
vertex-program oracle now compares every vertex entry's screen words and view
dwords, not only the object coordinates, so the new opcodes are checked against
the original rather than merely accepted. Traces: station-models
`34c1bf26…` (445 compared), child-models `cbe87dd9…` (437 compared),
vertex-programs `1afe0c45…` (454 compared), station-queue `35e55a26…`,
textures `f3164e1c…`.

The child trace's SHA-256 is **not stable across reference rebuilds**: it
carries event 26, the original's uninitialised base handedness, which the
comparison deliberately excludes. Every measurement of the child oracle here
should be read by its numbers (models compared, mismatches), not that hash;
`cbe87dd9…` was one build, `3d334db8…` another, both 440 compared and 0
mismatched.

The local cache was rebuilt: 1767 assets, 459 of 466 models (the previous
v10 file is kept at `.cache/data-v10-backup.bin`). The cache format did not
change, so the version stays v10; `build-data` now imports three more models.

## 2026-10-22 — Child models in the viewer and flight (v0.41)

`execute_scene` (v0.38) now drives the F6 viewer and the flight station, so
child models (station rings, lattice frames, ship parts) are drawn. **Cache
v10** adds `base:model/descriptors`: per table slot, descriptor +0x18 (child
scale offset) and +0x1c (F1687 cull radius), zero for slots not imported. The
viewer's `ChildTable` serves child calls from the cache; the environment holds
the original shades and the top-level light (length 0x8000, in the frame the
children are set up in).

Child primitives land in the root render order but index the child's own vertex
cache, so emissions now carry what drawing needs: `Emission::screen` (the
points F1708/F1712 projected when queuing) and `Emission::shade` (the face
normal's colour in its own instance's table); `PointEmission::screen` and
`LineEmission::screen` are F1479 projections of the resolved views, the line's
second vertex resolved on a copy of the cache so the original cache state is
unchanged (and a failure there leaves the line undrawn, never failing the
execution). All oracles are unchanged (station-models 443 / 0, child-models 417
/ 0, station, station-queue, textures).

If a child cannot run yet (vertex opcode 0xd in 315/444, explicitly undefined
child state), the viewer draws the parent without children and says so. 424 of
456 models render; the rest are child-only parts or planet-sized objects.
Station 74 is now the full station (drum, side pods, ring segments: 69 shapes);
the drum stays scaled to the old 40-unit size. Two tests added.

## 2026-10-22 — Original lighting and colours in the viewer (v0.40)

The viewer's shades had been a diagnostic 14..0 ramp, about double the
original, which washed faces towards white. The original's inputs are now
imported: F960 sets the default light `DATA_007231..7233` ((-18919, -18919,
18919), length 0x8000) and copies the default shade table `DATA_007234`
(7,7,7 7,7,7 6,6,6 5,5,5 4,4,4 3,3,3 2,2,2 1,1,1) to `DATA_008831`; F1535 swaps
in one of six star tables from `DATA_007759`; F1683/F1684 and F1740 copy the
eight shades into every instance and child. **Cache v9** adds
`base:lighting/default` (RVA 0xeec60) and `base:lighting/star-shades` (RVA
0x101858), byte-identical to JJFFE in floppy 1.02 (`model_lighting` parses
them; a CD cache omits them).

The viewer and flight station use those shades, a light of length 0x8000 as
F1518 normalises it (fixed in flight space; view-relative in F6), and every
flat colour goes through F1326 (`model_lighting::pack`: `r << 8 + g << 4 + b`,
no clamp) and the original fixed colour map / low palette, the route textured
faces already used. Still diagnostic: which star table applies (F1535's star
type), the light's direction relative to a real star, and the runtime upper
palette for colours outside `DATA_007770`. Oracles unchanged on the v9 cache.
Two tests added; version-rejection tests move to 10.

## 2026-10-22 — The real station in the flight view (v0.39)

The flight view draws the station as the original model 74 instead of the
placeholder wireframe cube: `ModelViewer::render_scene` runs the same verified
pipeline as F6 (executor, normals, textured faces, polygons) with any camera
frame and position, and `Screen::draw_station` places it with the flight
camera. The model's bounding radius is scaled to the old 40-unit placeholder,
so docking and safety distances are unchanged. Model 74's docking port is at
its +z end (an octagonal cap with the slot; the -z cap carries windows; four
axis-facing side panels use unlit black `0x2000`), so it is turned 180 degrees
about y to face the approach from -z. The flight projection now uses the
original F1479 focal length (x * 256 / z) for stars, crosshair and station
alike, instead of the scaffold's 150. Without a drawable model 74 the
placeholder remains. Camera, light and colour mapping are the viewer's
diagnostic choices; flight mechanics remain synthetic. One test added.

## 2026-10-22 — Fix: cache build with child-only models

After the child-model merge the importer accepts programs that read parent
vertices, but the cache's payload check still evaluated every vertex
standalone, so `build-data` failed ("External vertex -1 needs a parent
instance"). The merge had been verified against a cache built before it.
`model::validate_program` now evaluates each vertex with parent slots -1..-5
standing in at the origin and skips a vertex that needs another parent slot
(e.g. -6); every other error remains. The v8 cache now holds **456 models**
(1,749 assets). The F6 viewer draws 422 and shows a message for 34 child-only
models. Oracles re-run on the freshly built cache: station-models 443 / 0,
child-models 417 / 0, textures 1,736 views / 0 mismatched; traces unchanged
apart from the texture count. The release game was launched under Xvfb with
and without `--root` (30 frames each).

## 2026-10-22 — Polygons, textures and child models, in parallel (v0.38)

Three tasks ran in parallel worktrees and were merged here; their full
findings, formats and routine numbers are in `docs/notes/polygons.md`,
`docs/notes/textures.md` and `docs/notes/child-models.md`.

- **Complex polygons (opcode 5).** `model_polygon` decodes the six segment
  kinds and runs the F1721 handlers: projections, near-plane clipping (F1564),
  circles as two curves (F1554-F1557; F1467's `DATA_007977` is exactly
  isqrt(i << 21), computed by `model_math::table_root`), abandonment, and the
  F1795/F1796/F1797 queue entries. `model_polygon::edges` reproduces the curve
  subdivision (F1547/F1549). `verify-polygon.py`: 12,328 helper calls match
  (trace `fab32ea5…`). Station-model mode now sets +0x154 (normal records) as
  F1774 does; 452 fills / 1,601 entries match. The textured fill colour
  (F1691 reads past a small table) is not ported or compared.
- **Textured faces.** F1708/F1712 take the textured path under exact
  conditions; the 148-record descriptor table `DATA_007811` gives the bitmap
  id (primary, then secondary from 96), quad orientation and up to 7 colours;
  texels are colour slots; mapping is affine from corner tables. **Cache v8**
  adds `base:texture/descriptors` and `base:palette/fixed-map` (`DATA_007770`)
  from floppy 1.02 (a CD cache omits them). `verify-textures.py`: 4,096 raster
  cases match the whole buffer (`640cc781…`); 2,677 textured queue entries in
  1,720 model views match word for word (`65922b3c…`). The upper runtime
  palette and real lighting values remain unknown.
- **Child models (opcodes 14/27).** `execute_scene` runs children recursively
  (F1687 cull, F1750 instance, F1740 frame, F1774 arena with silent no-room
  skip, generation 2), with external vertices (−k from the parent pointers at
  +0x6c) accepted where the original sign-extends. Undefined child state is an
  explicit error. `verify-child-models.py` (`--child-model`): 417 models, 161
  with children, 1,239 child runs, 0 mismatched.

Merge fix-ups: both new oracles had used nested probe events 19-22; polygon
events are now 30-33 and their encoding is shared (`tests/common`), so the
child oracle also checks polygons inside children (trace `eaee2ad1…`). Child
caches get their model's normal records. `verify-station-models.py`: 443
compared, 0 mismatched (`b4c3aecf…`); station, station-queue, text/submodel and
vertex-program traces unchanged. Census **443 of 466**. 202 `#[test]`, 29
ignored, one Unix-only (172 runnable on Windows). The F6 viewer fills polygons
and draws textured faces with the real bitmaps; it does not run children yet.

## 2026-10-21 — Cache v7 and the F6 model viewer (v0.37)

The first visible use of the ported model executor. **Cache v7** imports the
vertices, commands and normals of every model table slot whose three blocks
decode: 439 of 466 on floppy 1.02 (1696 assets). Stations 74/75 must decode;
other failures are skipped. `ffe-content` accepts model assets for any slot
below 466 from v7, and only 74/75 before.

**F6** opens `ModelViewer` (game paused): Page Up/Down through the models,
arrows to turn, +/- to zoom, 0/5/9 test input. Each frame runs `execute_live`
with a camera frame (also the basis), full-detail visibility (depth 1), normals
from `model_normals::evaluate` with the camera and a light in object space, and
draws `Execution::ordered` in order: faces filled (scanline, even-odd) in
`resolve_colour` (now public) colours, lines, points, and curves as a cubic
through their four entry points (F1811 not ported). Polygons and labels are
counted as not drawn. A first pass measures the drawn extent and the second
fits it on screen.

Diagnostic choices, not the faithful renderer: floating-point camera, light
direction, nibble-to-RGB mapping, textured faces as lit grey, dark blue
backdrop. 420 of the 439 models render; 18 planet/star-sized objects exceed the
32-bit view space and one stops on an executor error. Models that only call
child models show nothing yet. Oracles are unchanged with the v7 cache
(`verify-station-models.py` 434 compared, 0 mismatched). One test added;
version-rejection tests move from 7 to 8. 178 `#[test]`, 25 ignored, one
Unix-only (152 runnable on Windows).

## 2026-10-21 — Opcode 15 sound requests (v0.36)

F1759 (opcode 15, two bytes) sums the absolute instance position components
(+0x4..+0xc), divides unsigned by 0x20000 and, below 63, calls F1908 with sample
`code >> 5 & 15` and volume 63 minus that distance. It changes no model state.
`SoundEmission` records the request; playback is not ported. The probe hooks
F1908 inside F1759 (nested event 18). Model 250's request (sample 0, volume 63)
matches; census **439 of 466**; 434 compared, 0 mismatched (trace `4bcf902e…`,
Linux LF). Opcode 16 replaces 15 as the unsupported example in the transactional
test. One synthetic test added; 177 `#[test]`, 25 ignored, one Unix-only (151
runnable on Windows).

## 2026-10-21 — Opcode 30 light points with no points (v0.35)

F1769 (opcode 30 `0x801e`) loops over the count in global word 4 (+0xa4 of the
+0x14c block). Each point index i takes entry byte `+0xa6 + 2i` (low nibble) to
select DATA_007820/7821/7828 materials for an F1699 point at vertex 2i, an
optional F1703 line 2i..2i+1 (global word 1 bit 5), and an F1764 label (string
0x8400, pick id from global dword 2 with i in its top byte) when vertex 0's view
z is at most DATA_007822[b]. Those tables are not imported, so a non-zero count
is an explicit error; a zero count returns at once, as the original does.
Census: **438 of 466** execute; 433 compared, 0 mismatched (trace unchanged).

## 2026-10-21 — Opcode 18 screen labels (v0.34)

F1763 (opcode 18, four bytes) calls F1764 with vertex `code >> 5`, the word at
+2 and a zero pick id. F1764 projects the vertex through F1471 (generation
check) and, when the screen point lies within x 2..=317 and y 4..=149 (unsigned
range tests), queues a 4-word entry for F1812, which expands the word as a
string (F1344) and writes it at that point. It then compares the point with the
pointer when `DATA_008949` bit 1 is set and stores the pick id; that test is not
modelled. `Label`/`LabelEmission`/`Primitive::Label` carry it.

The probe hooks F1777/F1778 inside F1764 (nested event 17: key and the screen
dword; queue event 6). F1764 is also the light-point routine's emitter (opcode
30 `0x801e`). Census: **437 of 466** execute; `verify-station-models.py`
compares 432 with 0 mismatched (trace `a80247eb…`, Linux LF). Only model 66
queues a label in this setup (others are off-screen); it matches. Model 89 now
stops at a truncated command: its floppy 1.02 program differs from the JJFFE
declaration from +0x12 and runs past the JJFFE extent, like model 361's vertex
data. Station, station-queue and text/submodel traces are unchanged. One
synthetic test added; 176 `#[test]`, 25 ignored, one Unix-only (150 runnable on
Windows).

## 2026-10-21 — Vertex-defined basis, opcode 28 `0xc000` (v0.33)

F1741's `0xc000` form is six bytes: vertices A, B, C at bytes +2..+4, each read
as an object position through F1472 behind a non-zero-stamp check
(`Vertices::object`). It writes the basis rows z = norm(B − A), then
x = norm((C − A) × z), then y = z × x (unnormalised), all with F1521 products.
norm divides each component's magnitude by F1466(v) + 1 through F1523 and
restores the sign.

Three helpers are ported literally into `model_math`: F1502 `square_root`, a
bit-by-bit root that is approximate (≈ sqrt(x)·2^15.5) and returns 0xffff for
zero; F1466 `length`, which shifts the absolute components by 30 − msb, sums
their Q31 squares / 4, roots and shifts back, and never returns less than the
largest component; F1523 `fraction`, unsigned (a << 31) / b, saturating to
`u32::MAX` when a / 2 >= b. F1467 (opcode 25's table root) is a different
routine and is still not imported.

Census: **435 of 466** execute; `verify-station-models.py` compares 430 with 0
mismatched (trace unchanged, `6ffe0c7b…`). Among them, models 351, 353, 356, 357
and 359 finish with non-axis-aligned bases (e.g. `[-1970604551, 0, -853274219]`
for row x of 351) that match exactly. Two synthetic tests added; 175 `#[test]`,
25 ignored, one Unix-only (149 runnable on Windows).

## 2026-10-21 — Offset curve vertices 0x17/0x18 (v0.32)

F1478 sends vertex opcodes 0x17/0x18 to F1477 with the *unsigned byte 2* as the
vertex index and F1474(bytes 3 | 4 << 8) as an extra parameter. F1477 reads the
record of that index (pair `byte2 >> 1`, mirror bit `byte2 & 1`), not the
current one: a 0x13/0x14 record gives the line P0 + 2·m(P1 − P0, t), any other
record the cubic of v0.29 through its bytes 3, 4, 4 + 2 and 3 + 2, with
t = (variable(its byte 2) + extra) << 14. The current vertex's own mirror bit is
not used. `ObjectCache::curve` now serves 0x13-0x18; validation follows the
selected record's control points.

`verify-vertex-programs.py` (minimum now 452): **452 of 452** comparable vertex
programs match; trace unchanged (`b3ed1b7d…`). The seven models that needed it
now stop at opcode 28 (`0xc000` form, 12 models). Census unchanged at 423 of 466;
`verify-station-models.py` still 418 compared, 0 mismatched. One synthetic test
added; 173 `#[test]`, 25 ignored, one Unix-only (147 runnable on Windows).

## 2026-10-21 — Opcode 22 curves (v0.31)

F1737 (opcode 22) is ten bytes: material word, four vertex bytes (+2..+5) and a
normal byte (+6). When the normal is not culled it resolves the colour through
F1692/F1693/F1691 with that normal's colour, projects +5, +4, +3 and +2 through
F1471 (cache-checked, so the cache is stamped even when nothing is queued), and
only if all four have view z >= 0x40 allocates a 10-word entry (draw routine
F1811) keyed by F1777 of +4's view, holding the screen points of +4, +2, +5 and
+3 and the F1326 palette colour. `Spline`/`SplineEmission`/`Primitive::Spline`
carry it; F1811's drawing is not ported.

The probe hooks F1777, F1778 and F1326 inside F1737 and records the key, the
packed screen points and the colour as nested events 14-16 (queue event 5);
`verify-station-models.py` compares them. Over the table the original queues 21
curve entries in 5 models; all match. Census: **423 of 466** execute; 418
compared, 0 mismatched (trace `6ffe0c7b…`, Linux LF). The station, station
queue, text/submodel and vertex-program traces are unchanged. One synthetic test
added; 172 `#[test]`, 25 ignored, one Unix-only (146 runnable on Windows).

## 2026-10-21 — Opcode 30 vertex-frame restore (v0.30)

F1767 (opcode 30, two bytes) has three forms. With bit 15 set it calls F1768 with
`(i16)code >> 4`: `0x801e` draws the object's light points (F1769, through the
point/line emitters and F1764), and every other value copies the current basis
(+0x9c) over the vertex frame (+0x14), nine dwords. Without bit 15 it advances
the vertex record base (+0x150) by 6·n and the vertex cache base by 72·n, where
n = F1473(byte `code >> 5`). Nine models use `0xc01e` (restore), one uses
`0x801e`; no executed path reaches the offset form.

The restore is ported; already cached vertices stay cached, as in the original.
The light-point and offset forms are explicit errors (context unchanged).
Census: **413 of 466** execute; `verify-station-models.py` compares 408 models
with 0 mismatches (trace `1c46a942…`, Linux LF). One synthetic test added; 171
`#[test]`, 25 ignored, one Unix-only (145 runnable on Windows).

## 2026-10-21 — Curve vertices and a whole-table vertex oracle (v0.29)

Vertex opcodes 0x15/0x16 (F1478 -> F1477 with a zero extra parameter) compute a
cubic point from P0 = vertex(byte 3), P1 = vertex(byte 4), P2 = vertex(byte 4 +
2) and P3 = vertex(byte 3 + 2), each XOR the mirror bit, with
t = variable(byte 2) << 14 and m(a, b) = (a·b) >> 31, all wrapping:

P0 + 2·m(3P1 − 3P0, t) + 4·m(3P2 − 6P1 + 4P0, t²) + 8·m(P3 − 3P2 + 3P1 − 2P0, t³)

The `4P0` in the t² term is what the original computes (a textbook Bezier has
3P0); it is ported exactly. Control points are evaluated in that order (it
matters for jittered sources).

`--vertex-program N` evaluates every vertex of any slot in index order,
cache-checked like command callers, with the start-up F1476 seed; it also
reports a hash of the reference build's own vertex records.
`vertex_program_reference.rs` reads each slot from FIRSTENC.EXE and compares
every slot the native evaluator supports: **445 of 445 match** (coordinates and
final generator state). Model 361's vertex records differ between the JJFFE
declarations (z 64/69) and floppy 1.02 (56/64); it is reported as a data
difference, not compared. 10 slots crash the original in this setup.
`verify-vertex-programs.py` checks repeat, three tampered fields and an empty
trace; Linux LF trace `b3ed1b7d…`.

Census: **405 of 466** execute natively; all 400 that the original completes
match in command order. One synthetic test added; 170 `#[test]`, 25 ignored,
one Unix-only (144 runnable on Windows). Also added `build.bat` (debug and
release builds of `ffe-rs`/`ffe-tools`).

## 2026-10-21 — Opcodes 17, 24 and 25 (v0.28)

- **17 (F1760 -> F1761)**: 14 bytes; word +0 material, word +2 two vertices
  (low/high byte, each projected with a near-plane test), words +4..+10 passed
  to F1761, which only writes queue records. Decoded as `Beam`; drawing not
  ported.
- **24 (F1766)**: word +0 material, word +2 an F1474 size operand, then 2-byte
  vertex pairs up to and including the first pair holding 0x7f (both the drawn
  and skipped paths stop there). Decoded as `Chain`; drawing not ported.
- **25 (F1771)**: 2 bytes. With bit 15, F1770 resets the current basis to
  identity. Otherwise it takes two components of the instance vector at
  +0x38..+0x40 (axis 0: +0x3c/+0x40, 1: +0x40/+0x38, 2: +0x38/+0x3c; axis 3
  reads uninitialised registers), normalises them with F1467 (a 2,048-entry
  square-root table, `DATA_007977`) and F1523, and rotates the basis with F1744.
  `VertexContext::direction` holds the vector. A zero pair is a no-op
  (`DATA_007977[0]` is 0); a non-zero pair errors until the table is imported.

Census: **403 of 466** execute natively; all 398 that the original completes
match. One synthetic test added (168 `#[test]`, 24 ignored, one Unix-only; 143
runnable on Windows). Remaining blockers: vertex opcode 0x15 (13), opcode 30
(10), 22 (9), 18 (3), vertex 0xd (2), and a handful of validation errors.

## 2026-10-21 — Data after the final terminator (v0.27)

18 models failed with "lacks a final terminator" and 8 with "opcode 0": their
declaration blocks carry data after the program's last terminator (for example
model 58: a child call, its operand, the terminator, then `0x07d4 0x1f40`). The
decoder swept the whole extent and read that data as commands. `decode` now
tries the whole extent first and otherwise takes the longest prefix that ends
in a terminator and decodes strictly with every branch inside it; the rest is
`Program::trailing`. A zero word that is not a record boundary cannot satisfy
this (the record spanning it would be truncated), a branch into the data
rejects the prefix, and with no valid prefix the original error is reported.
Size/count bounds apply to the whole input first. The cache build is
byte-identical.

Census: **368 of 466** execute natively; all 367 the original completes match.
One synthetic test added (167 `#[test]`, 24 ignored, one Unix-only; 142
runnable on Windows). Top blockers: opcode 17 (16), 25 (15), vertex opcode
0x15 (13), 24 (10), 22 (9), 30 (5).

## 2026-10-21 — Nested depth blocks (opcode 21, v0.26)

F1697 is a 2-byte command. Without bit 15 it resolves vertex `(code >> 5) & 0xff`
(F1470), keys it (F1777) and calls F1781: close the open group (F1780(0,0)),
queue a node with that key in the current list (callback F1790), and push the
node's own list as current (stack `DATA_009254`, depth `DATA_007831`, current
head `DATA_007832`). With bit 15, F1782 closes the group, pops back to the
parent list and records where the block ended. When drawn, F1790 sorts the
block's list with F1788 and continues after it.

`model_order::Order` now holds a stack of lists; `Order::nested(Some(key))` and
`nested(None)` mirror F1781/F1782, and `finish_with_keys` sorts each block's
members inside its node. A block left open at the end is an error, as the
original cannot draw it. `execute_live` handles opcode 21 (`Execution.blocks`,
`QueueEvent::Nested`); `execute` still rejects it.

The probe records F1781/F1782 calls inside F1697 (nested events 12/13, queue
kind 4) and passes them through. Census: **317 of 466** execute natively; all
316 the original completes match, including every block key. The sort within a
block is the verified F1788 order but nested composition itself has not been
compared with an executed original queue yet. Two synthetic tests added (166
`#[test]`, 24 ignored, one Unix-only; 141 runnable on Windows). Earlier traces
are unchanged.

## 2026-10-21 — Curved surfaces (opcode 31, v0.25)

F1874 resolves an origin vertex (byte +2, F1470 view) and three axis vertices
(bytes +3..+5, F1472 object), then passes word +0, words +6/+8/+10 and a
sub-stream starting at +12 to F1876, the curved-surface/planet renderer
(1,580 lines). F1876 advances the stream only through F1823, which walks
16-byte entries while their first word is non-zero and returns past the zero
word; every F1876 exit follows that parse. So the record is
**2 + 12 + 16·n + 2 bytes**.

Rust decodes `Surface` (entries raw) and emits `SurfaceEmission` in command
order after resolving the same vertices; patch rendering stays with F1876 and
out of `ordered`. Census: **312 of 466** execute natively; all 311 the original
completes match. One synthetic test added (164 `#[test]`, 24 ignored, one
Unix-only; 139 runnable on Windows). Top blockers: opcode 21 (33), 25 (15),
vertex opcode 0x15 (13), 24 (12), 17 (11), missing terminator (11), 22 (8).

## 2026-10-21 — Coloured child-model calls (opcode 27, v0.24)

F1748 is a second child-model call into F1750. Operands: byte +0 position
vertex (F1471), word +0 frame, byte +3 scale adjustment — high nibble as a
signed literal, or local variable `byte >> 4` when bit 3 is set — and word +4 a
child material resolved through F1694 (opcode 14 passes the instance colour and
a zero adjustment instead). The four optional extra vertices (byte 1 bit 7)
follow at +6, so the record is 8 or 12 bytes. `Submodel` gains `material` and
`scale` (`ChildScale::{Literal, Local}`); opcodes 14 and 27 share execution.

Oracle fix: station modes forced child culling with a negative descriptor
radius, which a negative scale adjustment shifts to zero, so some opcode 27
children ran their (hooked) frame/run steps. F1750's F1687 cull call is now
hooked: it always culls in station modes and passes through to the original
elsewhere. Earlier traces are unchanged.

Census: **277 of 466** models execute natively; all 276 that the original also
completes in this setup match. One synthetic test added (163 `#[test]`, 24
ignored, one Unix-only; 138 runnable on Windows). Top blockers: opcode 31 (39),
21 (33), vertex opcode 0x15 (13), 24 (12), 17/25 (11), missing terminator (11).

## 2026-10-21 — Thrusters (opcode 9, v0.23)

F1738 records are always 8 bytes: word +0 material, bytes +2/+3 vertices A and
B, word +4 a signed width. Every exit returns past the record. When drawn, F1738
projects A (F1471) and gives up if A's view z is at most 0x200; otherwise it
projects B with the same test, scales the width by `+0x148 - 6` and A's depth
(F1666), and either queues a line (F1704, width at most 4) or one or two glow
quads keyed by F1777 of A's view, depending on the screen length (F1467).

Rust decodes `Thruster` and emits `ThrusterEmission` in command order,
performing the same projections (B only when A is deeper than 0x200). Glow and
line queueing need F1666/F1467/F1704 and stay out of `ordered`, like text and
child calls. Census: **245 of 466** models execute natively and all 245 match
the original in command order. One synthetic test added (162 `#[test]`, 24
ignored, one Unix-only; 137 runnable on Windows).

## 2026-10-21 — Jittered vertices (vertex opcodes 7/8, v0.22)

F1478 dispatches vertex opcodes in pairs through `DATA_000044`/`DATA_000045`
(0-2, 3/4, 5/6, 7/8, 9/10, 11/12, 13/14, 15/16, 17/18, 19/20, 21/22, 23/24;
21/22 and anything above 0x18 go to F1477). Opcodes 7/8 evaluate the whole pair
at once: base vertex byte 4, then three F1476 draws each arithmetically shifted
right by byte 2 (offsets ox, oy, oz), then the mirror (byte 4 XOR 1). Vertex 0
is base + (ox, oy, oz); vertex 1 is mirror + (oz, oy, ox) — the offsets are
applied in swapped order, exactly as the original does.

F1476: a = s0; s0 += s1; s1 = rotate-left-16(a); return s1, with state
`DATA_008818`/`DATA_008820` seeded 0x12345678/0x456789ab at start-up. **Open
question:** that state is also read by other game code, so in the original,
drawing a jittered vertex can influence non-rendering code. The port keeps the
state inside the vertex cache (`Vertices::jitter`/`set_jitter`,
`model::evaluate_with_jitter`), owned by the caller and never shared with the
gameplay RNG; whether any gameplay path depends on it must be checked before
the flight simulation is ported (see KNOWN_ISSUES.md).

Oracle note: F1472 has no cache check, so calling it once per index (as
`--models` does) re-evaluates a jittered pair for its second half with fresh
draws. Real callers check the cache first; `--jitter-vertices` now does the
same. It evaluates every model with opcode 7/8 records with two seeds and
compares coordinates and the final generator state: 118 evaluations (59
models) match, 3 models are skipped for other vertex forms, 2 slots crash the
original in this setup. `verify-jitter.py`: repeat, three tamper fields and an
empty trace rejected; Linux LF trace `0fc5445f…`. `--station-model` now seeds
F1476 explicitly, as the game's start-up would.

Census: 232 of 466 execute natively and all 232 match the original (most of
the 59 newly decodable vertex programs stop at command opcodes). Top blockers:
opcode 9 (49), 31 (39), 27 (38), vertex opcode 0x15 (13), 24/25 (11), 17 (10),
21 (9), 22 (8). One synthetic test added, one updated. 161 `#[test]`, 24
ignored, one Unix-only (136 runnable on Windows).

## 2026-10-21 — Complex polygons (opcode 5, v0.21)

F1721 layout: word +0 material; byte +2 low 7 bits normal, bit 7 a flag; byte
+3 the segment length L; then L bytes of outline segments. The record is
**6 + L** bytes whether or not the polygon is drawn: a culled normal returns
past the segments at once. Otherwise F1721 keys the polygon by F1777 of the
view (F1470) of the first segment byte's vertex (low 7 bits), allocates one
queue record, and runs the segments through `DATA_007817` (seven entries,
indexed by `word >> 9`: 0 terminator, then F1734, F1724, F1730, F1733, F1723,
F1736). The segment handlers only build outline state in the instance and the
queue; they do not touch the parent program's variables.

`ffe-core` decodes `Polygon` (segments kept raw) and `execute_live` emits
`PolygonEmission` with its key (or `None` when culled), queues it as
`Primitive::Polygon` and logs `QueueEvent::Polygon`. Segment execution, and the
vertex projections it performs, are not ported yet.

The probe now records the F1777 key call inside F1721 (event 11) and its F1778
allocation (queue kind 3). The whole-table comparison therefore checks every
polygon key in command order. Because segment handlers project vertices in the
original, the probe's cache-miss vertex events (1, 7) can no longer be
predicted without segment execution; `station_models_reference.rs` compares
all other nested events (the station 74/75 test, which has no polygons, still
checks them exactly).

Census: **229 of 466** models now execute natively (was 180), and all 229 match
the original. Remaining top blockers: vertex opcode 0x7 (59), opcodes 31 (39)
and 27 (35), vertex opcode 0x15 (13), opcodes 17/24/25 (10 each), 21/22 (8
each). One synthetic test added; the unsupported-opcode example moved to 15.
159 `#[test]`, 23 ignored, one Unix-only (135 runnable on Windows). Earlier
probe traces are unchanged.

## 2026-10-21 — Whole-table comparison with the original (v0.20)

`--station-model N` runs any model slot through the original F1772 as
`--station` does, with consistent synthetic inputs: identity frame and basis,
position z 2000 (also the visibility depth), the descriptor scale at +0x148
(which the original uses as vertex scale, operand scale and visibility scale
alike), every normal visible, glyphs and children recorded but not executed.
The probe's face, point and nested-event capacities were raised; earlier traces
are unchanged. One process per slot: the original crashes in this synthetic
setup for 13 slots (15, 153, 179, 198, 199, 227, 259, 270, 316, 335, 369, 388,
458); they are excluded, not counted as failures.

`station_models_reference.rs` reads model bytes straight from FIRSTENC.EXE (via
`ffe-convert`, now a dev-dependency of `ffe-rs`) and compares every slot the
native executor completes. **All 180 match** the original: face count and
the stream of face arguments with locals/basis at emission, points/lines, text
and child calls, final locals, basis, colour and handedness.

Two probe details had to be modelled in the expected events, not the port: F1749
calls F1471 and F1747 calls F1470 only on a vertex-cache miss, so the probe's
vertex events appear only for vertices no earlier child call has projected (in
the probe build faces are hooked and do not project; F1470 never stamps).
Both station tests now apply that rule.

`verify-station-models.py --game <dir>` selects the table RVA by executable
hash, captures all slots twice, compares, and rejects four tampered fields and
an empty trace. Trace SHA-256 (Linux LF)
`3e04ba2e481427b2f2300736bbef8464064bdb4db75f87676ab5e963892cb153`.

158 `#[test]`, 23 ignored, one Unix-only (134 runnable on Windows).

## 2026-10-21 — Model census across the whole table (v0.19)

The original model table (`DATA_004681`, "Ship object table") has **466 slots**,
not 465: slot 149 is declared as four zero bytes (a null entry), which a
reference-only parse skips, shifting every later index. The floppy 1.02
executable's table matches the JJFFE declarations slot for slot once that is
read as raw 4-byte slots: every non-null descriptor's vertex count, normal count
and scale agree.

`tools/reference/model-extents.py` generates `ffe-convert::model_extents`
(committed): each slot's physical command-program extent (programs have no
length field) and its declared (vertices, normals, scale). These are layout
numbers only; stations still give 516 and 758 bytes. `model_command_bytes`
imports any slot's raw program and refuses an extent when the executable's
descriptor differs from the declaration; `decode_model_commands` adds the
decoder check. Normal programs with only the implicit pair (count 2) may have a
null pointer, as the original leaves it; they now import as empty. The cache
build is byte-identical (SHA-256 prefix `173d9f93…`).

`ffe-tools model-census` reports, read-only, how far each slot gets through
native import, decode and `execute_live` (identity frame, position z 2000,
every normal visible; glyphs/children recorded only). On floppy 1.02:

| Blocker (floppy 1.02, first failing stage) | Models |
|---|---:|
| decode: opcode 5 (complex polygon) | 69 |
| vertices: vertex opcode 0x7 | 59 |
| decode: opcode 31 | 39 |
| decode: opcode 27 | 26 |
| vertices: vertex opcode 0x15 | 13 |
| decode: opcodes 17 / 24 / 25 | 10 each |
| decode: opcode 22 (spline) | 7 |
| decode: opcode 30 | 5 |
| decode: stream lacks a final terminator | 5 |
| other vertex-record errors | 10 |
| decode: opcode 21 (nested block) | 3 |
| normals: invalid count | 3 |
| decode: out-of-range vertex in face/line/submodel | 7 |
| decode: opcodes 0 (non-zero word), 4 (flags), 9, 18 | 7 |
| vertices: vertex opcode 0xd | 2 |
| null table entry (slot 149) | 1 |
| **Executed to completion** | **180 of 466** |

This is a coverage map, not verification: only stations 74/75 are compared
with the original. The CD 1.01 executable was not available in this session;
its table RVA differs (0x0d0cf8) and should be checked with the same census.

## 2026-10-21 — Face draw queue (v0.18)

The original queues faces inside its raster path. F1708 (triangle) and F1712
(quad) return without queueing for a culled normal; otherwise they project
every vertex through F1471 (order b, c, (d,) a) and then:

- **Textured path** (`DATA_008812 >= 0`, its default; material texture
  non-zero; every screen point within x -2000..2320 and y -2000..2158; screen
  perimeter |a-b|+|b-c|(+|c-d|)+|last-a| at least 6): always queued.
- **Flat path** otherwise (F1709 / F1712 tail): queued if any point is on
  screen (0 <= x < 320, 0 <= y < 158); if none is, F1713 ORs the F1715/F1716
  outcodes (1 left/above, 2 inside, 4 right/below, 8 near-plane sentinel) and
  `DATA_007818` rejects 1, 4 and 8, i.e. all left, all right (or all
  above/below) or all behind the near plane. Otherwise F1714 queues a clipped
  polygon.
- The key is always F1777 of vertex a's view (F1709, F1714 and both textured
  allocations).

`execute_live` now applies exactly this: culled faces and rejected faces stay in
`Execution.emissions` but not in `ordered`, face vertices are projected
(stamping the cache as F1471 does), and `InstanceContext::textures` mirrors
`DATA_008812`. `Execution.queue` logs every group change and allocation in
program order (excluding the top-level close).

`--station-queue` lets faces run through the real F1708/F1712 and logs every
F1778 call inside F1708..F1714 and every F1780 call inside F1696 before passing
it on. Six views per station (z 100/600, two offset views, a 45-degree tilted
frame, a view partly off the left edge and one almost entirely behind the near
plane): `verify-station-queue.py` matches all 12 queues with repeat, 7 tamper
fields and an empty trace rejected. Trace SHA-256
`35e55a26689eb1e520dffa6b4c36f4a25dff36066d1b4c9ba969c7345425fb56` (CRLF);
Linux LF `350b6d2c…`. Earlier probe traces are unchanged. A first comparison
found Rust queueing all-behind faces: the 9th entry of `DATA_007818` (code 8)
also rejects, which had been misread.

Because sorting these operations is the verified F1788 order (v0.9), Rust's
`ordered` faces now match the original draw order for these views. Points and
lines are still keyed by their first view vertex and queued unconditionally;
their originals (F1700 discs, F1704) are not yet ported. One synthetic test
added; the composed-order oracle test now supplies a visible normal; the local
station-completion test now uses position z 100. 156 `#[test]`, 22 ignored,
one Unix-only (133 runnable on Windows).

## 2026-10-21 — Whole-station command-order oracle (v0.17)

`--station` runs the real model 74/75 command streams (from the reference
binary's own model table) through the original F1772 in the instrumented
adapter. Depth commands, vertex resolution (F1470/F1471/F1478), colour,
visibility and transforms run as original code; the face/point/line emitters
are recorded with the locals and current basis at each emission (as in
`--execution`), and text/child calls are recorded with their vertices resolved
through the real F1470/F1471. In this mode text reports no glyph-buffer room
after F1745 and every child culls (negative descriptor radius), so neither
glyphs nor children execute.

Inputs are consistent between both sides: identity frame (+0x14) and basis
(+0x9c), position (0, 0, 100) — +0xc is both position z and the visibility
depth — generation 2, zero scales/globals/locals, every normal visible with
zero colour. (The older local station check used depth 100 with position z 0,
which the original cannot represent; it stays as a completion check only.)

`station_reference.rs` matches, for both models: face count and the FNV-1a
stream of every face's arguments with locals and basis at emission (20 and 69
faces), all five points per model with flags, every text and child-call event
in command order, final locals, basis, colour (model 75 ends at 2,2,2) and
handedness. `verify-station.py` passes with repeat, 10 tamper fields and an
empty trace rejected. Trace SHA-256
`0ea8f4a7b9df2616dc6537a032b8527ab3ca0fde89e63856c260d23392f722b2` (CRLF);
Linux LF `b9e3c831…`. The probe's face cap rose from 64 to 256; earlier traces
are unchanged.

This is the first end-to-end evidence for real models, replacing the
helper-composition-only evidence for command flow. Not covered: render order
(the original's face queueing happens inside the hooked F1708/F1712 raster
path), glyph and child execution, primitive colour, clipping and pixels.

Also fixed a pre-existing Linux-only flake: the `ffe-tools` CLI test that runs
a freshly copied binary could hit ETXTBSY when another test thread forked; it
now retries only that error. 154 `#[test]`, 21 ignored, one Unix-only (132
runnable on Windows).

## 2026-10-21 — Correction: vertex frame vs current basis

`VertexContext::matrix` had stood for two different instance fields. Vertex
evaluation (F1478 via F1671) multiplies by the instance frame at **+0x14**;
the rotation commands (F1742/F1744, opcode 28) change the current basis at
**+0x9c**, which child-model and text frames (F1740) read and vertex
evaluation never does. Each earlier oracle exercised only one side (model
vertices set +0x14; execution/rotation-operand oracles fingerprinted +0x9c), so
they all passed while `execute`/`execute_live` would have moved a model's own
vertices after a rotation command.

Fix: `matrix` is now documented as +0x14 (vertex evaluation, and the source of
opcode 28's base forms); a new `basis` field holds +0x9c (rotation target,
default identity). The v0.16 `base` field is folded into `matrix`. Rotation
commands and F1739 now write `basis`; vertex evaluation still reads `matrix`.
The execution, rotation-operand, base-transform, model-vertex and live-instance
comparisons were rerun against the original and pass unchanged; their trace
hashes are unaffected (the captures did not change). Real stations still
complete with the same primitive counts.

## 2026-10-21 — Base-relative transforms; stations run to completion (v0.16)

**Opcode 28 base forms.** F1741 has three forms by `code & 0xc000`: `0x8000`
local-basis rotation (v0.6), `0xc000` vertex-defined basis (6 bytes, still
unsupported and rejected by the decoder), and `0x0000`/`0x4000`, which both
stations use (`0x0c3c` at `0x1da` in 74, `0x06bc` at `0x2ec` in 75). The base
form calls F1739 then, only for a non-zero operand word, F1743 (F1474 angle,
axis `code >> 5 & 3`). F1739 copies the instance frame (+0x14) into the
current basis (+0x9c): code bits 7..9 permute the rows (0/6 identity, 1..5 the
other five; 7 is undefined in the original and errors here), bits 12/11/10
negate source rows 0/1/2, and the handedness word (+0xf8 in, +0xfc out) is
complemented for permutations 3..5 and once per negation.
`VertexContext` gains `base_handedness` and `handedness` (and, after the
correction above, `basis`).

`--base-transform` calls the original F1741 directly for 6,048 cases (two base
frames, three handedness seeds, both base forms, permutations 0..6, all
negation masks, three axes, operands 0/0x1234/0xffff). `verify-base-transform.py`
passes with repeat, 8 tamper fields and an empty trace rejected. Trace SHA-256
`c9a87a6d023bacf3900dc0587285ac7522abc6353a80b9b6b62db43088c4a29c` (CRLF);
Linux LF `03070a9b…`.

**Open group at return.** Both stations then returned with a depth group
open, which `Order::finish` rejects. The original flush (F1788) crashes on an
open group, so that rejection stays. What closes it: the top-level object-draw
routines F1683/F1684 call F1780(0,0) right after the model run (F1773/F1774);
nested child and glyph runs do not. `execute_live` is the top-level entry, so
it now closes a group left open at return before finishing. (A probe flushing
the order-v1 sequences without the close segfaulted; it was removed.)

**Result.** With the existing synthetic station inputs (identity basis, all
normals visible, position zero), both station programs now run to completion:
model 74 executes 77 commands (20 faces, 5 points, 4 texts, 5 submodel calls;
25 ordered primitives), model 75 executes 100 (69 faces, 5 points, 1 text, 2
submodel calls, 2 colour commands; 74 ordered). The local station test now
requires completion and descending render keys. This is not yet compared with
the original running the same station; that whole-program trace is next.

One synthetic test updated for the top-level close and one for the base forms;
153 `#[test]`, 20 ignored, one Unix-only (132 runnable on Windows). Every
earlier reference trace hash is unchanged. F4, synthetic flight and cache
v6/385 are unchanged.

## 2026-10-21 — Colour command (v0.15)

Opcode 26 (F1762) sets colour state. Layout: word +0 material, byte +2 low 7 bits
normal index (lighting source), bit 7 target, byte +3 F1473 selector. When byte
+3 is non-zero the record is **20 bytes, not 6**: the low 3 bits of the selected
variable pick word +0 (k = 0) or one of seven alternatives at +2+2k. The v0.4
decoder's fixed 6 was wrong for that form; it is corrected.

Colour = F1692 nibble RGB (zero when textured), plus the instance colour
(+0xf4..+0xf6) when the material's 0x1000 bit is set, plus the normal's colour
unless the material is unlit (0x2000) or textured. Byte additions wrap. Bit 7
of byte +2 writes the result to normal entry 0's colour through F1691; otherwise
it becomes the instance colour that text (F1693) and child-model calls (passed
as the child's colour) use.

`VertexContext` gains `colour` (instance +0xf4..+0xf6), committed
transactionally like locals. `Execution` gains `colours` (selected material,
target, RGB) and `normal_zero_colour`; a later command lit through normal 0 sees
that override. A textured write to normal 0 needs F1691's runtime texture colour
table (`DATA_007812`, zero in the static data) and errors explicitly.

`--colour` runs 7 synthetic programs through the original F1772 (32 cases:
both instance colours, selector locals 0..9, normal-0 write then read, byte
wrap). `verify-colour.py` passes with repeat, 9 tamper fields and an empty trace
rejected. Trace SHA-256 `7d23226568032c92b4f062d4553cee39f4c89f8e51b742e9f958fb48ddaaa4b4`
(CRLF); Linux LF `95383fff…`.

Model 75 now passes `0x001a`/`0x234`; both stations stop at an unsupported
opcode 28 transform form. Two synthetic tests added, one updated (opcode 5 is
the unsupported example); 152 `#[test]`, 19 ignored, one Unix-only (132
runnable on Windows). F4, synthetic flight and cache v6/385 are unchanged.

Not covered: colour application to faces/points/lines (their handlers'
F1692/F1693 use), textured normal-0 writes, palette mapping and pixels.

## 2026-10-21 — Linux reference build; text and submodel operands (v0.14)

**Linux oracle.** `tools/reference/build-linux.sh` builds the same local-only
adapter as 32-bit ELF (needs `nasm` and `gcc -m32`). The original
`jjffe-master/linux/lnxconv.c` converter writes a generated copy of `ffe.asm`
(underscore-free externs); `abs` is escaped there because current NASM treats it
as a keyword. Supplied sources stay unchanged. Every recorded trace hash
reproduces once LF output is hashed with CRLF endings: core `accbe92f…`, depth,
depth-commands, order, composed-order, visibility, normals, transforms,
rotation operands, projection, live vertices, commands `2c0c1a67…`, execution
`d2a76e9c…` and point-line `afa955be…`.

**Probe generation.** The command-probe copy is now written by one script,
`tools/reference/make-probe.py`, used by both `build-smoke.ps1` and
`build-linux.sh`. Besides the four whole-file emitter hooks, it redirects nested
calls only inside F1747 (text) and F1749/F1750 (submodel) to recorders that
return synthetic vertices/descriptors. The original handlers still decode their
operands and return their own cursor; the earlier probe traces are unchanged.

**Correction.** Opcode 14 is not a second text form. F1749 resolves byte 0
through F1471 and calls F1750 with model `code >> 5`; F1750 looks the model up
(F1538), culls it (F1687), and otherwise sets up a child frame (F1740) and runs
the child program (F1773/F1774). It is the child-model call. Opcode 10 (F1747)
is 3D text: each glyph is a model program run nested, after F1344 expands the
string. Both push and pop their own vertex/normal buffer frame (F1745/F1746), so
the parent program's state is unchanged apart from lazy vertex resolution.

Verified layouts (`--text-submodel`, 7 cases):

- Text, 10 bytes: +0 material word, +2 normal index (a culled normal skips the
  text), +3 high nibble glyph-model set (F1539) and low nibble stored but unread
  before glyph execution, +4 position vertex (F1470), word +4 frame (F1740),
  word +6 string ID (F1344).
- Submodel, 4 or 8 bytes: model `code >> 5`, +0 position vertex (F1471), word +0
  frame (F1740). Bit 7 of byte 1 adds bytes +2..+5, resolved in the order
  +4, +2, +3, +5 up to the first `0x7f`, and only for a visible child. The record
  is 8 bytes whether or not the child is culled.

`ffe-core` decodes these as `Text`/`Submodel` and `model_execute` emits
`TextEmission` (with the normal skip as `culled`) and `SubmodelEmission` in
command order. They are never in `Execution.ordered`: their nested primitives
come from glyph/child programs that are not executed. In `execute_live`, the
text position is resolved through F1470 unless culled and the submodel position
is projected through F1471; extra vertices wait for child culling.
`verify-text-submodel.py` passes with repeat, 11 tamper fields and an empty
trace rejected. Trace SHA-256 `5ba06002572b4395481a3651d84c588dbdec4285a210b8fe7e4b0d1a8041f73d`
(CRLF, Windows-equivalent); the Linux LF capture is `b095c38c…`.

Real stations now pass their v0.13 stop points: model 74 stops at an
unsupported opcode 28 transform form; model 75 at opcode 26 (`0x001a`/`0x234`).
Three synthetic tests added and one updated (opcode 26 replaces 10 as the
unsupported example); 149 `#[test]`, 18 ignored, one Unix-only (130 runnable on
Windows). F4, synthetic flight and cache v6/385 are unchanged.

Not covered: glyph and child execution, string expansion, child culling
(F1687 with the child radius), material/colour for text, nested primitives in
render order, and pixels.

## 2026-10-21 — Point and line primitive emission (v0.13)

`ffe-core::model_commands` now decodes opcode 1 (point source) and opcode 2
(line): point material, raw F1474 angle, flag bit and vertex; line material and
two vertices. Both validate their vertex references against the model count.

`ffe-core::model_execute` emits `PointEmission`/`LineEmission` in command order
and, in `execute_live`, allocates them into the same `model_order::Order` stream
as faces. `Execution.ordered` is now `Vec<Primitive>` (`Face`/`Point`/`Line`)
with aligned `ordered_keys`; `emissions` remains face-only command order.
Standalone points/lines use the F1777 key of their first view vertex; an active
depth group absorbs them.

The local command probe now patches F1699/F1703 in a generated assembly copy and
captures the original handler arguments. `verify-point-line.py` compares point
material/angle/vertex/flag and line material/vertices against
`jjffe-commands.exe`; repeated trace SHA-256
`afa955be148192eaf2be79fb0b818386eafbc8d008e67e3fc56c3b7dd3e758a3`, with all
six tamper fields and an empty trace rejected.

Three new synthetic tests cover decoder payloads/out-of-range references,
command-order point/line emission, and depth-group ordering with a standalone
point. Total: 127 runnable Windows tests (145 `#[test]`, 17 local-only ignored,
one Unix-only). The real station check is unchanged: models 74/75 still advance
past opcode 6 and stop at text (`0x000a`/`0xf4`, `0x1c6e`/`0x1c8`). F4, synthetic
flight and cache v6/385 are unchanged.

Point/line are operand emissions, not projected/clipped/rasterized primitives.
Text (10/14), nested/submodel (21/26), other transform forms (28) and colour
state remain unsupported; see docs/MODEL_COMMANDS.md.

## 2026-10-21 — Composed depth/order oracle (v0.12)

Added `--composed-order` to the local headless adapter. It drives the original
F1696 depth commands, F1777 face keys, F1778/F1780 allocation/group lifetime and
F1788 sorting with marker callbacks, then prints the ordered marker IDs and
effective keys. It does not run F1772 or the face handlers; it is a composition
of the original helpers, not a dispatcher/pixel oracle.

Rust `execute_live` now exposes `Execution.ordered_keys` (aligned with
`ordered`) and `model_order::Order::finish_with_keys`; group members inherit the
group key, standalone primitives keep their own. The synthetic program orders
group key 9, standalone key 9 (earlier allocation wins the tie), then group key 0:
C and Rust both produce order `[2,3,1]`, keys `[9,9,0]`.

`verify-order.py` now covers three scenarios: 11,700 depth-command cases
(`ab5da346…`), 128 group/sort sequences (`4ab5453e…`) and the composed-order
program (`ce0d1b4f…`), with repeat/tamper/empty-trace rejection. The full
workspace remains green; 141 `#[test]` total, 16 local-only ignored, 124
runnable on Windows.

Also recorded the complete F1772 handler table in `docs/MODEL_COMMANDS.md`.
Point/line/text/submodel semantics remain unimplemented: their payloads must be
captured from the original handlers before porting, not inferred from record
sizes.

## 2026-10-21 — Depth groups composed into the bounded executor (v0.11)

`ffe-core::model_execute` now composes the verified F1696 depth commands and
F1780/F1788 group ordering with the live view-space slots in
`ffe-core::model_instance`:

- `execute_live(bytes, &mut Vertices, &mut VertexContext, visibility, table,
  InstanceContext, budget)` resolves depth vertices lazily with F1470 (view only,
  no projection), computes F1777 keys with the instance key scale and operand
  scale, opens/closes F1780 groups, and pushes face emissions into
  `model_order::Order`. The vertex count comes from the live cache.
- Opcode 6 modes 0/4/5/6 close; 2 uses one vertex; 1/3 use signed list extrema;
  7 handles position, first (`-1`), origin+z and vertex+z. The physical decoder
  and executor share `model_depth::decode`.
- `Execution.emissions` stays in command order; the new `Execution.ordered` is
  the depth-sorted stream. Standalone faces key from their first emitted view
  vertex, matching F1708/F1712's F1777 call; an active group ignores member keys.
- The executor works on cloned vertex slots and a local queue, finalizes the
  queue, then commits context and slots. Out-of-range depth vertices, an open
  group at return and budget exhaustion leave all three unchanged.
- `execute` keeps the earlier no-live-vertex behaviour and still rejects opcode 6
  with the original message; `execute_live` is the composed path.

Eight new synthetic tests cover grouped/standalone ordering, max/min/offset/
position/first, empty/replaced groups, cache commit/reuse and transactional
failure. Total: 124 runnable Windows tests (140 `#[test]`, 15 local-only ignored,
one Unix-only). The local-only real-station check now advances past opcode 6 and
stops at the next unsupported command: model 74 at `0x000a`/`0xf4`, model 75 at
`0x1c6e`/`0x1c8`; context is unchanged on those errors. No new executed-JJFFE
trace was captured for the composition itself; the existing depth/order/executor
traces are unchanged. F4, synthetic flight and cache v6/385 are unchanged.

## 2026-10-21 — Original projection and live vertex/instance slots (v0.10)

Added `ffe-core::model_instance` with two bounded, pure-integer pieces:

- `project([x,y,z])` — original F1479 integer projection. Rejects `z < 64`;
  divides by `z >> 8` when `z > 65536` and `value << 8 / z` otherwise; adds the
  160/79 view centre; arithmetically shrinks x/y while their wrapped magnitude
  exceeds `0x1f00`; returns the 320x158 logical-viewport pair. Rejected points
  leave the caller's screen words untouched; callers that require a value use
  `NEAR_SENTINEL = [-32766, -32766]`.
- `Vertices` — bounded live view-space slots. `resolve` mirrors the original
  command cache check: a zero stamp re-evaluates the object program (F1478),
  translation (F1470) computes view but does not advance the stamp, and
  projection (F1471) stamps the slot with the caller's generation. A matching
  stamp short-circuits evaluation exactly as original command callers do.
  Generation 0 and scale > 31 error explicitly; no external, screen-derived or
  random vertex forms are evaluated. On failure all slots, including dependencies
  computed during the request, are preserved.

Verified against unmodified executed JJFFE via `tools/reference/verify-instance.py`:

- 2,197 projection cases; repeated trace SHA-256
  `c794945388b5bae086ceb6cb8ad615d17d954a85e2b200bd89367166c1890f70`.
- 8,192 live-vertex cases (128 seeds x 64 steps, matrices/scales/positions/
  generations, explicit clears); repeated trace SHA-256
  `6fe8a6470d42d884d11e54f1cc9db26107794b5220cbfbb051a6ebf826e422cc`.

Both tamper families and empty traces reject. Five new synthetic tests cover
projection boundary/shift/divisor/wrap cases and live-slot memoization/stamping/
atomic failure. Total is 116 runnable synthetic tests on Windows (131 `#[test]`
with 14 `#[ignore]` and one `#[cfg(unix)]`). No external/screen/random vertices,
submodels, clipping or pixels are validated. This is not yet composed into the
model executor: `model_execute` still rejects opcode 6 until the composition in
`docs/archive/MODEL_EXECUTION_PLAN.md` lands. F4, synthetic flight and cache v6/385 are
unchanged; no owner playtest is required for this internal milestone.

## 2026-10-21 — Original depth commands and flat group ordering (v0.9)

Extended `ffe-core::model_depth` with F1696 command decoding and resolution from
explicit view-space points, position, key scale (+0x130) and operand scale
(+0x148). The physical model decoder now reuses this depth decoder, preserving
record boundaries with one implementation. Modes 0/4/5/6 close groups; the other
forms handle vertices, signed list min/max, position, fixed -1 key and wrapping
scaled z offsets. Operand pairs continue only while their high byte is 255.
Negative/external or unresolved point indices fail rather than access unmodeled
legacy memory. These points are not a substitute for original lazy cache state.

Added `ffe-core::model_order` for owned abstract primitive tokens. An active
group absorbs tokens and ignores their separate depth keys. Opening another
group closes the previous one, including empty groups. Finalization requires
explicit closure. Sorting uses descending unsigned keys and earlier allocation
on ties; members retain order. This differs intentionally from F1696's signed
list extrema. A 1..4096-operation development budget is transactional per queue
operation, not an emulation of the original 50,000-word allocator.

The unmodified original F1696 plus F1780 allocator matches 11,700 cases over
13 key scales, 10 operand shifts, three synthetic point sets, 15 command words
and two initial-group states. Check actual consumption, key, active state and
original allocation size. Repeated trace SHA-256
`ab5da34693bcb68d8df26253f7199be280fda09886106184ecf9d0e3739950d1`.
The original vertex cache is explicitly primed, so this comparison does not
validate lazy evaluation or external-instance vertices.

Original F1778/F1780/F1788/F1792/F1793 execute 128 synthetic 64-operation queue
sequences with marker callbacks, including all-equal-key cases. Native output
matches, including group contiguity, empty/replaced groups and tie ordering.
Repeated trace SHA-256
`4ab5453ea1fb9bf67daea15b42fcc11e05cfa36b603c9c625d174509cac1c813`.
The first marker adapter omitted the original primitive-completion flag and
crashed; reading the real primitive epilogues established state=1, after which
the original dispatcher and both repeated comparisons pass. No legacy sources
or additional assembly call sites were patched. Six altered-result families
and both empty traces reject; restoring the valid trace passes again.

Eight new synthetic tests cover all forms, signed-vs-unsigned behavior,
wrapping/masked offsets, malformed/truncated data, unresolved points, equal
keys, flat group lifetime and exact budget/rollback. Total: 111 synthetic Rust
tests. Strict formatting/clippy and release workspace builds pass. Prior
command/executor/core/startup hashes and fresh cache provenance/repeat/corruption
checks are unchanged. The native-window run initially encountered Windows
focus refusal during concurrent reference activity; run alone, the complete
Modern/Classic/held-input/lab/gameplay/save/relaunch regression passes.

This is standalone depth/group behavior, not game primitive storage, forced
headers, nested child-model lists or rasterization. The model executor still
stops explicitly at opcode 6 until lazy view-space point state and actual
primitive allocation are composed correctly. F4 and synthetic flight are
unchanged; cache v6/385 assets remains byte-identical. No additional owner
playtest is required for this internal milestone.

## 2026-10-21 — Modern default controls and original depth keys (v0.8)

The owner requested Modern W/S and arrows as the startup preset. That is now the
default on every launch, with the optional Classic-like preset still on F3.
Graphics defaults remain original-style; original mouse/keyboard fidelity is not
claimed. Help, README and the approved plan record the updated input decision.

Input now has an explicit release-to-rearm latch. Previously, keys pressed while
paused could remain armed on resume; clearing only on selected overlay toggles
was not sufficient. Disabled/modal/focus states block current keys, and explicit
modal/pause/preset transitions disarm them. A held key cannot be reintroduced by
a repeated press report until a release is observed. New keys remain usable.
Three synthetic regressions cover press/hold/release, disabled input, repeated
press reports and mode transitions.

The native-window test uses default W without first pressing F3, checks S and all
four arrow directions, verifies the optional Classic-like controls, and holds W
across both lab exit and pause resume. The initial modified test produced one
lab state mismatch before two clean repeats; per-field before/after diagnostics
were added. With the explicit latch, the expanded complete regression passes.
The negative-yaw assertion was corrected to the existing [0, 2pi) wrapping
convention rather than incorrectly expecting a negative stored yaw.
Trade/launch/thrust/dock/sell/save/reset/load, invalid-load rejection and
cross-process persistence remain covered.

Added `ffe-core::model_depth::key` for the original F1777 render-order key.
View-space integer x/y/z and instance scale are explicit inputs. Preserve
wrapping absolute magnitudes/addition, arithmetic magnitude shift, masked
logical scale shifts and the original packed overflow key, including equality
at its unsigned threshold. This is not Euclidean distance, a z-buffer,
depth-order command execution, group state or sorting.

The unmodified original routine matches 17,303 scale/point combinations on two
reference runs. Altered-key and empty traces reject. Trace SHA-256
`61fffc80501112118d10975309a52bab10dd8598bd14c26b4cd07ed39332655f`.
Two synthetic tests cover normal cases, nonpositive z, wrapping magnitudes,
shift masking and overflow threshold equality. The existing 900-context
composed command comparison still passes with its unchanged hash.

103 synthetic Rust tests, strict format/clippy, five hygiene tests and release
build pass. Fresh cache provenance/repeat/corruption and original RNG/timer/
new-game state comparisons pass unchanged. Cache remains v6/385 assets with
the previous hash. No supplied files or user snapshots were changed. F4 still
shows diagnostic catalogue views; flight remains synthetic. Next: model
depth-order group records and their state/lifetime before extending opcode 6.

## 2026-10-21 — Bounded composed model command execution (v0.7)

Added `ffe-core::model_execute`: follow physical instruction boundaries and actual
return/arithmetic/bit-branch/visibility-branch/local-basis-rotation control flow;
emit ordered triangle/quad operands (including reflected forms) with context
snapshots. Repeated loop emissions retain their individual state, unlike the
physical face catalogue. A real return commits state; any error discards the
invocation's output and leaves caller context unchanged.

Budget is 1..4096 commands, including the terminating instruction. Regression
checks prove the exact ten-fails/eleven-passes loop threshold, invalid budgets,
arithmetic failure, missing trig table, unknown reached operations, branch skips,
conditional returns, malformed boundaries and unmodeled aliased local selectors.
Legacy selectors 18..63 alias instance fields rather than independent locals;
the bounded executor rejects their use instead of inventing semantics.

The original F1772 dispatcher and original command handlers execute three newly
authored synthetic composed programs across 900 contexts. The separate reference
probe uses its existing four emission-call hooks, capturing operand order and
emission-time first-eight locals/matrix. Final locals/matrix and dispatcher-reset
state also match. Counted loops, forward branches, conditional returns, normal/
LOD predicates, rotations, sine/cosine math and mirrored faces all compose.
Repeat trace SHA-256
`d2a76e9caf8a94abf0ca1a06b3adcd6a785c24507bab258eca30ee05163c852b`.
All five altered result fields and empty traces fail; ordinary smoke mode rejects
the probe-only scenario.

Actual station streams are tested to stop at their first unsupported stateful
depth-order opcode 6 (74: 0xc; 75: 0x8), preserving the original input context.
These operations cannot be skipped to claim faithful station rendering.
Output is operand emissions, not transformed/cull-tested/shaded geometry.
Point/line/text, submodels, colour state, other transform forms, lazy instance
buffers and depth-order records remain next. No new F4 view or flight changes;
cache remains v6 and original files/snapshots stay untouched.

98 synthetic Rust tests, strict formatting/clippy and release build pass.
See the README executed-comparison command; reference traces are disposable.
The v0.7 native-window test also passes all four lab views and simulation/action/
snapshot isolation, then launch/thrust/dock/trade/save/reset/reload/relaunch.
Fresh cache import, repeat/corruption checks and prior RNG/timer/new-game state
checks pass; the root v6 cache hash and previous command/startup hashes are unchanged.

## 2026-10-21 — Exact integer trigonometry and basis rotation (v0.6)

Cache v6 adds the original 65,536-byte sine table (16,384 signed integer entries)
via verified executable DATA RVAs: floppy 0x106390, CD 0x105334. The first CD
validation rejected an incorrect font-relative RVA; unique full-table matching
established the actual location, then both source-profile validations passed.
No floating-point sine recreation or copied table in source/test fixtures.
385 assets/386 ZIP entries; older cache versions retain their existing scope.

Native Sin32 matches all 65,536 angle low words plus five wrapping cases.
F1744 basis rotation matches 135 cases over all axes and extreme/small matrices;
per-term Q31 rounding and source negation order are preserved. Both remaining
F1758 arithmetic operations match 210 cases. Repeated trace SHA-256
`b7ea38a86882aff69e3757c408387bf817d2da229ec6a332517cedc98fa53a58`.
Independent tampering in sine/rotation/arithmetic and empty traces all fail.
Rotation takes resolved angles, with an additional command helper described below.
Axis 3 explicitly errors rather than reproduce undefined legacy pointers.

Floppy v6 is 2,107,490 bytes, SHA-256
`173d9f932e65d46fa3a00426c273f9a845da386b931a0faba075fb85de92136f`.
CD v6 is 2,107,486 bytes, SHA-256
`5f18edada4436970ddf9b13f547c02cee47afb635030a630b8f43c1a89deafa5`.
Full reference bytes/provenance, repeat builds, corrupt cache rejection and
unchanged supplied source files pass for both releases.

Additional v0.6 work ports F1474 packed signed literals/global/local angle
operands and the pure 0x8000 transform form via F1742/F1743. All 65,536 encoded
words and 135 actual original handler matrix outcomes match; repeated trace
SHA-256 `08ce2914ec9f6a9654933b4948c49cd1b521d21de1487400d7cde2d66c1530c7`.
Tampering in either family and empty traces fails. The adapter treats local slot
62/global-pointer aliasing explicitly, so pointer addresses do not leak into
repeatable numeric traces. Native errors preserve the matrix. Base/object-relative
and vertex-defined transform forms are not silently approximated.

Further bounded work ports F1751/F1752/F1755 normal/LOD visibility predicates
and inverse branches. Explicit depth/scale/detail and evaluated-normal inputs
replace implicit pointers/globals. Preserve signed comparison, scale-add wrap,
masked shifts, threshold overflow round-trip, detail selector bit, conditional
return and forward/backward offsets. Missing normal IDs fail explicitly.
966 predicates and 5,796 branch outcomes match original routines; repeated
trace SHA-256
`d7e8cc0b57b0fa8f17e7823a23866b08ced025961df0bcc1c912b9fc05ae425c`.
Altered predicates/branches and empty traces fail. Shared branch-offset
handling preserves the previous 360 bit-branch results and command trace hash.
Synthetic evaluated normals isolate the predicate; generation remains covered
by the separate original station normal oracle.

94 synthetic Rust tests pass. Existing command, vertex, normal, font, bitmap,
startup and RNG/timer/new-game state hashes remain unchanged. Strict formatting,
clippy and release builds pass. The visible diagnostic views are unchanged from
v0.5; this is original-math preparation for command execution, not original
projection, full station rendering or faithful gameplay.
Actual native-window regression passes again with root v6, including all four
lab views and full launch/dock/trade/save/relaunch. Prior v5 is backed up privately;
supplied originals and existing user snapshots are unchanged.

## 2026-10-21 — Original station normal evaluation (v0.5)

Imported original normal records for models 74/75 via verified descriptor
pointers, retaining 48/258 bytes and 18/88 expanded entries. Cache v5 has
384 assets/385 ZIP entries; v1..v4 remain readable with their earlier lab views.
Importer and lab validate raw literal anchors, matching descriptors and face
normal bounds. Public tests use synthetic data only.

Native F1480/F1481 initialization, mirrored anchors/components, masked scale
shifts, wrapping observer/light dots and shade lookup match executed JJFFE:
504 contexts and 26,712 results, repeated trace SHA-256
`55b2060dfd8970cfc906c45f91944cbf06627e72c83f6bda417ef18aae1368f3`.
Altered hashes and empty traces are rejected.
Original vertex results and RNG/timer/state hashes remain unchanged.

Floppy/CD normal imports match all reference bytes and descriptor provenance;
deterministic repeat builds and corruption rejection pass. Floppy v5 cache is
2,041,663 bytes, SHA-256
`35838328430aaf673e53f0f96e3d33c3ceff670abb6fe57842211910300e31f6`.
CD v5 cache is 2,041,659 bytes, SHA-256
`9b218c4a7037e6195743eb71109fabc97fb6327cbb765c893adce41dd6704571`.
Root v5 activation atomically preserved the exact v4 archive privately.
Supplied releases/reference sources were not modified.

F4 Tab now cycles wire, face, points, then normal-filtered declared outlines.
The observer derives from the diagnostic lab camera; light and shade inputs
are test values. This still includes alternate command branches, not complete
station execution, projection or shaded pixels. Flight retains the synthetic
station. Private captures were inspected, then removed.

87 synthetic tests, strict clippy, formatting and release build pass.
Actual native-window tests pass all four lab views and isolation of simulation,
actions and snapshots, followed by launch/thrust/dock/trade/save/relaunch checks.
Next: original integer trig/transforms, bounded command execution and submodels.

## 2026-10-21 — Declared station surfaces and verified command helpers (v0.4)

- Continued the station drawing-command layer without replacing the flight
  cube with an incomplete model. F4 now has declared-face wire outlines,
  individual-face inspection (command offset/material/normal ID) and the old
  points view. Tab cycles views; comma/period steps faces; 1/2, arrows and
  0/5/9 retain their model/camera/test-input meanings.
- Model 74 has 96 physical records, 24 declared face emissions (4 triangles,
  20 quads); model 75 has 112 records, 71 faces (8 triangles, 63 quads).
  These include reflected emissions and alternate branches, not a list of
  primitives known to execute together. No invented hull/connectivity or captured
  mesh. Diagnostic white/green outlines are not original material shading.
- `ffe-core::model_commands` bounds every physical record, recognizes the actual
  root-stream forms, validates branch targets as record boundaries and bounds
  face vertex references. Unknown forms fail. Standalone F1758 arithmetic,
  F1753/1756/1757 bit-branch and F1692 material-descriptor helpers preserve
  wrapping, signed/unsigned comparisons, shift masking and 16-bit writes.
  Invalid lookup/division/unsupported integer trig fail without local mutation.
  This is not a complete command executor; normals, child programs, visibility,
  transforms/render ordering, actual animation and rasterization remain next.
- A separate local `jjffe-commands.exe` probes original triangle/quad handler
  arguments before emission. The build checks/replaces exactly two F1708 and
  two F1712 call sites only in an ignored generated assembly copy. Supplied
  source stays unchanged and ordinary smoke/font/vertex oracle assembly remains
  unmodified. The first comparison caught the triangle normal-byte placement
  (between B and C), which was corrected before validation.
- Executed-reference checks: 1,315 arithmetic cases, 360 bit branches,
  all 65,536 material descriptors, 32 face-record cases including reflection.
  Repeated traces match; corrupt math, branch, material and face results, and an
  empty trace all fail. Trace SHA-256:
  `2c0c1a67dd5cc297c54ebb8492bd98bcb123df4a19f56ae7d9c01c8177533635`.
  These certify helpers/operand order, not whole-station execution or pixels.
- Cache v4 adds `models/074.cmd` and `075.cmd` (516/758 bytes), stable IDs,
  descriptor and source offsets. Import follows original descriptor +0 pointers;
  fixed verified extents are deliberate because no program length is stored.
  Cache/import/lab share the decoder. There are 382 assets/383 ZIP entries.
  Tools retain v1..v3 compatibility; v3 keeps point inspection, v2 gameplay
  still runs and requests a lab-data upgrade. V4 command/vertex metadata must match.
- Floppy and CD imports match JJFFE declarations and original PE pointer chains;
  repeated builds are identical, corruption fails and source installations stay
  unchanged. CD verification extracts only required files into an auto-cleaned
  temporary directory, never alters the BIN/CUE or supplied installations.
  - Floppy v4: 2,040,600 bytes,
    `dc6de6b28642336cdceabc196f2cfc4aaba24d2d7335bf70825d641ea871dc35`.
  - CD v4: 2,040,596 bytes,
    `2782b831062ec53d6c43b5713d65f50d1866c9b70089c3a3e5b98f0ed91f3877`.
  Root cache activated with atomic replacement and exact v3 backup at
  `.cache/data-v3-backup.bin`; existing v1/v2 backups and user saves untouched.
- Existing original vertex/font/blit/startup/RNG/timer/state comparisons still
  pass with unchanged trace hashes. Strict Rust formatting/clippy/release
  builds and 82 synthetic tests pass; five repository-protection tests pass.
- Native-window validation exposed an automation issue: foreground ownership
  alone was not sufficient to prove minifb's target thread was active/focused.
  The driver now acquires and checks actual thread focus, reports denied focus
  explicitly, always detaches temporary thread-input attachments, and polls
  docking state rather than assuming uninterrupted wall time. The completed
  run passes all F4 modes and gameplay/save isolation, full launch/dock/trade,
  both control presets, pause/braking, invalid-load rejection and cross-process
  persistence. Private captures were inspected and removed.
- Next: complete bounded model execution with original integer trig,
  normals, visibility, transforms, render-order blocks and child models.
  Compare executed primitive streams and then pixels before replacing synthetic
  flight geometry. The faithful phase-3 gate remains open.

## 2026-10-21 — First original model interpreter slice and F4 vertex lab (v0.3)

- Owner asked to proceed with original model interpretation. Traced JJFFE's
  model descriptor/pointer table and F1472/F1477/F1521. This milestone implements
  the bounded vertex-program layer, not a completed station renderer. Keeping
  the main-view synthetic cube avoids inventing connectivity, skipping visibility
  branches or presenting an incomplete hull as the original station.
- Two original station record sets: model 74/0x4a (`DATA_002907`) and
  75/0x4b (`DATA_002912`), vertex records `DATA_002905`/`DATA_002910`.
  Expanded counts 54/88 include mirrored pairs, with 162/264 raw bytes and
  scales 13/14. PE model table RVA is 0x0d0cfc (floppy) / 0x0d0cf8 (CD);
  both actual executable pointer chains, descriptors and all record bytes
  verified against JJFFE, not inferred from bitmap/font layout deltas.
- Added pure `ffe-core::model`: signed literal coordinates, transformed mirror
  pairs, negation, midpoint, vector arithmetic and variable interpolation.
  Signed Q31 products truncate per term; additions/subtractions/shifts explicitly
  wrap. Nominal identity is 0x7fffffff and retains low-bit asymmetry. Midpoints
  wrap the sum then truncate towards zero. Variable selectors preserve separate
  literal/scaled/global-u16/local-i32 sources. Fresh evaluation memo tables avoid
  stale animated vertices; bounds, cycles, external references and unsupported
  screen-space/random/curve operations fail explicitly.
- Importer/cache share the same bounded validator. Cache v3 adds only
  `base:model-vertices/074` and `/075` (`models/074.vtx`, `075.vtx`), for
  380 assets/381 ZIP entries. V1/v2 stay readable; older schemas cannot contain
  the new kind. V2 still runs the game; F4 reports an explicit v3 upgrade error.
  Root cache was upgraded only after verification, preserving exact prior v2 at
  `.cache/data-v2-backup.bin` and the earlier v1 backup. No original game files
  or existing save snapshots were changed.
- F4 opens a separate paused point-cloud inspection view. 1/2 selects the record
  set; arrows rotate a diagnostic orthographic view; 0/5/9 supplies test values
  0/32768/65535 to global/local slots. White points are literal, green computed.
  This is not actual animation state, original projection, faces, shading,
  visibility or submodels. No original geometry is compiled into the executable.
  The next model milestone is drawing-command/normal/submodel import and
  executed-reference primitive emission before any gameplay replacement.
- Added local-only `--models` reference mode, exporting only context metadata and
  coordinate hashes. Two models x four matrices x three scales x eight variable
  seeds = 192 cases / 13,632 vertices, all matching Rust. Slot-specific values
  distinguish selector decoding; cases include extrema, wrapping scales and
  extrapolation. Traces repeat byte-identically and modified coordinates fail.
  `verify-models.py` also requires the expected ignored test to be discovered.
  Trace SHA-256:
  `7432278a3b1c032631f44668d5f43dcf2303adcb6b910ff7773a00ccc4bb2617`.
- Real-data v3 verification passes repeated cache equality, all payload/provenance
  comparisons, corruption/no-overwrite rejection and unchanged inputs, for both
  the floppy install and a disposable 48-file extraction of the actual CD ISO.
  Floppy cache: 2038563 bytes, SHA-256
  `d2ec47fb372f9668ade76fc5e10e586e8b08ad692598a556e18b807214210847`.
  CD cache: 2038559 bytes, SHA-256
  `9517947feba3b9ee9bfc0b2216ce8574564a9d4e3272155cfdcb188fccee1ce1`.
  ISO extraction is local test setup, not a shipped direct-ISO importer.
- Validation: 74 synthetic Rust tests, five repository-protection tests, strict
  clippy, formatting and release builds pass. Three local reference tests are
  deliberately ignored in public CI and pass explicitly. Previous 696 font/
  28 bitmap cases, RNG/timer/state vectors and 64-frame startup hash are unchanged.
  Native Windows test verifies F4 freezes simulation, blocks trade/save/load,
  does not catch up elapsed time, and requires a fresh thrust press after exit;
  then passes the full launch/dock/trade/save/reload loop and both input presets.
  Captures of both inspection views were examined locally. V1's required-font
  error and V2's working window/actionable F4 upgrade error were also checked.
  No remote CI run or complete model-renderer acceptance is claimed.

## 2026-10-20 — Original font/console presentation in playable v0.2

- Owner reported the v0.1 playable looked good while acknowledging its distance
  from the original; asked to continue. Recorded as prototype feedback, not a
  faithful-gameplay acceptance. V0.2 preserves flight, controls and save format.
- Inspected original model paths first: stations are drawing programs with
  computed/packed vertices, branches and submodel transforms, not static meshes.
  Replacing the cube faithfully needs an interpreter; no invented hull has been
  represented as original geometry. This bounded step instead replaces generic
  presentation with verified original font/console content.
- Main font: 116 records x 10 bytes, nine MSB-first row masks plus proportional
  advance. Identical in both exact original profiles and JJFFE
  DATA_007750/7751. Floppy RVA 0x1013b0/raw 0xeafb0; CD RVA 0x1003d0/raw 0xea3d0.
  Bounds-checked importer retains the source offset and bytes. No font/art bytes
  are compiled into code/tests; removed the `font8x8` dependency.
- New partial cache version 2 adds `base:font/main` (`fonts/main.fnt`), 378 assets/
  379 ZIP entries. Tools still read v1, but v1 cannot claim the new font asset
  schema; v0.2 game reports a required rebuild instead of a font substitution.
  The old generated root cache is retained privately as
  `.cache/data-v1-backup.bin`; verified v2 replaces only the generated cache.
- Original console primary slot 43 is blitted at y158, with clipping of its
  43rd source row to the 200-row canvas. This follows the observed source call,
  not a guessed panel resize. Fixed low RGB6 values expand by shift 2; cabin0
  keeps its separate 8-bit colours. Logical output is 320x200, nearest resampled
  to 640x480 for 4:3 correction and aspect-preserving window scaling. Diagnostic
  layout/scanner contact are still prototype UI, not original functional HUD.
  Help, scenario/reset controls and full-loop behavior are preserved.
- Local reference adapter adds bounded `--font` and `--blit` modes using executed
  JJFFE F1598/F1597/F1623, without source edits. All 116 glyphs at six vertical
  clip placements (696 cases), plus four opaque/masked images at seven positions
  (28 cases), match Rust framebuffer hashes/advances. Each capture repeats;
  deliberately altered font and bitmap hashes fail. The helper enforces both
  Rust comparison tests are discovered, not just a successful runner exit.
  Reference-only aliases handle MSVC C symbol decoration.
- Font trace SHA-256:
  `5642024bbc48ebda6affb820a2d54cde2bc0efbf53db278c152b7a69680486dc`.
  Bitmap trace SHA-256:
  `0cd9357e77769019b90c807505894202e68ecbb1a7b4d8d1c557ea5eb7e65489`.
  No pixel captures or raw glyphs are retained in public fixtures.
- All cache payloads and font bytes checked against floppy originals and JJFFE;
  repeated v2 output matches and corruption/no-overwrite tests pass. The actual
  CD's 48 inputs were extracted only into a disposable temporary installation
  and pass the same checks. Floppy v2: 2037328 bytes, SHA-256
  `a8475b311cc09c60d15e69330d4ba6b163e02ce9cb56c9e63a52bc183ffec70c`;
  CD v2: 2037324 bytes, SHA-256
  `a8929eb0907a7f7dd00c29dd054fb28d50c8828e0bdab9d44c372ff40a597962`.
  Original 71 game files are unchanged. Runtime upper palette remains unresolved.
- Full native-window test passes buy/launch/thrust/dock/sell/save/reset/load,
  Modern steering/brake/pause, invalid-load rejection and new-process reload.
  Original RNG/timing/state and 64-frame startup baselines remain unchanged.
  Synthetic tests cover bounds, masks, signed offsets, exact six-bit expansion,
  logical-to-output pixel mapping and rendering without gameplay mutation.
  No remote CI execution is claimed. Original models, faithful physics and
  economy, functional original console, audio and HD packs remain unfinished.

## 2026-10-20 — Experimental native Rust playable example

- Added a separate `ffe-rs` executable: real window/input, launch, inertial
  thrust/steering/braking, station approach/docking, one-item trade and native
  save/reload. F1 help, F2 named Docked/Approach/Free Flight reset scenarios,
  F3 Classic-like/Modern presets and pause/focus handling. Runs without any
  legacy executable, source tree, NASM or x86 runtime.
- This is explicitly a **prototype**, not faithful First Encounters or completion
  of the approved phase-3 gate. Flight is floating-point scaffolding, not the
  unported FFP/fixed-point physics. Station/stars/economy/HUD/font are synthetic.
  Original cached cabin0 is used as the docked backdrop; no game assets are
  bundled. `ffe-core` and its existing reference coverage are unchanged.
  Temporary differences are recorded as EXP-001 through EXP-003.
- minifb software presentation uses a 640x480 development canvas in an initially
  1280x960 resizable aspect-preserving window. Simulation uses 20 ms updates;
  presentation targets 60 Hz and caps per-frame catch-up to 100 ms. Help, pause,
  menus and focus loss stop simulation without catching up lost time. Held
  flight inputs are disarmed on focus loss to prevent latched thrust.
- Snapshot JSON has a distinct format/version, bounded 16 KiB reads, finite/
  range/docked-state validation, exact float round trips, unknown/duplicate-field
  rejection and exclusive-create numbered files. Prior files are never replaced.
  Failed/interrupted writes may leave an invalid newest snapshot; F9 reports
  this and retains live state, never silently falls back. No autosave, old-save
  import or final-save compatibility promise.
- Native unit tests cover the complete launched route, exact docking range/speed
  limits, alignment, steering, braking, speed cap, collision warnings, atomic
  trade errors, inventory/credits, persistence and corrupt-state rejection.
  Renderer tests use synthetic PNGs and assert presentation does not mutate state.
- `tools/test-playable.ps1` drives the actual Windows window with targeted key
  messages. Verified buy -> launch -> thrust -> coast/dock -> sell -> save ->
  reset/load -> Modern yaw/pitch/brake/pause -> bad-load rejection -> quit ->
  new process/reload. Inspected private window captures for help, station market,
  flight and visible load error; captures are not distributed. Temporary cache/
  saves and test processes are cleaned. The test helper's arrow messages needed
  extended scan-code flags, corrected before the final passing run.
- All 57 Windows Rust tests, formatting, strict clippy, release builds and five
  repository-protection tests pass. Existing combined asset/reference checks
  still pass; the legacy 64-frame smoke hash remains
  `d4439f95c9f2b0fca0edea7977edb76f21452772f8f228a1bb844dabae6a563d`.
  The original 71 input files remain unchanged. Remote CI is configured but not
  claimed executed. README provides a short owner test; owner acceptance of this
  new window is pending, distinct from the earlier technical PASS below.
- Still absent: original world/models, faithful physics/docking/economy, combat,
  planets/hyperspace, audio, legacy commander import, remapping/persisted options,
  automatic conversion, complete cache, HD image packs and Enhanced rendering.
  Next work replaces prototype scaffolding with executed-reference-tested ports,
  rather than treating it as verified original gameplay.

## 2026-10-20 — Owner confirmed both milestone checks pass

The owner ran `tools/test-milestones.ps1` and reported PASS for the content-cache
and reference/core checks. This confirms the combined technical test works in
the owner's run; it is not a flight, docking, trading or save/load playtest.
Next planned work remains object/state and fixed-point physics, targeted
executed-reference comparisons and launch/flight state traces.

## 2026-10-20 — Bounded new-game state traces and first native core primitives

- Extended the local reference adapter with original NASM symbol aliases,
  preincluded during assembly; supplied source files remain unchanged. The build
  manifest now fingerprints the aliases and records assembler commands, numeric
  and state scenario versions. Linked binary stays local and ignored.
- `--state 400` scripts Y then intro choice 1 after the input lockout. The actual
  JJFFE main loop reaches start type 1 at frame 161: day `0x121cf7`, fraction
  `0x80000000`, player state `0x30`, player index 107 and raw cash 1000.
  It samples selected scalar fields, RNG words and framebuffer/palette hashes,
  never raw pointers, personal saves or full game memory.
- Two independent disposable runs produce identical 400-row traces:
  SHA-256 `916e2d3c1abe847e89f9e38e1353c55c737e6ff032132e86a659935bd99c9e7b`.
  The old 64-frame smoke hash remains unchanged. Per-read synthetic time is
  explicitly not a real-time/fixed-timestep flight oracle.
- New pure `ffe-core` has no dependencies or legacy runtime. Implements the
  two-word RNG, unsigned wide timer multiply/divide with per-update truncation,
  wrapping system/accelerated counters, explicit division-overflow errors and
  wrapping 32-bit day-fraction carry. Error results leave counters unchanged.
- Numeric reference mode executes JJFFE F1530 and F1418 for 1536 RNG and 60 timer
  cases, including zero/edge seeds, pause, truncation and counter wrap. Rust
  matches all numeric outputs and 238 consecutive in-game clock transitions.
  Numeric JSONL SHA-256:
  `accbe92f957fc582a6b3f8600648d511d377503d55f6c140674073ee9bee0a84`.
- Corrected the historical inventory's reversed timer-counter roles and false
  49,710-ticks-per-day/one-real-second-per-day interpretation. The main game
  fraction carries on full 32-bit overflow; 994 is the normal 20 ms increment.
  This is a documentation correction from source/runtime evidence, not an
  intentional gameplay deviation.
- `verify-reference` enforces exact bounded scenario shape/coverage with explicit
  mismatches, no silent input defaults and a 2 MiB per-trace bound.
  `verify-core.py` runs both modes twice, delegates comparisons to Rust and
  deliberately changes an RNG result to verify failure. Temp files auto-clean.
  `tools/test-milestones.ps1` combines prior cache checks with the new reference/
  core milestone for the owner's short technical test; no window is expected.
- 45 synthetic Rust tests and five repository-protection tests pass; strict
  clippy, formatting and Windows x64 release build pass. Owner-facing combined
  script passes. Remote CI has not been run. Original game/source hashes remain
  intact; neither traces nor new proprietary fixtures are retained.
- Still absent: playable Rust game, complete physics/FFP/object state, faithful
  input/timing/clock-wrap policies, launch/fly/dock/trade/save scenarios, complete
  content, renderer and graphics-pack loading. The RNG call schedule is recorded
  but not fully ported. Next work is targeted numeric/object/physics differential
  tests and a flight/launch trace, not advanced rendering effects.

## 2026-10-20 — Verified embedded import and partial content cache

- Added original Rust PE32 DATA mapping and bounded bitmap-table decoding.
  Both exact-hash floppy 1.02/CD 1.01 layouts were independently validated
  against the JJFFE 2.8a7 declarations, not the rebuilt reference's addresses.
- Corrected the JJFFE-specific assumptions from the historical FFED3D inventory:
  96 primary/251 secondary slots, signed drawing offsets at header 4/6, mask-zero
  bit 0. There are 345 images plus two special 32768-byte zero-height texture
  blocks, not 347 ordinary images or a blanket 256/512-height encoding.
  Those blocks remain raw and excluded from inferred image dimensions.
- Imported the 128 fixed six-bit RGB colours. The upper half of the palette
  is assigned at runtime; no embedded RGBA/upscale previews guess its colours.
- Added `ffe-content`: bounded, deterministic stored-ZIP base-cache writer and
  reader with stable IDs, typed metadata, source provenance, payload SHA-256,
  CRC checks and explicit incomplete-content markers. Existing proposed full
  schemas remain future design; the actual v1 contract is in
  [FORMATS.md](FORMATS.md). Uses the Rust ZIP library with compression features
  disabled; no external unzip program is required.
- `build-data` imports 377 assets; `inspect-data` validates the local archive
  without extracting entries. New-only output, source containment guard, failed
  installation refusal and per-input rehashing reuse existing protections.
  A local ignored `data.bin` is generated; tools require no legacy source at runtime.
- `tools/reference/verify-assets.py` independently compares every full table
  record (including raw blocks), bitmap metadata, low palette, cabin PNG pixels/
  palettes and WAV samples with local originals/source. It verifies repeat
  equality, existing-output refusal, corruption rejection and source invariance.
  Floppy: 71 files unchanged; CD: 48 required files extracted temporarily from
  the supplied ISO and unchanged. Fixtures/caches are automatically cleaned.
- Both real caches contain 378 ZIP entries and are about 2.04 MB. Profile/source
  provenance and original executable offsets intentionally differ between them.
  No copyrighted payloads, source or test fixtures enter CI/repository artifacts.
- Windows synthetic suite: 34 Rust tests, including truncation, BSS/out-of-range
  pointers, aliases, zero-height special slots, path/ID/version/shape validation,
  missing/extra/reordered entries and payload damage. Remote CI remains unrun.
- Not done: gameplay/state traces, simulation, models, gameplay tables, remaining
  fonts, journal/music/CD conversion, startup cache freshness/rebuild, complete
  upscale export, pack loading or renderer. This milestone is content plumbing,
  not completion of the approved playable slice.

## 2026-10-20 — Local JJFFE smoke reference and external-asset decoders

- Built unmodified JJFFE 2.8a7 assembly/C with NASM 3.01 and MSVC x86,
  using a new original headless platform adapter. Scripts and hash manifest
  make the local build repeatable; legacy warnings are retained, not patched.
  Binary/objects remain ignored and must not be distributed.
- Two bounded 64-frame runs in disposable working directories produced identical
  JSONL traces: SHA-256
  `d4439f95c9f2b0fca0edea7977edb76f21452772f8f228a1bb844dabae6a563d`,
  with 49 distinct framebuffer hashes. This checks startup execution only:
  synthetic clock, scripted Y, no audio/video device/save support. It is not
  a deterministic gameplay oracle or proof of a playable Rust game.
- Added `ffe-convert`: bounded uncompressed 8-bit BMP decoding, lossless indexed
  and RGBA PNG, source 8-bit RGB palettes, PCM-to-WAV and UHD sizing.
  Real cabin files 1..4 have zero `bfSize`; support is explicit and tested,
  with row/palette/header validation unchanged.
- `export-external` requires a new destination outside the player's game folder.
  It writes six cockpit images, 23 effects, palette/indexed originals, previews,
  sizing CSV and a final manifest with all inspected source hashes/output hashes.
  The 44-file partial export is retained locally under ignored `extracted/`.
  It is not a full upscale export, base cache or installable graphics pack.
- Independently decoded real PNGs using Python chunk/CRC/zlib checks; every
  indexed row, RGB palette and RGBA pixel matched the original BMPs. Python
  `wave` verified all 23 WAV sample payloads unchanged, including the empty
  sentinel. UHD rows recommend 3520x1738 (11x), without resizing the source.
  Repeated export was byte-identical; no-overwrite behavior passed; all 71
  `game/` files were unchanged across validation.
- Windows synthetic Rust suite now has 23 tests; format, strict clippy and
  locked release build pass. Five repository-protection tests remain passing.
  Remote Windows/Linux CI is configured but not claimed as executed.
- Next: embedded data/image decoding, full cache provenance, gameplay state
  traces and simulation. Music, CD media, graphics switching and saves remain
  unimplemented. Current commands/limitations are in [README.md](../README.md).

## 2026-10-20 — First implemented Rust foundation

### Implemented

- Added a Cargo workspace, locked dependencies and unpublished `ffe-tools`
  package. Existing stable Rust was found at `%USERPROFILE%\.cargo\bin`;
  it was not necessary to install a toolchain. The prior PATH-only observation
  below is not a missing-toolchain blocker.
- `inspect-game --game <directory> [--json]` reads an installed copy without
  executing, patching or converting it. Default `game/` resolution is beside
  the executable, not shell cwd.
- Exact hash/size recognition for English floppy 1.02 and CD 1.01, independent
  of JJFFE simulation choice. Version-like strings alone cannot select a profile.
- Fingerprints 48 required files, compares cabin/journal hashes, checks expected
  sound/music file presence, even PCM lengths and HMP signatures, and counts
  optional AVI/BMP files. Audio is not fully decoded/hash-verified; optional
  video/still content is not decoded. All limitations appear in the report.
- Stable ordered JSON/text reports and explicit missing/mismatched/oversize/I/O/
  ambiguous-path diagnostics; unsupported versions do not use guessed offsets.
  Exit codes 0/1/2 distinguish inspection success, failed checks, and fatal
  argument/root/output errors.
- Configured Windows/Linux Rust CI jobs using synthetic inputs only.

### Verification completed locally

- Rust tests: 13 pass on Windows (2 profile tests and 11 synthetic/CLI tests).
  Another Unix-only ambiguity/symlink test is configured but not run locally.
- `cargo fmt --all -- --check`, strict all-target clippy and locked release build
  pass. Release executable PE machine is `0x8664` (Windows x64).
- Current `game/`: passed, 48 files; repeated JSON byte-identical. The additional
  floppy installation passed with an identical required-file manifest.
- Real required CD files were read from ISO9660 into an automatically cleaned
  local temporary directory; the inspector recognized CD 1.01. One real AVI/BMP
  pair verified optional media counts. This is a test fixture, not implementation
  of a disc-image importer.
- Temporary corruption/removal/spoofed-release cases failed with the expected
  diagnostic codes and exit status. Both patch package directories correctly
  failed as incomplete installations.
- All 71 original `game/` file hashes were identical before/after validation.
  No extracted assets/test fixture was retained or included in source.

### Still not implemented

No game, simulation, JJFFE reference build, converter/cache, embedded model/image
decoder, upscale exporter, renderer or save importer. This tool is the first
version/content inspection step, not completion of the approved playable slice.
Next work: reproducible JJFFE reference plus validated asset-layout decoding.
The latest runnable commands and limits are in [README.md](../README.md).

---

## 2026-10-20 — Owner decisions and new release inspection

This entry supersedes conflicting decisions/recommendations in the original
Phase 0 entry below. Current authority: [CLAUDE.MD](../CLAUDE.MD).

### Approved technical direction

- Use the supplied **JJFFE 2.8a7** tree for gameplay; FFED3D is rendering research,
  not the simulation oracle. Rebuild/instrument local copies as necessary.
- Windows x64 first; first playable outcome is launch/fly/dock/trade/save/reload.
- Match original visuals first; Classic first-run default. Live graphics switching
  is required, not a restart-based setting.
- Verified bug fixes may be enabled by default with regression tests and a
  deviation record. Do not import FFED3D's Aniso modifications wholesale.
- Player installs original files in `game/`; generate private `data.bin`.
  Private `upscaled.bin` overlays images only. Never distribute original files;
  source/derived-art redistribution and project license remain unresolved.
- Owner performs upscaling. Export all eligible images plus explicit per-asset
  3840 x 2160-profile guidance, manifest/CSV, contact sheet and exclusions.
  Do not give every texture the same output dimensions.
- Prioritize native resolution, HD images, filtering, MSAA, then modest bloom;
  meshes/gameplay mods/scripts and advanced effects are deferred.
- Legacy saves import one-way to a versioned native save; preserve originals.
  Offer Classic/Modern remappable controls independently of graphics.
- Start HMP-to-MIDI playback/synthesis. Pre-rendered music is acceptable if useful;
  MP3 requires synthesis first, and no converted music ships with the engine.
- Owner gets five short scenario-based playtests (about 15 minutes), not the full
  importer/numeric/version matrix. Future builds must supply exact running steps.

### Current evidence

- Added floppy installation: 69 files; its executable matches `game/` exactly
  (English 1.02), despite the unpatched description.
- CD input is a MODE1/2352 BIN/CUE image, volume `1STENC_US`, 713 files.
  Its executable identifies as English 1.01 and has a different hash/size.
  `DATA/` has 323 AVI clips and 323 BMPs. The six cabins, 23 effects,
  17 music tracks and journal match the current game files.
- Both new patches are installer/compressed-payload packages, not unpacked
  patched executables. Package labels 1.06/1.06d are not yet confirmed runtime
  versions. Inspect resulting executables before registering importer support.
- See [GAME_FILES.md](GAME_FILES.md) for exact hashes and limits. No new
  decompilation, extraction, installer execution or original-game launch occurred.

### Persistent changes and outstanding work

- Recorded all 18 answers in the plan and reconciled baseline, bug, pack,
  save, scope and distribution policies in related docs.
- Protected root reference/release folders and generated packs through `.gitignore`
  and CI. Added synthetic regression tests, including Windows path/case handling.
- No runnable Rust game/importer/exporter or JJFFE study build exists yet.
  Rust tooling was not found on PATH; this planning update did not install it.
  The next implementation work is the bounded JJFFE reference/importer foundation.
- No public redistribution approval, license choice, patch-output fingerprint,
  GPU minimum or delivery date was inferred from the owner's answers.

---

## 2026-10-20 — Phase 0 complete

### Snapshot

- Studied `legacy/ffed3d-master/` (called `legacy/ffed3d/` in the brief).
- Verified `legacy/Elite3/` and `game/` are byte-identical: 71 files, same SHA-256. Recorded in `docs/GAME_FILES.md`.
- `FIRSTENC.EXE` is a 32-bit PE, image base `0x00400000`, entry `0x00410000`, code section 429,568 raw bytes, data section 1,621,504 raw bytes (2,469,888 virtual). SHA-256 `d9195661…`.
- Executable self-identifies as **English Release 1.02 — 21/04/95**; the reference build is JJFFE 2.8a6 + Anisotropic mod 0.9c + FFED3D.
- `BLD_INFO.CFG` (byte−1 obfuscation) gives build dir `c:\games\firstenc\patches\eng-fp`, product `FIRST ENCOUNTERS`, disk `GAMETEK`, total 2052.00 KiB.
- `PATCHES.INI` is AIL/UltraSound patch configuration, not a First Encounters gameplay patch. `GAMETEK.1` is the installer's compressed executable payload; `IMAGE.DAT` is the installer manifest. Neither is read at runtime.
- No CD copy supplied; CD layout/hashes not guessed.

### Source study

- Parsed all 1,715 functions in `ffe.asm`: 323,169 bytes of function spans; 331 descriptive names; 142 `ANISO` markers and 26 `FIX:` comments.
- `ffedat.asm`: 4,486 labels / 1,203,054 bytes; `ffebmp.asm`: 4,323 labels / 1,314,768 bytes. Combined ≈ 2.52 MB, matching the original data virtual size.
- Main global state: one `0x556a`-byte block at `DATA_008804`, zeroed by `asmmain`; 116 object slots, 18 starports, 465 model slots, 33 goods, 64 equipment entries, 26 military missions, 60 packages, 6 contracts, 40 sample slots, 17 songs.
- Timing: `TimerAdvance` accumulates milliseconds; nominal 20 ms frame (50 fps configurable); 50 Hz input timer; 49,710 tics/day (`0xc22e`); 994 tics per 20 ms at 1×. Start `GameTics=0x80000000`, `GameDays=0x121cf7`.
- Numeric model: 16/32/64-bit integers, 1.31 matrices (`0x7fff8000` = 1.0), 16.16 values, 16-bit binary angles, FFP (16-bit mantissa + 16-bit exponent), and x87 in 170 instructions. `F1519_fp` is an x87 vector normaliser used by matrix orthogonalisation (FFED3D “TEST” change).
- Data formats: indexed bitmap header `u8 type, u8 height, u16 width, 4 zero bytes, pixels`; models are command streams, not triangle lists; `MISSION.DAT` is the journal text archive with a bit-permuted 4-byte encoding and FF-code text; saves are a compressed/scrambled subset (`0x984a` bytes plus selected globals); `FX/*.RAW` is 16-bit signed mono 22,050 Hz; music is HMP.

### Legacy build

- Original toolchain not installed (VS2010 + DirectX SDK Feb 2010). Built a **study binary** with VS 2026 (MSVC 14.51), NASM 3.02, Microsoft `Microsoft.DXSDK.D3DX` 9.29.952.8, Wine `d3d9.h`, and a DirectInput shim.
- Temporary compatibility patches in a scratch copy (never in `legacy/`): `#pragma pack(1)` around FFED3D structs, `round`→`xround`, `time`→`qtime`, `Dxerr.h` stub, DirectInput data formats/wrapper, splash bypass for observation.
- Result: 2.47 MB `ffed3d_test.exe` launches, displays the JJFFE/Aniso splash, then the Frontier copyright intro; render loop is live (window title updates). Not a supported build; not distributed.
- Could not exercise full gameplay, saves, audio or scripted input yet. Phase 1 harness will add fixed seed/timestep/input/trace.

### Decisions proposed (awaiting owner approval)

- **Simulation oracle:** treat the built FFED3D/JJFFE reference as frozen (matches Phase 1 traces); log DOS 1.02 differences. This is the only practical oracle with the supplied source. *Pending explicit approval.*
- **Classic look:** reproduce the original DOS software renderer, not FFED3D's D3D look. *Pending approval.*
- **Aspect:** original 4:3 with non-square pixels (DOS), not FFED3D's 16:10 `aspectfix`. *Pending approval.*
- **Palette expansion:** indexed reference; GPU display expansion rule to be chosen (`v<<2` vs full-range). *Pending approval.*
- **Content format:** RON for content, TOML for mod/config, JSON for manifests/schema; indexed PNG + palette sidecar; glTF GLB + semantic command-stream sidecar; WAV/MIDI. *Pending approval.*
- **Workspace:** `ffe-core`, `ffe-content`, `ffe-convert`, `ffe-render`, `ffe-platform`, `ffe-tools`, `ffe-rs`. *Pending approval.*
- **No scripting in v1**; propose Rhai later, leave `ScriptHost` extension point. *Pending approval.*

### Discrepancies / gotchas found

- FFED3D's software buffer is **640×400** (`DATA_009226` = 256,000 bytes; `F1601` stride 640; `F1882` blits 640×400), while the logical UI is 320×200. DOS is 320×200. Do not inherit the 640×400 buffer into Classic.
- FFED3D uses **1.6 aspect** (`curwidth/1.6`) and a 35°/2.0 projection; DOS was 320×200 on 4:3. Do not inherit 16:10 as Classic.
- FFED3D replaces the bitmap font with Arial (`D3DXFont`), the HUD panel with a 3D panel + 1024×1024 render target, and textures with linear filtering. None of these are Classic.
- FFED3D's `C_PlaceStation` comment says “FIX: change from >> 4 to >> 3” but the code uses `>> 4`; the comment is stale. Do not “fix” it in the port.
- `F1519_fp` (“TEST: New vector normalisation…”) is in the normal matrix path and uses x87 floats. A naive Rust `f32`/`f64` port will drift.
- Several `FIX:`/`ANISO` changes are unconditional (laser invulnerability, collision radius, missile retardation, orbital drift, starports below ground, hyperspace time-travel, groundhog day, etc.). These are deviations from DOS 1.02, not just presentation.
- The source labels do not map 1:1 to original file offsets. The converter needs a generated symbol/layout map or signature-based extraction; verify against the original data section hash.
- `FFED3D` starts by `_chdir("game")` if `mission.dat` is absent. The finished product should use an explicit path search, not a working-directory hack.
- `ffed3d.vcxproj` Release links the **debug** CRT (`/MTd`) and disables optimisation; both configs use `StructMemberAlignment=1Byte`. Our study build needed `#pragma pack(1)` around FFED3D structs to avoid heap corruption.
- `PATCHES.INI` is not the FFE patch; `IMAGE.DAT` says the installed EXE is 2,101,248 bytes while our `FIRSTENC.EXE` is 2,097,152 — the installer size includes a 4 KiB trailer/rounding, not a different file. Do not infer a version from that alone.
- Splash text explicitly says owning a reverse-engineered copy is illegal even with the original; legal docs reflect this.

### Open questions

See `docs/archive/PHASE0_INVENTORY.md` §10 and `docs/archive/PHASE0_SUMMARY.md`. The blocking ones are: simulation oracle, Classic look/aspect/palette, music synth, CD version, legacy submodule, rule-4 gate, trace-harness patches, and reference build toolchain.

---

## 2026-10-20 — Repository hygiene

- Added `.gitignore` for `game/`, `legacy/`, `data.bin`, `to-be-upscaled/`, `upscaled/`, `extracted/`, frames/screenshots, build artefacts, saves.
- Added `.gitattributes` with `upscaled.bin filter=lfs diff=lfs merge=lfs -text`.
- Added `ci/check_forbidden.py` and a GitHub Actions job that fails if forbidden material is tracked (game data, legacy source, `data.bin`, original art, frames, FFED3D binaries).
- Added `LICENSE` placeholder (no licence chosen), `docs/LEGAL.md`, `docs/TAKEDOWN.md`.
- Did **not** add a Rust workspace: the port starts only after Phase 0 approval.

## Ship statistics, eyes and turret views

Cache v12 adds `base:ship/stats` (466 x 48-byte blocks from descriptor +0x38).
The engine glow now uses the original thrust word (F600: throttle times the
ship's forward or reverse limit) instead of the 4095 approximation. Front and
rear views sit at the ship's eye offset (FUNC_001053), and the top/bottom turret
views (F1 cycle, F8, console button 0x16, FUNC_001038 aiming) are ported for
ships with more than two gun mounts. verify-assets counts were bumped to v12;
its byte-for-byte check of the stats blocks against JJFFE is still to be added.

## The original line rasteriser (v0.48f)

F1616 DrawLine with F1601 DrawHLine is ported in `ffe-core::model_line` and
verified against the executed original: the new `--line-raster` adapter mode
draws 4096 synthetic on-screen lines and `verify-lines.py` compares every
320x200 buffer (4096 compared, 0 mismatched; tampered fields and an empty
trace rejected). Opcode 2 lines now take F1704's clipped, stored points (the
verified `model_thruster::line`) instead of raw projections, so every line
entry, narrow thrusters included, is drawn with it. Notes:
`docs/notes/lines.md`.

## The original curve rasteriser (v0.48g)

F1811's curves now draw with the original F1546 (`ffe-core::model_line::
draw_curve`): F1549's forward-difference steps (shared with the verified polygon
edges), F1561's one-end clip and F1616 per segment, with no closing edge. The
new `--curve-raster` mode runs the original on 4096 synthetic curves into a
320x800 buffer (so writes F1561 leaves outside the view are captured);
`verify-lines.py` compares both traces (4096 + 4096, 0 mismatched). The
viewer's stand-in cubic is gone.

## The original flat-face rasterisers (v0.48h)

F1604 DrawTriangle and F1607/F1608 DrawQuad (with F1602, F1603 and F1601) are
ported in `ffe-core::model_fill` and verified by the new `--fill-raster` mode
and `verify-fills.py` (4096 faces, 0 mismatched). Every on-view untextured
triangle and quad now draws with them, so most of a model's surface is the
original's pixels; faces reaching off the view and polygons still use the
diagnostic fill. F1616's slope now shares F1603's gradient (re-verified). The
flight-view test no longer expects a pixel from a degenerate triangle, which
the original fills with nothing. Notes: `docs/notes/fills.md`.

## Polygon spans and off-view faces (v0.48i)

The polygon span path is ported in `ffe-core::model_fill` (`Spans`,
`fill_polygon`: F1468, F1560 with F1563/F1559/F1558, F1613 with F1600/F1601)
and verified by the new `--span-raster` mode (4096 synthetic polygons, 0
mismatched). F1714, which queues a flat face reaching off the view (or across
the near plane) as an outline, is ported as `near_outline`; the texture-queue
probe now records those entries and the oracle compares 5,666 of them, 0
mismatched (model 361 left out: its data differs between JJFFE and this
release). The executor keeps each queued face's views for it. Polygons and all
queued flat faces now use the original rasterisers, and faces crossing the
near plane, previously skipped, are drawn.

## Oracle repairs (v0.48j)

`verify-assets.py` now expects cache v14 with 1,781 files, and the child-model
oracle masks the surface-resolution events 50-54 (F1874, verified for 448
models by `verify-station-models.py`) and 60/61 (F1876 queue words, not ported
yet), which the original now emits and the child execution does not model.
Both child runs report 0 mismatched (440 and 437 models) and the full set
passes: 31 of 31 oracles.

## Thrust limits (v0.49a)

F598, F599 and F600 (the clamp, per-axis cap and proportional scale of a
thrust request against an object's limit words) are ported in
`ffe-core::thrust` and verified by the new `--thrust-limits` mode and
`verify-thrust.py` (4096 cases, 0 mismatched). The ship statistics give each
craft its limits (`ShipStats::limits`). The first flight-core step: the
in-game thrust is not wired to them yet. Notes: `docs/notes/thrust.md`. Also
recorded: the owner decision to isolate drawing from the simulation RNG
(SIM-010).

## Velocity from thrust (v0.49b)

F601 and F602 (velocity from thrust over a time step: the Q31 transform of the
thrust words, the three time-step regimes, F601's caps) are ported in
`ffe-core::physics` and verified by the new `--accelerate` mode and
`verify-physics.py` (4096 cases, 0 mismatched). F602 differs from F601 in its
halving start (0xf, not 0x10) and its arithmetic shift of the time. Notes:
`docs/notes/physics.md`.

## Orientation update (v0.49c)

F585 (angular rates spent as small rotations of the orientation rows), with
F584 (step sizes), F611 (the rotation and its drift counter) and F1520 with
F1519 and F1552 (re-orthogonalising the matrix) are ported in `ffe-core::physics`
and verified by the new `--orient` mode and `verify-physics.py` (4096 cases,
447 through F1520, 0 mismatched). Notes: `docs/notes/physics.md`.

## The player's free-flight update (v0.49d)

F1041 (the player's orientation and thrust request in free space) is ported in
`ffe-core::player` with F607 (the set-speed request, F1515/F1516) and F1043 (the
engines-off thrust), and verified by the new `--player-orient` mode and
`verify-physics.py` (4096 cases, 0 mismatched). Frames of reference, gravity and
the planet auto-roll are not ported yet and the port refuses those cases.
Notes: `docs/notes/player.md`.

## Position from velocity (v0.49e)

F1512/F1511 (the velocity over a 64-bit time added to the 64-bit position,
with the original's shift normalisation) is ported as `Motion::advance` and
verified by the new `--advance` mode in `verify-physics.py` (4096 cases, 0
mismatched). With F601, F585 and F1041 this completes the physics core of one
free-flight step; the input layer (F1037/F1038), frames of reference and the
unit calibration for the playable still remain.

## A whole free-flight tick (v0.49f)

`Player::step` composes F1041, F601/F602 and F1512 in the order F1021 runs
them, and `--flight-step` compares it with the original chain over 4096
synthetic players (orientation, thrust, velocity and 64-bit position all
match). The physics core of free flight is now verified end to end; the unit
calibration for the playable, the input layer and frames of reference are next.

## The input layer (v0.49g)

F1037 (set speed from Return and Shift), F1038 (mouse and key steering, or the
turret in a turret view) and F1121 (the set-speed step) are ported in
`ffe-core::player` and verified by the new `--steer` mode (4096 cases, 0
mismatched). The comparison found an original quirk, now kept: with mouse roll
on, the roll's low byte is replaced by the view mode. Notes:
`docs/notes/player.md`.

## Original flight in the playable (v0.49h)

Key O switches the playable to the original flight model
(`ffe_rs::original_flight`): the verified tick (set speed, original steering
keys, thrust limits from the ship's statistics, velocity and position, full
orientation including roll) with the result copied back into `Flight` for the
views, engine glows, docking checks and the safety reset. Saves, docking and the
prototype model are unchanged; the save validator's speed bound rose from 250 to
5,000 flight units per second for the original ship's real acceleration. Notes:
`docs/notes/player.md`.

## Console buttons and time acceleration (v0.49i)

The console's real buttons are decoded from `DATA_005100` and `DATA_007684`
(`docs/notes/console.md`): the lower-left bar is the time acceleration meter
(not a throttle), beside four screen buttons. The meter and buttons are drawn
from the cache, the meter's steps and the 6 7 8 9 0 keys set the time
acceleration (1x to 10000x, scaling the original flight model's world time but
not its key time), the pause mark pauses, and the screens that are not ported
say so. Hot areas follow the original: inclusive edges, last match wins. A
readout shows speed, set speed and the time scale. The original console has no
speed or engines button in this build.

---

## 2026-10-23 — Terrain data in the cache, v17 (v0.51a)

- Cache v17 adds the planet engine's data: `base:terrain/tables`
  (`DATA_007836..7899` with the code pointers as ranks, checked against the known
  pattern), `base:terrain/maps` (the two height maps) and `base:model/collision`
  (each model's collision stream, cut by walking F582's grammar; 465 of 466 slots
  have one, 36 planet models carry the `0xff` terrain record). 1787 assets.
- Next: the terrain query (F1876 distance mode), starting with the midpoint
  routines F1849-F1867 against a node-level oracle.

---

## 2026-10-23 — The terrain query, shape collisions and starports (v0.51)

Notes: `docs/notes/terrain.md`, `surface.md`, `objects.md`.

- Cache v17 adds the planet engine's tables, the height maps and each model's
  collision stream with its region words.
- `ffe_core::terrain`: the arenas, the nineteen midpoint routines F1849-F1867 (one
  pattern with six switches), F1823's feature tree, F1834 and F1876 in distance
  mode. `--terrain-mid` (6,144 node cases) and `--terrain` (2,048 planets) match.
- `ffe_core::surface`: F576, F579-F583 (the collision streams, child models and the
  terrain record). `--planet` (4,076 tests, planets and ordinary models as the
  shape) matches.
- `ffe_core::landing`: F575's reorienting contact (terrain and sphere). With it all
  17,762 physics objects of `--worldgen` match byte for byte with no mask: a
  starport's position, orientation and velocity come out of the terrain query.
- Harness lessons: a record on a planet's face tree must not sit above another (the
  original indexes a table with a pool address); features of kind 12 only (the other
  kinds have no height map); planet types 3, 5 and 6 have no routine pair for faces
  with features; animation words overlap `+0x138` (write them first); the midpoint
  table's second label (`DATA_007845`) is four slots on.

---

## 2026-10-23 — Name generation, cache v18 (v0.51g)

Notes: `docs/notes/name-generation.md`. Cache v18 adds `base:text/names`. `ffe_core::names`
ports F791 (types 1 to 0x14), F793/F794, F795-F799 and F921; `--names` matches 8,130
names. Wiring names into system generation (F893) is next.

## v0.51h: names in system generation

`ffe_core::sysgen` builds every object's name as the original does (F881's star,
planet and moon strings, F877's `A,B` centre, F893 through `names::Names`), and
`Placement::create` copies it to the physics object's `+0x124` (F880's `strcpy`).
`--sysgen` (4,156 systems) and `--worldgen` (1,024 systems, 17,762 physics objects)
now run with names unmasked for generated systems; only the F877 uninitialised-counter
names and JJFFE's accented spellings are left out of the comparison
(`docs/notes/sysgen.md`).

Cache v19 appends the hand-coded systems' text (`DATA_007139`, 7,706 bytes) to
`base:text/names`; F873/F874 and F878's hand-coded path copy their names from it, so
the hand-coded systems' names are compared too (nothing in the system oracles is
masked any more).

## v0.52a: planet surface subdivision (F1876 draw mode)

`ffe_core::terrain_draw` ports F1876's draw mode up to the emitter: the draw set-up,
F1834, F1868 and F1869, F1826/F1827, and the screen functions F1715/F1716/F1847
(`Terrain::projection`). `--surface-draw` (probe build, forks per case) runs the original
on 512 planet patches and records every emitter call: 461 cases complete (51 fill the
50,000-word order buffer, where the original exits), 207,689 triangles match field for
field with the arena counts. Next: the emitter F1840 (queue entries via F1837-F1839, the
colour tables of F1841), F1877 for the textured planets, and the F1806/F1808 handlers.
Notes: `docs/notes/terrain.md` ("Drawing").

## v0.52b: the planet surface emitter (F1840-F1845)

`ffe_core::terrain_emit`: the four emitters by planet type (F1840 with its sea-level
crossings F1837 and colour slots F1838/F1839, F1842, F1844, F1845), the colour tables of F1841
and F1843, and the order queue (F1778, F1326). `--surface-draw` now prints the queue: for the
102 completing cases without a word-8 parameter every word matches, and the 10 cases that fill
the 50,000-word queue fill it in the port too. Next: F1877 (the haze and its queue entries) and
the F1806/F1808 draw routines.

## 2026-11-04: Modern graphics use FFED3D's assets (owner decision)

The owner decided that Modern graphics draw with FFED3D's models, textures and
shaders, as much of FFED3D's rendering as possible, through the port's own
wgpu engine, and that Modern needs a full FFED3D install (Q11 superseded for
Modern). The launcher's FFED3D check now accepts an installed FFED3D (AJ's
`MODELS/`, `Textures/`) and no longer wants `ffewin.cfg`; its Engine row says
Modern uses FFED3D's models, textures and shaders, LAUNCH refuses Modern
without a complete FFED3D, and the game takes the folder as `--ffed3d`. The
FFED3D source is cloned locally into `ffed3d-master/` from
`github.com/corpsofengineers/ffed3d`. What FFED3D does and the staged plan:
`docs/notes/ffed3d-modern.md`.

## 2026-11-05: owner overrides and upscaled cockpit sheets

Imported FFED3D art now resolves loose owner files, then packed owner files, then FFED3D, independently for each file. Panel/model meshes, skins and placement metadata share that resolver. Navigation/battle sheets and icon/void sheets sample native resolution with original normalized rectangles. Regressions cover stale packs, per-file fallback, imported mesh/skin/placement/panel overrides, non-integer upscales and source rectangles; actual owner navigation and battle sheets were captured in the dashboard harness.

## 2026-11-18: space dust matches the executed original (owner report)

Owner report from the DOS game in DOSBox-X: at about 10,000 km/s the original's dust is "+" glints
spread over the whole view; the port's was single grey pixels at the centre. Two causes, both in
inputs the first dust slice took as constant: `DATA_008865` (F1035's term in the cloud's `+0x7e`)
is F1099's F1475 speed class of the player's velocity, not 0 (the clouds grow with speed, so the
pattern is the same at any speed), and F1751's detail words `DATA_008812/8813` are `1, 1` for the
default High shape detail, not `0, 0`. The draw now applies F1680's scale/shift rule exactly. Replaying
the executed original's 45 clouds at 9,994 km/s through the port's draw gives 153 dust pixels against
the original's 151, all 151 covered. Details, method and regression tests: `docs/notes/dust.md`
("Speed class and detail words"); the local probe: `tools/reference/dust-probe.patch`.

## 2026-11-18: the player's autopilot approach replays the executed original (PAR-11)

`--flight dock` records the original's whole 6,570-frame approach (frames 1090-7660) to the
Lave Station clearance point. The port's `source_command` now follows it within 0.2% of the 4.1 km
leg on every frame (`launch_replay.rs`, FFE_DOCK_POSES). Three fixes: the AI runs once per
whole-second event and holds its command between events (first event frame 1104); F742's modifier is
the ship's `+0x101` (1 for the Sidewinder-class limit), only F746's final leg is the constant -2; and
an event whose F610 distance shift is 6 or less only requests clearance and leaves the thrust alone.
Details: `docs/notes/autopilot-source-audit.md`. Still open under PAR-11: obstacle avoidance, the
accelerated frame match, rotating-parent F589, ship targets.

## 2026-11-18: the Hope launch and the no-key launch replay (PAR-04)

`takeoff_replay.rs` takes `FFE_START_TYPE` (1 Ross, 0 Hope): the type-0 launch (`--start-key=0x39`) matches the
executed original on modes, set speed and hand-off frames, with velocity within 3 units. The replay had omitted F998's
F997 (the type-0 ship's rows 0 and 2 reversed), which `new_commander` already applies. `--flight hover 900` (no key
held) replays on both starts: the hand-off to 0x26 on altitude alone is on the original's frame (315 Hope, 292 Ross).
Harness: `--flight` accepts up to 20000 frames; `FFE_ENGINES_OFF_AT` and `FFE_LIMITS_CUT_AT` apply the engines-off and
damaged-thruster cases. Both differ from the port (engines off: a 30-unit velocity step the port lacks, so F1043's
engines-off request needs reading; halved limits: 0.6% by frame 248), so PAR-04 stays open on those.

## 2026-11-18: launcher audit

Audit of the Classic launcher and the changes it led to. **Videos:** the game reads the CD clips where they are (the game folder's `DATA`, or the folder chosen in the launcher, `--videos`), so nothing is copied into `FMVs`; a second Play row shows the count of AVI files and Verified (PLT-009, `notes/cd-media.md`). **Release:** the game row's icon and subtitle name the release found (CD-ROM 1.01, floppy 1.02, shareware 1.06). **Settings:** all five video filters are listed with what each does (xBRZ 3x was missing), with the scanline strength; Full screen starts the game borderless full screen (PLT-010); the Classic/Modern line changes with the choice; the video folder can be reset to the game's `DATA`. **Look:** the sidebar and its selected item are rounded, and a GitHub entry opens https://github.com/louisvarley/ffe-rs-classic. **Icon:** `ffe-rs.exe` carries the launcher's icon and sets it on its window. Checked by screenshots of the debug launcher (Play, both Settings tabs) and the unit tests in `ffe-launcher` and `ffe_rs::fmv`; the in-game video playback from a picked folder, full screen from the launcher and the window icon were not run in a live game.

## 2026-11-18: the galaxy map's F7 description page (ui-25)

The map's four console icons open pages the port answered with "not ported yet". F7 (the four-planets icon) now shows
F842's description: F956's cabin, the selected system's body diagram, "Stable system with N major bodies." at (10,95) in
colour 18 and the system description wrapped at 300 pixels from (10,105) in colour 17. Gateway's frame matches the
owner's capture of the original (15 bodies; same text and wrap). New: `ffe_core::system_text` (F830/F1343's `0xff` codes:
the FF30 integers, 1-3 decimal fractions, code variables, tonnes and nested system strings), `ffe_core::map_diagram` (F850/F851's
layout over F878's objects, with `DATA_006822`'s 35 sprite records and each body's hot square `0x80 + index`),
`ffe_rs::map_pages` and `Screen::draw_map_page`. The description is the handler-0 message string F895 reads
(`0x800c`, in `base:text/messages`), selected by the second dword F869 reports; the first dword stays the console summary.
F7 does nothing without a selected star, as F843's `0xf7` branch; F6 returns to the map. F8 picture mode, F9/F10's economy
and people pages and the click on a body (the hot squares are laid out but not routed) are ui-26 to ui-28.

## 2026-11-19: the galaxy map's economy and people pages (ui-26)

F9 (the graph icon) shows F843's `0xf9` page: the headings (`0x84b4`) in white at (2,59), then each commodity of
F870's stock string by its flag byte in yellow, nine pixels apart from y 69: bit 7 is an illegal good (x 256), supply
class 5 or more a major good and 3 or 4 a minor one, flag bit 3 making it an export (major imports 2, minor imports 65,
major exports 128, minor exports 193). Stock byte 32 is the 0x0d the original leaves after the string, which is why
Chaff is a major export. F10 (the people icon) shows `0xfa`: government (`0x84cb + (word & 0x3f)`), economy (`0x84bb +`),
allegiance (`0x84b7 + word >> 6`) and population (`0x84db + the population byte`) at (40,99), and the sector code and the
star's number at (40,139). Both match the owner's captures of Gateway. A selected star with no population byte has no
pages (DATA_008962 is zero): F9 and F10 draw nothing and F7 shows "Unexplored System". Code: `map_pages::economy` and
`people`; F7/F9/F10 in `main.rs`. ui-27 (click a body) and ui-28 (picture mode) remain.

## 2026-11-19: xBRZ 3x removed, rounded icon

Owner: xBRZ 3x looked poor in game. The filter, its draw path and its test are removed; the launcher lists the four that remain (Nearest, Linear, xBRZ 2x, Scale2x). A `filter=xbrz-3x` already saved in a `config.cfg` loads as xBRZ 2x, so the file is not discarded. The launcher and game icon (`crates/ffe-launcher/assets/icon.png` and `.ico`) is redrawn as a rounded square in the same design, to match the Launch button; the old one had chamfered corners.

## 2026-11-19: the galaxy map's body clicks and picture mode (ui-27, ui-28)

A click on a body of the diagram (F851's hot squares: `GuiAddHotRect` about the pen, half-extent the sprite width plus one)
is F843's `0x80 + index`: the diagram again with the body's name and type (`0x8476 + type`) at (10,95), or (10,125) for a
star pair or station; for a type 1 to 31 body its mass (F1494 of the exponent-less-0x3f short float, F1515 by `0xab5a`, `>> 2`,
two decimals), surface temperature (`+0x24` less 0x111) and starports (the type-0x23 objects that follow, joined by commas, else
`None`); and for a body with an orbit its period (hours, days or years by the period's exponent) and radius (F1482 by
`0xffdb759a`), or "co-orbital period / separation" doubled for a star pair's companion. `ffe_core::short_float` is F1482,
F1494 and F1515. Gateway 5 reads exactly as in the owner's capture. F8's picture mode (F844 to F849, F846's names) draws
F853's star density for each pixel (`zoom` sectors to a pixel, the pen starting at the bottom right) through `DATA_006821`'s 64
greys, then the 14 `DATA_006812` names in red wherever they fall in the view; all nine names of the capture appear within a
few pixels with the map centred on Gateway. F11/F12 step the zoom 1 to 255 (`DATA_008966`). Divergences: the original
refines the sky over three frames (4x4, 2x2, 1x1 blocks), the port draws the last pass; and in picture mode the original's
F9/F10 leave the map, where the port shows its economy and people pages.

## 2026-11-19: map page clicks fixed; the 3D system map found missing (ui-27 fix, ui-29, ui-30)

Owner report against ui-25 to ui-28. (1) A click on a body of the galaxy map's diagram never showed its facts: `main.rs` set
`mouse_was_down = mouse_down` at the FMV-input line, before the page's own `mouse_down && !mouse_was_down` test, so the
test was always false. It now uses a `mouse_edge` taken before that line. This builds; it has not been exercised in the running
game, and no test covers the loop. (2) The page in the owner's screenshot is the original's 3D system map, module 5
(`DATA_007239`: F963 enter, F982 keys and mouse, F977 draw, F985 time step; F975/F981/F986 draw the objects and orbit rings,
F978 the name labels, F980 the time-point slider, F983 a click on an object, `DATA_007243` the 'Time point:' text and
`DATA_007244` 'Click on a target or destination for autopilot'). Its F6-F10 hand over to the galaxy pages (F840). The port
has none of it: only the Tab body list and the `n` places list. Added `ui-29` and `ui-30`; nothing of them is implemented.

## 2026-11-19: the original's pirate AI traced; NPC guidance now runs once a second (parity-15)

The owner asked for a valid pirate trace after the forced-state experiments failed. Findings from the executed original
(`--flight ... --start-key=0x39`; `FFE_START_KEY` is ignored by `--flight`): (1) Hope's launch works (`--flight takeoff` and
`hover` reach free flight at frame 385); the frame-648 stop is `--flight paddock`'s docking stage (mode 0x2c) and was not
investigated further. (2) Over 20,000 frames at 10000x with the player docked, none of the nine pirate-group ships changes state,
parent or target; they ride F1507's orbit step. (3) `FFE_PLAYER_TO_PIRATE` (new, `tools/reference/headless.c`) moves the player,
not the NPC, 20 km from the first pirate leader; the original's own F750 then flies the leader at the player (target `+0xfe` is the
player's object) through F596 with modifier 1, adding the ship's waypoint words (F593). (4) The leader's thrust words, caps and
turn rates change only every 0xc22e tics (47.67 frames), with the `+0x14d` timer; F727's physics applies them every frame.
`autopilot::guide` with modifier 1 reproduces the words exactly; `Motion` with that cadence reproduces the record's orientation,
thrust, caps and rates exactly and position/velocity to a unit over 2,268 frames (`tests/npc_pirate_replay.rs`, local only).
Port change: `NpcShip::fly` recomputed guidance every frame; it now does so once per `THINK_TICS` (0xc22e) and passes modifier 1
(`think_tics`; a new ship thinks at once; `guidance_is_refreshed_once_per_game_second_not_every_frame`). ffe-core's 212 tests
and ffe-rs's suite pass except `the_black_space_colour_still_draws_the_stars` (`tests/prefs.rs`, star drawing, another session's
files, not NPC code); the local `npc_physics` tests still pass (a Cobra closes to 5 km in 21 s). Not done: the waypoint offset,
the global step phase, the port's pirates still attack from the anchor orbit rather than flying in from 1e15, and a trace of
attack states 1/2 (the leader drops to state 1 inside distance class 10).


## 2026-11-19: the 3D system map page (ui-29, ui-30)

Owner asked for it after the report above. The page is module 5 of the original (`docs/notes/system-map.md`): F6 on the
galaxy map opens this system's map and `n`/`r` the selected star's; the page draws the orbit of every non-starport object with a
parent, the names, "Time point: ..." and the target marker, moves the time point with the console slider (F982's action 0x80: the
offset from x `0x12c`, squared here and cubed for another system, `<< 18` of a clock that is 2^32 to the day; 26 pixels left is the
capture's 59 minutes) or Left/Right, and targets the body within eight pixels of a click (F983), flying to it after `t`.
`Body::orbit_centre` (the system list's parent, which may be a barycentre), `Navigator::bodies_at`/`date_at`/`engage_body` and
`galaxy_map::date_text` for times before the new game's start are new. Gateway's frames were looked at (orbits, names, picked body,
an hour earlier). The projection has no perspective and the orbit is a polyline (EXP-025); the slider's pressed-arrow redraw and
F984's initial zoom are not ported, so `ui-29` and `ui-30` stay in progress until the page is compared with the original's frames
and tried in the running game; nothing here was played, only drawn headless.


## 2026-11-19: a launch from Lave Station left the slot 82 degrees off and crashed

Owner report: leaving Lave Station the ship is not aligned with the slot and sometimes explodes. Reproduced by replaying a
hand-made input file (L pressed at frame 120 of a `--start 3` game) through the real loop with `FFE_TRACE=1`: the hull went
400 to 0 in the first frames out (contact flags 0xe2, the F575 crash bit), with the ship's first row 82 degrees from the
station's. Cause: `station_turn`, the record the F572 co-rotation turns the ship by from one frame to the next, was last set by
the scan frames of the docking (the scan also runs while docked) and was never cleared, so the first launch frame turned the
ship, already placed on the station's current rows, by all the spin since (about 16 s of game time at Lave's speed). It is now
cleared where the ship is placed at the door. After it the same replay survives, flags 0x62, speed 171 to 559 units within 0.2 s.
`tests/lave_launch_alignment.rs` (local only) flies the release, co-rotation and scan the way the loop does and asserts the rows
stay within a degree of the station's; it cannot see the stale record, which lives in `main.rs`, so the replay is the check for
that. A launch from another orbital station, or one after a long dock, had the same fault.

## 2026-11-19: laser beams drawn at the wrong scale (owner report: lasers fire in all directions)

Owner: lasers go off in every direction at every start, laser type and view, first
person and external. Deep dive into F931/F1776 and the executed original
(`docs/KNOWN_ISSUES.md`, "Laser beams"; `docs/notes/combat.md` 6.3).

- **Cause.** `Screen::draw_beam` drew model 0x9c at the firing ship's scale 8, not its
  own scale 17: 512 times too small, so the beam was a stub a few ship-widths long that
  pointed along the nose and looked different from every camera angle. The 2026-11-17
  change that did this rested on a probe that fired at the end of a frame, where JJFFE's
  F149 "Laser invulnerability" loop (`ffe.asm:15045`) clears the draw flag before F1536
  draws, so it never saw a beam. With Space held through the real input path the
  executed original draws a wedge (Eagle front view, 13,206 pixels) or a one-pixel ray
  (Saker, 286 pixels) and its `+0x7e` is 0 at the shot. The port now matches those
  pixel counts, colours and apex, and the Saker front ray exactly.
- **Gun origin and basis** now follow F931/F1776: the ship's own rows for every mount (a
  turret's aim turns, its gun does not), the rear gun at `+forward * word` with the
  forward row alone negated for drawing. The earlier rear/turret tests pinned the old
  arithmetic and are changed with the source lines cited.
- **Recorded, not fixed:** a few pixels of difference for a muzzle behind the camera (the
  Eagle's front wedge, rear views); NPC shots are not drawn (and in JJFFE only NPC beam
  lasers would be). `tools/reference/headless.c` gains `FFE_SPACE_FROM/TO`,
  `FFE_LASER_CODE`, `FFE_PRINT_E6`.

## 2026-11-19: the clipboard (F2 status) button did nothing docked in a station

Owner: the ship-overview clipboard button does not work in space stations; does the
original do this? **No.** The executed original (`tools/reference`, start key `0x04`,
docked at Lave in state `0x2a`, F2 = scancode `0x3c`, frame 400) shows the Ship Equipment
Status page over the cabin backdrop with the flight console, as it does on a pad
(start key `0x02`). The port handled F2 only in `Mode::Flight` (flight and pads), so
docked (`Mode::Docked`) the button and the key were ignored. F2 now opens the page
docked too, unless a docking or launch sequence is running; the docked screen draws the
flight console while it is up, F1 closes it (without also showing the bay), F4 returns
to the services board, the station's market/trade keys ignore input while it is up, and
it closes on launch. `tests/status_docked.rs` (local, needs the cache) draws it. The
status pages' other keys (F6..F12, R) work docked as in flight. Not changed: the galaxy
map (F3) and the other console buttons are still flight-only docked.

## 2026-11-19: Upgrades Fit/Remove results were never shown (commander-07: "remove does nothing")

Owner: commander-07 cannot remove the interplanetary drive to fit a hyperdrive; Remove does nothing.
- **The save.** A Turner Class (ship 46, statistics `+0x14` word `0x800d`, negative: an integral Class 4
  Military drive) whose drive was mis-jumped down to class 1 (the save's hold carries 908 t of Rubbish).
  F911/F948 set class 1 on any hull. F411 then refuses to remove the drive of a hull whose word is
  negative (message 0x98fb), and F409 refuses a second drive, so **the original also offers no way out**:
  the port's refusal ("The drive is an integral part of your ship.") is the original's rule, pinned by
  `outfitting::tests::a_misjumped_integral_hull_is_stuck_with_its_class_1_drive`.
- **The real bug.** The refusal was never displayed. F387-F389 print every Fit/Remove outcome through F352,
  the same console line as a station prompt, but the port logged only the screens' fixed prompts
  (`station::is_prompt`), so the result of Fit and Remove (success, "cannot afford", the refusals, the
  F428 mounting prompt) went nowhere. They now post to the log as `Source::Station`.
- Not changed: the stuck hull itself (a deviation to let an integral-drive hull swap a degraded drive
  would need the owner's decision).


## 2026-11-19: console messages folded away at once under time acceleration

Owner report: a new console message shows cut off (the date/time header with a sliver of the text under it) instead of being
shown for its time and then folding down. Replaying the owner's 18 November recording with a trace of F273's scroll showed it:
"WARNING: Undercarriage down." arrived at 100x (85,048 tics a frame) and "WARNING: Internal Fuel Tank Empty." at 10000x, and
F273's idle test (0x3cae6 accelerated tics, `DATA_008799`) passed in the next frame, so the entry went straight to the folded rest
(-0x20: header at y 191, the text's first row at 199). The original does not do this because F349, which writes nearly every game
message (F236, F246/F247, F289-F307, F323/F324, F656, F867, F910, F929-F944, F1022/F1023/F1033, F1097), calls F217 first: time goes
back to 1x unless the ship is in hyperspace. The port only did that for the Docking, Launch, Legal and Mission sources; the ship
warnings, combat, market and script lines now do it too (`Source::resets_time_acceleration`). The station prompt, the journal
notice (F89's direct F271) and the replace readouts (F352) still leave the speed alone. A message posted in hyperspace folds quickly,
as the original's does. Test: `a_message_at_high_acceleration_folds_at_once_unless_time_is_reset`. Not checked in the running game.


## 2026-11-19: system map follow-ups (owner playtest)

(1) The console's F5 icon, the map's own icon while the galaxy map or its system map is up, also swapped the left console page to the
combat console (`engines_page`, F1056's 0xf5): that handler had no guard for the map owning F5. It now skips it while
`map_keys::owns_keys` holds. (2) F6 only cleared an open F7-F10 page, so from the sky (F8) picture it took a second press to reach the
system map; it now opens it from any page. (3) The system map screen did not apply the map's console row (only `screen.galaxy` did), so
its icons showed the flight row; it now does and F6/N/R light the 0x23 icon. None of the key handling is covered by a test (it lives in
the main loop); the render test now draws the map row. The original "clicks give no details" report could not be reproduced by the
owner after the mouse-edge fix, so it is closed as fixed.

## 2026-11-19: the original's police response ported (parity-15)

Owner: port the original police behaviour. F238 (the module's day event, enabled by the wanted bit `DATA_008646` bit 0), F239
(the launch) and F240 (the frame event, enabled by the chase flag `DATA_008647`) are now `ffe_core::police`, acting on the offended
starport (`DATA_008804+0x50dc`, set by F236's witness) whose record holds the police left (`+0xdc`, initially `+0xce * 8 + 8`) and
the clock (`+0xde`). Open-air starports (`+0x14c` bit 0x40) start a chase at clock 0 and then roll F1530 below 0x4000 a day for one
police ship (F922(4), state 1); other stations launch two in state 0x22 when idle (mode 0xff) and F748 sends them in (state 1) when
the door has opened; clearing the wanted bit stops F238 and sends attacking police to state 8. Checked against the executed original
with the new `FFE_POLICE_TRACE` hook (`--flight idle ... --start-key=0x02` and `0x04`, the laser fired at frame 200) and its F1530/F922
ledger: the replay `crates/ffe-core/tests/police_replay.rs` matches all 7,798 Hope frames (11 launches) and all 4,798 orbital-station
frames (10 launches). Removed: the port's mass provocation of every police and military ship when the player is wanted (it never
existed in the original); only the ship that was shot answers. Wiring: `Navigator::offended`, `tick_police`, `spawn_police` (called
from `main.rs` after `tick_ships`, F772(0xe) through `ffe_core::spawn::create`), `NpcShip::{scrambled, turned, stand_down}`,
`Starport::{response, heat}`, `Legal::chase`; `plan_traffic`'s world building is now `with_spawn_world`. Local `npc_ai` test rewritten.
Open: F659's launch positions (the port uses its station launch point), F748's 17-frame delay, the global day phase, the port's own
non-original patrol, mission-script offences without an offended starport, and the broadcast that also clears the wanted bit.

## 2026-11-19: parity-15 closed as accepted (about 80 percent)

Owner request: finish `parity-15`. The tractable gaps the police work left were closed and the rest is recorded in
[ACCEPTED_PARITY.md](ACCEPTED_PARITY.md) rather than disguised.

Closed: (1) **F748's delay.** The executed original's orbital-station police leave state 0x22 143 frames (three AI steps of
0xc22e tics) after they are made, 17 frames after the door opens (frame 126), in all five launches of the trace: F748 looks at the
door only on the ship's own step. `NpcShip::scramble_tics` now gates it the same way (`a_waiting_police_ship_attacks_on_its_first_step_after_the_door_opens`).
(2) **The day phase.** F238's day is F152's global marker, not one that starts at the offence: `Navigator::police_clock` now runs
from the start of the game and is no longer reset by a jump. Both traces (Hope, orbital) fit the same phase (the marker's zero is
frame 6.9 from boot, day boundaries at harness frames 150, 198, 246, ...), so the phase is a property of boot timing, not a game
constant. (3) **No patrol.** F688 draws no police and nothing but F238 spawns one, so a normal entry (`spawn_plan` and the legacy
`spawn`) brings none; only the scripted capture (`guarantee`) keeps the port's patrol its frames were built with. (4) **Mission-script
offences.** F236 (script statement 40) now also records the offended starport (`Action::Offended`, the starport near the player's
ship), so F238's response acts on it. `police_replay` still matches 4,798 of 4,798 orbital and 7,798 of 7,798 Hope frames.

Accepted gaps (not closed, see the register): F659's launch positions, F593's waypoint offset (F749 sets `+0x102..+0x106` from the
shared pair; about 1 km on a 20 km approach), F702's system-wide placement (the pirates stay on the stand-in orbit; the original's
ships ride F1507's orbit at zero thrust until the player is in their frame, so there is no approach to compare while docked), attack
states 1/2 (untraced), missile rows 0x13-0x1b, traders/escorts/heavy freighters (no dynamics in the original without the player in
their frame), and F233's event 1 (clears the wanted bit; raised by a GUI key action through F150, no gameplay path found).

## 2026-11-20: police shoot a landed player; pirates hold still over a pad (parity-15 follow-up)

Owner report: at Hope, shooting the laser and fast-forwarding time, the police never destroyed the player as in the original, and
no pirate had attacked. Cause: `main.rs` skipped `tick_ships` whenever the player was on the pad or the ground, and `tick_ships`
held every NPC's fire while docked or landed, so nothing launched, flew or fired (the police clock and F238 sit inside
`tick_ships`). The executed original does neither: with the laser fired at frame 200 and the player idle on the Hope pad
(`--flight idle 12000 --start-key=0x02`, `FFE_POLICE_TRACE` now also prints the player's `pshield`/`phull`), the police launch, fly in
and cut the hull from 100 to 12 in nine hits from frame 1252, and the nine pirate-group ships ride F1507's orbit for the whole run
(also at 10000x). Fixed: NPCs are ticked and can shoot a player who is on a pad or the ground (only a player inside an orbital
station is out of reach, as the docked mode already was); the pirate group's stand-in orbit is held still while the player is landed
or docked, so a new game does not start with pirates on the pad. Local tests in `npc_ai.rs`: `the_police_shoot_a_landed_player_at_hope`
(1x: first hit on frame 966, no pirate hit; 100x: hit on frame 19) and `pirates_attack_a_player_in_flight_at_hope`. The port's police
are about 90 frames earlier than the original's first hit (F659's launch geometry is still the port's). Not yet checked by a person
playing the real window.

## 2026-11-20: the laser mounting choice is F428's menu

Owner asked for it to follow the original. F428 (`ffe.asm:30112`) counts the empty (or, for Remove, matching) gun mountings;
none is 0x98ff, one fits there at once, more than one runs F342 (clear the lists), F348 once per candidate (the entry named
0x9870 + k) and F334 with 0x9902. So the console line is "Select Mounting." and the Upgrades list is replaced by the candidates, each
a row of the list with the row button; F429-F432 are the four rows' handlers (F433 then F434/F435, and F383 restores the list).
`station::Mounting::{prompt, slots, rows}` and `main.rs` do that now: the Fit button, a digit (the nth visible row, as F354 maps
them, not the mounting number the old keys used) or Enter picks, Go Back or Backspace returns to the Upgrades list, and the rows
are drawn by the ordinary list code. Not known without a reference frame: whether F327 redraws the Upgrades column headers over
the mounting rows (the port keeps them) and what the rows' price columns show (the port draws none).


## 2026-11-19: a bought hull could not move (energy pool left at zero)

Owner report: start at Ross, Page Down to the Turner Class, part-exchange it for a Spar attack fighter with a class 2 drive, buy
the scanner, atmospheric shielding and autopilot, launch: the set speed changes but the actual speed stays 0 and the ship does not
leave the pad. Cause: `buy_ship` reset the player's `Combat` to its zero default, which leaves the energy pool (`ship+0x11e`) at
0. F441, the original's purchase reset, writes `0x20000000` there (`ffe.asm:30788`). F938's thrust energy goes through F929, which
fails the engine (zeroes the six limits) when the pool cannot pay, so the first second of flight stopped the ship (and logged
"Internal Fuel Tank Empty"). `Flight::install_hull_combat` now does F441's reset: lasers, pylons and cooling booster cleared,
the pool one tonne, the hull's drive byte, no engine failure, and the thruster words dropped so the new hull's statistics fill
them (the old hull's words used to stay, giving the Turner the Eagle's limits). The Page Down ship switch drops the thruster
words too. The second-event clock and laser heat are no longer reset by a purchase. Test:
`a_bought_hull_has_a_tonne_of_energy_and_its_own_thrusters`; `crash.rs::a_turner_class_launches_from_the_start_pad` (local) shows
a Turner leaving the pad when the pool is full. The 18 November recording's "Internal Fuel Tank Empty" at 10000x is probably the
same fault. Not played through by hand: the purchase screens are mouse-driven and were not scripted.

## 2026-11-20: the shipyard's View button shows the hull (F439's View page)

Owner: the View button when buying ships does not show the ship. It printed a one-line
summary that was never displayed (the port logs only the screens' fixed prompts), and the
original's page was a known gap. Ported F438/F439/F559/F562: the ship list becomes the viewed
hull's one row, the model turns at the left, and the four text blocks (masses, mountings, crew,
thrusters, drive, registration code, hyperspace range per class) are drawn at F439's
positions from the original's own templates and F902's ranges. See `docs/notes/station.md`.
`tests/ship_view.rs` (local) draws the Turner, Saker and Eagle pages; `ship_view` unit tests
pin the range rule and the template expansion. Not compared pixel for pixel with the original
and the fitted-equipment display of the model is not drawn.

## 2026-11-20: F1612's circle ported; v0.48-16 closed

Reading F1609-F1612 against the port found F1610/F1611 (particles) executed-oracle checked, F1609 (sprite) ported with hand-traced
tests, and F1612 (circle) not ported at all: `targeter::circle` used a square root per row and `Screen::circle` drew a 32-segment
polygon. `ffe_core::model_dot::circle` is now F1612 line for line: the radius is the size word halved, the decision value starts at
`8 - size` (`-2` for 7), a table of `2 * r` `(left, width)` rows is filled top, bottom and middle outward, and rows `y - r` to
`y + r - 1` are drawn cut to the view's 158 rows only, the first table row and the last row drawn as whole runs and the rest as
their two end pixels. F1111's circle (size 12) is therefore twelve rows, 157..160 across the top, one pixel left of symmetric
(153 and 164 on the middle rows), where the old code drew a symmetric outline. Hand traces for sizes 4 and 12, the top and bottom
clips and a size sweep (2 to 400, table bounds) are in `crates/ffe-core/tests/model_dot.rs`. No executed oracle: the reference
tool's `--particle` and `--dots` modes do not reach F1609 or F1612, and it cannot be built on this machine. A `--sprite` and
`--circle` mode in `tools/reference/headless.c` would close that if the Linux build is available.

## 2026-11-20: NPC lasers are drawn (NPC missiles already were)

Owner asked whether NPCs' missiles and lasers can be seen. Missiles: an NPC's F941 object goes into the same `MissileField` as the
player's and `main` draws every live one through the ship renderer, so they were visible (`npc_fire_view`, frame `npc-missile`).
Lasers were not: `NpcShip::fire`/`fire_at` returned a `Shot` only when the ray hit, nothing recorded the shot, and the hit event
dropped its `beam` flag, so a pirate's fire damaged the player with no beam on screen. F931 sets the object's fired flag (`+0x151`
bits 2-4) hit or miss and F1536 then runs F1776, which draws model 0x9c at that object's gun, for any object. Now `NpcShip.shot`
(`ShotMark`: the laser code, F931's `+0xe6` length - the hit distance >> 7 or 0xffff - and about one original frame, 1/18 s) is set
on every shot, `main` hands the visible ones to `Screen.ship_beams`, and `combat::npc_beam` places the gun at the model's front-gun
words along the ship's forward and up rows and draws it with the existing `draw_beam` (the code picks the pulse's red or the beam's
yellow, as the player's does). Only the front gun is used, as in the original: F773 fits only `+0xd2` and the AI's fire loop calls F931 with mount 0, so NPCs
never fire the rear or turret guns (F1776 would draw them, but no flag asks for it). Not done: at high time acceleration the 1/18 s of game time passes within a frame so the beam
flickers off; the player's beam uses real time. Tests: `npc_fire_view` (local cache; pulse miss, beam hit, missile frames) and
`a_shot_is_remembered_for_the_beam_hit_or_miss` in `npc.rs`.

## 2026-11-20: police and pirates now act on the player at Hope; the hunting AI is the original's (parity-15)

Owner report: starting at Hope, firing the laser and fast-forwarding brought no police, no pirate attack and no damage, and the
second and later missile warnings never showed on screen. Found and fixed, in the order they were found:

1. **Nothing ticked on the pad.** `main.rs` skipped `tick_ships` while the player was landed and `tick_ships` held every NPC's fire
   while docked or landed, so F238's clock (inside `tick_ships`) never ran. The executed original does neither: its police fly
   to the pad and cut the landed player's hull (`FFE_POLICE_TRACE` now prints `phull`/`pshield`). NPCs now tick and can shoot a
   player on a pad or the ground; the pirate group's stand-in orbit is held still while the player is landed (the original's pirates
   never reach a pad, 20,000 frames at up to 10000x).
2. **The ships were left behind by the planet.** Hope orbits at about 7.6 km/s; the player rides it (the original flight model's
   absolute position) but the NPC ships stayed in the inertial frame, so police launched on the pad and pirates in flight could
   never catch the player. `Navigator::refresh` now carries every ship by the displacement of the nearest world, station or
   starport (the shuttles and convoys already were).
3. **The hunting AI was the port's standoff model.** F750/F753/F754/F755/F756 are ported (`NpcShip::hunt_step`): the distance class
   (F592's shift) picks state 1/2 (F598's full deflection, F754's 60 degree facing test for state 2), state 3's break-away (F756,
   F588's turn away, sideways thrust from the relative velocity) or F749's reposition, once per game second; F724 (state 2) is
   the only state that fires, once a frame; F726 relaxes the velocity towards the target's in combat; F725 turns before it
   thrusts. A police ship replayed from the original's raw object record (`police_attack_cycle_follows_the_executed_original`,
   `FFE_POLICE_RAW=1`) matches the original's state and thrust words on all 148 frames of its life, through the 1, 2, 3 cycle.
4. **Equipment and launch geometry.** F773's outfit (the 0xa0 beam, no missiles) is applied to launched police (the port's own fit
   gave them missiles: the sounds the owner heard); police appear 270 m from the starport on the far side from the pad (the
   original's four launch points are 250 to 306 m) with the player's orientation (every launched ship of the trace has the
   player's rows).
5. **Hit frequency.** The original's beam reaches the hull on a handful of frames per pass (two or three, at 6.5 to 9 m from the
   player's origin) and only on some passes; the port keeps its simplified box test but at 8 m and at most three hit frames per
   state-2 pass (`PLAYER_HIT_BOX`, `MAX_HITS_PER_PASS`; DEVIATIONS EXP-023).
6. **Messages.** `Channel::post` ignored a message equal to the newest one, so every missile warning after the first vanished;
   discrete combat events now repeat (`Event::repeating`). The original's warning strings carry `0x01 0x02 WARNING: 0x01 0x03
   text`: `WARNING:`/`ATTENTION:` now draw in the red 0x10 and the text in 0x5f.

What the original does at accelerated time (checked, not guessed): `FFE_ACCEL_AT=300:3` after the shot, 5,839 frames: the hull
stays at 100, the police stock is spent in a few frames and the wanted bit clears (frame 383). The port matches (100x: no hits,
wanted clears). Tests: `police_attack_cycle_follows_the_executed_original` (ffe-core, local), `the_police_shoot_a_landed_player_at_hope` and
`pirates_attack_a_player_in_flight_at_hope` (ffe-rs, local), `a_warning_message_draws_its_prefix_in_red`,
`a_repeating_event_posts_again_but_a_repeated_state_line_does_not`. Real window (the replay of a synthetic recording, 1x, laser at
frame 300): the first police ship arrives at frame 1100, hull hits begin at 1739; the port's police are somewhat deadlier than the
original's (the trace's hull 100 to 12 over 720 frames; the port kills in about 1,700): launch velocity, the original's ground
collisions that remove police ships (state 28) and the exact hit shape are not ported.

## 2026-11-20: the navigation computer needs the equipment (Ross start)

Owner report: at Ross the navigation computer's icon is missing but a click on its spot still opened the list. The icon is right: F991
draws it only with equipment bit 5 and the cheap start ship's F998 arm sets `0x40240000` (no bit 5; Hope's has `0x40240020`). The click
and the `n` key were wrong: F1054's action 0x6e tests the same bit and does nothing without it. `main.rs` now ignores the icon's click
and the `n` key without the navigation computer; a real-game replay shows Hope's click opens the places list and `n` closes it, Ross
opens nothing. Also this day: the player's exit cloud after a hyperspace jump now lasts F1094's `0xff9e8e` tics (about 337 game
seconds) instead of one control tick (`PLAYER_EXIT_CLOUD_TICS`).

## 2026-11-20: ui-30 done, ui-5 and world-8 closed as accepted

Owner request: close off `world-8`, `ui-5`, `ui-30`, `content-2` and `ui-12`. `ui-30` is done on evidence (a click on a planet, moon or
station picks it, locks it, and after `t` sends the autopilot; the pad refuses with the `K` key's message, which the system map now
also shows). `ui-5` and `world-8` are closed at about 90% with the gaps in `ACCEPTED_PARITY.md`; the Escape path was replayed through the
real game (pause, pages, Load). `content-2` and `ui-12` stay open: `content-2`'s `Sy`/`Ci` work (F858) and `ui-12`'s object system and
effect frame trace are another session's open files. The stale "Not done" list of `docs/notes/galaxy-map.md` is corrected.

## 2026-11-20: content-2 done: every F76 macro, checked against the executed original

Owner request: finish `content-2`. `Pl`, `Sc`, `Sr` and `Si` are ported, which completes `DATA_102307`. An oracle now runs F77 in
the executed original (`FFE_EXPAND`, two new exports and a hook in `tools/reference/headless.c`) and the port's `macro_oracle` test
compares text and the shared random pair: 27 cases at four pairs, no difference. The oracle corrected two readings: `Pl` is not `Ci`
(it names the picked object's parent: a planet, `Gupta's Rock`, where `Ci`'s argument form gives `Sarajevo`), and the F858 form of `Ci`
also names the parent. `Si` is F791 kind 0x12 from the ship's `+0xa0` (`TY-198` at Hope); `Sc` writes `(-1,4)` for Gateway; `Sr`
picks a model from `0xf`. The bitmap font and blit oracle (`verify-font.py`) passes against the executed original, and every archive
article with `Sy`, `Ci` or `Pl` expands. One deviation is recorded (EXP-026): in the executed JJFFE the `(argument)` forms always fall
back to the current system because the buffer F76 compares starts at the `(`; the port keeps the DOS game's intent, which I could not
verify against the DOS executable. Not an oracle case: `Sl`, `Oo`, `Re`, `Ri`, `Rf`, `Zz` and the numeric macros (unit tests).

## 2026-11-20: ui-10 done (accepted at about 90 percent): one palette build per flight frame

Owner request: finish `ui-10`. The original queues every object of a frame, registering each colour with F1326, and builds the
64-box palette once with F1330 before F1789 draws; the port built it once per model render. `ModelViewer` now has a registering
pass (`begin_registration`, execute and register, build and draw nothing), `finish_registration` (one F1330 with the background) and
a drawing pass; `Screen::draw` runs the flight view's objects (background stars, discs, stations, ships, effects, lift, beam) through
both, saving and restoring its frame, jitter cursor, object queue, ship beams and haze sky around the first. The planet radius probe
no longer clears or builds. `Screen::palette_build_per_object` keeps the old behaviour for comparison. Measured at the landed Hope
start against `--game-frame 500 0`: 17 of 50,560 view pixels change (at most 8 levels), the upper palette holds 64 entries rather
than 50, and the view still differs from the original in 18,693 pixels (starport and terrain structure). The original has no palette
fades; its screen transitions are F915/F917/F919's clear plus an empty F1330 and F1331's upload on each `FlipScreen`. Open items are
in `docs/notes/palette.md` section 6 and `ACCEPTED_PARITY.md`; `crates/ffe-rs/tests/palette_frame.rs` (local) pins the build count.

## 2026-11-20: ui-29 done (accepted at about 90 percent): the system map uses the original's view

The page's camera was the port's own rotation and scale. It is now the original's, read from F963, F982, F984, F1676, F1680 and F1479
and checked against the executed original, which renders this page in the harness: `jjffe-smoke --game-frame 560 0x3d 0x40` (F3,
then F6) with a new `FFE_MAP_DUMP=<file>` that writes the object table `DATA_008801`, the camera block `DATA_008968`, the focus, turn and
zoom words and the time point. What the dump showed: the camera is an object whose parent is the focus (`DATA_008979`), standing
`mantissa * 2^(exponent - 16)` millimetres back along the third row of F1676's matrix (the mouse accumulators, which F969 starts at
0x100 each, as its two angles), and the whole scene goes through F1537/F1680/F1479; the zoom starts at `0x7fff`, exponent `0x2a`
(F963, whatever the system) and the focus at slot `0x6c`, `0x6f`, `0x60` or `0x70` by start type (F963's `arg >> 8`: 0, 2, 3, else
`0x6f`), which F964 keeps for the next visit. F11/F12 multiply the distance by `0x7998` at exponent 0 and `0x435d` at exponent 1
(F11 no closer than F984's model size plus ten, F12 no farther than exponent `0x3f`). The port's `ffe_core::model_animation::
setup_object` (F1680) gives every body's screen position and sprite: all six bodies at the Ross 154 start match the original's
`+0x98`/`+0x9a` exactly (once the port's clock is the original's time point, 1.68 s after the start), the names (F978: stars,
planets and moons only, at x + 3, y - 4, left out when the parent's dot is within ten pixels) match, and 837 of the original's 977
overlay pixels are shared. Rings: F986 draws model `0x96` for kind-1 objects with a parent, as a far object whose size is its
orbit radius's exponent less 0x19; the port draws F1507's circle as a polyline when F1680 would draw that ring (a star's orbit
about its barycentre, thousands of pixels across, is not drawn by the original and is dropped). Oracle: `tools/reference/
verify-system-map.py`; cache v35 is needed (an older cache has no object-dot tables, and the dots fall back to the port's discs).
Open: F986's ring model itself (colour and plane differ by a few pixels), the planet model when zoomed in, the slider's pressed-arrow
redraw and F271's replace flag for `t` (EXP-025). `ui-29` is closed in `ACCEPTED_PARITY.md`; 22 of 242 items remain open.


## 2026-11-20 - ui-12 and parity-08: the explosion, pods and sparks traced frame by frame against the executed original

Owner request: finish `ui-12` (hit particles, explosions, debris) and `parity-08` (the third-person player explosion before GAME OVER). Method as for `content-2`: an executed-original oracle. `--death-probe` (and `--flight takeoff|planet` with `FFE_DEATH_FLIGHT`) now sets the shared pair (`FFE_PAIR`), logs F922/F1530 draws with their callers (`FFE_RNG_TRACE`) and prints every effect object each frame (`FFE_DEATH_OBJECTS`: slot, model, control, AI, `+0x7e`, `+0xa4`, `+0x9e`, `+0x14e`, position, velocity words, rows; the ship's parent, mass, F603 fraction words and the occupied slots at the hit). `crates/ffe-core/tests/death_replay.rs` (local, `#[ignore]`) replays `effects::explosion` and `EffectTable::sweep` from the same pair and compares every frame.

The first replay was nowhere near equal, and everything it found was fixed in `ffe_core::effects` (details and source lines in `docs/notes/death.md`): F942/F939 draw two F1530 words for `MatBuildYZT` (the port used `F922(65536)`), F939's timer is the first of them `| 0x8000`, the cargo pods index `DATA_007212` by `F1530 & 0x1f`; a pod keeps the ship's rows, is placed at the ship plus its drift (F1514) and clones its F603 fraction words and parent; F713 runs F603 after F1512 (a pod at a planet's surface falls about a word a frame); the object list is swept from slot 0x72 down with a spark allocated at once (visited again this frame if it lands below its thrower, with a zero move), dead objects free their slot at once (the probe had been listing them because F940 clears the flag byte, not `+0x87`); F708/F712's F939 first draws one F1530 and throws nothing when bit 15 is set; the pods' `+0x14e` counts the global `0xc22e` second boundaries; sparks keep the thrower's velocity only at `DATA_008867 >= 0x9c40`. F719's missile explosion is now one F1530 draw (second angle zero), read from the source and not executed.

Result: 0 of 450 frames differ in the pad death with three pairs, the Lave orbital-station and Hope starport starts, and 0 of 800 and 500 in two moving-ship deaths in flight. New unit test `effects::f942_and_the_first_sweep_match_the_executed_death_probe` pins the pad death's first frame. `Navigator::push_burst` gives a burst's pods the planet or moon they are inside as parent, `Navigator::tick_effects` supplies the clock word, `main` supplies the player's speed and fraction words. Accepted at about 90% (`docs/ACCEPTED_PARITY.md`): the parent frame is the nearest planet or moon, F709's distance-class cull and a station parent's velocity zeroing are not modelled, no pixel comparison of the fragments, crash/laser/missile deaths share F942 and were not captured separately. `parity-21` (all effects from the source object states) stays with the object-system session; 20 of 242 items remain open.

### 2026-11-20 — Combat Targeting Computer reworked (enhancement-10/11)

Owner request: the NPC labels enhancement (launcher: Combat Targeting Computer, "Adds new dashboard button for seeing local ships and auto targeting them") was misaligned and a separate screen. The button now takes the escape-capsule icon's slot at (237,174), the system map's zoom-out button (`Click::Capsule`; F12 and X still launch the escape craft), the list is drawn as the navigation computer's places list (marker bitmap 0x31, names, 9 rows by 2 columns, no panel), the selected ship has a red square and the off-view arrow is a solid, outlined head with a tail. Unit tests in `combat_computer` and `targeter`; no simulation change.

### 2026-11-20 — Pirate groups start on F702's far orbit

Owner report: pirates attacked within seconds of taking off from Hope. The port's `spawn_plan` lurked the group 28 km out, so six pirates in the Attack state reached the player after about 1,200 frames. The executed original (`--npc-trace`) places them 1.3e15 from the root (F702/F1507's `0x5000/0x33` orbit), further out than Hope's planet, so the owner chose to match it exactly (the decision was theirs). `spawn_plan` now places each group at a random phase and height on `PIRATE_ORBIT` with escorts at the leader's formation offset; hostile ships are never recycled by distance, so they stay. `pirates_do_not_reach_a_player_who_has_just_launched_at_hope` replaces the old in-flight test (local). At normal speed the groups effectively never arrive; the scripted capture and `SpawnOptions::near_player` still put one in front of the camera. Also: `npc-targeting.png` is read from beside the executable or the working folder when present (25x25), else the embedded copy, so the button art can be tweaked without a rebuild.

## 2026-11-20 - gameplay-41 closed: the military office lists the scripts' rows

F493 builds the military office list from `DATA_009004` and then pulses global 0 with 3; mission 23's state 0 answers with F346 (`24 0xa9b9`) and waits for that code. The port drew only `Navigator::military_offers`, so the script row existed in the menu engine but never reached the screen. Entering F326's screen 7 now seeds the menu with the station's offers as native entries (the regenerated `DATA_009004`, `scripts::native_entry`), the signal inserts the scripts' rows in F346's place (the displaced offer moves to the end, as the source does), and the list draws the menu's order through the shared `station::menu_rows` model. Selecting a native row keeps F495/F496/F508/F506's briefing; selecting a script row runs F360, so mission 23's row starts mission 25 and its F331 conversation page. Cache-backed `mission_scripts::the_other_station_screens_signal_the_scripts` starts mission 23 with a future deadline, asserts the 0xa9b9 row after screen 7, selects it and asserts mission 25's 0x98d8 page with 0xa9bb and 0x983f; `station::f326s_rows_keep_the_scripts_entries_in_the_menus_order` covers the row order. Accepted gaps: F669's event 0x17 after the run (deliberate), the attackers' time-accelerated closing (`parity-11`), and no headless full playthrough check. 14 of 242 items remain open.

## 2026-11-20 - v0.48-8 closed: the planet-scale surface quad oracle

F1876 only queues a patch when the eye is at planet scale, so the surface oracle has to be the draw-mode table (`--surface-draw`), not the z-2000 station context. The executed-original table saved in `.cache/parity07/surface-draw-cases.jsonl` is 56 cases: the four near-pad records (`--surface-near`, about one radius) and 52 random planet-scale patches of models 113-147, details -1 to 4. 47 of the 54 completing cases are beyond 1.4 radii (median 4.7, up to about 70), so the table already covers the distance band the item was opened for. The port matches all 54 completing cases: 21,380 emitter triangles, the F1877 haze/sky state, and 54 order queues (369,811 words) word for word, with the 2 queue overflows reported as `Fault::QueueFull`. The recorded corner projections are checked separately: F1849's coarse branch averages the two ends' screen words at close range, so a quad needs one corner that projects to its queued point, while the queue itself stays the source's contract.

The item's own gate is `surface_draw_reference::executed_reference_planet_surface_draw`: it requires 40 cases beyond 1.4 radii, so the four near-pad records alone fail it (verified), and `tools/reference/verify-surface-planet.py` checks the saved table's shape and runs the test. `tools/reference/verify-surface-near.py` still regenerates the four near records and runs the near test; on the local reference binary `--surface-near 0..3` reproduced `surface-near.jsonl` byte for byte.

Accepted gap: the full 512-case `--surface-draw` run forks a child per case (an overflow or fault leaves the original unusable) and writes through `open_memstream`, so it is Linux-only; the Windows reference refuses `--surface-draw CASE` with "Use --surface-near CASE on Windows (no fork)". Its 465-queue/207,689-triangle result stays the recorded Linux run in `docs/notes/terrain.md`, not re-run here. The saved 56-case table is the Windows-runnable regression. `v0.48-8` is closed at about 90% in `docs/ACCEPTED_PARITY.md`; 7 of 242 items remain open.

### 2026-11-22 — Release preparation

Owner request: auto-update in the launcher, a tidy repository and downloadable builds. The launcher now checks GitHub releases and installs the Windows package in place (`update.rs`, checksummed, undone on failure; Linux and macOS open the release page). The Release workflow (`release.yml`) builds `ffers-<tag>-Windows-x64.zip`, `ffers-<tag>-x86_64.AppImage`, `ffers-<tag>-macOS.zip` and the same three as `MyRecomp-<tag>-...` on a tag push, with `tools/package.ps1`, `tools/package-appimage.sh` and `tools/package-macos.sh`. The repository root keeps `README.md` (rewritten: downloads, launcher, enhancements, building, releasing), `CLAUDE.MD`, `AGENTS.md`; `md-inventory.csv` moved to `docs/archive/`; the fifteen `death-*.ppm` captures were removed and `*.ppm` is ignored and forbidden by `ci/check_forbidden.py`. Clippy and rustfmt were brought to a clean `-D warnings` run for CI. Not verified: the Linux and macOS packages and the release job have not run (this machine is Windows; they run on first tag). The macOS build has no sound (`ffe-audio` outputs samples on Windows and Linux only). The statically linked LGPL-2.1 synthesiser still needs the relinking arrangement the burndown's release-4 audit records.

### 2026-11-22 — macOS audio and the LGPL decision

`ffe-audio`'s CPAL output (Linux only until now) now also builds for macOS (CoreAudio): the cfg gates in `sink.rs` and `audio.rs` and the dependency in `Cargo.toml` cover `linux` and `macos`; it type-checks for `aarch64-apple-darwin` here but has not run on a Mac. The same pass fixed a compile error the Linux path already had with cpal 0.18 (`SampleRate` is a `u32`, so `.0` no longer exists), which the Linux CI job would have hit. Owner decision: shipping the statically linked LGPL-2.1 `oxisynth` crates is approved. The arrangement is `docs/LGPL.md` (public source, pinned `Cargo.lock`, a `[patch.crates-io]` build recipe), carried in every package next to the licence texts; `tools/third-party.py` and `docs/THIRD_PARTY.md` now say so. This closes the open owner decision of `release-4`.

### 2026-11-23 — RecompLauncher manifest and automatic releases

`recomplauncher.json` (validated against https://recomplauncher.com/schema/recomplauncher.json, in CI too) describes ffe-rs for RecompLauncher: one `game` input (a folder holding `FIRSTENC.EXE`), a prepare step (`ffe-rs --prepare-music --game {input:game} --root {data}`, a new flag that renders `music.bin` and exits), the start command (`ffe-rs --game {input:game} --root {data}`), the saves folder, system minimums, and settings for the start location, skip intro, controls, display and the eleven enhancements (written to `saves/config.cfg` as an ini file). Not run inside RecompLauncher yet: the meaning of `{data}` and `{input:game}` is read from the schema's descriptions, and its documentation was not reachable. The `MyRecomp-<tag>-...` packages are built for it: zips with their files at the top level (`tools/package.ps1 -Flat`, `package-macos.sh --flat`), and the AppImage's `AppRun` runs the game itself when its first argument starts with `--`. The Release workflow now runs on every program-changing push to main: `tools/next-version.py` picks the version (first release as is, then a patch bump of the latest tag; `#minor`/`#major` in the commit message or the dispatch input choose more; a hand-raised Cargo version is released as is), commits `Release vX.Y.Z` to main, builds and smoke-tests the three platforms and creates the release on that commit; the release commit does not trigger another. `ci/test_next_version.py` covers the rules.

## 2026-11-25 - Updater on every platform; CI fixes

Owner request: the launcher updater must work on all platforms, and the Windows build and a CI test failed. `docs/THIRD_PARTY.md` was stale (the packaging step's licence check refused it); regenerated. `Expand-Archive` failed on the CI runner (no `ExtractToDirectory` overload), so unpacking is now in-process with the `zip` crate (refuses paths that leave the folder; keeps Unix modes) and a `Compress-Archive` regression test covers the real release zip through unpack, package folder and install. The updater now picks the package by platform (`Platform`: `-Windows-x64.zip`, `-x86_64.AppImage`, `-macOS.zip`) and, for the two non-Windows packages, verifies the download against the release's `SHA256SUMS.txt`: Linux renames the checked AppImage over `$APPIMAGE`; macOS unpacks beside `ffers.app`, swaps the bundle and keeps `ffers.app.old` until the next start (put back on failure). Not verified here: the Linux and macOS paths run only in CI unit tests (this machine is Windows); a real update on those systems needs the first release built by this code.

## 2026-11-25 - Pad launch at a busy starport; laser sight colour; update popup

Owner report: from the launcher's Sol (Mars) start, Services > Launch Request did nothing and L then gave "Launch refused: traffic control is busy." with no take-off (the game log showed the refusal on every press). Another ship's F654 grant was holding Olympus Village (a grant lasts eight game hours, `CLEARANCE_SECONDS`), and `Navigator::begin_launch` turned F654's refusal into an error for the open-air pad too. The pad's launch action (F1056's action 9 at state 0x30) takes the ship off whatever traffic control says (a launch without clearance is the F237 offence the caller already records); only Launch Request asks F654, and an indoor station's lift still needs the grant. `begin_launch` now ignores the refusal at an open-air starport (local test `a_busy_starport_does_not_stop_the_pad_launch`, plus `sol_start_takes_off_from_mars`). Why a ship holds the Mars pad at the start is not investigated. The fixed laser sight (`enhancement-13`) now draws in the fitted laser's colour instead of one fixed colour. The launcher's update popup shows only the version and an Update button, with padding.
