# Classic port burndown

What is still to implement for a like-for-like Rust reimplementation of
Frontier: First Encounters, as of 2026-11-18 (after the accepted-95 parity closures, see [ACCEPTED_PARITY.md](ACCEPTED_PARITY.md)). This checkout is Classic only: the Modern GPU engine, the FFED3D integration, `assets.bin` art
packs and the Enhanced renderer and graphics-pack plans are out of scope and have
been removed. (The launcher was rebuilt as a Classic launcher.)

The source of truth is [`dashboard/burndown.js`](../dashboard/burndown.js)
(open `dashboard/index.html` for the chart). This page is a summary of it. Update
the dashboard first, then regenerate or edit this page to match.

## Summary

**0 of 242 work items are open: 0 in progress, 0 not started.** The foundation,
model-engine, visible-slice and audio milestones are complete (45 items).

| Area | Open | In progress | Not started | Done / total |
|---|---:|---:|---:|---:|
| Parity blockers (2026-10-26 audit) | 0 | 0 | 0 | 24/24 |
| Flight core | 0 | 0 | 0 | 14/14 |
| Galaxy and systems | 0 | 0 | 0 | 13/13 |
| Gameplay systems | 0 | 0 | 0 | 44/44 |
| Original drawing: models | 0 | 0 | 0 | 19/19 |
| Original drawing: screens and effects | 0 | 0 | 0 | 31/31 |
| Content import | 0 | 0 | 0 | 7/7 |
| Saves | 0 | 0 | 0 | 25/25 |
| Classic split cleanup | 0 | 0 | 0 | 2/2 |
| Release and sign-off | 0 | 0 | 0 | 7/7 |
| Enhancements | 0 | 0 | 0 | 11/11 |
| **Total** | **0** | **0** | **0** | **242 / 242** |

## Critical path

1. **Parity blockers**: the P0 items first (event units, takeoff handler,
   contact rules, Hope start, player explosion), then the P1 items.
   Most of the flight, world and gameplay work depends on them.
2. **`flight-1`, the object system and agent records** (`cstruct`/`gmstruct`).
   Not started, and much of the remaining flight and gameplay work builds on it.
3. **Flight core, galaxy and gameplay systems**, in the order given in
   [STATUS.md](STATUS.md#work-list-to-finish-the-game--2026-11-04-handoff).
4. **Original drawing**: the planet surface quads, the whole-frame pixel
   oracle, sprites and particles, and the last 17 model slots.
5. **Saves, content and release**: legacy commander import, CD 1.01
   verification, full playthrough checkpoints and the owner's sign-off.

## Open items by area

`[~]` is in progress and `[ ]` is not started.

### Parity blockers (2026-10-26 audit) (0 open)

- [x] `parity-08` **P0** Restore the complete third-person player explosion before GAME OVER (accepted at about 90%: the executed original's F942 death traced frame by frame, see ACCEPTED_PARITY.md)
- [x] `parity-15` **P1** Replace NPC speed caps and simplified flight state transitions (accepted at about 80%, see ACCEPTED_PARITY.md)
- [x] `parity-20` **P1** Replay market, shipyard, BBS and daily mission events using source state — accepted at about 90%: `--daily` traces the first day boundary's 125 F922/F1530 draws (F633's job/news chances, the 99 market steps, F621's board and F634's ship queue), `daily_reference.rs` replays every pair step and `Navigator::tick_days` ends on the same pair with the board/job state changed; the per-station market streams remain EXP-006.
- [x] `parity-21` **P1** Drive all explosion, debris and hit effects from source object states — accepted at about 90%: F1512/F1511 movement + F928/F927/F924/F767 lifecycle for every effect kind, `death_replay` 450/450; F719's missile-end explosion is now on hit/timeout and a missile-killed NPC gets F942/F926 (`parity-22`).
- [x] `parity-22` **P1** Replay missile, ECM, escape and legal outcomes in the complete event loop — accepted at about 90%: `--missile-encounter hit/ecm/naval/mine/escape` traces a model-6 hit and F719, standard-vs-naval ECM (models 6/7), the model-5 state-0x14 mine and F1098's model-16 Stowmaster with the abandoned state-0x16 hull; F719 now also runs on a missile hit/timeout and a missile-killed NPC gets F942/F926.
- [x] `parity-23` **P1** Remove silent fallback to the synthetic playable from normal starts/loads
- [x] `parity-24` **P1** Use the same source new-game setup after death and on every resume path (done 2026-11-20: every new-game entry calls the one `boot::new_commander` initializer and both load entries share `apply_save`; a cache-backed fresh-boot/post-death-restart comparison covers starts 0 and 1)

### Flight core (0 open)

- [x] `flight-1` Object system and agent records (cstruct, gmstruct) — accepted at about 90%: the object/agent record layer, slot table, jump cycle/clock, F606 frame-of-reference, F802/F610 avoidance, F939 smoke, F682/F680 carried jump and the orbital/starport overlay; renderer-consumer and F802-shape gaps recorded.
- [x] `flight-5` Original input processing, key accumulation and remapping — F1037/F1038/F1121 and the 730-frame replay were already verified; `config.cfg`'s `bind.*` lines now drive the input mapping (game-key or action names, unknown names ignored) and a remap changes the delivered flight action. Only the in-game remap screen is unported (out of scope).

### Galaxy and systems (0 open)

- [x] `world-8` Navigation: system and galaxy maps
- [x] `world-9` Lighting by the local star (ships, stations, lift) and day/night — F1481's source light sign feeds all three object paths; F1535 star-class tables (including Ross 154's flagged class) and the planet terminator are verified with focused cache-backed regressions.
- [x] `world-13` Source model scale and near-surface radius/frame transitions — accepted at about 90%: F1680's operand/key scale (+0x148/+0x130) and cull now drive near-surface and distant bodies (verified by unit tests and the near-surface executed-original oracle); the rotating-frame branch and a matched whole-frame planet capture remain reference-gated.

