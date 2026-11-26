# Flight controls

The port has two flight presets, chosen on the Escape pages' `Advanced Options` page
(the source **F3** is the galaxy map, not a preset switch):

- **Classic** (default): the original's scheme. **Hold the right mouse button and move the
  mouse to steer**; **Enter raises the set speed** and right Shift lowers it; the
  keyboard steering is `,`/`.` yaw, A/Z pitch and `[`/`]` roll. The cursor is
  hidden and re-centred each held frame, so steering never stops at the window edge.
- **Modern**: the port's own WASD/arrow scheme.
- **Custom**: your own bindings (see below), set in the launcher.

The routing lives in `ffe_rs::input` (`flight_controls`, `original_keys`,
`turret_keys`, `MouseAccumulator`) and is applied in `main.rs`. It is pure and
unit-tested, so a preset cannot silently pick up the other preset's keys.

## Control schemes and the Custom bindings

The launcher's **Settings > Controls** page chooses the scheme: **Classic**, **Modern** or
**Custom**. Classic and Modern show their keys and the controller layout as read-only lists.
**Custom** lists the 32 actions of the tables below in five groups (Flying, Weapons, Targets,
autopilot and docking, Views and maps, Other), each with up to three bindings:
keyboard keys, game controller buttons, triggers and sticks (by their standard names, so an
Xbox, PlayStation or Switch pad all work), and the raw buttons and axes of a joystick. Click
**+ Add** beside an action and press the control to bind it: a prompt says what to press (Esc
cancels; the bare modifier keys have buttons in it, as the page cannot hear them). Bindings are
coloured by device (keyboard, controller, joystick); click one to remove it. A binding another
action also uses has an amber outline that says which. A box at the top finds an action or a
key by name, a per-action **Reset** appears where an action differs from Modern, and **Reset
all to Modern** asks first. A new Custom scheme starts
as a copy of Modern, with the controller layout, and is saved whichever scheme is selected.
The in-game `Port Options` page has a **Custom** row as well.

Custom reads like Modern, and works by delivering the game key an action uses in Modern while
one of its bindings is held. **While flying with nothing open, the default keys of every action
are taken away**, so an action you rebound no longer answers to its old keys; the menus, the
docked screens and the maps keep the normal keyboard, and your bindings work there too. The mouse
does not steer in Custom, and the E.C.M. answers to `;` (Modern's `E` is its roll key). Escape,
6-0 (the speed of time), U, F5 and Shift+F5 are fixed; so are a controller's Start (the options
pages) and D-pad up and down (time faster and slower) unless you bind them to an action.

`config.cfg` carries `controls=classic|modern|custom` and one
`custom.<action>=<input>|<input>` line per action (`none` for an action with nothing bound).
An input is `Key:<minifb key name>`, `Pad:<name>` (`South`, `LeftBumper`, `LeftStickUp`, ...),
`Joy:Button<code>` or `Joy:Axis<code>+` / `-`; one that does not parse is dropped on its own.

## Where the bindings come from

- **Set speed:** F1037 (`DATA_007692` Return raises it, `DATA_007694` right
  Shift, scancode 0x36, lowers it; left Shift, `DATA_007693`, only selects the
  shifted action table), ported as `Player::update_set_speed`
  (`crates/ffe-core/src/player.rs`). F1121's step grows with `DATA_007690`, the
  50 Hz count of how long the last key pressed has been held (it restarts when
  any other key is pressed or repeats, and clears when that key is released):
  `ffe_core::keyboard` keeps it, and the per-key steering counts, from every held
  key's original scancode (`input::scancode`). F1037 runs only while the flight
  view's module is showing (`DATA_008630`): under the F2 status screen or the
  map the set-speed and steering keys do nothing (`input::flight_module_active`);
  the navigation computer and the F5 engines page leave them working.
- **Steering:** F1038, ported as `Player::steer`. It reads the mouse accumulator
  `DATA_009188` (x in the low word, y in the high word), subtracts the previous
  value, shifts each delta left six and negates it unless its byte is set
  (`+0x3f54` x, `+0x3f55` y; zero by default, so x is negated by default). The mouse
  is counted as the original counts it, in the device's own relative counts (Windows raw
  input), not in pointer pixels. Right turns right; down pitches up, a flight stick's pull
  back (the original's default is the other way for y; the port keeps this on the owner's
  request and the Reverse up-down option flips it back). The held keys are `0x2c`/`0x1e` pitch, `0x33`/`0x34`
  yaw and `0x1a`/`0x1b` roll (`docs/notes/player.md`).
