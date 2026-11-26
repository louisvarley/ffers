# Optional enhancements (launcher toggles)

Owner request (2026-11-20): a set of optional, toggleable improvements to the
like-for-like port. Each is a single toggle in the launcher under an
"Enhancements" page, stored in `saves/config.cfg` (session/config scope), never
in the commander save. The step-by-step recipe for adding a new one is
[adding-enhancements.md](adding-enhancements.md).

## Rules (owner, 2026-11-20)

1. Every enhancement is a clear on/off toggle.
2. Toggling it mid-game, or saving/loading, never changes or corrupts game state.
   So enhancements must be applied at a safe boundary (input/frame/render), and
   their state lives in `config.cfg`, not the save.
3. Defaults are decided per item; bug-fix items may default on, balance/QoL items
   off, but each stays individually toggleable.

## The items

The launcher groups these under headers: **Combat enhancements** (the items in
the Combat & missiles and Combat display sections below), **Flight**,
**Autopilot**, **Missions** (the mission fixes and the tracker), **Galaxy map**,
**Hyperspace** and **Stations** (the price guide and the autosave). The
`enhancement-n` numbers are the original request numbers and are not the
display order.

### Combat & missiles

- `enhancement-1` — **Turrets turn while time is paused.** When time is paused the
  turrets currently cannot turn; make them turn while paused.
- `enhancement-2` — **Missiles ignore their launcher's collision on launch.**
  Disable ray-collision for the missile against its own launcher.
- `enhancement-6` — **Missiles: swept (continuous) collision detection.** Missiles
  do little damage at close range (tunnelling: a fast projectile steps past a thin
  target between two collision checks). Swept collision would fix it without
  changing any saved state.
- `enhancement-8` — **ECM can disable NN500 missiles.** Currently the NN500 series
  cannot be stopped by ECM.
- `enhancement-13` — **Fixed laser sight.** In the first-person views (front,
  rear and the two turrets) draw the laser as a clean line from the foot of the
  view to the reticle — the default Saker's look — instead of the beam from the
  gun mount. The line is drawn in the fitted laser's own colour (the beam model's ray:
  red, orange, white, yellow, blue or green by laser type). The external (third-person)
  view is unchanged.

### Autopilot (three toggles)

- `enhancement-3` — **Offset waypoint around intervening bodies.** Before each leg,
  test the flight path against the collision spheres of nearby bodies and add an
  offset waypoint if it hits one.
- `enhancement-4` — **Braking accounts for retro thrust + gravity, step-independent.**
  The current stopping distance is calculated without the weaker retro thrust and
  the planet's gravity, so the outcome changes with time acceleration and the
  autopilot can crash on approach. Calculate the braking point from actual retro
  thrust minus local gravity, independent of the time step. (Check the JJFFE source.)
- `enhancement-5` — **Collision test against every body in range.** Test collisions
  against every body in range, not just the current reference body.

### Mission fixes (one toggle covering all of these)

- `enhancement-7` — **Mission fixes** (from Jades' known-issues list; check which
  are already fixed before implementing):
  - Assassination kills not credited (Rast/Innitu destroyed after hyperspacing or
    in deep space).
  - Hitch-hiker mission marked incomplete / contradictory messages.
  - Early Federal and Imperial medals never appear (both v1.06 versions).
  - Hidden deadline at Alioth (~30 Dec 3252) not stated — safest fix is UI-only:
    show the deadline in the briefing, leave the mission logic alone.
  - Blocked docking bays (Thargoid station, Polaris shuttle) — make the occupying
    NPC launch/move off when the player is waiting.
  - YES/NO buttons stop responding in the Thargoid station (accelerated time) —
    block time acceleration or queue input while a mission dialogue is open.
  - Missing assassination targets (starport full) — reserve a slot for mission NPCs.

