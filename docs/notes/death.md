# Player destruction, the death page and escape craft (F1104/F1098)

Status (2026-10-25): the player's hull reaching zero now runs the DOS FFE
sequence. The old substitute that recovered the ship at the nearest starport is
gone. With the Escape Capsule (or the "Stowmaster" fighter) fitted, F1098 moves
the player into the craft and the game continues; without one, F1104's
external-view hold ends on F917's spinning tombstone: the `GAME OVER` face and
the `COMMANDER <name> DIED <date>` face. Source: `jjffe-master/ffe.asm` /
`ffedat.asm` (JJFFE 2.8a7 is the reference build, DOS FFE is the behaviour
source). The screen itself is `docs/notes/gameover.md`.

## 1. The source path

### 1.1 F953 -> F942 -> the death event

- `F953` (`ffe.asm:71190`) applies a hit to the shields (`+0xe0` against
  `+0xe2`), then the hull (`+0xe4`, mass * 4). When the remaining damage is at
  least the hull, `71279-71287` calls **F942** (`70650`) and returns.
- `F942` plays sample `0x05` (`70713`), creates a model `0x9b` (155) object
  with AI state `+0xff = 0x1c` and a random orientation (`70717-70741`), and
  for a hull of mass 10 or more plays `0x1f` (`70758-70761`). For ship models
  `0x0f..0x3f` it spawns F926 debris (`70788-70835`). It finishes with
  **F924** (`69099`, called at `70837-70840`), which removes the object from
  the AI lists.
- The destroyed player object raises an object event. **F1103**
  (`81442`) is the entry in `DATA_007254` (`ffedat.asm:18552-18564`); it calls
  **F1102** (`81394`) with the object slot and a flag. F1102 clears the
  navigation/combat target (`F1101`, `81346`) and calls **F1104** (`81458`).
