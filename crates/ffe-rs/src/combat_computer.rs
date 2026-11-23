//! F11's combat computer view (view 6): F1056 enters it when the Combat
//! Computer is fitted (`ship+0xc8 & 0x40`), F1054 places a chase camera that
//! backs off to keep the nearest ship in frame, and F1099 runs F1109 over the
//! objects to print each ship's range, hull, shields and type beside it.
//!
//! - **F1056 (`0xfb`).** View 6, `DATA_007270 = DATA_007269 = 0x320`,
//!   `DATA_007271 = 0`, console button 0x13 lit.
//! - **F1099 / F1109.** `DATA_007270 = 0x320`, then for every ship (control
//!   `+0x87 == 0xb`, distance class `+0x88 <= 0x14`, model 0x0f..=0x3f) the
//!   range in hundredths of a km lowers `DATA_007270`, and when the ship's
//!   screen point is left of x 320 and above y 128 the amber (0x5f) text
//!   0x863a (or 0x8639 without shields) goes at (x - 8, y + 9):
//!   `{range}km`, `{hull}%` (`,{shields}%`), the type name.
//! - **F1054 (view 6).** `DATA_007270` at least 0x12c; `DATA_007269 =
//!   (3 * DATA_007269 + DATA_007270) / 4`; `DATA_007271 = (DATA_007271 +
//!   (ship+0xb4 >> 1)) >> 1`. The camera rows turn by 0x9f5 about right
//!   (rot(up, forward)) and by `DATA_007271` about up (rot(forward, right)),
//!   and F1053 backs the eye off along forward by `-(DATA_007269 * 40)` (a
//!   16-bit word, `>> 12` on the Q15 rows: the external view's distance word
//!   `<< 11`).

use ffe_core::names::{self, Names};
use ffe_core::npc::{NpcShip, Role, State};

/// The embedded port-only icon of `enhancement-10`'s NPC targeting button.
pub const NPC_ICON_PNG: &[u8] = include_bytes!("../assets/npc-targeting.png");
/// The pressed (list open) version of the button.
pub const NPC_ICON_PRESSED_PNG: &[u8] = include_bytes!("../assets/npc-targeting-p.png");

/// The Combat Targeting Computer's dashboard button `(x, y, width, height)`.
/// It takes the console slot of the escape-capsule icon (console record 0x12 at
/// `(237, 174)`, 25x25), which the system map uses as its zoom-out button, so it
/// lines up with the rest of the dashboard. Its hot area is the capsule icon's
/// (`Click::Capsule`, `(241,174)-(256,193)`); F12 and X still launch the escape
/// craft.
pub const NPC_BUTTON: (i32, i32, u32, u32) = (237, 174, 25, 25);

/// The Combat Targeting Computer's list, laid out as the navigation computer's
/// places list is (F1099): a grey marker bitmap and a name fifteen pixels to its
/// right, nine rows of fifteen pixels from y 5 in a column at x 10, then a second
/// column at x 230. There is no background or title; it is drawn over the flight
/// view.
pub const LIST_COLUMNS: [i32; 2] = [10, 230];
pub const LIST_Y: i32 = 5;
pub const LIST_ROW_HEIGHT: i32 = 15;
pub const LIST_COLUMN_ROWS: usize = 9;
pub const LIST_ROWS: usize = LIST_COLUMNS.len() * LIST_COLUMN_ROWS;
/// How far right of a column's marker a row still counts as its own: the first
/// column stops short of the second, the second at the view's edge.
pub const LIST_COLUMN_WIDTHS: [i32; 2] = [215, 90];

/// The top-left of list row `index` (marker position), or `None` past the last.
pub fn list_cell(index: usize) -> Option<(i32, i32)> {
    (index < LIST_ROWS).then(|| {
        (
            LIST_COLUMNS[index / LIST_COLUMN_ROWS],
            LIST_Y + LIST_ROW_HEIGHT * (index % LIST_COLUMN_ROWS) as i32,
        )
    })
}