- **Original key names:** Z/A pitch, comma/period yaw, `[`/`]` roll, Enter/right Shift
  set speed (`docs/notes/player.md`, `docs/NAVIGATION.md`, README).
- **Fire, missiles, targets, autopilot, views, maps:** the port's existing
  bindings, unchanged by the preset (see the shared table below).

The original does not use a visible fire button for the mouse in flight: the
right button is the steering hold. The left button remains the console pointer
(buttons, target picking, map dragging and docked hot areas).

## Classic, full table

| Action | Classic binding |
|---|---|
| Steer the ship (pitch and yaw) | Hold the **right mouse button** and move the mouse |
| Raise / lower the set speed | **Enter** / **right Shift** (left Shift does not change it, as in the original) |
| Pitch up / down | **A** / **Z** |
| Yaw left / right | **,** / **.** |
| Roll left / right (original flight model) | **[** / **]** |
| Brake (port addition) | **B** |
| Fire the current view's laser | **Space** (hold; F931's refire/heat gates apply) |
| Launch a missile at the missile target | **M** |
| Fire the E.C.M. (standard or Naval) | **E**, **;** or **Insert** (needs the fitted system) |
| Deploy chaff | **C** (needs the Chaff Dispenser and Chaff in the hold) |
| Lock next / previous body or ship | **T** / **Shift+T** (or click a body; Shift+T also toggles the combat/missile lock) |
| Autopilot to the locked body (docks at a station) | **K** (or the AUTOPILOT button) |
| Stop the autopilot | **Backspace** |
| Request / cancel docking clearance | **D** |
| Launch from a station or landed state / toggle labels in flight | **L**, or the original action 9: **Tab** while landed |
| Navigation computer (stations and starports) | **N** |
| Galaxy map: choose the star at the centre as the jump destination | **F3** (the source map screen; **G** is the port shortcut) |
| Hyperspace to the chosen destination (also after the map closes) | **H**, or the jump icon on the ship diagram (F1116's record 0x45, shown when the destination is in range with the fuel and the ship is at least 12,000 km out) |
| Engines on/off in flight (F1056's action 9; cancels the autopilot when it is flying; switching on engages it on a locked target) | Click either engine nozzle on the ship diagram |
| Re-centre the galaxy map | **C** |
| Enhanced galaxy map (launcher toggle): add or remove a route star / pan / zoom / fly the route / clear the route | **click a star** (a far one plots a course through other stars; click again to remove it and its stops; **X** does the same for the selected star) / **arrow keys** or **right-drag** (Shift slows the arrows) / **mouse wheel**, **Page Up/Down** or **+/-** / **H** flies every leg as one long jump / **Backspace**; **Home** or **C** recentres, and hovering a star opens a panel at the right of the map (the left when the star is over there) with its name and distance, whether it is populated (green) or unpopulated (red), the original map's own text (government, economy such as small mining, allegiance and population), how dangerous it is, its description and the jump it needs ("Insufficient Fuel" or "Out of range" in red). **The keys only move the view: a star is selected by the mouse alone** (hover for its information, click to select it and add it to the route). A click that adds a star the hold has not the fuel to reach is refused, and says so. **F7** (or its console icon) shows the current system's map page, with **F9**/**F10** its economy and people; **I** over a star shows that star's page instead, until F5, F6 or F8 leaves the pages. Click a body in the page's diagram for its facts, as on the 3D map |
| Map page (while the map is open): controls / system map / system description / star sky | **F5 / F6 / F7 / F8** (the icons on the console row do the same; F7 needs a selected star, and a click on a body of its diagram shows that body's facts) |
| System map (F6 on the map: this system; **N** or **R** with a star selected: that star's system) | Click a body to target it, **T** then click to autopilot to it, click the arrows at the lower right or hold **Left/Right** to move the time point, right-drag to turn, **F11/F12** or Page Up/Down to zoom, **C** centres on the picked body, **Home** the star, **F6**/**Delete** back to now; **F5**, **F7-F10** return to the map's own pages |
| Map page: economy / people; zoom in / out (the sky's zoom in picture mode) | **F9 / F10; F11 / F12** |
| Time acceleration 1x/10x/100x/1000x/10000x | **6 7 8 9 0** (or the meter) |
| Pause | **P** (or the meter) |
| Views: cycle / front / rear / turret / external | **F1 / F6 / F7 / F8 / F9** |
| Aim the turret | **A**/**Z** pitch, **,**/**.** yaw, or the held right mouse button |
| External camera: turn / zoom | **arrows** / **+** **-** |
| Status page | **F2** |
| Combat computer (source F11; the port has no view 6 yet) | **F11** (shows the not-ported note) |
| Escape capsule or Stowmaster (if fitted) | **X** (the original's key) or **F12** |
| Preference pages | **Escape** twice: the first press pauses, the second opens them; Escape again returns to the game with time still stopped (the original's `FUNC_000224` order) |
| Starport services at a pad | **F4** (or the console's phone icon), as `F343`; **F1** or **Backspace** returns to the landed view |
| Save / load newest snapshot | **F5** / **Shift+F5** |
| Change ship | **Page Up / Page Down** |
| Docked rows / actions | **Up/Down**, **Enter**, **1**..**4**, **Backspace**, **B**/**N** trade |
| Scroll a station list (market, shipyard, outfitting...): one row up / down | the **up / down arrows** on the list (original), or **Up / Down** keys, or the **mouse wheel** (always on, in every control scheme) |
| Journal / communications pages (docked) | **J** / **C** (port pages; the original journal is the ship library's viewer and the comms list is `F343`'s; see `docs/notes/missions.md`) |
| Quit from the preference pages | click `Quit`, or select it and press Enter |

The galaxy map's F5-F12 are **map-local**: they drive the original's F834/F840
page only while the map is showing. Outside it the global keys are unchanged
(F5 save, Shift+F5 load, F6-F10 views, F11 screenshot, F12/Shift+F12 model
viewer/lab). The `n` places list is not the original map page and keeps the
global keys. Clicking the map's console icons does not
change the flight view while the page is open.

An open-air takeoff is flown, not automatic: the launch action starts F1013's
fixed up-thrust (`0x24`); above 200,000 mm it hands over to `0x26`, and the
starport's `0x20` effect region only clears when the ship flies out of it. Raise
the set speed (**W** Modern, **Enter** Classic) and pitch up (**Up** Modern,
**A** Classic) to leave the pad; F1031 releases the bay when the region is
behind.

Mouse movement is counted in real pixels of pointer movement, the port's stand-in for the
original's mouse counts (JJFFE adds the device's raw relative counts at sensitivity 100,
whatever the window size), so the turn rate does not depend on the window or display scale.
The mouse steering is relative, as F1038 is: while the right button is held each
frame's movement adds to the ship's rates. Releasing the button stops the input
and clears the accumulator, so a later press starts from zero instead of
snapping the ship. In the **prototype** flight model (no O) the same movement is
mapped to a short-lived stick deflection because that model has no rate state;
one full deflection is 200 pixels of real mouse movement. Both paths use the same gate (Classic
only, right button held, ship or external view).

## Modern, full table

| Action | Modern binding |
|---|---|
| Raise / lower the set speed | **W** / **S** |
| Pitch up / down | **Up** / **Down** |
| Yaw left / right | **Left** / **Right** |
| Roll left / right (original flight model) | **Q** / **E** |
| Brake (port addition) | **B** |
| Turret aim | **arrows** |
| Mouse steering | none (the right button does not steer) |

Every other action in the Classic table above is shared: Space, M, T/Shift+T,
K, Backspace, D, L, Tab, N, G, H, C, 6-0, P, F1/F6-F10, F2, F12, Escape, F5,
Shift+F5, Page Up/Down, the docked keys and the left-button console pointer.
In the external view the arrows belong to the camera in both presets; Modern has
no ship steering there, while Classic's keys and right-button mouse still steer.

Both presets honour the original's `Reverse left-right controls` and
`Reverse up-down controls` (`Others`, indices 3 and 4); for Classic this swaps
the yaw keys/axis and the pitch keys/axis, and flips the mouse x and y.

## Switching and persistence

- The two `Advanced Options` rows call `Action::Modern`. The routing reads the new
  value on the next frame, so the change applies immediately. **F3 is the
  source galaxy map**, not a preset switch.
- Switching disarms every held key and resets the mouse accumulator; a key held
  through the switch must be released and pressed again (the same rule as pause
  and focus loss).
- The choice is stored in `saves/config.cfg` as `modern` or `classic`
  (`ffe_rs::settings`). A missing or malformed file falls back to Classic. The
  other preference values remain session-only.

## Display (port extension)

- **Alt+Enter** toggles a borderless full-screen window, in the game and on the
  intro screen (where Enter alone starts the game). The same toggle is the
  `Fullscreen` row on `Escape` -> `Advanced Options` -> `Display Options`.
- The display page also sets window scale 1x-6x, pixel-perfect or fill-window
  fit, nearest or linear filtering and a 4:3 DOS or widescreen picture (the old
  `square` setting is read as widescreen). Choices apply
  immediately (the window is reopened because minifb 0.28 cannot resize live)
  and are stored in `saves/display.cfg`. Full-screen is session-only.
- These are presentation choices only: the 320x200 framebuffer, the simulation
  and every capture are unchanged (`docs/notes/display.md`, `EXP-017`).

## Audit and fixes (2026-10-25)

| Action | Before | Now |
|---|---|---|
| Right-button steering | Not wired at all: the right button did nothing in flight and the original mouse word was always zero | Classic only; F1038 accumulator in original flight, deflection in the prototype; right button held |
| Classic thrust | Enter/Left Shift (right Shift missed in the prototype) | Enter / either Shift; from 2026-11-10 Enter / right Shift only, as F1037 reads |
| Classic yaw/pitch/roll | Comma/period, A/Z, brackets | Same (unchanged, now routed explicitly) |
| Modern thrust/steer | W/S, arrows, Q/E in original flight **plus** all the Classic keys (`both`) | W/S, arrows, Q/E only; Classic keys do nothing |
| Modern mouse | n/a | Right button does not steer |
| Turret aim | Both presets accepted A/Z, comma/period **and** arrows | Modern arrows only; Classic A/Z and comma/period plus mouse |
| Modern external view | Fell back to the Classic comma/period/A/Z keys | Arrows are the camera; no Modern ship steering there |
| Preset switch | Changed `prefs.modern`; held keys were not always cleared; no persistence | Immediate, disarms keys, resets the mouse, persists to `saves/controls.cfg` |
| Page/help text | "Classic-like", stale roll/toggle wording | `docs/CONTROLS.md` and `Advanced Options` name the real scheme; the port's `?` help page was removed, so F1 stays the original view cycle |

No action was duplicated between the presets after the change. `Enter` and
`Shift` are still shared with the menu/station contexts and Shift+T /
Shift+F5, but the flight branch only sees them when no list, page or overlay is
open, so these are context chords rather than preset leakage. `X`, Space, M, T,
K, Backspace, D, L, Tab, N, G, C, F1-F10, F12, F5, Shift+F5, Page Up/Down, 6-0,
P and the left-button pointer are intentionally shared. F11 is the source
combat-computer key and has no action until view 6 is ported.

## Tests

- `crates/ffe-rs/src/input.rs`: per-preset routing, Classic Enter/Shift thrust,
  no Classic keys in Modern and no Modern keys in Classic, right-button steering
  only in Classic, reverse options, the mouse accumulator, immediate preset
  switching through `Prefs`, the source view keys (F1 cycle, F6-F9 direct,
  F10 missile), and the developer-command gate (nothing without `--dev`, then
  Ctrl+F11/F12 and O only).
- `crates/ffe-rs/tests/views.rs`: `f1_cycles_front_rear_turret_external` and the
  optional `f1_cycles_the_view_without_painting_the_removed_help_page` frame
  pair (`FFE_WORLD_CACHE` + `FFE_RS_CLEANUP_CAPTURE`).
- `crates/ffe-rs/tests/controls.rs`: 60-frame flights showing Classic
  Enter+right-button moving the ship and Modern ignoring the same input; an
  optional frame/input strip (set `FFE_RS_CONTROLS_CAPTURE`).
- `crates/ffe-rs/tests/original_flight.rs`: the F1038 mouse word turns the
  original model, and a disabled mouse is ignored without a jump.
- `crates/ffe-rs/tests/prefs.rs`: the Advanced Options rows, including the radio
  marks on the page and no F3 preset route.
- `crates/ffe-rs/src/settings.rs`: the `controls.cfg` round trip and malformed
  file fallback.
- `crates/ffe-rs/src/input.rs`: the `bind.*` action vocabulary (names, the keys
  each action delivers in both presets, unknown names ignored) and a remap
  changing the action `flight_controls` delivers. `crates/ffe-rs/src/session.rs`:
  the frame-level key/action remap and the recorded-config replay.
- `crates/ffe-rs/src/display.rs` and `tests/prefs.rs`: fit rectangles, nearest
  and linear scaling, mouse mapping, the display page and the `display.cfg`
  round trip (`docs/notes/display.md`).
- `crates/ffe-rs/tests/playable.rs`: the retired help page draws no frame.
- Evidence: `docs/evidence/controls/README.md`, `docs/evidence/display/` and
  `docs/evidence/cleanup/` (F1 front/rear pair).

## Removed port-only UI (2026-10-25)

The player-facing build no longer has the `?` help page, the playtest scenario
menu, the F11 screenshot, the F12 model viewer/Shift+F12 model lab or the O
prototype-flight toggle. These were port additions, not source behaviour. F1 is
the original view cycle again, F11 is left for the source combat computer (view 6
is not ported yet) and F12 is the source escape-capsule action (since
2026-10-28 X, the DOS key, launches it too, and the brake is on B). The port's
`PAUSED` overlay (with `P Resume` / `F2 status`) is also gone: pausing only stops
time, as F224 does, and the game frame stays as it was.

The developer capability is still available behind `--dev` or `FFE_DEV=1`:
Ctrl+F11 screenshot, Ctrl+F12 model viewer, Ctrl+Shift+F12 model lab and O
prototype flight. The CLI `--capture*`, `--smoke-frames`, `--skip-intro` and
`--start 0..5|sol` inputs are unchanged for the oracles and evidence runs.

## Remaining differences and owner decisions

- The original has no separate "Modern" scheme; Modern is a port addition and
  keeps its own keys. No remapping UI.
- The prototype flight model has no roll axis, so Modern's Q/E roll only has an
  effect on the original flight model (as before).
- The original's `Elite control method in space` and the other unimplemented
  `Others` rows do not change the scheme; only the two Reverse rows do.
- Mouse sensitivity is one logical pixel per F1038 count. The original scales
  DirectInput mickeys, so this may need a sensitivity option after an owner
  playtest.
- `saves/config.cfg` (version 2) is the persisted preference file; it holds the
  preset, display choices and the other applied pages. `saves/controls.cfg` and
  `saves/display.cfg` are read once as a migration. The original's commander-file
  preference block is not imported. The `bind.*` lines are the remapping table
  (no in-game remap screen); see [Remapping keys](#remapping-keys).

## Remapping keys

`saves/config.cfg` takes two forms. Both are applied to every frame's input
before any screen or flight code sees it (`session::bindings` then
`session::Frame::remap`), so a binding covers the whole game, and a replay
applies the same bindings from its recorded config.

**Game keys** — `bind.<game key>=<physical key>`, with minifb's key names (`A`,
`Up`, `LeftShift`, `NumPad8`, `F1`, ...). Each makes the physical key act as the
game key, for example:

```
bind.A=Up
bind.Z=Down
```

**Actions** — `bind.<action>=<physical key>`, with the action names below. An
action follows the preset, so one entry keeps working after `controls=` changes:
`bind.pitch_up=NumPad8` pitches up with Modern's Up and Classic's A. The two
presets' steering and thrust keys are disjoint, so delivering both cannot
double an axis.

| action | game keys it delivers |
|---|---|
| `thrust_up` / `thrust_down` | Modern W, Classic Enter / Modern S, Classic right Shift |
| `pitch_up` / `pitch_down` | Modern Up, Classic A / Modern Down, Classic Z |
| `yaw_left` / `yaw_right` | Modern Left, Classic `,` / Modern Right, Classic `.` |
| `roll_left` / `roll_right` | Modern Q, Classic `[` / Modern E, Classic `]` |
| `brake` | B |
| `fire` | Space |
| `missile` | M |
| `ecm` | E, `;`, Insert |
| `chaff` | C |
| `target` | T (Shift+T stays the previous/combat toggle) |
| `autopilot` | K |
| `stop_autopilot` | Backspace |
| `docking` | D |
| `launch` | L, Tab (Tab launches on a pad or the surface) |
| `navigation` | N |
| `galaxy_map` | F3, G |
| `hyperspace` | H |
| `pause` | P |
| `status` | F2 |
| `view_cycle` | F1 |
| `view_front` / `view_rear` / `view_turret` / `view_external` / `view_missile` | F6 / F7 / F8 / F9 / F10 |
| `combat_view` | F11 |
| `escape_capsule` | X, F12 |
| `map_centre` | C |

A remapped physical key no longer acts as itself. An unknown action or key name
is ignored and left in the file, so a hand edit with a typo changes nothing.
The bindings are read at start-up; there is no remapping screen. Recordings
keep the raw keys and the config, so a replay applies the same bindings.

## Game controllers (Windows)

A connected game pad (XInput or any pad `gilrs` supports) is read every frame
and turned into the active control preset's keys, so it works everywhere the
keyboard does and is recorded with `--record`:

| pad | action |
|---|---|
| left stick | pitch and yaw |
| right / left trigger | faster / slower |
| right / left bumper | roll right / left |
| A (south) | fire |
| B (east) | launch a missile (M) |
| X (west) | combat target (T) |
| Y (north) | next view (F1) |
| d-pad up / down | time acceleration up / down |
| Start | the Escape pages |
| Back / Select | the galaxy map (G) |

Linux builds do not open pads (the backend needs udev); `ffe_rs::gamepad`
documents the mapping.