- `F1104` requires the event slot to be the player's (`81464`: `cmp eax,
  [DATA_008857]`) and `DATA_008870 != 0x2c` (`81466`), so a second hit on a
  dying ship does nothing.

### 1.2 F1104: the death hold and the page

`F1104` (`81458-81523`) runs in this order:

1. If `DATA_008630 >= 0`, call `F1002(0)` (`81468-81472`).
2. Play sample `0x1f` (`81474`).
3. Call **F1098** on the player's ship (`81477-81479`). If it returns
   non-zero, return: an escape craft launched (`81481-81482`).
4. Set the player state `DATA_008870 = 0x2c` (`81483`) and post the GUI
   redraw (`F48(1,0,0xe)`).
5. Scan all 0x72..1 object slots and pass each to **F1105** (`81527`), which
   remembers the model-0x9b, control-0xb, class-0xff explosion object in
   `DATA_008861` (`81538`).
6. Set `DATA_008872 = 4` (the external view), `DATA_008895 = 0xf9000000`
   (the camera distance request), `DATA_008884 = 0x3cae6` (248 550 tics =
   5.001 s at 49 700 tics/s), `DATA_008631 = 0`, clear the GUI hot areas, set
   `DATA_008835 |= 0x21`, and play sample `0x1f` again (`81507-81519`).

The GAME OVER strings are in `ffedat.asm:5626-5634`:

```text
DATA_005129: "\rGAME\rOVER"
DATA_005130: "\r\r\r\r\r\r\r  COMMANDER\r   %s\r     DIED\r  %s"
```

They are not a status page. **F1107** (`81557`, the second handler of the
`0x2c` state entry in `DATA_007255`) subtracts the frame tics from
`DATA_008884`; when it expires it calls **F1106**/F918(1), which runs **F917**
(`68047`). F917 draws the tombstone model 0xac (172) — or 0xaf (175) when
`DATA_008885 == 0xeb6973d` — and its model text records reach the strings
through `DATA_005104` (`ffedat.asm:5243`) and the FF9C expansion `F277`
(`ffe.asm:21602`; entry `ffedat.asm:23186`). F917 spins for 950,000 tics
(about 19.1 s) and longjmps back to the front end. F286/F283 are the separate
status/equipment pages and are not part of the death screen.

The `0x3cae6` timer is therefore F1104's 5.001 s external-view hold, consumed
by F1107, and F917's loop is the screen. See `docs/notes/gameover.md` for the
model, text codes and exact angle.

### 1.3 F1098: the escape craft

`F1098` (`80355-80485`) is the same routine for the death event, the `x` key
(`F1054`, `78206`) and the F12 console icon (`F1056`, `78528`):

1. Read the general-equipment dword `[esi+0xc8]`. Bit 3 (`0x8`) selects the
   "Stowmaster" fighter and is tested first (`80366-80370`); otherwise bit 27
   (`0x8000000`) selects the Escape Capsule (`80372-80374`). The equipment
   table's records are code `0x13` (Stowmaster) and code `0x0b` (Escape
   Capsule), and `F393`'s `(code ^ 0x10) & 0x1f` maps them to bits 3 and 27.
   **There is no cabin, cash or cargo check** in F1098.
2. Allocate model `0x10` (16, Stowmaster) or `0x0e` (14, Escape Capsule) with
   `F928` (`80383-80386`); return 0 if allocation fails.
3. Make it the player's ship: `player+0xc8 = new object`
   (`80393`), `player+0xc0 = new object id` (`80401`). The old ship becomes an
   AI object (`+0x87 = 0xb`, `+0xff = 0x16`, `80402-80403`), and the player
   enters state `0x24` (`80404-80405`).
4. `F992` resets the combat words (`80406-80408`).
5. New general equipment `0x40200000` (`80411`/`80418`): bit 21
   (automatic pilot) and bit 30 (atmospheric shielding).
6. Stowmaster branch (`80409-80416`): front laser `0x88` (1MW pulse), drive 2,
   `player+0x15a = 2`, `player+0x120 = 2` (two cabins). Capsule branch
   (`80417-80419`): no laser. The capsule's drive is whatever `F1533` leaves
   (the old drive unless model 14's statistics have a negative drive word);
   `player+0x120` is not written, so the capsule inherits the old cabin count.
7. Clear the laser target, all ten pylons, the rear/turret lasers, both shield
   words, and the player's HUD/message words (`80421-80446`).
8. Energy `0x20000000`, copy the old position, `F1514` places the new object,
   `F250`/`F1120`, play sample `0x20`, and redraw the console
   (`80447-80474`).

`F1533` (`90970`) gives the new object its hull (`mass * 4`, `91084`) and
clears its shields (`91096-91097`), so an escape craft starts with a full hull
of its own model. Cash, cargo and the hold live in the player block and are not
cleared.

### 1.4 Sound and music

- F942: `0x05` (`shipexp`) and, for mass >= 10, `0x1f` (`boom`).
- F1104: `0x1f` before F1098 (`81474`) and `0x1f` after the scan (`81517`);
  only the no-craft branch reaches the second one.
- F1098: `0x20` (`launch`).
- F770 (`50345`, `50460`) stops the song when the player is killed
  (`F1903`). F1104's escape branch returns before the game-over branch and
  F1098 does not stop music.

## 2. What the port implements

- `ffe_rs::death`:
  - `escape_craft` reads the two general bits in F1098's order.
  - `begin` is F1104's F1098 test: with a craft it calls `board`, without one
    it sets `Flight::death = Death::exploding()`.
  - `board` is F1098's new-ship setup: model 14/16, general `0x40200000`, no
    pylons/shields/rear or turret lasers, energy `0x20000000`, the Stowmaster's
    `0x88`/drive 2/two cabins, the capsule's inherited drive and cabins, and
    `F1533`'s hull from the model's mass.
  - `Death` is F1104's state `0x2c` and `0x3cae6` hold; `tick` moves from
    `Exploding` to `GameOver` at 5.001 s.
  - `player_explosion` spawns F942/F926's model-155 explosion and debris at the
    old position; `destroyed_message`/`escaped_message` are the port's log
    lines.
  - `play_destroyed_audio` / `play_destroyed_escape_audio` / `play_escape_audio`
    post the samples above and, for the no-craft branch, F770's song stop.
- `Flight::death` (serde default) carries the state and is rejected by
  `Flight::validate`, so a save after destruction fails rather than silently
  dropping the sequence.
- `main`:
  - The NPC hit path calls `playership_destroyed`: it spawns the explosion,
    runs `death::begin`, rebuilds the original flight model for the new craft,
    sets the external view on the no-craft branch, and posts the cues/message.
    A second hit on a ship already in the hold is ignored (`81466`).
  - A planet/starport surface crash reaches the same path from the flight step:
    `Flight::take_contact_damage` applies F575's F953 amount, and the frame's
    destroyed-hull check (`state.destroyed()`) starts F942/F1104. See
    `docs/notes/collision.md`.
  - The F12 console icon (`Click::Capsule`) is F1056's F1098 call: it launches
    the fitted craft with sample `0x20`, or says `No escape capsule fitted.`
  - While `Flight::death` is set, the flight controls, NPC traffic and lasers
    are held; F942's effects still tick through `Navigator::tick_effects`.
  - F1104's external view is set on death. When the 5.001 s hold expires,
    `GameOver::tombstone` draws F917's model 172/175 and its engraved text;
    there are no death-screen options. At the end the game returns to the
    front end (the intro), whose 1-5 choices include the Commander Load screen.
- `ffe_core::effects` needed no change; `Navigator::tick_effects` exposes its
  existing effect ticker to the death state.

## 3. Evidence

- `cargo test -p ffe-rs --lib death` (6 tests): F1098's bit order and the
  equipment table's codes, the `0x40200000` field set and the craft model
  numbers, the `0x3cae6` hold (248 550 tics, 5.001 s), and `gameover`'s phase
  timing.
- `cargo test -p ffe-rs --test death` (8 tests):
  - no craft -> `Exploding` -> `GameOver`;
  - Escape Capsule -> model 14, general `0x40200000`, no lasers/missiles/
    shields, model hull, cash/hold/position/velocity kept;
  - Stowmaster -> model 16, front `0x88`, drive 2, two cabins;
  - a dead ship cannot be saved and an old save loads alive;
  - F942's explosion/debris and the source cue sequence (including F770's song
    stop and the automatic-escape cue order);
  - the `DATA_005130` commander lines and the port's two log messages;
  - F953's `Hull` -> `Destroyed` before F1104.
- `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test
  death_view -- --ignored --nocapture` (2 tests): the cached frames
  `death-explosion.ppm`, `death-gameover.ppm` and `death-capsule.ppm` (PNGs in
  `docs/evidence/death/`). It checks model 155 is present, the external view,
  the 5.001 s phase change, the game-over text ink and the capsule's model 14
  state.
- `cargo test --workspace --locked`, clippy `-D warnings` and fmt are clean.

## 4. Deviations and remaining owner decisions

- **Screen art/layout.** The port renders F917's real model 172/175 and its
  model-text records through the same call F917 makes (the counter is the
  distance and `MatBuildYZT`'s third argument), so the slab turns about its
  vertical axis; the text records' own normals pick the visible face. Palette
  values and the exact obituary line anchors are still not pixel-exact. The
  earlier text-page placeholder and its `N`/`Q` options are gone.
- **Commander name.** The port has no commander-name entry; it uses the
  original's default `Jameson` (`DATA_004963`), uppercased on the tombstone as
  the source's font does. Decide whether to add a name field to
  `Flight`/saves and to the new-game front end.
- **Hold trigger.** `DATA_008884 = 0x3cae6` is consumed by F1107 after the
  external-view hold; the port models it as `Death::Exploding` and starts
  `GameOver::tombstone` at 5.001 s. F917's own 950,000-tic spin ends with the
  longjmp to the front end.
- **Old ship derelict.** Done 2026-11-03 (see the section below): the old hull
  drifts in AI state 0x16 until F767 removes it.
- **Escape ending.** F1098 also clears the police alert (`DATA_008647 = 0`,
  `DATA_008646 &= 0xfe`) and selects the front view (`+0xea = 0`); the port
  does both on either path (`main::escaped`, `Legal::clear_alert`). It sets
  the mode word `+0xe8 = 0x24` too; the port does not (its 0x24 is the
  starport climb, which a capsule in space would never finish), so that is
  not ported.
- **Manual key.** DOS FFE's `x` launches the capsule (`F1054`, `78206`); the
  port does too (2026-10-29, owner decision), alongside F12 and the console
  icon. The port's brake, which the original does not have, moved to `B`.
- **Cash/cargo.** F1098 neither clears nor transfers the hold; the port keeps
  it, even though the capsule's cargo space is smaller. That is the source
  behaviour, but it is visible in the cargo screen.
- **Cabin count.** The capsule branch inherits `player+0x120`; the port keeps
  the old `Loadout::cabins`, matching the source. The Stowmaster sets two.
- **Music.** F770's stop is applied to the no-craft branch. The source's
  escape branch keeps the current song; the port does too.
- **Game-over options.** There are none in the original state: F917 is a timed
  spin. The `N`/`Q` placeholder is gone; the original returns to the intro,
  whose choice 5 is the Commander Load screen (the port's load UI is still not
  complete).
- **No cabin requirement.** There is no cabin or passenger check in F1098; the
  only requirement is the general-equipment bit. The task's "cabin/equipment
  requirement" is therefore the equipment bit (and, for the Stowmaster, the
  two cabins it grants, not consumes).

## Side-by-side with `--death-probe` (2026-11-17, parity-08)

The executed original (`jjffe-smoke --death-probe 1200`, Ross 154 start, hull
destroyed at frame 250, frames every 21 ms) and the port (`FFE_DEV_DESTROY_AT=0.6
ffe-rs --dev --start 1 --skip-intro --capture-start <dir> --smoke-frames 299`) were
compared frame by frame. What matches:

- the sequence: the parked ship vanishes in a flash at the starport, the model-0x9b
  burst with its twelve fragments grows over the first second, grey smoke puffs
  (model 0x9e sparks) and debris pods fly, the external camera turns about the
  burst, and the 0x3cae6 hold (238 frames of 1,043 tics) ends with the tombstone
  spinning in from far away at about frame 240-260 after the hit;
- the colours: the harness writes the game's 6-bit VGA palette without the x4
  scale, which made its PPMs look dark. The station hull is (10, 10, 8) there and
  (40, 40, 32) in the port: the same colour;
- the object kinds and word values the earlier probe checks in `effects.rs`: one
  model-0x9b object (AI 0x1c), model-0x99 pods (AI 0x15, global word 1 = 15 for a
  wreck pod) and model-0x9e sparks (AI 0x1a, timer falling by tics >> 1), and the
  timer wrapping when a spark dies.

The sentence that used to end this section ("what cannot be compared ...") is
superseded by the frame-by-frame trace below.

## Frame-by-frame object trace against the executed original (2026-11-20, ui-12 and parity-08)

`jjffe-smoke --death-probe` (and, for a moving ship, `--flight takeoff|planet N` with
`FFE_DEATH_FLIGHT=<frame>`) now takes the shared pair from the environment and prints one JSON row
per frame for every effect object. `FFE_PAIR=<a>,<b>` sets `DATA_008818/8820` at the hit,
`FFE_DEATH_OBJECTS=1` prints a `death_hit` row (the destroyed ship, its parent, the parent's mass, its
F603 fraction words and the occupied object slots) and then, from right after F953, a `fx` row per
frame with slot, model, control, AI state, `+0x7e`, `+0xa4`, `+0x9e`, `+0x14e`, position, velocity
words and the matrix rows of every model 0x99..0x9e object; `FFE_RNG_TRACE=<file>` logs every F922
and F1530 draw with its caller. `crates/ffe-core/tests/death_replay.rs` (local, `#[ignore]`) replays
`effects::explosion` and `EffectTable::sweep` from the same pair, the same occupied slots and each
row's tics and clock, and compares the lists.