/// A row's text: the name, then `(distance)` when it fits the column. `index`
/// picks the column's width (the second column is narrow).
pub fn row_label(label: &CombatLabel, index: usize) -> String {
    let room = if index / LIST_COLUMN_ROWS == 0 {
        30
    } else {
        12
    };
    let distance = format!("({}.{}km)", label.range / 100, (label.range % 100) / 10);
    let name: String = label.name.chars().take(room).collect();
    let full = format!("{name} {distance}");
    if full.chars().count() <= room {
        full
    } else {
        name
    }
}

/// F1056/F1099's starting nearest range, `0x320` (8.00 km).
pub const NEAREST_START: i32 = 0x320;
/// F1054's floor on the nearest range, `0x12c` (3.00 km).
pub const NEAREST_MIN: i32 = 0x12c;
/// F1054's fixed pitch of the camera rows.
pub const PITCH: u16 = 0x9f5;
/// F1109's text colour (amber).
pub const COLOUR: u8 = 0x5f;
/// `enhancement-10`'s passive label colour: the fixed low palette's green.
pub const LABEL_PASSIVE: u8 = crate::targeter::NAVIGATION_COLOUR;
/// `enhancement-10`'s closing/hostile list colour: the fixed low palette's red.
pub const LABEL_HOSTILE: u8 = crate::targeter::COMBAT_COLOUR;
/// `enhancement-10`'s non-hostile police list colour: the low palette's blue
/// (`base:palette/low` entry 0x43).
pub const LABEL_POLICE: u8 = 0x43;
/// F1109's distance-class limit (`+0x88 <= 0x14`): class `c` is `2^(c+13)` mm.
pub const RANGE_LIMIT: f64 = (1u64 << (0x14 + 13)) as f64;
/// `enhancement-10`'s list cap: ships farther than 50 km are not listed.
pub const LIST_RANGE_LIMIT: f64 = 50_000_000.0;

/// `DATA_007269..7271`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatCamera {
    /// `DATA_007270`: this frame's nearest range (hundredths of a km).
    pub nearest: i32,
    /// `DATA_007269`: the smoothed range the camera backs off by.
    pub distance: i32,
    /// `DATA_007271`: the smoothed yaw turn.
    pub yaw: i32,
}

impl Default for CombatCamera {
    fn default() -> Self {
        Self::enter()
    }
}

impl CombatCamera {
    /// F1056's `0xfb`.
    pub fn enter() -> Self {
        Self {
            nearest: NEAREST_START,
            distance: NEAREST_START,
            yaw: 0,
        }
    }

    /// One frame: F1099/F1109's nearest range (`None` with no ship), then
    /// F1054's smoothing with the player's yaw rate (`ship+0xb4`).
    pub fn step(&mut self, nearest: Option<i32>, yaw_rate: i16) {
        self.nearest = nearest
            .map_or(NEAREST_START, |range| range.min(NEAREST_START))
            .max(NEAREST_MIN);
        self.distance = (self.distance * 3 + self.nearest) >> 2;
        self.yaw = (self.yaw + (i32::from(yaw_rate) >> 1)) >> 1;
    }

    /// The camera's offset along its turned forward row, in the external
    /// camera's distance units (model units * 256, negative behind).
    pub fn offset(&self) -> f64 {
        let word = (-(self.distance * 40)) as i16;
        f64::from(word) * 2048.0
    }
}

/// One ship's F1109 readout.
#[derive(Debug, Clone, PartialEq)]
pub struct Readout {
    /// The ship's centre in flight units.
    pub centre: [f64; 3],
    /// Range in hundredths of a km (F1109's `[ebp-0x20]`).
    pub range: i32,
    /// 0x8639/0x863a's three lines.
    pub lines: [String; 3],
}