**What was already right:** the medal *award* path is source-correct
(F307's bit test, statement 53's side/points/medal and `Career::award_medal`
all match `ffe.asm:24395-24628`), and the broken-mission data fixes from
`gameplay-42` (missions 78, 55, 20, 83 and 81) remain always-on and separate
from this toggle. The port also never dropped a mission NPC for a full
starport (`Action::Departure` ignored bays), so the original's "missing
target" could not happen — but the target also never occupied a bay. The
runtime fixes add the missing behaviours; the Alioth item is the only
UI-only change and names the date without touching mission state. See
EXP-046.

### Combat display

- `enhancement-9` — **NPC relative speed.** When an NPC is selected, show its
  relative speed along with its hull and shield.
- `enhancement-10` — **Combat Targeting Computer.** Adds a dashboard button for
  seeing local ships and auto targeting them. The port-only button (the embedded
  `npc-targeting.png`) takes the console slot of the escape-capsule icon at
  (237,174), the system map's zoom-out button, so it lines up with the dashboard;
  F12 and X still launch the escape craft. It opens the navigation computer's
  own list style over the flight view for all NPC ships within 50 km: grey
  marker bitmaps and names, nine rows in each of two columns, no separate
  screen. Names are red for pirates/hostiles, blue for non-hostile police and
  green for neutrals; clicking a row locks that ship, returns to the flight
  view and puts a red square round it (the ship's marker turns red in the
  list). `enhancement-11`'s off-view arrow is a smooth solid arrow with a dark outline.
- `enhancement-11` — **Edge target indicator.** An arrow/bar showing the direction
  to the target when it is out of view.

### Galaxy map

- `enhancement-12` — **Enhanced galaxy map.** Replaces the original's 3D star map
  with a flat, easier-to-read top-down starfield (stars lower in the galaxy are
  drawn smaller for a depth cue), and adds: right-drag pan, middle-wheel zoom,
  hover notes for a system, pointer lines from the
  middle of the view to Sol and to the player's own system (with the distance
  when it is off view), `Home`/`C` recentre, and multi-step routes. **A click on a
  star selects it and adds it to the route; a second click takes it off again**
  (`X` does the same for the selected star). **A star beyond one jump gets a plotted
  course:** the computer finds the most efficient stars to hop through (the least
  fuel, then the fewest jumps, then the shortest, preferring inhabited and quieter
  stops; as far as the fuel aboard allows, with a 40-jump guard on the search) and adds
  them as numbered stops, leaving the view where it is; clicking the star again takes
  the stops it plotted off with it. The route is drawn as rings and a line from the current system through each
  waypoint, a leg the range or the fuel cannot cover in red, with a total
  distance and fuel line (red when it cannot be flown). **The arrow keys pan the
  map** (Shift slows them, as on the 3D map), as does the right-drag; the wheel,
  Page Up/Down and `+`/`-` zoom. **Hovering a star says** whether anyone lives
  there (a green "Populated" or a red "Unpopulated", with its faction) and, for a
  populated one, how dangerous it is to arrive in, then the fuel the jump needs from
  where the route has got to; for a star beyond one jump it plots the course there and
  then and says "Course: 3 jumps, 3t of fuel" in white, and "Out of range" in red
  only when the fuel aboard cannot pay for it ("need 25t, have 20t") or no chain of
  stars reaches it; the console panel names the selected star's
  danger too. `H` flies the whole
  route **as one jump**: every leg is checked first, the legs' jump times run
  together as a single transit, and the tunnel and the hyperspace sound last for
  the distance flown (2.5 s + 0.22 s per light year + 0.5 s per extra leg, up to
  20 s), where the original plays about 0.1 s a light year. The extra time is
  more tunnel at the original's speed, not a slower one: the opening and closing
  frames play once and the swirl between them repeats. A single selected star
  with no route keeps the original `H`/Enter jump (with the same lengthening).
  The hyperspace icon and `H` follow a flyable course: with a valid route marked
  (open on the map or left when it closed) the icon shows and `H` flies it, even when
  the star the map points at is out of range; an unflyable route says which leg and
  why (a leg longer than the drive reaches, or the fuel the whole course needs against
  the hold). The map draws every star in view at any zoom (it used to draw only the
  nine sectors around the centre). The course does not plan refuelling stops: when the
  hold is short it says so.
  Danger is the system record's patrol byte, the value F688 draws a new arrival's
  pirate groups from (`RND(patrol * 4)` groups): none is "Safe", 1-2 "Low", 3-4
  "Moderate", 5 and over "High".

