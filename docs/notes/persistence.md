# Saves, commander files and configuration

Status: **native version-2 saves, quick-save/load, versioned config; legacy
commander files are recognised, decoded read-only and replayed field for field
against the executed original.** Implemented in
`crates/ffe-rs/src/saves.rs`, `navigation.rs` (`NavigatorSave`), `flight.rs` and
`settings.rs` on 2026-10-25; the DOS field map and the reference replay landed
with `persist-4` on 2026-11-21.

This note maps the original's commander I/O, defines the port's own format, and
records what is deliberately not claimed.

---

Native timer update (2026-11-15): version-2 saves preserve the daily servicing
marker and fuel-scoop cooldown. Older files default to an uninitialized marker
and zero cooldown; the first servicing call seeds the current day without
replaying past wear. Cooldowns must be finite and within the runtime's range.

## 1. The original's commander I/O (JJFFE 2.8a7)

### 1.1 The save/load path

| Routine | `ffe.asm` | Role |
|---|---|---|
| `F65` | 6898 | writes a commander: version dword, global block, table blobs, object array, object payloads; then `F67(handle, 0)` |
| `F66` | 6994 | the read side of `F65`; `F67(handle, 1)` rebuilds the object links |
| `F67` | 7100 | read/write the per-object payloads for the 150 `(0x96)` present-object records |
| `F69`/`F70` | 7227/7513 | write/read the loose global arrays (400-byte dwords, 20-byte records, 5 x 0x38 maps, mission/station lists) |
| `F170`/`F171` | 15526/15607 | compress zero runs, XOR-scramble, append/check the checksum; the body is the object array `DATA_009133` (0x984a bytes), every other dword from `DATA_008862` to `DATA_008653`, 0x40 bytes at `DATA_008663`, the player block at `DATA_008804` (0x556a) and strings at `DATA_008656` (0x51) |
| `F174`/`F175` | 15776/15825 | public save: a dialog, `DirMakeCmmdrName`, `F69` + `F65`, then `fwrite` the mangled remainder |
| `F176`/`F177` | 15889/15947 | public load: `fopen`, `F70` + `F66`, read the remaining bytes, `F171` before the caller sees them |
| `F178` | 16015 | remembers the last load's `FileInfo` |
| `F184` | 16246 | increments the two-character extension (`DATA_009119/9120`, `000`..`999`) and wraps |
| `F185` | 16264 | load selection: rejects a file outside `0x258..DATA_008670` bytes, calls `F176`, then `F171`, then clamps fields (bay, velocity/cargo words, rank negatives, the F228 "fudge" defaults) |
| `F210` | 17245 | save selection: `F214` zeroes unused object slots, `F170` mangles, `F174` writes, `F184` increments the extension |
| `F192`/`F194`/`F222` | 16526/16589/18280 | load/save dialogs and the load hotkey; `F220`/`F185` load the remembered file |

`F65`'s bytes, in order:

```
u32 version (must be 1)
0x47 dwords (284 bytes) from DATA_008548 = 32-byte block + 252-byte block
0x69 (105) table blobs: F65 writes each descriptor's +0x18 size from +0x14
0x96 (150) records x 0x24 bytes from DATA_008561
F67(0): for each present record, its payload (F69 writes them)
F175: the caller's F170 body (already compressed/scrambled, with a trailing CRC)
```

The file therefore is **not** a memory dump and is not portable to another build
without that build's table sizes and field map. The exact name is
`COMMANDR\<name>.<3 chars>` on the DOS/JJFFE side; `DirMakeCmmdrName` prepends the
directory and appends the extension.

### 1.2 What the source save contains

- `DATA_008548`/`DATA_008549` (284 bytes): the global commander header block,
  followed by the table blobs and the object records. The exact field map of the
  284-byte block is not established from the source alone (it is used through
  many separate labels, e.g. `DATA_008550..8560`); do not label offsets without
  a reference specimen.