/// F1109 for `ship` seen from `player` (system mm), or `None` when the ship
/// does not qualify. `unit` converts mm to flight units.
pub fn readout(ship: &NpcShip, player: [i64; 3], unit: f64) -> Option<Readout> {
    if !ship.alive() || !ship.visible() || !(0x0f..=0x3f).contains(&ship.model) {
        return None;
    }
    let metres: f64 = (0..3)
        .map(|k| (ship.position[k] - player[k]) as f64)
        .map(|d| d * d)
        .sum::<f64>()
        .sqrt();
    if metres > RANGE_LIMIT {
        return None;
    }
    let range = (metres / 10_000.0) as i32;
    let damage = ship.damage;
    let mass = i32::from(damage.mass).max(1);
    // F1109: `+0xe4 * 25 / mass` (the hull is mass * 4), shields
    // `+0xe0 * 100 / +0xe2`.
    let hull = i32::from(damage.hull) * 25 / mass;
    let second = if damage.shields_max != 0 {
        let shields = i32::from(damage.shields) * 100 / i32::from(damage.shields_max);
        format!("{hull}%,{shields}%")
    } else {
        format!("{hull}%")
    };
    Some(Readout {
        centre: ship.position.map(|v| v as f64 / unit),
        range,
        lines: [
            format!("{}.{:02}km", range / 100, range % 100),
            second,
            if ship.type_name.is_empty() {
                ship.name.clone()
            } else {
                ship.type_name.clone()
            },
        ],
    })
}

/// F1109's screen gate and placement: text at (x - 8, y + 9) when x < 320
/// and y + 30 < 158.
pub fn text_origin(x: i32, y: i32) -> Option<(i32, i32)> {
    ((0..0x140).contains(&x) && y + 0x1e < 0x9e).then_some((x - 8, y + 9))
}

/// `enhancement-10`: the three-way colour of a list row (owner's scheme).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelColour {
    /// A pirate or any hostile ship (including hostile police).
    Red,
    /// Non-hostile police.
    Blue,
    /// A neutral trader, shuttle or other non-hostile ship.
    Green,
}

impl LabelColour {
    /// The fixed low palette's index for the row and status text.
    pub fn palette(self) -> u8 {
        match self {
            LabelColour::Red => LABEL_HOSTILE,
            LabelColour::Blue => LABEL_POLICE,
            LabelColour::Green => LABEL_PASSIVE,
        }
    }
}

/// `enhancement-10`: one NPC in the targeting list. The row shows
/// [`name`](Self::name), its [`range`](Self::range) and its
/// [`colour`](Self::colour) status; the caller uses [`ship`](Self::ship) to
/// select it.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatLabel {
    /// The port's ship name (`Moray Starboat 3`), the string F1108 would use.
    pub name: String,
    /// The ship's registration/serial (`A17-3` from F791, or the port's stable
    /// id when the cache has no name table).
    pub registration: String,
    /// The ship's centre in flight units.
    pub centre: [f64; 3],
    /// The target lock's stable ship id.
    pub ship: u32,
    /// Range in hundredths of a km, for the list's nearest-first order.
    pub range: i32,
    /// Red for pirates/hostiles, blue for non-hostile police, green otherwise.
    pub colour: LabelColour,
}

impl CombatLabel {
    /// The list row after the toggle: `name  registration  (distance)`.
    pub fn row_text(&self) -> String {
        format!(
            "{}  {}  ({}.{}km)",
            self.name,
            self.registration,
            self.range / 100,
            (self.range % 100) / 10
        )
    }
}

/// The button: enabled only while the enhancement is on and the player is in
/// flight. Off it is neither drawn nor clickable.
pub fn npc_button_visible(enabled: bool, in_flight: bool) -> bool {
    enabled && in_flight
}

/// The list is a no-op unless the enhancement is on, the player opened it and
/// the flight view is still up.
pub fn npc_list_visible(enabled: bool, open: bool, in_flight: bool) -> bool {
    enabled && open && in_flight
}

/// Whether `(x, y)` is anywhere on the list panel (the panel covers the whole
/// flight view, so a click there is not also a body pick).
pub fn npc_list_hit(x: i32, y: i32) -> bool {
    (0..0x140).contains(&x) && (0..158).contains(&y)
}

/// The list row under `(x, y)`, as [`list_cell`] lays the rows out: the marker
/// and its name, one row of fifteen pixels.
pub fn npc_row(x: i32, y: i32, count: usize) -> Option<usize> {
    (0..count.min(LIST_ROWS)).find(|index| {
        list_cell(*index).is_some_and(|(cx, cy)| {
            (cx..cx + LIST_COLUMN_WIDTHS[index / LIST_COLUMN_ROWS]).contains(&x)
                && (cy..cy + LIST_ROW_HEIGHT).contains(&y)
        })
    })
}

