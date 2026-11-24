# Status and handoff

For the active task, authorization and next verification command, read
[WORK_HANDOFF.md](WORK_HANDOFF.md). This page is the broader project inventory;
it is not a requirement to re-audit every subsystem before a focused change.

## Two workstreams

This checkout is **Classic only** (owner decision, 2026-11-10): a like-for-like
reimplementation drawn the original's way. The Modern GPU engine, the FFED3D
integration, `assets.bin` art packs, the launcher and the Enhanced/graphics-pack
milestones were removed. [BURNDOWN.md](BURNDOWN.md) summarises what is left.

| Workstream | Owns | Next work |
|---|---|---|
| **Game** | Shared simulation, controls, world data, collision, traffic, economy, missions and saves. | Verify the normal start, takeoff, flight, approach and docking loop against the original. Follow with autopilot and time acceleration, hyperspace, and save/resume. The parity audit below also tracks remaining economy, mission, combat and NPC behaviour. |
| **Classic Engine** | Original software drawing, model fidelity, palette, cockpit and screen presentation. | Close the remaining terrain and clipping, model and effect, and whole-frame comparison gaps in [Original drawing](#7-original-drawing-classic). |

Use the [burndown](../dashboard/burndown.js) for individual task status. The work
list below is the November 4 handoff, with the Modern section dropped. Verify
reopened tasks against current replay evidence before marking them complete.

Classify a bug by what changed. A ship occupying a pad it cannot dock at is a
Game bug if the simulation put it there; if the simulation has it in the right
place and Classic draws it wrongly, fix the drawing. The Game track uses state
and reference replays and Classic uses original frame comparisons.

## Work list to finish the game — 2026-11-04 (handoff)

This is the order to work the burndown's items in, by burndown id; the live
counts are in [BURNDOWN.md](BURNDOWN.md) (0 of 243 open on 24 November; `release-1`
closed the playthrough checkpoints — `--trade`, `--save-roundtrip` and `--jump`
plus `verify-playthrough.py`'s nine-stage chain — `v0.48-14`
closed the whole-frame scene oracle and `persist-4` the legacy commander replay,
and `gameplay-8`, `parity-20` and `parity-22` closed the combat/day/missile
replay verifiers; the agent
items `gameplay-6`, `gameplay-7` and `gameplay-18` are done as accepted at about
90% on F659's launch tables, F233's arrival event, F770's trading approach, the
missile rows 0x13-0x1b and the ground-collision removal, with the recorded gaps in
[ACCEPTED_PARITY.md](ACCEPTED_PARITY.md); parity-24 is done on the one shared new-game initializer and a cache-backed fresh-boot/post-death-restart comparison; ui-11 is done as accepted at about 90% on F1680's far/dot classes running for every object class and the planet approach's executed-original checks; ui-12 and parity-08 are closed as accepted at about 90% on a frame-by-frame trace of the executed original's F942 death; content-2 is done on an executed-original oracle; world-8, ui-5, ui-10, ui-29 and gameplay-41 are accepted at about 90% and ui-30 is done; parity-01, -04, -05, -07, -09, -10, -11, -12, -13, -14, -17 and -18 are closed as accepted at about 90 to 95%, and parity-15 at about 80%, see [ACCEPTED_PARITY.md](ACCEPTED_PARITY.md)). Every item is ported from
`jjffe-master/ffe.asm` and checked against the executed original where a
reference tool can do so; nothing is guessed. Each change updates the burndown
in the same commit and records any approved difference in `docs/DEVIATIONS.md`.
The parity blockers and `flight-1` are the critical path: most of sections 2–4
depends on them.

### 1. Parity blockers (the 2026-10-26 audit; P0 first)

- **P0** `parity-01` fuel, shield and hull repair event units (F938/F929; 17 November: the
  reserve matches the executed original exactly for the first 300 frames of a
  flight and within 6 units of about 5,600 to frame 899;
  F152's whole-second events now come from the day fraction and its marker,
  checked against the executed original's `DATA_008808` on 10 November, and
  no F938 while the autopilot flies (F1007's mode 2); the `+0x10e` countdown
  and a full reserve trace remain);
  (`parity-02`, refuelling and fuel bookkeeping, and `parity-03`, the drive
  state, are done: 4 November);
  `parity-04` the complete takeoff handler (F1013) and same-frame mode order
  (10 November: a frame-by-frame replay of the original's takeoff matches every
  mode change and set speed; the pad seat now matches; F1014's atmospheric
  drag and heating now run in flight, so free-flight speed tracks within 0.2%;
  the climb now matches the original's velocity to 1 unit (17 November: the
  pad world integrates in the planet's rotating frame) and the clear is 1 frame
  early; free flight after the clear also matches to 1 unit through frame 899
  (a per-frame F1520 on the ship's rows had cut the thrust 1.5%), see
  KNOWN_ISSUES);
  `parity-05` the original's frame and contact rules instead of the launch
  altitude stand-in and collision grace; (`parity-06`, set-speed controls,
  input gates and speed persistence, is done: 10 November; F1121's held-key
  scale and the 50 Hz key accumulators are ported, the Classic slow-down key is
  right Shift only, F1037 is off while the F2 status screen or the map shows,
  and a recorded-input chain test covers the release gate, reverse speed,
  engines off and reload); `parity-07` the original Hope start
  and cockpit view (11 November: the start frame's HUD text, message window
  and hull line now match the executed original pixel for pixel; an orange
  tower's depth, one small object and the scanner contacts remain); `parity-08` (closed 20 November, accepted at about 90%: the explosion, pods and sparks match the executed original frame by frame).
- **P1** `parity-09` the original's docking contacts instead of the corridor
  tests and pushes; `parity-10` the indoor departure placement and moving
  lift/door state; `parity-11` the autopilot's world and target adapters,
  including F597's time-accelerated branch (it reads the approach words F669
  draws: without it a script attacker at 10000x holds a few hundred km off);
  `parity-12` one set of event clocks, time acceleration and physical
  integration; `parity-13` hyperjump start, transit, misjump and arrival;
  `parity-14` fuel-scooping gates and capacity; `parity-15` the NPC speed caps
  and simplified flight states (closed 2026-11-19 as accepted at about 80%: pirates
  and the police response follow the original; F702's system-wide placement and
  the other gaps are in ACCEPTED_PARITY.md; `parity-16`, F688's spawn, is done);
  `parity-17` the original's RNG state and call order everywhere (the new game
  matches the executed original up to the end of F688); `parity-18` saving and restoring the
  complete live state; `parity-20` market, shipyard, bulletin-board and daily
  mission events from the original's state; `parity-21` every explosion, debris
  and hit effect from the original's object states; `parity-22` missile, ECM,
  escape and legal outcomes in the full event loop; `parity-23` no silent fallback to the synthetic playable (completed 2026-11-15); `parity-24` the original's new-game setup
  after death and on every resume path (completed 2026-11-20).

### 2. Flight core

Closed 2026-11-18 as accepted at about 90% (see [ACCEPTED_PARITY.md](ACCEPTED_PARITY.md)): `flight-4`, `-6`, `-9`, `-10`,
`-11`, `-12`, `-13` and `-14`. Closed 2026-11-20: `flight-5`. Still open here: `flight-1` and `flight-8`.

`flight-1` the object system and agent records (`cstruct`/`gmstruct`; not
started, much depends on it); `flight-4` the fixed 50 Hz step, independent of
the frame rate; `flight-5` the original input processing and key accumulation
(the `bind.*` remap is applied; only an in-game remap screen is unported);
`flight-6` time acceleration and its RNG use; `flight-8` player
events and the state machine; `flight-9` entering and leaving a station's
rotating frame; `flight-10` docking rules and the station's position and
orientation; `flight-11` the launch sequence; `flight-12` scripted flight
scenarios traced against the reference; `flight-13` open-air starport takeoff;
`flight-14` the original autopilot in full (guidance, avoidance, rotating
frames, whole-route replay).

### 3. The galaxy and its systems

`world-4` galaxy generation and its RNG stream question; `world-5` planet
surfaces and the landing approach; `world-6` starport services and events;
`world-7` hyperspace jumps and arrival; `world-8` the system and galaxy maps;
`world-9` lighting by the local star, day and night; `world-13` the model scale
and near-surface radius and frame transitions.

### 4. Gameplay systems

- `gameplay-41` mission scripts (`docs/notes/mission-scripts.md`): closed as
  accepted at about 90%. F493's military office prints and selects the
  scripts' rows: the station's offers are F326's native entries, signal 3
  inserts the scripts' F346 rows (mission 23's 0xa9b9 opens mission 25), and
  the list draws the menu's order through `station::menu_rows`. Recorded
  gaps: F669's event 0x17 is raised after the run rather than mid-statement,
  the attackers' time-accelerated closing is `parity-11`, and no executed
  in-play mission chain is traced (`release-1` records it as its accepted gap;
  the station side is `--daily` plus the ten `mission_scripts` cases).
- `gameplay-42` fix the source-verified broken missions (always-on bug fixes):
  5 of the audited missions are corrected (78, 55, 20, 83, 81; EXP-030..034) and
  5 orphaned ones are documented not-patched (37/38/39, 90/91 have no starter in
  the source; EXP-035). See `docs/notes/broken-missions.md`.
- Closed 2026-11-20 as accepted at about 90%: `gameplay-6` agents and AI
  behaviour (F770's trading approach and the missile rows 0x13-0x1b), `gameplay-7`
  police, pirates and encounters (F659's launch tables, F233's arrival event and
  the ground-collision removal) and `gameplay-18` NPC flight/combat AI, ship fire,
  police/wanted and jump persistence. The recorded gaps (F702's orbit, F597's
  accelerated placement, F589/F671, EXP-023's beam box, F770's taunt text and
  F941's missile selection) are in [ACCEPTED_PARITY.md](ACCEPTED_PARITY.md).
- In progress: `gameplay-1` market, economy, pricing and cargo; `gameplay-2`
  shipyard and outfitting; `gameplay-3` fuel, damage and repair; `gameplay-34`
  subsystem and crash damage and the repair rows; `gameplay-5` missiles and
  countermeasures; `gameplay-9` bulletin board and missions;
  `gameplay-10` journal and mission state; `gameplay-11` shuttles and station
  traffic; `gameplay-12` communications and messages; `gameplay-13` and
  `gameplay-25` death, the escape capsule and GAME OVER; `gameplay-38` the module
  clock and event routing; `gameplay-39` the autopilot at run time;
  `gameplay-dust` space-dust spawning,
  lifetime and flight drawing.
- Not started: `gameplay-8` a combat scenario replayed against the reference.

### 5. Saves

15 November: ten scoped native-resume follow-ups (`persist-6` through
`persist-15`) are complete. Career, live missiles and chaff, effects, ship
autopilot targets, launch permission, warning latches, clearance fractions,
mission-target spawn records and initialized empty NPC populations now survive
save/load. Twelve cache-backed regression cases cover these fixes, migration
and invalid inputs. PAR-18's broader live-state parity remains open.
A second ten follow-ups (`persist-16` through `persist-25`) preserve transit,
jump pursuers, changed shipyard and bulletin state, unvisited boards, BBS job
memory, daily events and mission deadlines, autopilot retry history and frame
selection. Twelve more cache-backed scenarios verify these behaviors and migration.


`persist-1` read-only import of original commander saves; `persist-4` imported
saves replayed against the reference (done: the DOS player block and object
array are byte-exact and 14 commander fields match; map in
`docs/notes/persistence.md` 1.2.1).

### 6. Content import

The ship, equipment and economy tables in `content-1` are imported and
verified (2026-11-16). `content-2` the remaining message strings, journal and
fonts; `content-4` an inventory of the optional CD media; `content-5` the
import checked on CD 1.01 as well as floppy 1.02. Mission-cargo behavior stays
under its existing gameplay items.

### 7. Original drawing (Classic)

`v0.48-7` the planet surface's quad emission (F1834/F1835); `v0.48-8` the
planet-distance surface quad oracle (done, accepted at about 90%: 54 of 54 saved
planet-scale and near-pad cases match, 21,380 triangles and 54 queues; the full
512-case table stays Linux-only); `v0.48-14` the whole-frame pixel check against
the original (done: a fixed station and three ships, 63,946 of 64,000 pixels
exact, the 54 residuals in `ACCEPTED_PARITY.md`); `v0.48-16` sprites, particles
and circles; `v0.48-17` the last 17 of
466 models and the CD 1.01 census; `ui-5` menus and text on every screen;
`ui-7` the intro and new-game flow; `ui-10` palette effects and screen
transitions (done, accepted at about 90%: one palette build per flight frame); `ui-11` far/near transitions, clipping and planet approach (done,
accepted at about 90%: F1680's far/dot classes and F1686's cull for every object class, and the approach's executed-original checks);
`ui-12` hit particles, explosions and debris (done, accepted at about 90%: `docs/notes/death.md`).

### 8. Classic split cleanup

Done (`classic-split-2`, 10 November): the viewer no longer records the
Modern engine's side data, and `ffe-core`'s terrain is back to its pre-split
code. The executor still records view-space points for some primitives; F1714's
near-plane outline uses them.

### 9. Release

`release-1` full-playthrough checkpoints against the reference is done (accepted
at about 90%): `tools/reference/verify-playthrough.py` runs one checkpoint per
stage — import/intro, launch/fly/dock, trade, save/load, jump, combat,
missions/BBS and render — each against the executed original with its port test,
and all nine pass. The new modes are `--trade` (F555/F556 with every refusal),
`--save-roundtrip` (F214/F170/F174 write, F176/F171 restore) and `--jump`
(F908/F901/F909 Gateway→Titican). The accepted gaps are no executed in-play
mission chain, F909's arrival draw ledger, the map destination cursor and a
JJFFE save's string-section commander name; see
[ACCEPTED_PARITY.md](ACCEPTED_PARITY.md). `release-4`
packaging and a distribution audit; `release-5` importing a player's data on a
clean machine; `release-6` the Rust code licence (MIT, chosen 2026-11-20); `release-7`
the owner's like-for-like sign-off.

## Where to look

- [docs/README.md](README.md) is the map of the documentation.
- [PORT_LOG.md](PORT_LOG.md) has every change with its evidence, newest first.
- [DEVIATIONS.md](DEVIATIONS.md) lists the approved differences from the source.
- [audits/gameplay-parity-2026-10-26.md](audits/gameplay-parity-2026-10-26.md)
  and its JSON are the parity items (PAR-nn) with their progress.
- `docs/notes/` holds one note per subsystem; `docs/evidence/` holds the
  captures and runs that back them.
- The status entries from October (incident fixes, the model-engine milestone
  record, the v0.48 plan) are kept in
  [archive/STATUS-history-2026-10.md](archive/STATUS-history-2026-10.md).