- The `F170` body: the 0x984a-byte object array (position, velocity, orientation,
  ship model, equipment, shields/hull, cargo, AI state for every object), the
  player block (`DATA_008804`, 0x5535 bytes in the supplied DOS file), the
  galaxy/system tables, starport and mission/contract arrays, the two RNG words
  `DATA_008818/8820`, the game clock and the commander name/settings.
- The two RNG words, the object array and the starport records are why a loaded
  original game resumes mid-flight, including its current traffic.

### 1.2.1 The DOS player-block field map (`persist-4`)

The supplied `game/FIRSTENC._01` was loaded by the executed original's own
F70/F66/F171 (`jjffe-smoke --legacy-save`) and its raw `DATA_008804` block
compared with `saves::legacy_import`. The DOS block uses the source `gmstruct`
layout exactly (`jjffe-master/srctools/gmstruct.txt`); the earlier note that the
name sits at `+0x22E` was an artefact of the seven-dword scatter misparse
(`F170` copies `DATA_008622..8653`, which is 0x200 bytes = **0x40** every-other
dwords, not seven; the missing 0x100 bytes shifted the player block and its
text). The established offsets are:

| Field | Offset | `FIRSTENC._01` |
|---|---|---|
| clock tics / days | `+0x000` / `+0x004` | 1,014,497,784 / 1,187,064 |
| shared RNG pair (`+0x24` is 24-bit) | `+0x024` / `+0x028` | `0x38acd1` / `0x0fc168d5` |
| player object index | `+0x0c0` | 87 |
| credits | `+0x114` | 10,000 |
| cargo capacity / used | `+0x118` / `+0x120` | 7 / 1 |
| hydrogen / military fuel / radioactives | `+0x15a` / `+0x15c` / `+0x16e` | 1 / 0 / 0 |
| commander name / uppercase copy | `+0x1bc` / `+0x1d2` | Jameson / JAMESON |

The player's ship is the object array's entry `player_object` (87): model 31
(the Saker Mk III), registration `TY-198` at `+0x124`. The DOS player section is
0x5535 bytes, not JJFFE's 0x556a; F171's fixed 0x556a read takes 0x35 bytes of
the following string section, which is harmless for every field above and is
why the import decodes the file's 0x5535-byte section, then the 135-byte string
block (save name `FirstEnc`, extension `._01`, Jameson, Gateway, `Red giant
star`).

The reference replay is `tools/reference/verify-legacy-save.py`: it runs the
original twice, requires the object array and the player block byte for byte
and the 14 fields, then replays an altered field, an altered raw byte, a
truncated offset and an empty trace. `crates/ffe-rs/tests/legacy_save.rs` is
the field-for-field comparison.

### 1.3 When a save is allowed

`F185`/`F210` are reachable from the SysMenu in flight and while docked; the
source has no "docked only" gate in the save path. The only hard exclusion is the
death state: `F1104`/`F917` take over the loop, and the tombstone's
`F918(1)` returns to the front end, so no save dialog exists after `DATA_008870 =
0x2c`. The port mirrors that: flight, landed and docked saves are allowed; a save
after `Flight::death` or on the tombstone is refused.

The load dialog (`F192`) is available from the front end (`F220`/`F222`) and
from the SysMenu. `F185` rejects a file under 0x258 bytes or over `DATA_008670`
with "not a commander" and never modifies it.

---

## 2. The port-native format (version 2)

**It is not DOS/JJFFE compatible and is not claimed to be.** A legacy file is
recognised as non-JSON (a bounded "non-JSON ⇒ legacy" test plus F170 body
validation), not by a leading version dword — real DOS-era files put F69's loose
block first, so the version dword 1 sits at offset 0x40E. Legacy files are
imported read-only (`saves::legacy_import`, `persist-1`) and the port never
writes to a legacy path.

Files live in `<root>/saves/`:

| File | Meaning |
|---|---|
| `quick.json` | F5's quick save; atomically replaced |
| `commander-NNNNNNNN.json` | Ctrl+F5's new-only numbered save (8 digits, 00000001..) |
| `<name>.json` | A save made from the Escape pages' **Save** row (2026-11-18): a name box offers the next `commander-NNNNNNNN`, the player may clear it and type another (1-24 letters, digits, spaces, `-`, `_`; not `quick` or a Windows device name), and `.json` follows the box and cannot be edited. Enter refuses a name that is invalid or already taken and shows why; nothing is ever overwritten. The Load and Delete lists show every `*.json` save (quick first, then newest first) |
| `config.cfg` | version-2 preferences (`controls.cfg`/`display.cfg` are read once as migration) |

JSON shape (one top-level object, `deny_unknown_fields` at every level):

```json
{
  "format": "ffe-rs-save",
  "version": 2,
  "commander": {
    "name": "Jameson",
    "ranks": [0, 0],
    "missions": []
  },
  "state": { "...": "Flight: mode, position, velocity, yaw/pitch, full axes, "
                    "credits, hold, combat (lasers/missiles/energy/heat/drive), "
                    "shields/hull, loadout, service days, death=null" },
  "navigation": {
    "system_code": 111449879,
    "clock": 102562290000.0,
    "start_clock": 102562286400.0,
    "new_game": {"start_type": 1, "...": "..."},
    "page": 0, "view": "Closed", "selected": 0,
    "target": null, "locked": null, "destination": null,
    "altitude": 2147483647, "docked": 4,
    "legal": {"wanted": false, "alert": false, "illegal_fire": false,
              "offences": 0, "fines": [0, 0, 0, 0, 0, 0]},
    "ship_lock": null, "missile_lock": null, "missile_view": null,
    "sequence": null, "takeoff": null, "landed": null,
    "final_leg": false, "spawned": null
  },
  "original_flight": false,
  "acceleration": 0
}
```

- `navigation` is absent in version-1 snapshots and the prototype loop.
- `state.axes` is the full original-flight orientation; older saves omit it.
- `system_code` identifies the system, so the bodies are regenerated from the
  cache at the saved `clock` (the source does the same from its clock/date).
  Body indices (`docked`, `target`, `locked`, `landed.index`, `sequence.station`)
  are checked against the regenerated list; a bad index is a load error.
- `commander.ranks` are the two words `DATA_008912/008913` the galaxy tables use
  (the port does not change them). `missions` is a reserved list: no mission
  engine exists in the port, so it is empty in play; it round-trips for a future
  subsystem.
- `original_flight` says the key-O model was running; `acceleration` is the
  console step. Both are port-only facts.
- The live NPC ships and the stations' traffic records are stored (PAR-18);
  missiles and short-lived effects now also resume from their saved state. The game's shared random pair
  (`DATA_008818/8820`, `Navigator::random`) is saved as `random` (PAR-17), so
  a loaded game continues the same draws. Saves without it resume from F147's
  seed. The current system's visited markets are saved too (`markets`:
  stock and prices after trading, the daily drift's stream and day; PAR-20),
  so a load neither restocks a station nor reprices it. Bulletin boards and
  offers keep their per-station streams (`EXP-006`), regenerated from the
  saved clock and system.

### 2.1 Live resume records (2026-11-15)

The optional `navigation.runtime` block closes ten bounded PAR-18 gaps, tracked
as `persist-6` through `persist-15`. It contains career points and medals; the
missile field including its next id, guidance accumulator and lifetime; decoys
and their next id; effects with their exact countdowns, cargo and script tags;
the ship autopilot target; launch permission; the per-port warning latch;
fractional traffic seconds; and the mission-target spawn ledger. The existing
`spawned` marker now distinguishes an exhausted NPC population from one that
has not been generated, even when there are no ships left.

All fields are taken from the live simulation; restoring them consumes no new
shared RNG draws. Script canisters are still written in the older field, but a
save with `runtime` restores the complete effects list once. Files without
`runtime` retain the previous script-canister migration and default live state.
The format version remains 2; older builds may reject the additional field.