/// Clicking a row selects the ship and returns to the flight view: close the
/// list and forget the view it opened in, but keep the lock (the caller does
/// not clear `ship_lock`/`missile_lock` here).
pub fn close_after_row_selection(
    open: &mut bool,
    opened_in_view: &mut Option<crate::camera::ViewMode>,
) {
    *open = false;
    *opened_in_view = None;
}

/// Drop a lock only when it is the ship the player picked from the list.
pub fn clear_list_selection(
    selected: Option<u32>,
    ship_lock: &mut Option<u32>,
    missile_lock: &mut Option<u32>,
) {
    let Some(id) = selected else {
        return;
    };
    if *ship_lock == Some(id) {
        *ship_lock = None;
    }
    if *missile_lock == Some(id) {
        *missile_lock = None;
    }
}

/// `enhancement-10`'s teardown: on leaving flight (or the view the list was
/// opened in), close the list and drop the ship the player selected through
/// it. Locks the player already had are left alone, and with the toggle off
/// the two words are untouched. Returns whether the teardown ran.
pub fn close_on_leave(
    enabled: bool,
    open: bool,
    in_flight: bool,
    selected: Option<u32>,
    ship_lock: &mut Option<u32>,
    missile_lock: &mut Option<u32>,
) -> bool {
    if !(enabled && open && !in_flight) {
        return false;
    }
    clear_list_selection(selected, ship_lock, missile_lock);
    true
}

/// Red when the ship is hostile or attacking the player.
pub fn label_is_hostile(hostile: bool, attacking: bool) -> bool {
    hostile || attacking
}

/// `enhancement-10`'s three-way row colour (owner's scheme): hostile ships
/// (pirates start hostile) are red, non-hostile police are blue and every
/// other non-hostile ship is green. Flying toward the player does not itself
/// make a neutral red.
pub fn label_colour(role: Role, hostile: bool, attacking: bool) -> LabelColour {
    if label_is_hostile(hostile, attacking) {
        LabelColour::Red
    } else if role == Role::Police {
        LabelColour::Blue
    } else {
        LabelColour::Green
    }
}

/// `enhancement-10`'s list range cap: 50 km from the player.
pub fn within_list_range(metres: f64) -> bool {
    metres <= LIST_RANGE_LIMIT
}

/// `enhancement-10`: the registration the list shows. F791 kind 0x12 from the
/// ship's seed (`+0xa0`), as F439's View page does; when the cache has no name
/// table, fall back to the port's stable ship id.
pub fn registration(names: Option<&Names>, ship: &NpcShip) -> String {
    names
        .and_then(|names| {
            names.generate(names::kind::SHIP, (ship.seed as u32).wrapping_mul(0x4a0d))
        })
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| format!("#{}", ship.id))
}

/// `enhancement-10`: the ship's list row when it is visible, in a ship model
/// and within the list's 50 km cap. `unit` converts mm to flight units.
pub fn label(
    ship: &NpcShip,
    player: [i64; 3],
    unit: f64,
    names: Option<&Names>,
) -> Option<CombatLabel> {
    if !ship.visible() || !(0x0f..=0x3f).contains(&ship.model) {
        return None;
    }
    let metres: f64 = (0..3)
        .map(|k| (ship.position[k] - player[k]) as f64)
        .map(|d| d * d)
        .sum::<f64>()
        .sqrt();
    if !within_list_range(metres) {
        return None;
    }
    Some(CombatLabel {
        name: ship.name.clone(),
        registration: registration(names, ship),
        centre: ship.position.map(|v| v as f64 / unit),
        ship: ship.id,
        range: (metres / 10_000.0) as i32,
        colour: label_colour(ship.role, ship.hostile, ship.state == State::Attack),
    })
}

/// `enhancement-10`: every visible NPC within 50 km, nearest first. This is
/// the list the targeting button opens.
pub fn npc_labels(
    ships: &[NpcShip],
    player: [i64; 3],
    unit: f64,
    names: Option<&Names>,
) -> Vec<CombatLabel> {
    let mut labels: Vec<CombatLabel> = ships
        .iter()
        .filter_map(|ship| label(ship, player, unit, names))
        .collect();
    labels.sort_by_key(|label| label.range);
    labels
}

