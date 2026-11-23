# Optional enhancements (launcher toggles)

Owner request (2026-11-20): a set of optional, toggleable improvements to the
like-for-like port. Each is a single toggle in the launcher under an
"Enhancements" page, stored in `saves/config.cfg` (session/config scope), never
in the commander save.

## Rules (owner, 2026-11-20)

1. Every enhancement is a clear on/off toggle.
2. Toggling it mid-game, or saving/loading, never changes or corrupts game state.
   So enhancements must be applied at a safe boundary (input/frame/render), and
   their state lives in `config.cfg`, not the save.
3. Defaults are decided per item; bug-fix items may default on, balance/QoL items
   off, but each stays individually toggleable.

## The items

The launcher groups these under four headers: **Combat & missiles**,
**Autopilot**, **Mission fixes** and **Combat display**. The `enhancement-n`
numbers are the original request numbers and are not the display order.

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

## Implementation convention

### Keys

The launcher's **Settings → Enhancements** tab shows one toggle per item, grouped
under the four headers above. Each writes `enhancement.<name>=on|off` into
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

`enhancement-10` also closes its list and drops the ship selected through it
(`nav.ship_lock`/`nav.missile_lock`) when the player leaves flight, opens a page,
changes the view the list was opened in, or clicks the button again. Locks the
player already had are left alone, and the teardown is gated on the toggle.