**Result: 450 of 450 frames equal in the Ross 154 pad death with three different pairs and in the
Lave orbital-station and Hope starport starts (500 to 800 frames in two moving-ship deaths in flight).**
The comparison covers slot, model, AI state, `+0x7e`, `+0xa4`, `+0x9e`, the pod spark count, position,
velocity words and the matrix rows. The draw ledger shows no other consumer of the pair between the
effect draws in any of these runs (the starport's `F922(12)` draws in the Hope start come before the
first spark and the replay resumes from the trace there).

What the first comparison found wrong in the port, all fixed in `ffe_core::effects`:

- **Draws.** F942's and F939's matrices are `MatBuildYZT(F1530, F1530)` (two `& 0xffff` words of the
  F1530 stream) and F939's timer is the first of them `| 0x8000`; the port used two `F922(65536)`
  draws. F942's cargo pods index `DATA_007212` with `F1530 & 0x1f`, not an F922 draw. F719's missile
  explosion is one F1530 draw with the second angle `eax >> 16 = 0` (read from `ffe.asm:47573`, not
  yet executed against the original).
- **Pods.** F926 does not randomise the orientation: a pod keeps the ship's rows (F1533's clone) and
  has no extra draws. F1514 places it at the ship plus the drift vector, so the pod starts the drift
  away from the ship. The pod clones the ship's F603 fraction words and parent.
