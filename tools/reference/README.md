# Local reference oracles

`tools/reference/` holds the differential tests that compare this port with the
executed JJFFE 2.8a7 reference and with values pinned from the original's source
(`jjffe-master/ffe.asm`, `ffedat.asm`) or from the supplied game files.

Nothing here is public CI. The reference binaries embed the player's game data
and must not be distributed. They are built locally and are ignored by git.

## Build and run

```powershell
cargo build --release --locked -p ffe-tools -p ffe-rs
.\target\release\ffe-tools.exe build-data --game .\game --out .\data.bin
.\tools\reference\build-smoke.ps1 -Nasm .\.cache\tools\nasm-3.01\nasm.exe
python .\tools\reference\verify-all.py
```

`build-smoke.ps1` produces `.cache/jjffe-reference/jjffe-smoke.exe` (the plain
new-game adapter) and `jjffe-commands.exe` (the same game with primitive-emission
hooks). It needs NASM and the MSVC x86 tools. Rebuild both after changing anything
under `tools/reference/`; trace hashes that include original pointers change with
the rebuild (the child-model trace's event 26 is the documented example), so read
those oracles by their counts as well as their hash.

**Build `data.bin` from the same checkout as the code.** The cache format moves
between streams; a cache built by another branch fails the older tests with
`unknown variant` before any comparison runs.

`verify-all.py` checks every input first and then runs each oracle, printing one
line each:

```
PASS smoke (0.3s)
SKIP world-linux (60.1s)
     SKIP sysgen: ...
54 passed, 1 skipped, 0 failed of 55 oracles in 708.1s
```

Exit status is 0 when every oracle passes or is environment-gated, 1 on a real
failure and 2 when an input is missing. `--only NAME` runs a subset, `--list`
prints the names, and `--timeout N` bounds one oracle. Child scripts run with
UTF-8 I/O; paths outside the console codepage no longer break the report.

## What the oracles prove

Each row is one `verify-all.py` entry. A "differential" oracle compares the Rust
port with a trace produced by executing the original; a "pinned" oracle checks
values taken from a cited label in `ffe.asm`/`ffedat.asm` or from the game's
assets, with no reference run. Current result is the 2026-10-25 run in this
checkout.

### Core data, primitives and the model pipeline

| Oracle (script) | What it proves | Result |
|---|---|---|
| `smoke` (`verify-smoke.py`) | the reference adapter starts, presents frames and produces a bounded trace | PASS |
| `core` (`verify-core.py`) | F922's RNG, the timer and the game clock (1,536 + 60 + 238 cases) | PASS |
| `assets` (`verify-assets.py`) | floppy 1.02 / CD 1.01 profile hashes and every imported asset against `game/` | PASS |
| `font` (`verify-font.py`) | F1399 glyphs and the blit helper | PASS |
| `models` (`verify-models.py`) | F1479's vertex interpreter over the model table | PASS |
| `normals` (`verify-normals.py`) | F1480/F1754 normal records | PASS |
| `transforms` (`verify-transforms.py`) | F1520/F1519/F1552 rotations and the transform helpers | PASS |
| `rotation-operands` (`verify-rotation-operands.py`) | the rotation operand decoder | PASS |
| `visibility` (`verify-visibility.py`) | F1687/F1688's visibility tests | PASS |
| `depth` (`verify-depth.py`) | F1777's depth key | PASS |
| `instance` (`verify-instance.py`) | F1479 projection and the live vertex slots | PASS |
| `jitter` (`verify-jitter.py`) | F1476's jitter streams | PASS |
| `order` (`verify-order.py`) | F1696/F1777/F1780/F1788's composed order | PASS |
| `commands` (`verify-commands.py`) | the opcode decoder and the command helpers | PASS |
| `execution` (`verify-execution.py`) | the bounded executor | PASS |
| `point-line` (`verify-point-line.py`) | F1698/F1702 operand events | PASS |
| `text-submodel` (`verify-text-submodel.py`) | F1747 text and F1749/F1750 child-model calls | PASS |
| `colour` (`verify-colour.py`) | F1762's colour records | PASS |
| `base-transform` (`verify-base-transform.py`) | F1741/F1739 base transforms | PASS |
| `polygon` (`verify-polygon.py`) | opcode 5's polygon keys and the raster helpers | PASS |
| `textures` (`verify-textures.py`) | the texture queue/rasterisers | PASS |
| `lines` (`verify-lines.py`) | the line rasteriser | PASS |
| `fills` (`verify-fills.py`) | the fill/span rasterisers | PASS |
| `thrust` (`verify-thrust.py`) | F1121's thrust limits and acceleration | PASS |
| `physics` (`verify-physics.py`) | F1041/F603's gravity and the player step | PASS |
| `station` (`verify-station.py`) | models 74/75 through the whole F1772 executor | PASS |
| `station-queue` (`verify-station-queue.py`) | F1708/F1712/F1709/F1713/F1714's face queue | PASS |
| `station-models` (`verify-station-models.py`) | all 466 table slots the original completes, in command order | PASS |
| `child-models` (`verify-child-models.py`) | every model's child runs (1,330 child runs) | PASS |
| `child-models (depth 0x50000)` | the same with a model deep enough for far thruster vertices | PASS |
| `vertex-programs` (`verify-vertex-programs.py`) | every comparable vertex program, screen and view position | PASS |
| `thrusters` (`verify-thrusters.py`) | F1738/F1704/F1805's thruster glows | PASS |
| `animation` (`verify-animation.py`) | F866/F1675 spin, orbits and F1537/F1680/F1683 set-ups | PASS |

### World, flight and the playable

| Oracle (script) | What it proves | Result |
|---|---|---|
| `world` (`verify-world.py --scenarios worldgen terrain-mid terrain planet names galaxy`) | F880's generated systems and physics objects at synthetic clocks, the nineteen terrain midpoint routines, F1876's distance query, F576's planet contact, F791's names and F868-F871's galaxy tables | PASS (Windows worldgen fix, below) |
| `world-linux` (`verify-world.py --scenarios sysgen surface-draw`) | F875/F878's 4,160 systems and F1876's draw-mode emitter queue | SKIP on Windows: `--sysgen` needs a working SIGALRM watchdog and `--surface-draw` needs POSIX `fork`/`open_memstream`; both run on Linux |
| `surface-near` (`verify-surface-near.py`) | four near-pad F1876 draw-mode records regenerated with `--surface-near` and the port's emitter/queue comparison | PASS |
| `surface-planet` (`verify-surface-planet.py`) | the saved planet-scale `--surface-draw` table (56 cases, 47 beyond 1.4 radii) through the port's emitter/queue comparison; verifies only, since the full table needs Linux | PASS |
| `traffic` (`verify-traffic.py`) | F650-F655, F657 and F867's station traffic record | PASS |
| `pads` (`verify-pads.py`) | F650's pad offset, including the open-air branch (8,192 cases) | PASS |
| `market` (`verify-market.py`) | F619/F620's price and stock formulas | PASS |
| `lift` (`verify-lift.py`) | the docking/launch lift scenes | PASS |
| `intro` (`verify-intro.py`) | F961/F962, the F829/F998/F994 start and the new game | PASS |
| `intro-palette` (`--intro-live <frames>`) | the real boot/intro loop (F961/F962/F1330/F1331) with no start key: each frame's tics, the upper palette it builds, and the final 320x200 frame + palette (`game-frame.bin`) | local evidence, no verifier |
| `collide` (`verify-collide.py`) | F569/F570/F575/F578's contacts (4,096 cases) | PASS |
| `world-tick` (`verify-world-tick.py`) | a whole F1041 player tick in a seven-body world (4,096 cases, 584 contacts) | PASS |
| `sectors` (`verify-sectors.py`) | F868's sector lookup and grid | PASS |
| `hyperspace` (`verify-hyperspace.py`) | F830/F840's distances, fuel and transfer time | PASS |
| `system-names` (`verify-system-names.py`) | the imported system name strings | PASS |
| `map-stars` (`verify-map-stars.py`) | F1769's star light points | PASS |
| `dots` (`verify-dots.py`) | F1699-F1701/F1800/F1801's map dots | PASS |
| `galaxy-map` (`verify-galaxy-map.py`) | a whole map frame from `--game-frame` against the port's frame | PASS |
| `system-map` (`verify-system-map.py`) | the 3D system map (module 5) from `--game-frame 560 0x3d 0x40` with `FFE_MAP_DUMP`: focus slot, zoom, camera and every body's screen position exact, 86% of overlay pixels shared | PASS |
| `surface` (`verify-surface.py`) | the landed frame's haze sky and terrain coverage from the executed original (`sky_matches: true`, `header_close` 0.9934) plus `planet_view`'s landing sequence | PASS |
| `scene-frame` (`verify-frame.py`) | a fixed station and three ships through the executed original's F1537/F1680/F1772, F1330's palette and F1789's depth-sorted flush: 63,946 of 64,000 pixels exact, the 54 residuals the unported F1747 name-board text and one station-edge depth tie | PASS |
| `legacy-save` (`verify-legacy-save.py`) | the supplied DOS-era `game/FIRSTENC._01` through the original F70/F66/F171 and through `saves::legacy_import`: the object array and the DOS player block byte for byte, plus 14 commander fields (name, credits, cargo/hold, fuel, clock, the two RNG words, the ship) | PASS |

### The 2026-10-25 wave: combat, NPCs, HUD, takeoff and preferences

| Oracle (script) | What it proves | Result |
|---|---|---|
| `combat` (`verify-combat.py`) | F998's start-type 1 loadout from the executed new game (`--state-combat`: model 23, one `0x88` mount, two `0x82` missiles, 1 t energy, 1000 CR); F931's hitscan calls (F802/F930/F953/F939, no F941/F773/F1533); F1111's bitmap 97 at (129,48) in laser views only, F1112's bitmap 98, F270's bars, F1099/F1100's text and model 156's muzzle cone through `combat_view` | PASS |
| `ship-types` (`verify-ship-types.py`) | F635's nineteen `(count, model list)` records parsed from `ffedat.asm` `DATA_006199`/`DATA_006180..6198` against `SHIP_TYPES` (19 records, 237 models) | PASS |
| `npc` (`verify-npc.py`) | F635/F688 counts and roles, F922, F953/F942 damage/death, F1536's model-157 flash and the traffic view (`npc_view`) | PASS |
| `legal` (`--legal-probe`, local evidence) | firing once at the **default** Gateway/Hope/Old Blackelk pad (`--legal-probe 166 0x39`) and at Ross 154: `DATA_008860` bit 2, `DATA_008646` bit 0, `DATA_009095 += 6000`, unchanged cash and F236's exact `$600` message (`docs/evidence/legal/original-legal-probe-default.jsonl`) | local evidence |
| `dashboard` (`verify-dashboard.py`) | F271's ten-entry message log in the 195..320 right-hand window, F273's scroll, F1099's HUD positions and F2/record 0x0c's lamps and engines-off page | PASS |
| `takeoff` (`verify-takeoff.py`) | F1013's `0x26` hand-off above 200,000 units (or `DATA_008918 != 0`), F1031's `0x20`/station-bit decisions and the observed pad-to-flight modes `[0x24, 0x26] -> 0x00` | PASS |
| `flight` (`verify-flight.py`) | the scripted new game from boot: the idle `0x30` pad, F1056's launch action and F1013/F1031's `0x30 -> 0x24 -> 0x26 -> 0x00` frame continuity, and the planet `+0x56` retained through free flight; the port's idle/takeoff scenarios match the same mode machine and planet model | PASS |
| `combat-replay` (`verify-combat-replay.py`) | `--combat` against the Hope pirate group: the original's F750/F754 AI, F931, hostile state changes and F939/F942 effects; the leader's F750 guidance/physics replay is 398/399 thrust, 399/399 caps and positions, 396/399 velocities | PASS |
| `daily-replay` (`verify-daily-replay.py`) | `--daily`: the first day boundary's 125 F922/F1530 draws (F633's jobs/news, 99 market steps, F621's board, F634's queue) and `Navigator::tick_days` ending on the same pair; the per-station market streams are EXP-006 | PASS |
| `missile-replay` (`verify-missile-replay.py`) | `--missile-encounter hit/ecm/naval/mine/escape`: a model-6 hit and F719, standard-vs-naval ECM, the model-5 state-0x14 mine, F1098's Stowmaster, plus the port's F719 hit/timeout path | PASS |
| `trade` (`verify-trade.py`) | `--trade`: the real F555/F556 buy and sell; ten cases, all five `0x98e1..0x98e5` refusals and the success path against `market::Market::{buy,sell}` | PASS |
| `save-roundtrip` (`verify-save-roundtrip.py`) | `--save-roundtrip`: F214/F170/F174 write, a changed state, F176/F171 restore; the imported file's fields and 38 cargo words match the original's own before row | PASS |
| `jump` (`verify-jump.py`) | `--jump`: F908/F901/F909 from Gateway to Titican (442 units, 189,322 time units, 1 t of fuel); `Navigator::hyperspace`'s plan, 2.89-day transit and arrival orbit | PASS |
| `prefs` (`verify-prefs.py`) | every page title, shape/space/sound/other/map label and music entry against `DATA_004963..67` (61 string checks) plus the six prefs unit tests (including the sound-row toggles and the captured space-colour ramp) | PASS |

