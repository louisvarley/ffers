# Playthrough checkpoints (release-1)

One checkpoint per stage of a playthrough, each against the executed JJFFE 2.8a7
build and each with a port test. `tools/reference/verify-playthrough.py` chains
them; `verify-all.py` also runs the three new ones individually.

## Stage inventory

| Stage | Reference mode / oracle | Port evidence |
|---|---|---|
| Import and start | `--legacy-save game/FIRSTENC._01` (the real F70/F66/F171) via `verify-legacy-save.py` | `crates/ffe-rs/tests/legacy_save.rs`: field and byte match; `saves::legacy_import` |
| Intro | `--intro` (the real F961/F962) via `verify-intro.py` | `intro` tests (`crates/ffe-rs/tests/`), the intro frame and actions |
| Launch | `--flight takeoff|hover|launch` in `verify-flight.py` | `flight_reference.rs`, `takeoff_replay.rs`: per-frame mode, set speed and hand-off |
| Fly | `--flight idle|planet|keyboard` in `verify-flight.py` | `flight_reference.rs`, `physics`/`autopilot` oracles |
| Dock | `--flight dock|paddock` in `verify-flight.py` | `launch_replay.rs`, `navigation.rs` docking stages |
| Trade | **`--trade 220`** (new) via `verify-trade.py` | **`crates/ffe-rs/tests/market_trade.rs`**: `Market::{buy,sell}` |
| Save/load | **`--save-roundtrip 260`** (new) via `verify-save-roundtrip.py` | **`crates/ffe-rs/tests/save_roundtrip.rs`**: `legacy_import` on the written file |
| Jump | **`--jump 2000`** (new) via `verify-jump.py`; `--misjump` for F909/F910 | **`crates/ffe-rs/tests/jump_reference.rs`**, `hyperspace.rs` |
| Combat | `--combat`, `--missile-encounter` via `verify-combat-replay.py`, `verify-missile-replay.py` | `combat_replay.rs`, `missile_encounter.rs` |
| Missions/BBS | `--daily 2000` via `verify-daily-replay.py`; `mission_scripts.rs` for the scripts | `daily_reference.rs`; 10 cache-backed mission-script cases |
| Render | `--scene-frame`, `--game-frame` via `verify-frame.py`, `verify-galaxy-map.py` | `scene_frame.rs` (63,946/64,000 pixels), `galaxy_map_view.rs` |

## The new modes

### `--trade <frames>` (start key `0x04`, act at frame 200)

Starts the real Lave orbital-station game. At frame 200 it points the player
block's market pointer (`+0x48f8`) at a fixed 33-entry record and the tonne
count (`+0x48e4`, 64 per tonne) at the requested quantity, then calls the real
F555 (buy) or F556 (sell). Ten cases cover:

- a three-tonne purchase (cash and stock fall, the cargo word and used rise);
- a two-tonne sale;
- `0x98e1` (cash short), `0x98e2` (a negative price the cash cannot cover),
  `0x98e3` (hold full), `0x98e4` (no stock) and `0x98e5` (nothing to sell);
- a partial purchase that runs out of stock, and a partial sale that runs out of
  cargo.

Each row carries the cash, cargo word, hold used, station stock and F557 message
code before and after. `market_trade.rs` rebuilds the same entries and hold and
replays the loop through `ffe_core::market::Market::{buy,sell}`. F556's
`0x98e5` is both its initial value and its "nothing to sell" code, so a sale's
code alone is not a success flag; the state fields distinguish them.

### `--save-roundtrip <frames>` (start key `0x04`, act at frame 240)

Runs F214 (zero unused objects), F170 (mangle the F175 body) and F174 (F69 + F65
+ the body) to write a commander file to `FFE_SAVE_FILE`, then changes every
saved field (name, cash, capacity, used, clock tics/days, both random words,
player object, all 38 cargo words, fuel), prints the changed state, and runs F176
(F70 + F66 + the body) and F171. The reference's own in-process check requires
the restored state to equal the pre-save state; the trace's `before`,
`perturbed` and `restored` rows show it. `save_roundtrip.rs` then imports the
same file with `saves::legacy_import` and requires the field map (name, credits,
capacity, used, hydrogen/military/radioactive fuel, clock, random pair, player
object, ship model and registration, 38 cargo words) to match the before row.

### `--jump <frames>` (start key `0x39`, act at frame 400 in free flight)

Tab launches from the Gateway/Hope pad; once F1031 has handed the ship to free
flight, the mode sets F908's selected-destination words (`+0x268`, `+0x26c` =
442, `+0x27c` = Titican's code `0x12a53716`), raises the altitude above F908's
`0xb71b00` gate and answers the F34/F35 dialog yes. F1530 is advanced normally
but returns `0x1000` (`<= 0xfe00`), so the jump cannot become a random
mis-jump. The trace carries F901's plan (mass 28, class 2, 189,322 time units,
1 t of fuel), F908's mode 4 and timeout, F909's arrival, and the shared pair
before and after. `jump_reference.rs` rebuilds the same jump through
`Navigator::hyperspace` and requires the distance, fuel, jump time, days,
transit and the 7.4..11.3 AU arrival orbit.

## Accepted gaps

- No executed in-play mission chain (accept and complete through the script
  engine). The station side is `--daily`'s F633 board/jobs plus the ten
  cache-backed `mission_scripts` cases.
- F909's arrival draw ledger: with the same pair before the call, the port and
  the original draw different counts through the arrival, so the arrival is
  checked as the destination system plus the F1507 orbit, not the final pair.
- The map's destination cursor (F832 and the galaxy map's selection words) is
  not driven; `--jump` supplies the selected destination and the dialog answer.
- On a JJFFE-written save, the importer's string-section `commander_name` is two
  bytes short ("eson"); the player block's own name copy is exact.

## Commands

```powershell
python tools/reference/verify-playthrough.py --cache .cache/ui11/data.bin
python tools/reference/verify-trade.py --reference .cache/jjffe-reference/jjffe-smoke.exe
python tools/reference/verify-save-roundtrip.py --reference .cache/jjffe-reference/jjffe-smoke.exe
python tools/reference/verify-jump.py --reference .cache/jjffe-reference/jjffe-smoke.exe --cache .cache/ui11/data.bin
```