### Gameplay systems (0 open)

- [x] `gameplay-5` Missiles and countermeasures - player launch/lock/guidance/fuse/damage + F10 view, standard/naval ECM, chaff, NPC launch/evasion, F754 alignment; residuals in combat.md (F937 blast, F935 ring, F989 walk, chaff decision).
- [x] `gameplay-6` Agents: AI behaviour (cagent) - accepted at about 90%: F770's trading approach (state 0x09), the missile/object rows 0x13-0x1b as a typed table and F659/F233 were closed 2026-11-20; F702's orbit, F597's placement, the taunt text and F941's missile selection are recorded gaps.
- [x] `gameplay-7` Police, pirates and other encounters - accepted at about 90%: F659's launch tables replace the fixed offset, F233's arrival clears the wanted state and an attacker that hits a world is destroyed as state 0x1c; EXP-023's beam box, F589/F671 and the fines office are recorded gaps.
- [x] `gameplay-8` Combat scenario replay against the reference — accepted at about 90%: `--combat` traces the Hope pirate group, `combat_replay.rs` replays F750's guidance (398/399 thrust words exact, 399/399 caps and positions, 396/399 velocities) and checks the state changes, fire and hostile damage; the F754 state-1/2 pass is the recorded residual.
- [x] `gameplay-9` Bulletin board and missions (cbbs) — accepted at about 90%: F460 removal conversation, F291 non-player branch, F329 rows, service flags.
- [x] `gameplay-10` Journal and mission state — accepted at about 90%: F302's overdue-failure pass, F301's notice and F303's branches plus signed reputation; F300's removal-list page, the F304/F305 rank page and F485's abandon remain as recorded gaps.
- [x] `gameplay-12` Communications and messages — accepted at about 90%: F460's civilian removal conversation and the F503/F470 fidelity; removal-contract/F291-placement/message-page gaps recorded.
- [x] `gameplay-18` NPC flight/combat AI (DATA_006258), ship fire, police/wanted and jump persistence - accepted at about 90%: the hunting AI, per-frame fire, F953, F674/F675/F680 persistence, the F238 police response and the missile rows are ported and replayed; the same gap list as `gameplay-6` applies.
- [x] `gameplay-26` Saker Mk III default arm, F2 loadout and start records — source comparisons cover start types 0/1/2/3/5; cache-backed default Hope, Ross 154 and F2 hold regressions pass.
- [x] `gameplay-38` Runtime module clock, source F3/F4 and landed/flight event routing — 5 runtime and 3 player-event regressions pass; one original-capture comparison remains fixture-gated.
- [x] `gameplay-40` Starport pad descent F741-F746/F652 with 0x59 geometry import — pad landing runtime and model 0x59 collision geometry tests pass with the local content cache.
- [x] `gameplay-41` Mission-script engine (F41/F31): script import, journals and scripted missions — accepted at about 90%: the interpreter, journals, station conversations and object statements run, the scripts' adverts and rows reach the station screens, and F493's military office now prints and selects the scripts' F346 rows (mission 23's 0xa9b9 leading to mission 25) through the shared menu row model. Recorded gaps: F669's event 0x17 is raised after the run rather than mid-statement, the attackers' time-accelerated closing is `parity-11`, and there is no headless in-play playthrough check.
- [x] `gameplay-42` Fix the source-verified broken missions as always-on bug fixes — 5 fixed (78, 55, 20, 83, 81; EXP-030..034), 5 orphaned documented not-patched (37/38/39, 90/91; EXP-035).