- **Gravity.** F713 (pods and derelicts) runs F603 after F1512: a pod at a planet's surface gains
  about one velocity word per frame towards the parent (the probe's pods go `-214` to `-215` in the
  first frame). `Effect::parent`/`fraction` carry the parent's centre and ffp mass and
  `Motion::gravity` does the arithmetic. The explosion and the sparks (F708, F709) have none.
- **The sweep.** The object list is walked from slot `0x72` down to 1, and a spark is allocated (F928)
  the moment it is thrown, so a spark landing below its thrower is visited again in the same frame
  (its timer drops by `tics >> 1`, but its F1512 step is zero) and one above is not. A dead object's
  slot is free at once (F940 clears the slot's flag byte, not `+0x87`, which is why an earlier
  version of the probe listed dead sparks with wrapped timers).
- **The F939 gate.** F708's and F712's F939 call (flags 2 and 4, bit 0 clear) first draws one F1530
  and throws nothing when its bit 15 is set, about half the frames; only F931's laser hit (flag 1)
  always spawns. A spark is born at the thrower's position and velocity before F1512 and F603 of that
  update. The port threw every spark, from the moved position.
- **The pod countdown.** F768's `+0x14e` counts F152's global second boundaries (every `0xc22e` tics
  of the game clock), not the pod's own age. `Effects::tick_clock` takes the clock's low word before
  the frame (`Navigator::tick_effects` derives it from the world clock).