### Station screens

- `enhancement-14` — **Mouse-wheel scrolling in station lists.** The wheel scrolls
  the market, shipyard, outfitting and the other station lists a row a notch
  (wheel up shows earlier entries, as the up arrow does), and the keyboard row
  stays on screen. The arrows themselves are not part of the toggle: both move
  one entry, as the original's F355(±1) does (the port's down arrow used to page
  a whole window, so a few clicks ran to the end of the market).

### Hyperspace

- `enhancement-15` — **Elite Dangerous Hyperspace** (the launcher's name; earlier
  "Streaking hyperspace"). While a jump runs, the view shows the
  stars and dust streaking past (in the manner of Elite Dangerous) instead of the
  original's tunnel, over the player's space colour: the view that was showing (its
  stars, nebula and bodies) stretches into streaks and rushes outwards as the speed
  gathers over 2.2 s, with more stars fading in among them, while space brightens round
  a black hole in the middle; through the cruise the hole grows a little and the
  brightness and streaks dim; over the last 2.2 s they slow, and a bright flash brings
  the new system, whose stars and nebula draw in from the edges and settle over 2 s as
  the flash clears and the field fades (the departure in reverse). H first charges the
  drive for 5 s, with "CHARGING" over a red `XXX.XXX` countdown (H again cancels); once
  the jump is up to speed, "ARRIVAL IN" counts down to the exit. The jump plays for
  7 s plus 0.35 s a light year (half a second more a leg on a route), between 9 and
  45 s; the game time the jump takes is unchanged. Its sound is the
  owner's three clips (`hyperspace_sound`): the ramp up plays as the charge starts (its
  first 5 s are the charge's lead-in, then its rise is the 2.2 s speed-up; a cancelled
  charge stops it), the ramp down is timed to end with the jump (its fall is
  the 2.2 s slow-down), and the drone fills the gap, fading in over the last second of
  the ramp up and out over the first second of the ramp down. The clips are the owner's
  own WAV files, embedded in the game (`crates/ffe-rs/assets/hyperspace/`); should one
  fail to decode, the original sample repeats as before. Off, the original tunnel, sound
  and timing are kept.

### Ideas from modern games (2026-11-26)

- `enhancement-16` — **Velocity markers** (launcher group Flight). F1112's velocity
  marker (bitmap 98, the direction the ship is moving) is drawn in every
  first-person view (front, rear and both turrets), not only the front, and a
  retrograde marker (an amber ring and cross) sits opposite it: point the nose
  there to brake. Both stay inside F1112's own bounds and are not drawn when off
  the view; the retrograde one is not drawn when the ship is still.
- `enhancement-17` — **Mission tracker** (group Missions). The most urgent active
  contract (earliest deadline) is shown at the top of the flight view, centred
  between the shields/hull text and the target's readouts: "Mission: <place>,
  <system>" and then the jumps (a straight-line count at the drive's range, or "In
  system") and the time left to the end of the deadline day ("3d 04h left",
  "Overdue" in red). A military photograph or bombing that has been carried out is
  tracked back to the system it was accepted in. On the enhanced galaxy map every
  active contract's destination has an amber diamond and a "Mission: <system>"
  label, or an amber arrowhead at the edge pointing to it when off view; clicking
  the star plots a course there as any star does.