### Original drawing: models (2 open)

- [x] `v0.48-7` F1834 / F1835 quad emission (with F1868-F1873) — F1835 now explicitly source-dispatched; F1834/F1868/F1869 verified word-for-word against the executed original (54 cases, 21,380 triangles, 54/54 order queues); F1870-F1873 query flags matched by `Terrain::distance` (2,048 cases).
- [x] `v0.48-8` Surface quad oracle at a planet-scale distance — the saved executed-original table (56 cases: 4 near-pad, 52 planet-scale) matches the port in 54 of 54 completing cases (21,380 triangles, 54 queues / 369,811 words word for word, 2 queue overflows); 47 cases are beyond 1.4 radii (up to about 70). `executed_reference_planet_surface_draw` and `tools/reference/verify-surface-planet.py` gate the planet-scale half; the full 512-case `--surface-draw` run is Linux-only (`fork`/`open_memstream`) and is the recorded Linux result.
- [x] `v0.48-14` Pixel-level whole-frame oracle (framebuffer mode, station and three ships) — `--scene-frame` draws a fixed station and three ships through the executed original's F1537/F1680/F1772/F1330/F1789; `verify-frame.py` and `scene_frame` match 63,946 of 64,000 RGB pixels (accepted: 54 from the unported F1747 name-board text and a two-pixel station-edge depth tie).
- [x] `v0.48-16` Sprites, particles and circles (F1609-F1612) — particles checked against the executed original (6,000 cases); sprite and circle by hand traces (no oracle mode exists for them)
- [x] `v0.48-17` Remaining 17 of 466 model slots and the CD 1.01 census — 448/466 executed; the 16 child-only slots verified via their real parent call, model 89 (truncated in both 1.02 and 1.01) and null slot 149 are source/data-verified refusals; CD 1.01 census matches floppy per slot.

### Original drawing: screens and effects (0 open)

- [x] `ui-5` Menu and text rendering for every screen
- [x] `ui-7` Intro and new-game flow — owner authorized closure of the remaining user QA gate; recorded intro checks were already passing. Direct `L`/load-picker behavior is outside this item.
- [x] `ui-10` Original palette effects and screen transitions — accepted at about 90 percent: one palette build per flight frame (the objects run in a registering pass, one F1330, then draw); 17 view pixels change at the landed Hope start against the original's 18,693 structural differences; the original has no palette fades. Open: DAC upload (RGB here), EXP-013, station name-board glyph colour (docs/notes/palette.md section 6).
- [x] `ui-11` Far and near transitions, clipping and planet approach fidelity — accepted at about 90%: F1680's far/dot classes and F1686's cull now run for every object of the flight view (stations, NPC ships and missiles as well as the planets); `far_objects` (4, local) covers the model→glint→absent transitions and the station/ship boundaries, `planet_view`'s 26 local cases cover the approach, surface, horizon, rings and landing; the unported lit shaded sphere is unreachable for system bodies (largest dot 7 px), and the rotating-frame branch is `world-13`'s
- [x] `ui-12` Hit particles, explosions and debris (F939/F942/F926) — accepted at about 90 percent: every frame of seven deaths matches the executed original (slot, model, state, timers, position, velocity, rows); see docs/notes/death.md
- [x] `ui-25` Galaxy map F7 description page: F850/F851 body diagram, F830/F1347 string templates, F842 text — Gateway's frame matches the original capture; 5 unit tests and 2 cache-backed tests pass.
- [x] `ui-26` Galaxy map F9/F10 economy and people pages (F843) — Gateway's two frames match the original captures; the cache-backed test checks every good and line.
- [x] `ui-27` Galaxy map body click info: mass, temperature, starports, orbit (F843 0x80-0xbc) — Gateway 5's facts (299.02 Earth masses, -99 C, 104.7 years, 21.050 A.U.) match the original capture.
- [x] `ui-28` Galaxy map F8 picture mode: the star-name sky (F844-F849) — all nine names land where the original capture has them; the sky is the final of F844's three passes.
- [x] `ui-29` 3D system map page (module 5, F963-F982/F985): orbit curves, body labels, the 'Time point' line, mouse turn and zoom, F5-F12 into the galaxy pages — accepted at about 90 percent: the original's start (F963), camera (F982/F1676) and projection (F1680/F1479); all six bodies' screen positions, the focus, zoom and camera match the executed original (`verify-system-map.py`), 86 percent of its overlay pixels are shared; F986's ring model is drawn as a polyline.
- [x] `ui-30` System map object click (F983/F986): target or autopilot destination for a clicked planet, moon or station.
- [x] `ui-24` Display filters and scanlines — owner authorized closure of the remaining user QA gate; focused implementation/settings checks were already passing.