Validation rejects invalid motion, cooldowns, duplicate object ids, malformed
decoys, and out-of-range collections before resume. Body and ship references
are checked against the rebuilt system. Twelve cache-backed regressions cover
the ten tasks, old-save migration, and malformed records. The second batch below
extends this state. Complete simulation resume remains open; these bounded tasks do not claim full reference-compatible saves.

### 2.2 Station and event continuation (2026-11-15)

`navigation.runtime.continuation` closes another ten scoped tasks:

| Task | Preserved behavior |
|---|---|
| `persist-16` | Mid-jump remaining/total time and animation progress |
| `persist-17` | F675/F680 pursuers, equipment and player anchor |
| `persist-18` | Purchased/part-exchanged shipyard stock, daily RNG and context |
| `persist-19` | Removed bulletin posts, variables, daily RNG and context |
| `persist-20` | Unvisited F622 boards and the system entry marker |
| `persist-21` | BBS jobs and remembered destination words |
| `persist-22` | Last processed day and undelivered day events |
| `persist-23` | Pending deadlines and queued expirations |
| `persist-24` | Autopilot angle, transfer count, drift and clearance retry time |
| `persist-25` | Navigation frame selected before the next orbital refresh |

Station maps serialize in sorted system/station order. Boards already expanded
into station offers are stored once in the offers list; other boards stay raw
until the first visit. The original per-station RNG state and generation context
are restored together, so daily replacement follows uninterrupted play.

An earlier save without `continuation` keeps the previous migration behavior,
with the daily marker seeded at its saved clock. It does not replay every day
since boot or invent missing deadlines. New saves preserve both unprocessed day
boundaries and already queued events. Validation rejects malformed transit and
station records, duplicate keys/deadlines, invalid motion, event queues, retry
times and missing frame/station references.

The regression suite compares transit completion, pursuer spawn, station daily
updates, deadline delivery and the two-second clearance retry boundary across
save/resume. These additions do not serialize the full original player physics,
space-dust field, presentation/message queues, or grant PAR-18/PAR-20 sign-off.

### 2.3 Validation and errors

- File: regular, non-symlinked, `<= 1 MiB` (JSON), else a clear error.
- `format` must be exactly `ffe-rs-save`; `version` must be 2.
- `Flight::validate`: finite/bounded position/velocity, yaw/pitch, hold length,
  credits/stock, ship range, loadout, lasers/missiles, shield/hull order, docked
  consistency, orientation rows near unit length, and `death == None`.
- `NavigatorSave::validate`: finite/bounded clocks, non-negative altitude,
  known docking/launch/takeoff/landing states, index bounds (after rebuild),
  legal/new-game scalar checks.
- A save on the game-over screen or with `Flight::death` is refused before
  serialization: `SAVE REFUSED: the commander is dead.`
- A load leaves the live state unchanged on any error (the caller applies only a
  fully decoded and validated `SaveGame`). There is no fallback to an older file.

### 2.2 Atomic writes

`save_atomic` writes `.<name>.<pid>.tmp` beside the destination, `sync_all`s it,
then renames it over the destination (replace-existing). `save_numbered` keeps
the old new-only `create_new` guarantee. A failed write removes its temporary
file and leaves the previous save intact. The numbered path is used by Ctrl+F5;
quick save replaces `quick.json` atomically.

### 2.3 In-game keys and the front end