- `enhancement-18` — **Autosave on docking** (group Stations). Each docking with an
  orbital station and each landing at a starport writes `saves/autosave.json`, the
  same save the save key writes, replacing the last autosave. "autosave" is a
  reserved save name, and the load and delete lists put it straight after the
  quick save. A dead commander is never saved. The log says "Game saved
  automatically." (or why it failed).
- `enhancement-19` — **Market price guide** (group Stations). After each price the
  market draws a green down arrow when the price is more than 10% below the
  commodity's galactic average, an orange up arrow when it is more than 10% above,
  and a grey dash in between; the selected commodity's average and the difference
  ("$12.3  +15%") show at the lower left above the cargo block. The average is the
  commodity's base price (`DATA_006169`): F619's price with no supply variance in a
  danger-0 system, whose 0.5 multiplier cancels F619's doubling. Dangerous systems,
  local supply and the black market's doubling move a market off it.

## Implementation convention

### Keys

The launcher's **Settings → Enhancements** tab shows one toggle per item, grouped
under the headers above. Each writes `enhancement.<name>=on|off` into
`saves/config.cfg` (never into a commander save). Writing a config never removes
the other settings: the launcher loads `config.cfg` with the game's own reader and
saves the whole file with its writer, so `bind.*`, display and preference pages
are preserved. The page saves as soon as a checkbox changes, while the game is not
running (the existing Settings page is disabled during play; the game writes
its own options on exit).

| Item | Key | Field in `settings::Enhancements` | Default |
|---|---|---|---|
| `enhancement-1` | `enhancement.turrets_paused` | `turrets_paused` | off |
| `enhancement-2` | `enhancement.missile_launch` | `missile_launch` | off |
| `enhancement-3` | `enhancement.autopilot_waypoint` | `autopilot_waypoint` | off |
| `enhancement-4` | `enhancement.autopilot_braking` | `autopilot_braking` | off |
| `enhancement-5` | `enhancement.autopilot_collisions` | `autopilot_collisions` | off |
| `enhancement-6` | `enhancement.missile_swept` | `missile_swept` | off |
| `enhancement-7` | `enhancement.mission_fixes` | `mission_fixes` | off |
| `enhancement-8` | `enhancement.ecm_nn500` | `ecm_nn500` | off |
| `enhancement-9` | `enhancement.npc_speed` | `npc_speed` | off |
| `enhancement-10` | `enhancement.npc_labels` | `npc_labels` | off |
| `enhancement-11` | `enhancement.edge_indicator` | `edge_indicator` | off |
| `enhancement-12` | `enhancement.enhanced_galaxy_map` | `enhanced_galaxy_map` | off |
| `enhancement-13` | `enhancement.laser_sight` | `laser_sight` | off |
| `enhancement-14` | (was `enhancement.list_scroll`) | always on since 2026-11-25; the line is ignored | n/a |
| `enhancement-15` | `enhancement.hyperspace_streaks` | `hyperspace_streaks` | off |
| `enhancement-16` | `enhancement.velocity_markers` | `velocity_markers` | off |
| `enhancement-17` | `enhancement.mission_tracker` | `mission_tracker` | off |
| `enhancement-18` | `enhancement.autosave` | `autosave` | off |
| `enhancement-19` | `enhancement.market_average` | `market_average` | off |

Values are `on|off` when written; `on|off|true|false|1|0` are accepted when
read. A missing key keeps the default (off). An unknown `enhancement.*` name is
ignored and is not written back. A known key with a bad value keeps that item's
default without rejecting the rest of the file.

### Game side

`settings::load_config` fills `Config::enhancements` at start-up
(`crates/ffe-rs/src/settings.rs`); read a toggle as
`config.enhancements.<field>`. The in-game options save path
(`config_from_prefs` in `main.rs`) copies the block through, so the game's own
options never clear the launcher's choices. Each behaviour agent reads its
field at the frame/input/render boundary where the behaviour lives (autopilot,
missile tick, mission dialogue, HUD) and must leave the original path
byte-for-byte when the toggle is off. Each item also carries a `DEVIATIONS.md`
entry (`EXP-xxx`, `enhancement`) and a regression test.