- **Spark velocity.** A spark keeps its thrower's velocity words only when the player's own speed
  (`DATA_008867`) is at least `0x9c40`; the port zeroed it. `Effects::set_player_speed` (main sets it
  each flight frame) carries the speed. The parent class bit `0x10` (a station parent) zeroes it too
  and is not modelled.

`Navigator::push_burst` gives a burst's pods the planet or moon whose atmosphere contains it as their
parent (the port's `Body::mass` is the `+0xa4` word), and `main::playership_destroyed` copies the
player's fraction words. The unit test `effects::f942_and_the_first_sweep_match_the_executed_death_probe`
pins the first frame of the pad death (pair after F942 `936602760,3400778596`, the three sparks into
slots 107, 99 and 98).

Crash, laser and missile deaths all end in F953 -> F942, the routine compared above; the cause only
changes the inputs (velocity, parent, the hit particle before it). A laser death's F939 is flag 1, which
has no gate and is the port's `effects::hit`; a crash's F575 -> F953 is the same F942. The missile
route's F719 explosion was changed from the source and is not executed against the original.

Still open (accepted at about 90%, `docs/ACCEPTED_PARITY.md`): the parent frame is the port's nearest
planet or moon, not the original's `+0x56` after F570's frame switching, and the pods are not in the
planet's rotating frame (`+0x57`); a hit particle farther than distance class 0xd (F709's
`+0x88 > 0xd`) is not culled; F942's distance word for hulls of 400 t or more; pixel-level comparison
of the model-155 fragments against the executed frames (the objects they are drawn from are now
equal; `effects_view`/`death_view` still check the drawing against the cache).

## F1098's derelict (2026-11-03)

When an escape capsule or Stowmaster launches (the automatic death path or the
F12 icon), F1098 keeps the old ship's object: `+0x118 = 1`, its thrust words
`+0xb6..+0xba` zeroed, control `+0x87 = 0xb` and AI state `+0xff = 0x16`, where
F713 only moves it and F767 removes it at distance class 0xd. The player's
object becomes the new craft, and the wanted state (`DATA_008647`,
`DATA_008646` bit 0) is cleared. The port leaves an `effects::Kind::Derelict`
with the old model, position, velocity and axes, removed at `DEBRIS_RANGE`,
and calls `Legal::clear_alert` (`main.rs::leave_derelict`).