## Scripted flight scenarios (`--flight`)

`--flight <scenario> <frames>` runs the real new game from boot with a scripted
Y/start-key sequence instead of an in-process probe. Three scenarios cover the
takeover report's flight items:

| Scenario | Script | What the trace shows |
|---|---|---|
| `idle` | no game keys | the intro frames, then the new game's `0x30` pad: player index and planet identity unchanged, ship position constant and velocity zero |
| `takeoff` | Tab (action 9, launch) at frame 170; Return (set speed) and keypad 8 from frame 260 | `0x30 -> 0x24` (F1056) -> `0x26` (F1013 at altitude > 0x30d40 or set speed != 0) -> `0x00` (F1031's cleared region) |
| `planet` | the same, to frame 899 | the same hand-off and the planet `+0x56` still the player's parent after free flight |

Each frame from 140 emits one JSONL row. The bounded fields are the frame,
view mode, frame tics, player index and start type, `DATA_008867` (altitude) and
`DATA_008918` (set speed), the player planet (`+0x56`: index, model, parent and
64-bit position), the ship's 64-bit position and 32-bit velocity, and an FNV-1a
`state_hash` chained through `prev_hash` over exactly those integers. No frame is
pixel-dumped and no game data or memory is written; 900 frames produce 760 rows
per scenario. `verify-flight.py` runs each scenario twice (byte equality),
checks the hash chain and the documented mode/planet invariants, then runs
`crates/ffe-rs/tests/flight_reference.rs`, which also flies the port's idle pad
and `Navigator`/`OriginalFlight` takeoff and compares the mode machine, planet
model and released bay. Every trace field, the chain and an empty trace must fail
the Rust test.

## Whole-frame scene and legacy-save modes

`--scene-frame` (no arguments) draws a fixed scene — station model 74 and ship
models 31, 23 and 25 at fixed positions — through the real F1537 object loop,
F1680 set-up, F1772 model executor, F1330/F1331 palette and F1789 depth-sorted
flush. It writes `scene-frame.bin` (64000 indexed pixels then 768 palette bytes)
and one JSONL row with the exact camera matrix/position, each object's matrix,
position, scale and 64 global words, the tics, the light and the 8 shade
colours. `verify-frame.py` runs the port's equivalent frame and compares every
pixel in RGB; the accepted residual is under 64 pixels (the unported F1747 name
board text and one station-edge depth tie).

`--legacy-save <path>` loads a DOS-era commander file the way F176 does without
the dialogs: F70 reads F69's loose globals, F66 reads the F65 stream and the
remaining bytes go through F171 (magic, F169 checksum, F166 zero-run expansion)
into `DATA_009133` and `DATA_008804`. One JSONL row carries the container
offsets, the commander fields and the raw object array (0x984a) and player
block (the DOS section's 0x5535 bytes) as hex. `verify-legacy-save.py` compares
that row with `saves::legacy_import`; `docs/notes/persistence.md` records the
field map. `game/FIRSTENC._01` is read only and never copied into the tree.

## Replay modes (`--combat`, `--daily`, `--missile-encounter`)

These three modes write one JSONL row per frame (or per RNG draw) and are
compared by Rust tests plus the `verify-*-replay.py` scripts. They are local
oracles: the reference binaries are not distributed.

| Mode | What it runs | Comparison |
|---|---|---|
| `--combat <frames>` (`--start-key=0x39`) | the default Hope new game, the player on the pirate leader's forward axis (20 km behind by default, 2 km in the verifier; `FFE_COMBAT_OFFSET` to change), the original AI and F931 firing from frame 520 | `crates/ffe-core/tests/combat_replay.rs`: hostile states/fire/damage/effects; leader F750 guidance and F727 physics |
| `--daily <frames>` (`--start-key=0x02`) | the Ross 154 pad at 10000x from frame 200; every F922/F1530 draw and the first day boundary | `crates/ffe-rs/tests/daily_reference.rs`: 125-draw ledger, pair chain and `Navigator::tick_days` |
| `--missile-encounter <hit\|ecm\|naval\|mine\|escape> <frames>` | the Hope launch; F1096/F941 at frame 500, F935's E.C.M. at 502/530, F1098 for `escape` | `crates/ffe-rs/tests/missile_encounter.rs`: missile hit/F719, standard-vs-naval ECM, mine state and escape craft |

`FFE_RNG_TRACE` also works with these modes. `--combat` and
`--missile-encounter` default to the Hope start only when `--start-key=0x39` is
passed; the test traces use that key.

## Playthrough checkpoints (`--trade`, `--save-roundtrip`, `--jump`)

`release-1` adds one checkpoint per stage that had no executed-original trace.
They run the real docked or free-flight game and then the original's own
functions on fixed inputs:

| Mode | What it runs | Comparison |
|---|---|---|
| `--trade <frames>` (`--start-key=0x04`, act at frame 200) | the real Lave docked game; F555 buys and F556 sells tonnes in `DATA_008804` with fixed market entries, cash, hold and stock; ten cases cover the success path and F557's `0x98e1..0x98e5` refusals | `crates/ffe-rs/tests/market_trade.rs`: `Market::{buy,sell}` cash, cargo, used, stock and every refusal code |
| `--save-roundtrip <frames>` (`0x04`, act at frame 240) | F214 (zero unused objects), F170 (mangle), F174 (F69 + F65 + body) write a commander file; the fields are changed; F176 (F70 + F66 + body) and F171 must restore them | `crates/ffe-rs/tests/save_roundtrip.rs`: the original's own restore check plus `saves::legacy_import` on the written file |
| `--jump <frames>` (`--start-key=0x39`, Tab, act at frame 400 in free flight) | F908 with Titican selected (code `0x12a53716`, 4.42 ly) and the F34/F35 dialog answered yes: F901's plan, mode 4's timeout and F909's arrival; F1530 advances but never returns a random mis-jump | `crates/ffe-rs/tests/jump_reference.rs`: `Navigator::hyperspace`'s distance, jump time, fuel, days, transit and 7.4..11.3 AU arrival |

`python tools/reference/verify-playthrough.py --cache data.bin` chains the stage
oracles (import, intro, launch/fly/dock, trade, save/load, jump, combat,
missions/BBS, render) into one gate; `--stages` runs a subset and `--list` prints
the stage table. The accepted gaps (no executed in-play mission chain, F909's
arrival draw ledger, the string-section commander name of a JJFFE-written save)
are in `docs/ACCEPTED_PARITY.md`.

## The Windows `--worldgen` fix

`--worldgen` writes each generated object's 68 name bytes. F877/F878 build part
of the name from an uninitialised byte, so the same case can read
`Testname C,D1\xe7` in one run and `...\xf3` in the next under Windows ASLR.
`verify-world.py` now compares the two runs after masking the object-name fields
`0x26..0x3f` (system records) and `0x124..0x138` (physics objects), which are
exactly the ranges `world_reference.rs` treats as loose/uncertain through
`unspecified_name()`. The **raw first run** is still what the Rust oracle
compares, so no compared byte was dropped: the normalisation is only the
repeatability decision. All other scenarios still compare byte for byte.

The split `world`/`world-linux` entries make the platform gates explicit:
`--sysgen` can hang in F878 on Windows because `win-compat.h` stubs the SIGALRM
watchdog (the script times out after 60 s and reports SKIP); `--surface-draw`
cannot run at all because its per-case fault catcher needs `fork` and
`open_memstream`. Both remain required on Linux.

## Hashes

No earlier oracle was re-run for this wave, so the recorded reference hashes
that those oracles pin are unchanged. The reference **was** rebuilt for
`--flight` (the adapter and `build-manifest.json` change); the new trace has no
original pointers, so its hash below is stable across rebuilds. The values
checked here:

| Oracle | Trace SHA-256 | Note |
|---|---|---|
| `world-tick` | `4258a0b2756c09c53c102a805a365cce9e6b7c51770894d801875603fa43d6ac` | unchanged; cache v26 built from this checkout |
| `flight` | `29eebd424848c391b6f9f6a1d5f83b7072949d971f1b3e0173228c0780abdd70` | new scripted trace (3 x 760 rows); no pointers, so it is stable across reference rebuilds |
| `ship-types` | `ae421da6e844b377033ca95d094458ea0894575fc0a0f1ca7acc350e8391d2cd` | new source-pinned trace; the table fix below changes no other hash |
| `worldgen` | varies with ASLR | the raw trace keeps the original's uninitialised name byte; read it by its counts (1,024 cases), not this hash |

`ship-types` found and fixed one transcription error: table slot `0x3` had
`DATA_006181`'s list (`0x1d, 0x1e, ...`) instead of `DATA_006183`'s
(`0x17, 0x19, 0x38, 0x38, 0x3a, 0x3a, 0x41, 0x41`). Slots 2/4/5 had the right
data but wrong Federal/Imperial labels. The playable's Ross 154 trader uses
allegiance 2 (slot 5), so gameplay is unchanged; the oracle now pins all
nineteen records.

## Gaps

- `--sysgen` and `--surface-draw` are not exercised on Windows (see above); the
  Linux build (`build-linux.sh`) still runs them.
- F688's spawner is not executed by a reference probe: the port's placement and
  minimum-one-trader/pirate rules are deliberate deviations, so the oracle
  covers the F635 table and the view/unit expectations rather than F688's exact
  count sequence. F635's population < 4 trader scaling (`DATA_008991`, F635 at
  `ffe.asm:43597`) is also not wired into `model_for`; the table oracle pins the
  data, not that branch.
- HUD, takeoff, NPC and preference expectations are pinned from the asm/cache,
  not from a whole-frame original capture. The 3D scene frame has a
  pixel-level oracle (`scene-frame`); the cockpit/HUD and message pages do not
  yet have one. `galaxy-map` and `scene-frame` are the frame-level comparisons
  today.
- The child-model trace hash moves with every reference rebuild (event 26's
  uninitialised base handedness); read that oracle by its counts (440 compared,
  0 mismatched).

## Test hooks for the reserve, firing, acceleration and descent replays

`--flight` runs also take these environment variables (stderr and stdout stay as above):

| Variable | Effect |
|---|---|
| `FFE_RESERVE_DUMP=1` | per frame, on stderr: the reserve `+0x11e`, the laser's refire `+0xe8` and heat `+0xea`, the shield, hull and hull-heat words, the fuel units, F152's marker and the RNG pair |
| `FFE_ROWS_DUMP=1` | per frame, on stderr: the player's parent, its rotating flag, position in the parent's frame, velocity, rows, thrust words and limits |
| `FFE_FIRE_FROM` / `FFE_FIRE_TO` | the real F931 on the front mount at the end of every frame in the range (it refuses while the laser counts down, is hot or the reserve is empty) |
| `FFE_ACCEL_AT=<frame>:<step>,...` | F208's writes at the end of that frame: step 0 is 1x, 1 is 10x, 2 is 100x, 3 is 1000x, 4 is 10000x; steps 1 to 4 pull F152's marker back one event |
| `FFE_RNG_TRACE=<file>` | with `--flight`: one row per F922 draw and per F1530 draw (`frame seq before_a before_b limit result caller`; limit 0 is F1530) |
| `FFE_ENGINES_OFF_AT`, `FFE_LIMITS_CUT_AT` | the engines off, and the six thrust limits halved, at the end of that frame |
| `FFE_PLAYER_TO_PIRATE=<frame>[,<offset>]` | with `--flight` (use `--start-key=0x39`, the Hope default start, which has the nine-ship pirate group): at that frame the player (not the NPC) is put `<offset>` units (default 20,000,000) along z from the first state-4 type-0xfb ship, in its frame and with its velocity, so the original's own AI runs; then one row per pirate-group ship per frame (`ptp`, state, target, velocity, thrust words, position relative to the player, hull) |
| `FFE_PTP_RAW=1` | with the above: one row per frame (`raw`) with the leader's, its target's and the player's whole 0x152-byte object records as hex (replayed by `crates/ffe-core/tests/npc_pirate_replay.rs`; local only, never commit it) |
| `FFE_POLICE_TRACE=1` | with `--legal-probe` or `--flight`: every frame from 160, F236's offended object (`DATA_008804+0x50dc`) with `+0xc9` mode, `+0xca` current, `+0xd8` timeout, `+0xdc` police left, `+0xde` clock, `+0x14c` flags and `+0xce` face, the chase and wanted bytes, the player's shield and hull words (`pshield`, `phull`), and every police ship (state, model, position relative to the starport); with `FFE_FIRE_FROM=200 FFE_FIRE_TO=200` (the offence) and `FFE_RNG_TRACE`; replayed by `crates/ffe-core/tests/police_replay.rs` (`--start-key=0x02` Hope, `0x04` an orbital station) |
| `FFE_DEATH_OBJECTS=1` | with `--death-probe <frames>` or `--flight` + `FFE_DEATH_FLIGHT`: a `{"death_hit":..}` row (the destroyed ship's model, `+0x116`, position, velocity words, rows, parent, the parent's `+0xa4` mass, F603 fraction words, the occupied object slots and the shared pair) and then one `{"fx":..}` row per frame from right after F953 with every model 0x99..0x9e object that holds a slot flag: slot, model, control, AI state, `+0x7e`, `+0xa4`, `+0x9e`, `+0x14e`, position, velocity words and rows. Replayed by `crates/ffe-core/tests/death_replay.rs` (`FFE_DEATH_TRACE_FILE=<stdout capture>`, with `rng.txt` beside it) |
| `FFE_PAIR=<a>,<b>` (hex) | with `--death-probe`/`FFE_DEATH_FLIGHT`: the shared random pair set just before F953, so F942's draws are reproducible (also used by `FFE_EXPAND`) |
| `FFE_DEATH_AT=<frame>` / `FFE_START_KEY=<scancode>` | with `--death-probe`: the frame of the hit (default 250) and the intro start key (0x02 Ross 154 pad, 0x04 Lave orbital station, 0x39 Hope) |
| `FFE_DEATH_FLIGHT=<frame>` | with `--flight takeoff|planet <frames>`: F953 destroys the moving player at that frame (the death probe's rows, in flight); `FFE_RNG_TRACE` also works with `--death-probe` |
| `FFE_FLIGHT_SHIPS=1` | with `--flight`: every 25th frame from 160, one row per live ship object (model, AI state, type, parent, velocity, thrust words, position, target) |

`--flight paddock` (use `--start-key=0x02`) is the planet-pad counterpart of `dock`: the Tab launch, Return and keypad 8 held for
frames 260 to 299, and at frame 770 the starport the ship left becomes the navigation target and F1020 engages the autopilot,
which flies the descent to the pad (mode 2 until frame 15,704). The Rust replays that read these traces are in
`crates/ffe-rs/tests/takeoff_replay.rs` (`FFE_FIRE_TRACE`, `FFE_LONG_TRACE`, `FFE_PAD_POSES` with `FFE_AUTOPILOT_AT`) and
`crates/ffe-rs/tests/event_replay.rs` (`FFE_ACCEL_TRACE` with `FFE_ACCEL_AT`).

## Firing in the real game (`--game-frame`)

With `--game-frame <frames> 0xff` (no key), two environment variables script
the real game: `FFE_TAKEOFF=1` sends the launch key at frame 170 and holds
Return/keypad 8 from frame 260 (as `--flight takeoff`), and `FFE_FIRE_AT=<frame>`
calls the real F931 on the player's front mount at the end of that frame and
prints `+0x151`, the model and its statistics pointer to stderr. Use
`--start-key=0x02` (Ross 154/Eagle) or `0x39` (Hope/Saker). The final frame is
`game-frame.bin`; compare a run with and without `FFE_FIRE_AT`
(`docs/notes/combat.md` §6.2).

**`FFE_FIRE_AT` does not show the beam.** It fires at the end of a frame, and
JJFFE's "Laser invulnerability" loop at the end of F149 (`ffe.asm:15045`) clears
the draw flag (`+0x151` bit 1) before F1536 can call F1776, so a pulse shot is
never drawn (a beam laser has no refire delay and re-fires). To see the beam the
way a player's key shows it, hold Space through the input path, which runs after
F149 and before the draw (`docs/KNOWN_ISSUES.md`, "Laser beams"):

| Variable | Effect |
|---|---|
| `FFE_SPACE_FROM=<frame>` / `FFE_SPACE_TO=<frame>` | Space held for those frames (one frame if `TO` is absent); the beam is drawn on the frame after |
| `FFE_LASER_CODE=<code>` | all four laser mounts hold this equipment code from frame 600 (`0x88` pulse, `0xa0` 1MW beam, `0xb3` 20MW, `0xc6` mining, `0xcf` Thargoid) |
| `FFE_PRINT_E6=1` | frames 698-704: the shot's length word `+0xe6`, the `+0x151` flags, refire, heat, the render shift `+0x7e` and the model |

Example (Eagle, external view F9 is scancode `0x43`, rear F7 `0x41`): `FFE_TAKEOFF=1
FFE_SPACE_FROM=700 jjffe-smoke.exe --game-frame 702 0x02 0x43` against the same
run without `FFE_SPACE_FROM`; the pixels that differ are the beam.

## Mis-jump probe

`FFE_RNG_TRACE=<file>` with any `jjffe-smoke` state mode (for example
`--npc-trace 210 0x39`) writes one text row per F922 (bounded RNG) call:
`frame sequence state_a_before state_b_before limit result caller`. It needs the
`_ref_rnd_func` export (`exports.inc`), so rebuild with `build-smoke.ps1`. The
caller is the return address in the executable; F688's draws are the rows whose
callers fall in `0082BDxx`/`0082BExx`.

`jjffe-smoke --misjump <outcome index 0-15> <forced 0|1> <frames>` runs the new
game, launches, and at the first free-flight frame makes F908/F909's jump a
mis-jump with the chosen `DATA_007190` entry (random mis-jump for forced 0,
Left Alt held for 1). It prints one JSON row every 25 frames: mode, clock rate,
visual timeout, the player's drive and parent, the last message and the live
objects. Run it in a scratch folder; see `docs/notes/hyperspace.md`.

`FFE_POLICE_RAW=1` (with `FFE_POLICE_TRACE`) adds each police ship's whole 0x152-byte object record (`raw`) and the player's
(`praw`) as hex to every row, as `FFE_PTP_RAW` does for the pirate trace, plus `pp` (position relative to the player), `v`, `t`
(thrust words), `w` (the `+0x108` direction words and distance class), `tm`, `tg`, `f`, `cb`, `c8` and `fl`. Every row also carries the
player's `pshield` and `phull`. `ffe-core`'s `police_attack_cycle_follows_the_executed_original` replays the first ship from it.

`FFE_EXPAND=<hex text>` with `--flight idle <frames> --start-key=0x39` (and optionally `FFE_PAIR=<a>,<b>` in hex and `FFE_EXPAND_AT=<frame>`, default 200)
runs F77, the `[` macro expander, on the text at that frame after setting the shared random pair, and prints
`{"expand": "<hex of the expansion>", "pair": [a, b]}` (the pair left behind). `ffe-rs`'s `macro_oracle` test compares the port's
macros with it (`docs/notes/journal.md`). The harness's `exports.inc` gains `_ref_expand_brackets`, `_ref_pair_a` and `_ref_pair_b`, so
`ffe.asm` must be assembled again (`nasm -f win32 -p tools/reference/exports.inc`) before this variable works.