| Key | Action |
|---|---|
| F5 | quick-save `saves/quick.json` |
| Ctrl+F5 | new numbered save `commander-NNNNNNNN.json` |
| Shift+F5 | load `quick.json`, else the newest numbered save |
| `L` on the intro | load `quick.json`, else the newest numbered save (the original's Load Commander choice) |
| `--load <path>` | load a named native save and skip the intro |
| `--capture-save <dir>` | evidence: save, drop the live objects, load, rebuild the navigator, compare, write frames/log |

`--load` also works after the tombstone returns to the front end (the intro's
`L` is handled there too).

---

## 3. Configuration migration

`config.cfg`:

```
version=2
controls=modern|classic
space_colour=0..4
shape_detail=0..3
other_0..9=on|off
map_0..4=on|off
sound_0..9=on|off
scale=1..6
fit=pixel-perfect|fill-window
filter=nearest|linear
aspect=dos-4:3|square
bind.<key|action>=<key>    # input remap: game key or action -> physical key
```

- A missing file is the defaults.
- If `config.cfg` is absent, the old `controls.cfg` (`modern`/`classic`, or the
  new `version=2/preset=...` form) and `display.cfg` are read once. The next
  change writes `config.cfg`; the sidecars are not rewritten.
- A malformed or future-version `config.cfg` is ignored as a whole (defaults),
  not silently replaced by stale sidecars.
- Writes are atomic. Unknown keys are ignored for forward compatibility; a known
  key with a bad value rejects the file. Full-screen is session-only and is not
  written.
- Input remapping is the `bind.*` lines. `session::bindings` reads them at
  start-up; a minifb game-key name (`bind.A=Up`) or an action name
  (`bind.pitch_up=NumPad8`, see `crate::input::Action` and
  `docs/CONTROLS.md#remapping-keys`) maps the physical key to the delivered game
  key. `session::Frame::remap` applies it to every frame before any screen or
  flight code reads the keys. An unknown key or action name is ignored (the line
  stays in the file), and a binding that would map a key to itself is dropped.
  There is no in-game remap screen; the file is the remap table.

---

## 4. Evidence

- `cargo test -p ffe-rs --lib saves` and `settings`: complete round trips,
  version-1 migration, corruption/unknown/newer version, oversized input,
  legacy recognition leaving the file byte-identical, atomic replace, quick and
  numbered naming, config migration.
- `cargo test -p ffe-rs --test saves -- --ignored` with `FFE_WORLD_CACHE`:
  complete flight commander (combat/legal/rank/mission/axes), docked pad, landed
  body, and bad-index/stage rejection through the real cache.
- `cargo run -p ffe-rs -- --root <root> --capture-save <dir>`:
  `roundtrip.json`, `save-before.ppm`, `save-after.ppm`,
  `save-roundtrip.txt`; it drops the navigator/state, reloads from disk,
  regenerates the system and asserts both halves compare equal.
- `cargo run -p ffe-rs -- --root <root> --load <save> --smoke-frames 2`:
  the real binary starts from a loaded commander.
- Existing v1 snapshot tests in `combat.rs`, `death.rs`, `views.rs` and
  `playable.rs` still pass, and the old `ffe-rs-experimental` documents migrate.

## 5. Remaining owner decisions

1. **Legacy import.** The specimen `game/FIRSTENC._01` is supplied and the
   container is reverse-engineered (`persist-1`, done): F175's layout (F69 loose
   block, F65 stream, F170 body), the F170 zero-run/XOR codecs and checksum, and
   the commander/system/save identity are recovered read-only. `persist-4`
   establishes the gameplay-state field map (section 1.2.1): credits 10,000,
   Saker Mk III (model 31), 7 t hold with 1 t of hydrogen fuel, the clock, the
   two RNG words and the raw blocks are replayed against the executed original.
   The port still imports read-only and never writes a legacy path.
2. **Traffic/RNG exactness.** The port has no single source RNG stream, so a
   loaded game respawns traffic instead of restoring the source's object array.
   Matching it needs the source object/RNG fields and a decision on whether
   despawned/killed traffic is restored.
3. **Mission/rank model.** The format reserves `ranks` and `missions`, but the
   port has no mission engine or rank progression; whether to add them (and in
   what source layout) is open.
4. **Input remapping screen.** The `bind.*` table is applied (`session::bindings`,
   `docs/CONTROLS.md#remapping-keys`); the in-game screen that would edit it is
   still unported, so a hand-edited `config.cfg` is the only way to change it.
5. **Named slots/UI.** The source's file dialog, directory navigation and
   extension bump are replaced by quick/numbered files. A full save/load screen
   (and the `F192`/`F194` layout) is still open.