### Implemented

| Item | Where | Behaviour when off |
|---|---|---|
| `enhancement-1` `turrets_paused` | `main.rs` paused frame, `camera::turret_aims_when_paused` | paused input block is skipped exactly as before |
| `enhancement-2` `missile_launch` | `ffe_core::missile::TickOptions::ignore_launcher`, `main.rs` missile tick | off keeps F575's launcher skip in F718's point contact test (the original path, so `M` flies); on also keeps the launcher out of `enhancement-6`'s swept ray |
| `enhancement-9` `npc_speed` | `combat_computer::relative_speed_line`, `screen::Screen::target_speed` | no extra readout line |
| `enhancement-7` `mission_fixes` | `mission_fixes` module, `navigation::{request_clearance,reserve_script_bay,script_ship_id}`, `scripts::{next_ship_id,apply,expand_at}`, `screen::draw_profile`, `main.rs` system/view/dialogue boundaries | no id serial, no bay eviction, no F1125 pulse, three medals, no deadline text, `Full` stays `Full`, acceleration unchanged, departures get no bay |
| `enhancement-10` `npc_labels` | `combat_computer::{npc_labels, label_colour, npc_button, npc_row, close_on_leave}`, `main.rs` NPC button/list, `screen::{npc_button,npc_list_open,draw_npc_list}` | no button and no list; `l` is the original text-label toggle, no target changes |
| `enhancement-11` `edge_indicator` | `targeter::edge_indicator`, `screen::Screen::edge_indicator` | no arrow pixels; F1100's text forms unchanged |
| `enhancement-12` `enhanced_galaxy_map` | `enhanced_map` module, `screen::{EnhancedGalaxy, draw_enhanced_galaxy}`, `main.rs` map open/input/draw/route | the original 3D `GalaxyMap` path is unchanged; the flat map, route and Sol pointer are only built when the toggle is on |
| `enhancement-13` `laser_sight` | `screen::{Screen::fixed_laser, first_person_view}`, `main.rs` | off keeps F1776's beam from the gun mount; on draws a fixed foot-to-reticle sight line in the front, rear and turret views only |
| `enhancement-15` `hyperspace_streaks` | `hyperspace_streaks` module, `navigation::{Transit::real_seconds, Navigator::stretch_transit_to}`, `screen::Screen::hyperspace_streaks`, `main.rs` jump/sound/draw | F1066's tunnel (model 0xa5) and the original transit length; nothing of the streak field is drawn |
| `enhancement-16` `velocity_markers` | `screen::{Screen::velocity_markers, draw_retrograde_marker}`, `combat::velocity_marker` | F1112's marker in the front view only, no retrograde marker |
| `enhancement-17` `mission_tracker` | `mission_tracker` module, `enhanced_map::mission_marks`, `screen::{Screen::mission_tracker, draw_mission_tracker}`, `main.rs` flight HUD and map scene | no tracker lines and no map marks; the contracts are only read |
| `enhancement-18` `autosave` | `saves::{save_auto, auto_path, AUTO_SAVE}`, `main.rs` docking (`Traffic::Docked`) and starport landing | no file is written; `autosave` stays a reserved name |
| `enhancement-19` `market_average` | `trade::{galactic_average, trend, percent_text}`, `screen::{MarketRow::average, draw_market_averages}` | the market rows are drawn as before |
| `enhancement-14` (always on since 2026-11-25) | `station::DockState::{scroll_by, arrow, wheel}`, `main.rs` docked-screen input | the wheel always scrolls station lists (it is no longer a toggle); the arrows are the original's one-row F355(±1) either way |

`enhancement-10` also closes its list and drops the ship selected through it
(`nav.ship_lock`/`nav.missile_lock`) when the player leaves flight, opens a page,
changes the view the list was opened in, or clicks the button again. Locks the
player already had are left alone, and the teardown is gated on the toggle.