### Content import (0 open)

- [x] `content-2` Message strings, journal and remaining fonts
- [x] `content-4` Optional CD media inventory and lossless packaging (videos and stills) — 323 AVI + 323 BMP, verified FMVs manifest; see [CD media](notes/cd-media.md)

### Saves (1 open)

Completed 2026-11-15: `persist-6` through `persist-25` are twenty bounded native
resume fixes under PAR-18; see [persistence](notes/persistence.md#21-live-resume-records-2026-11-15).
The twenty additions are complete, so the remaining count is unchanged. The summary
also includes the previously added, still-open display-filter task `ui-24`.

- [x] `persist-1` Legacy commander import (read-only) — F175-layout container + F170 codec/checksum proven against the supplied original; commander (`Jameson`), system (`Gateway`) and save identity recovered read-only; the gameplay-state field map (credits/ship/cargo/clock/RNG) is established and replayed in `persist-4`.
- [x] `persist-4` Imported saves replayed against the reference — `--legacy-save` loads `game/FIRSTENC._01` through the real F70/F66/F171; the 0x984a object array and the DOS player block are byte-exact, and 14 commander fields match `saves::legacy_import` (Jameson, 10,000 CR, Saker 31/TY-198, cargo/fuel/clock/RNG). The field map (gmstruct offsets; 0x5535-byte DOS block) is in `docs/notes/persistence.md` 1.2.1.

### Release and sign-off (0 open)

- [x] `release-1` Full playthrough checkpoints against the reference — `verify-playthrough.py` chains one checkpoint per stage: `--legacy-save`/intro, `--flight`, `--trade`, `--save-roundtrip`, `--jump`, `--combat`, `--daily` and `--scene-frame`, each with its port test. All nine stages pass; the accepted gaps (no executed in-play mission chain, F909's arrival draw ledger, the map destination cursor and a JJFFE save's string-section commander name) are in [ACCEPTED_PARITY.md](ACCEPTED_PARITY.md).
- [x] `release-6` Choose the Rust code licence — MIT (owner decision, 2026-11-20)
- [x] `release-7` Like-for-like parity sign-off by the owner — signed off 2026-11-21; the burndown is at zero.

### Enhancements (0 open)

Optional, toggleable improvements — each a launcher toggle, save-safe and mid-game toggleable. See [enhancements](notes/enhancements.md).

- [x] `enhancement-1` Turrets turn while time is paused
- [x] `enhancement-2` Missiles ignore their launcher's collision on launch
- [x] `enhancement-3` Autopilot: offset waypoint around intervening bodies
- [x] `enhancement-4` Autopilot: braking accounts for retro thrust + gravity (step-independent)
- [x] `enhancement-5` Autopilot: collision test against every body in range
- [x] `enhancement-6` Missiles: swept (continuous) collision detection
- [x] `enhancement-7` Mission fixes (assassination credits, hitchhiker, medals, Alioth deadline, docking bays, dialogue input, mission-NPC slots) - all seven runtime behaviours behind `enhancement.mission_fixes` (EXP-046); the medal award path and the `gameplay-42` script corrections were already source-correct.
- [x] `enhancement-8` ECM can disable NN500 missiles
- [x] `enhancement-9` NPC relative speed (selected NPC shows relative speed + hull + shield)
- [x] `enhancement-10` Combat Targeting Computer (dashboard button, list of local ships, auto target)
- [x] `enhancement-11` Edge target indicator (arrow/bar for out-of-view target)