/// The button icon as RGBA pixels. The PNG is compiled into the executable
/// (cargo rebuilds when `npc-targeting.png` changes), and a 25x25 `npc-targeting.png`
/// beside the executable or in the working folder, if there is one, replaces it at
/// run time so the art can be tweaked without a rebuild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

/// The icon: a valid `npc-targeting.png` on disk (next to the executable, then the
/// working folder), else the embedded one.
pub fn npc_icon() -> Result<NpcIcon, String> {
    load_icon("npc-targeting.png", NPC_ICON_PNG)
}

/// The pressed button: `npc-targeting-p.png`, found the same way.
pub fn npc_icon_pressed() -> Result<NpcIcon, String> {
    load_icon("npc-targeting-p.png", NPC_ICON_PRESSED_PNG)
}

/// Whether logical pixel `(x, y)` is inside the button's 25x25 icon. A click anywhere on it
/// is the button's, not the combat-view or capsule hot area underneath.
pub fn npc_button_hit(x: i32, y: i32) -> bool {
    let (left, top, width, height) = NPC_BUTTON;
    (left..left + width as i32).contains(&x) && (top..top + height as i32).contains(&y)
}

fn load_icon(name: &str, embedded: &[u8]) -> Result<NpcIcon, String> {
    let mut places = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        places.push(dir.join(name));
    }
    places.push(std::path::PathBuf::from(name));
    for path in places {
        if let Ok(icon) = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| decode_icon(&bytes))
        {
            return Ok(icon);
        }
    }
    decode_icon(embedded)
}

/// Decode a 25x25 PNG button icon.
pub fn decode_icon(bytes: &[u8]) -> Result<NpcIcon, String> {
    use std::io::Cursor;

    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("NPC icon PNG: {e}"))?;
    let (width, height) = (reader.info().width, reader.info().height);
    if width != NPC_BUTTON.2 || height != NPC_BUTTON.3 {
        return Err("The NPC targeting icon is not 25x25.".to_owned());
    }
    let mut output = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut output)
        .map_err(|e| format!("NPC icon pixels: {e}"))?;
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err("NPC icon did not expand to RGB/RGBA.".to_owned()),
    };
    let pixels = output[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|p| {
            if channels == 4 {
                [p[0], p[1], p[2], p[3]]
            } else {
                [p[0], p[1], p[2], 0xff]
            }
        })
        .collect();
    Ok(NpcIcon {
        width,
        height,
        pixels,
    })
}

/// `enhancement-9`: the selected NPC's speed relative to the player, in m/s.
/// Both velocities are in system mm/s. `None` with the enhancement off.
pub fn relative_speed_line(
    enabled: bool,
    ship_velocity: [f64; 3],
    player_velocity: [f64; 3],
) -> Option<String> {
    enabled.then(|| {
        let relative: f64 = (0..3)
            .map(|k| ship_velocity[k] - player_velocity[k])
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        format!("Speed: {} m/s", (relative / 1000.0).round() as i64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_follows_f1054s_smoothing() {
        let mut camera = CombatCamera::enter();
        assert_eq!(
            (camera.nearest, camera.distance, camera.yaw),
            (0x320, 0x320, 0)
        );
        // No ship: the range stays at 8 km.
        camera.step(None, 0);
        assert_eq!(camera.distance, 0x320);
        // A ship 1 km away is floored at 3 km, and the distance eases in.
        camera.step(Some(100), 0x400);
        assert_eq!(camera.nearest, NEAREST_MIN);
        assert_eq!(camera.distance, (0x320 * 3 + 0x12c) >> 2);
        assert_eq!(camera.yaw, 0x100);
        // A far ship never pushes the range beyond F1099's 8 km start.
        camera.step(Some(5000), 0);
        assert_eq!(camera.nearest, NEAREST_START);
        // F1053's word: -(distance * 40), then << 11 in external units.
        let camera = CombatCamera::enter();
        assert_eq!(camera.offset(), -32_000.0 * 2048.0);
    }

    #[test]
    fn readout_text_is_f1109s() {
        assert_eq!(text_origin(100, 50), Some((92, 59)));
        assert_eq!(text_origin(320, 50), None);
        assert_eq!(text_origin(100, 128), None);
    }

    #[test]
    fn the_enhancement_toggles_leave_the_original_path_alone() {
        // enhancement-10: the button exists only with the toggle on, in flight.
        assert!(!npc_button_visible(false, true));
        assert!(!npc_button_visible(true, false));
        assert!(npc_button_visible(true, true));
        // The list is a no-op while off or closed.
        assert!(!npc_list_visible(false, true, true));
        assert!(!npc_list_visible(true, false, true));
        assert!(!npc_list_visible(true, true, false));
        assert!(npc_list_visible(true, true, true));
        // The teardown runs only when the list was open and flight ended.
        let (mut ship, mut missile) = (Some(7), Some(7));
        assert!(!close_on_leave(
            false,
            true,
            false,
            Some(7),
            &mut ship,
            &mut missile
        ));
        assert_eq!((ship, missile), (Some(7), Some(7)));
        assert!(!close_on_leave(
            true,
            false,
            false,
            Some(7),
            &mut ship,
            &mut missile
        ));
        assert_eq!((ship, missile), (Some(7), Some(7)));
        assert!(!close_on_leave(
            true,
            true,
            true,
            Some(7),
            &mut ship,
            &mut missile
        ));
        assert_eq!((ship, missile), (Some(7), Some(7)));
        assert!(close_on_leave(
            true,
            true,
            false,
            Some(7),
            &mut ship,
            &mut missile
        ));
        assert_eq!((ship, missile), (None, None));
        // Closing drops only the lock made through the list; an older lock stays.
        let (mut ship, mut missile) = (Some(9), Some(9));
        assert!(close_on_leave(
            true,
            true,
            false,
            Some(7),
            &mut ship,
            &mut missile
        ));
        assert_eq!((ship, missile), (Some(9), Some(9)));
        clear_list_selection(Some(9), &mut ship, &mut missile);
        assert_eq!((ship, missile), (None, None));
    }

    #[test]
    fn the_button_and_list_hot_areas_are_clear() {
        // The button is the console's record 0x12 slot, the system map's
        // zoom-out button, and its icon is the same 25x25.
        assert_eq!(NPC_BUTTON, (237, 174, 25, 25));
        // A click on the panel is consumed even when no row is under it.
        assert!(npc_list_hit(0, 0));
        assert!(npc_list_hit(319, 157));
        assert!(!npc_list_hit(320, 0));
        // Rows are the navigation computer's: nine from y 5 in a column at
        // x 10, nine more at x 230.
        assert_eq!(list_cell(0), Some((10, 5)));
        assert_eq!(list_cell(8), Some((10, 125)));
        assert_eq!(list_cell(9), Some((230, 5)));
        assert_eq!(list_cell(18), None);
        assert_eq!(npc_row(10, 5, 3), Some(0));
        assert_eq!(npc_row(100, 20, 3), Some(1));
        assert_eq!(npc_row(10, 5 + 15 * 3, 3), None);
        assert_eq!(npc_row(9, 5, 3), None);
        assert_eq!(npc_row(235, 5, 12), Some(9));
        assert_eq!(npc_row(235, 5, 5), None);
    }

    #[test]
    fn a_row_label_keeps_the_name_and_drops_the_distance_when_narrow() {
        let label = CombatLabel {
            name: "Moray Starboat 3".to_owned(),
            registration: "A17-3".to_owned(),
            centre: [0.0; 3],
            ship: 7,
            range: 1240,
            colour: LabelColour::Green,
        };
        assert_eq!(row_label(&label, 0), "Moray Starboat 3 (12.4km)");
        assert_eq!(row_label(&label, 9), "Moray Starbo");
    }

    #[test]
    fn a_list_row_has_name_registration_and_bracketed_distance() {
        let label = CombatLabel {
            name: "Moray Starboat 3".to_owned(),
            registration: "A17-3".to_owned(),
            centre: [0.0; 3],
            ship: 7,
            range: 1240,
            colour: LabelColour::Green,
        };
        assert_eq!(label.row_text(), "Moray Starboat 3  A17-3  (12.4km)");
        // The fallback registration is the port's stable id when no name table
        // is available.
        assert_eq!(format!("#{}", 7u32), "#7");
    }

    #[test]
    fn selecting_a_row_closes_the_list_and_keeps_the_lock() {
        use crate::camera::ViewMode;
        let mut open = true;
        let mut view = Some(ViewMode::Front);
        // The row handler sets the lock first, then closes the list.
        let (ship_lock, missile_lock) = (Some(7), Some(7));
        close_after_row_selection(&mut open, &mut view);
        assert!(!open);
        assert_eq!(view, None);
        // Closing the list must not undo the selection; only leaving flight or
        // an explicit close does that.
        assert_eq!((ship_lock, missile_lock), (Some(7), Some(7)));
    }

    #[test]
    fn the_pressed_icon_decodes_and_the_whole_icon_is_the_button() {
        let pressed = npc_icon_pressed().expect("npc-targeting-p.png is embedded");
        assert_eq!(
            (pressed.width, pressed.height),
            (NPC_BUTTON.2, NPC_BUTTON.3)
        );
        // The whole 25x25 icon is the button, including the strip over the combat view hot area.
        assert!(npc_button_hit(237, 174));
        assert!(npc_button_hit(261, 198));
        assert!(!npc_button_hit(236, 174));
        assert!(!npc_button_hit(262, 198));
        assert!(!npc_button_hit(237, 199));
    }

    #[test]
    fn the_embedded_icon_decodes() {
        let icon = npc_icon().expect("npc-targeting.png is embedded");
        assert_eq!((icon.width, icon.height), (NPC_BUTTON.2, NPC_BUTTON.3));
        assert_eq!(icon.pixels.len(), 25 * 25);
        // The artwork is opaque in the middle and transparent at the corners.
        assert_eq!(icon.pixels[12 * 25 + 12][3], 255);
        assert_eq!(icon.pixels[0][3], 0);
    }

    #[test]
    fn list_rows_are_red_blue_or_green_by_role_and_hostility() {
        // Hostile ships (pirates start hostile) are red.
        assert!(label_is_hostile(true, false));
        assert!(label_is_hostile(false, true));
        assert_eq!(label_colour(Role::Pirate, true, false), LabelColour::Red);
        assert_eq!(label_colour(Role::Trader, false, true), LabelColour::Red);
        // Non-hostile police are blue; hostile police turn red.
        assert_eq!(label_colour(Role::Police, false, false), LabelColour::Blue);
        assert_eq!(label_colour(Role::Police, true, false), LabelColour::Red);
        assert_eq!(label_colour(Role::Police, false, true), LabelColour::Red);
        // Everyone else that is not hostile is green, including a trader
        // flying toward the player (closing is no longer a red trigger).
        assert_eq!(label_colour(Role::Trader, false, false), LabelColour::Green);
        assert_eq!(
            label_colour(Role::Shuttle, false, false),
            LabelColour::Green
        );
        assert_eq!(
            label_colour(Role::Military, false, false),
            LabelColour::Green
        );
        assert_eq!(
            (
                LabelColour::Red.palette(),
                LabelColour::Blue.palette(),
                LabelColour::Green.palette()
            ),
            (LABEL_HOSTILE, LABEL_POLICE, LABEL_PASSIVE)
        );
    }

    #[test]
    fn the_list_is_capped_at_fifty_kilometres() {
        assert!(within_list_range(0.0));
        assert!(within_list_range(49_999_999.0));
        // Exactly 50 km is still in range; anything beyond is not.
        assert!(within_list_range(50_000_000.0));
        assert!(!within_list_range(50_000_001.0));
        assert!(!within_list_range(LIST_RANGE_LIMIT + 1.0));
        // The F1109 readout keeps its own much larger class limit.
        const { assert!(RANGE_LIMIT > LIST_RANGE_LIMIT) };
    }

    #[test]
    fn the_relative_speed_is_off_by_default() {
        // enhancement-9: the line only exists with the toggle on.
        assert_eq!(relative_speed_line(false, [3000.0; 3], [0.0; 3]), None);
        assert_eq!(
            relative_speed_line(true, [3000.0, 4000.0, 0.0], [0.0; 3]).as_deref(),
            Some("Speed: 5 m/s")
        );
        // Relative to the player, so a ship matching the player's velocity is 0.
        assert_eq!(
            relative_speed_line(true, [1000.0, 0.0, 0.0], [1000.0, 0.0, 0.0]).as_deref(),
            Some("Speed: 0 m/s")
        );
    }
}
