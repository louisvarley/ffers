use crate::camera::{ship_axes, Camera, ViewMode, ViewState};
use crate::console::{Console, ShipSystems};
use crate::flight::{Flight, Mode};
use crate::hud::{
    PlayerHud, AUTOPILOT_POS, HUD_COLOUR, HULL_POS, SHIELDS_POS, TURRET_COLOUR, TURRET_POS,
};
use crate::indexed::{palette_rgb6, BitmapFont, IndexedImage, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use crate::intro_text::IntroText;
use crate::messages::MessageLog;
use crate::model_lab::{LabView, ModelLab};
use crate::model_viewer::{AnimationInputs, Frame, ModelViewer, Shape, THRUST_GLOBAL};
use crate::prefs::{self, Mark, Prefs};
use crate::scanner;
use crate::station::{self, DockRow, DockView, Screen as StationScreen, Status};
use crate::targeter;
use ffe_content::{AssetKind, VerifiedCache};
use ffe_core::model::JITTER_SEED;
use ffe_core::model_fill::{fill_polygon, fill_quad, fill_triangle};
use ffe_core::model_lighting::pack;
use ffe_core::model_line::{draw_curve, draw_line};
use std::io::Cursor;

pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 480;
const VIEW_HEIGHT: usize = 158;

/// A star, planet or moon seen as a lit disc: flight-space centre and radius, the
/// unit direction toward the light, and the palette indices (lit, dark).
#[derive(Debug, Clone, Copy)]
pub struct Disc {
    /// Original system object type (record +0x0a), when available.
    pub system_type: Option<u16>,
    pub centre: [f64; 3],
    pub radius: f64,
    pub light: [f64; 3],
    pub colours: (u8, u8),
    pub star: bool,
    /// Stellar effective temperature from the generated object, when available.
    pub temperature: Option<u16>,
    /// The body's model and surface seed, when it has a drawable surface.
    pub model: Option<(u32, u32)>,
    /// The object's rows in flight space.
    pub orientation: [[f64; 3]; 3],
}

/// A station of the system in the flight view: its own model at its place in the
/// system, turning about its docking axis as F866 turns it (set by the caller each
/// frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemStation {
    /// Surface port metadata from the gameplay snapshot.
    pub surface: Option<SurfacePort>,
    pub model: u32,
    /// Centre in flight space.
    pub centre: [f64; 3],
    /// The object's rows in flight space.
    pub orientation: [[f64; 3]; 3],
    /// F866's signed speed byte (`+0xc8`); 0 does not turn.
    pub speed: i8,
    /// The station record's door word (`+0x9e`, global 1): 0 is open, 0xffff
    /// closed (F865).
    pub door: u16,
    /// The game tics the spin and the animation are taken at.
    pub tics: u32,
    /// The object's random seed (`+0xa0`, global words 2 and 3). Global 2's
    /// low bits pick the starport's pad colour (model 29's colour command,
    /// textures 1..17).
    pub seed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePort {
    /// F650 pad centres in system millimetres. Enclosed ports have no pads.
    pub pads: [Option<[i64; 3]>; 8],
}

/// The farthest a station is drawn from (millimetres from the eye): the viewer's
/// view space is 32-bit, and an orbital city is under a pixel beyond it.
const STATION_DRAW_RANGE: f64 = 2.0e9;

/// The lift scene of a docking or launching stage (`ffe_core::lift`), drawn in the
/// front view after the outside world (or alone, inside the station).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftView {
    pub objects: Vec<ffe_core::lift::Object>,
    /// Draw the stars, planets, stations and the ship first; false where only
    /// the lift is drawn (every stage view, F1069-F1087 and F1092).
    pub world: bool,
    /// The game tics the models' time word reads.
    pub tics: u32,
    /// The station's random seed (object `+0xa0`, global words 2 and 3): it picks the
    /// posters on the walls, so every station has its own.
    pub seed: u32,
    /// An open-air starport (`+0x14c` bit 5): global word 4 is 0 there, 1 in an orbital
    /// station (F1070).
    pub starport: bool,
}

/// The galaxy map as the screen draws it: the scene's shapes over the cabin art and the corner
/// and console texts (`galaxy_map`).
#[derive(Debug, Clone, Default)]
pub struct MapScene {
    pub shapes: Vec<Shape>,
    /// "Sector: x,y", at the lower left of the view.
    pub sector: String,
    /// The fuel the selected jump needs, or why it cannot be made.
    pub fuel: Option<String>,
    /// The game's date and time, in green at the top of the console text.
    pub date: String,
    /// The selected star's description, wrapped to the console text's width here.
    pub info: String,
    /// F7-F10's page (the system description, ...): drawn over the cabin in
    /// place of the 3D map.
    pub page: Option<crate::map_pages::Page>,
}

/// The system map page as the screen draws it (`system_map`): its frame over the
/// cabin art and the console's date.
#[derive(Debug, Clone, Default)]
pub struct SystemMapScene {
    pub frame: crate::system_map::Frame,
    /// The game's date and time, in green at the top of the console text.
    pub date: String,
}

/// The enhanced (flat) galaxy map as the screen draws it (`enhanced_map`).
#[derive(Debug, Clone, Default)]
pub struct EnhancedGalaxy {
    /// Every star dot: x, y, radius (pixels), colour (faction or danger).
    pub dots: Vec<(i32, i32, i32, u8)>,
    /// System-name labels at screen points (inhabited systems and the selection).
    pub labels: Vec<(i32, i32, String)>,
    /// The selected star: x, y, radius.
    pub selected: Option<(i32, i32, i32)>,
    /// The star under the mouse: x, y, radius.
    pub hovered: Option<(i32, i32, i32)>,
    /// The route as projected points: the home first, then each waypoint.
    pub route: Vec<(i32, i32)>,
    /// Whether each leg of the route (the point after `route[i]`) can be flown
    /// with the range and the fuel left; a leg that cannot is drawn red.
    pub leg_ok: Vec<bool>,
    /// Waypoints (screen points) whose arrival strands the player: uninhabited
    /// and without the fuel to leave, drawn with a warning triangle.
    pub warnings: Vec<(i32, i32)>,
    /// The drive's reach circle radius about the home point.
    pub range_radius: i32,
    /// The player's own system: its projected point and the line to it from
    /// the middle of the view.
    pub home: (i32, i32),
    pub home_pointer: crate::enhanced_map::Pointer,
    /// Sol's projected point (possibly off-view) and its distance from home.
    pub sol: (i32, i32, i32),
    pub sector: String,
    /// The grid under the stars (see `EnhancedMap::grid_lines`).
    pub grid: Vec<crate::enhanced_map::GridLine>,
    pub date: String,
    /// The route's total distance and fuel, when a route is set.
    pub route_summary: Option<(String, u8)>,
    /// What hovering a star says (text and colour), drawn as a panel at the side of
    /// the view: its name and distance, whether it is inhabited, the original map's
    /// facts about it, how dangerous it is, its description and what a jump there needs.
    pub hover: Vec<(String, u8)>,
    /// What the dot colours mean: (colour, name).
    pub legend: Vec<(u8, &'static str)>,
    /// The controls line along the top of the view.
    pub hint: String,
}

/// One commodity line of the station's market screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketRow {
    pub name: String,
    pub price: i32,
    pub stock: i32,
    pub held: u16,
}

/// The station's market as the docked screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarketView {
    pub station: String,
    pub system: String,
    pub rows: Vec<MarketRow>,
    /// The first visible row (the original's `D9050`).
    pub scroll: usize,
    pub selected: usize,
    pub cash: i32,
    pub used: i32,
    pub capacity: i32,
    /// F454's illegal-goods market: `station` holds the `0x98d7` company name
    /// instead of the starport name, and the title is that name alone.
    pub black_market: bool,
}

/// Rows of the market list shown at once (the original's 10-pixel rows from
/// y 15 to before y 148).
pub const MARKET_ROWS: usize = 13;

/// The controls button in the top bar: left edge and right edge (logical pixels).
/// F1066's hyperspace tunnel model.
pub const HYPERSPACE_MODEL: u32 = 0xa5;

pub const CONTROLS_BUTTON: (i32, i32) = (118, 236);

/// The autopilot buttons beside them: target the next body, fly to the locked one (and dock
/// at a station), stop.
pub const AUTOPILOT_BUTTONS: [(i32, i32, &str); 3] = [
    (154, 204, "TARGET"),
    (208, 268, "AUTOPILOT"),
    (272, 312, "STOP"),
];

/// Half the side of the Combat Targeting Computer's red target square.
const NPC_SQUARE_HALF: i32 = 10;
/// The length of each corner bracket of the red target square.
const NPC_SQUARE_ARM: i32 = 5;

/// The locked body as it shows in the view: its screen position, a pick radius in pixels,
/// and whether the autopilot is flying to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetMark {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    pub engaged: bool,
}

/// F1100's target readout: the target's screen position (`object+0x98/+0x9a`,
/// unclipped where possible), the converted range (`DATA_008882/3`), and the
/// off-screen state. The range number is in km or AU according to
/// [`TargetHud::au`]; `colour` is the COLPAL index F1099 passes (0x11 for the
/// navigation target's green, 0x10 for the combat target's red).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetHud {
    pub x: i32,
    pub y: i32,
    pub value: i32,
    pub au: bool,
    /// The target is behind the camera (F1100's 0x8000 screen words).
    pub behind: bool,
    /// The target is in front but has no valid screen words (F1100's
    /// out-of-range branch). Off-view targets with a projection use the arrow
    /// forms instead.
    pub offscreen: bool,
    /// F1099's COLPAL: [`crate::targeter::NAVIGATION_COLOUR`] or
    /// [`crate::targeter::COMBAT_COLOUR`] (0x5f in the missile view).
    pub colour: u8,
}

/// F1099's target shields/hull readout (`0x8625`/`0x8626`), when a ship target
/// carries them (`docs/notes/npc.md`). Stations and planets have no such state,
/// so this stays `None` for a body target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetDamage {
    /// Hull per-mille (`0..=1000`), F1099's `hull * 250 / mass`.
    pub hull: i32,
    /// Shield percent, or `None` when the target has no shield generator
    /// (`0x8626`).
    pub shields: Option<i32>,
    /// The descriptor's shield-strength word (`0x8625`'s last value).
    pub shield_strength: i16,
}

/// An object's name label in the flight view (the original F1108's text from
/// object `+0x124`). `centre` is in flight units, as the camera takes it.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyLabel {
    pub name: String,
    pub centre: [f64; 3],
}

/// An NPC ship to draw in the system view: its model, centre and axes in flight
/// units, and whether F1536's shield-hit flash (model 0x9d) is showing.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemShip {
    pub model: u32,
    pub centre: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub flash: bool,
    /// A destroyed ship: the ship model is gone; F942's effect draws instead.
    pub dead: bool,
}

/// An NPC's laser shot to draw: F1776's beam at the firing ship's gun
/// ([`crate::combat::npc_beam`]); `model` finds the gun's position words.
#[derive(Debug, Clone, PartialEq)]
pub struct ShipBeam {
    pub model: u32,
    pub centre: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub code: u8,
    pub length: u16,
}

/// A short-lived system effect to draw: F939's particle (model 0x9e), F942's
/// explosion (0x9b) or F926's debris pod (0x99).
#[derive(Debug, Clone, PartialEq)]
pub struct SystemEffect {
    pub model: u32,
    pub centre: [f64; 3],
    pub axes: [[f64; 3]; 3],
    /// Object global word 4 (`+0xa4`): F708/F709's countdown, which model
    /// 155's fragment branches read.
    pub timer: u16,
    /// Object global word 1 (`+0x9e`): F942's `0xffff` on the explosion,
    /// model 155's fragment size (`global1 >> 7`); 0 otherwise.
    pub size: u16,
    /// The effect model's scale shifted by its distance word, or `None` to
    /// draw at the model's own scale.
    pub scale: Option<u32>,
}

impl SystemEffect {
    /// One of the navigator's effects as the renderer draws it. `unit` is the
    /// original units per flight unit. The effect's distance word shifts its
    /// model's own scale (F939's sparks draw at +2 or -1).
    pub fn from_effect(
        scene: Option<&ModelViewer>,
        effect: &ffe_core::effects::Effect,
        unit: f64,
    ) -> Self {
        let scale = scene
            .filter(|_| effect.shift != 0)
            .and_then(|scene| scene.find(effect.model))
            .map(|model| (model.scale as i32 + i32::from(effect.shift)).max(0) as u32);
        Self {
            model: effect.model,
            centre: effect.position.map(|v| v as f64 / unit),
            axes: effect.axes(),
            timer: effect.timer,
            size: match effect.kind {
                ffe_core::effects::Kind::Explosion => ffe_core::effects::EXPLOSION_SIZE,
                // Clouds' ramp, debris pods' and sparks' cloned `+0x9e`.
                _ => effect.size,
            },
            scale,
        }
    }
}

/// A missile object (F941, models 4..13) drawn through the same object renderer
/// as the ships. Missiles carry no flash or death marker.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemMissile {
    pub model: u32,
    pub centre: [f64; 3],
    pub axes: [[f64; 3]; 3],
}

/// The retired port-only map buttons. Kept as constants so the integrator's
/// layout tests can see where the non-original UI used to be; nothing draws or
/// clicks them now (the original's navigation is the `n` console icon).
pub const MAP_BUTTONS: [(i32, i32, &str); 2] = [(2, 62, "SYSTEMS"), (66, 150, "THIS SYSTEM")];

/// F1775's background object: the nebula and star field (model 315).
const BACKGROUND_MODEL: u32 = 0x13b;
/// Unverified port depth; DOS F1775 supplies 0x200 directly (see parity-07 handoff).
const BACKGROUND_DEPTH: f64 = 0x2000 as f64;
/// F318's title colour (`DATA_008673`, the executed boot's index 34).
const STATUS_TITLE: u8 = 34;

/// F283's page: everything it prints, resolved by the game loop.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EquipmentStatus {
    /// `0x9c04`: "SHIP EQUIPMENT STATUS - " and the ship's name.
    pub title: String,
    /// `0x9c12`'s three lines, columns split at F1399's tab stops by '\t'.
    pub status: Vec<String>,
    /// The fitted general equipment's names (`DATA_005106`) and whether each
    /// is damaged.
    pub equipment: Vec<(String, bool)>,
    /// The gun mounts: mount name (`0x9870..`), laser(s), and the tab x.
    pub lasers: Vec<(String, String, usize)>,
    /// `DATA_008686`: the first row each column shows when the list scrolls.
    pub scroll: usize,
}

/// F283's list layout for `count` fitted items: rows shown per column, the
/// column break and the furthest scroll (`DATA_008687`). Up to 16 items show
/// eight to a column; more scroll seven rows of two `(count + 1) / 2` columns.
pub fn equipment_rows(count: usize) -> (usize, usize, usize) {
    if count > 16 {
        let half = count.div_ceil(2);
        (7, half, half - 7)
    } else {
        (8, 8, 0)
    }
}

/// F286's page-0 hot areas (`DATA_005108`): (20..28, 112..123) is action 6,
/// one row on; (32..40, 112..123) is action 5, one row back.
pub fn equipment_scroll_click(x: i32, y: i32) -> Option<i32> {
    if !(112..=123).contains(&y) {
        return None;
    }
    if (20..=28).contains(&x) {
        Some(1)
    } else if (32..=40).contains(&x) {
        Some(-1)
    } else {
        None
    }
}

/// One line of F295's roster (`0x9c15`..`0x9c18`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterRow {
    /// The client, or "Fed.Military (by <date>)" / "Imp. Navy (by <date>)".
    pub client: String,
    /// The group size, "Parcel" or "-".
    pub group: String,
    pub destination: String,
    /// Whole credits.
    pub reward: u32,
}

/// One two-line entry of F316's outstanding contracts (`0x9c1e`..`0x9c22`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractRow {
    /// "Kill <target>, return to <station>", wrapped to 120 pixels.
    pub contract: String,
    pub system: String,
    pub station: String,
    /// The deadline date.
    pub time: String,
    pub reward: u32,
    /// A mission script's record (kind 7): `contract` is the row the
    /// scripts gave, wrapped to 0x13b pixels with no other columns.
    pub wide: bool,
}

/// One line of F317's mining equipment list (0xa200/0xa201).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiningRow {
    /// The find (0xa202 + `+0x1f`) or "In Cargo Bay".
    pub status: String,
    pub system: String,
    pub planet: String,
    /// The hopper's tonnes; empty for a machine aboard.
    pub hopper: String,
}

/// F286's seven status pages in section order, as the port's console note
/// names them (F318's titles are `0x9c04..0x9c0a`).
pub const STATUS_TITLES: [&str; 7] = [
    "Ship equipment status.",
    "Commander profile.",
    "Crew roster.",
    "Cargo inventory.",
    "Passenger and small items roster.",
    "Outstanding contracts.",
    "Mining equipment list.",
];

/// F292: the sections F6..F12 (actions 0xf6..0xfc) select.
pub const STATUS_KEY_SECTIONS: [u8; 7] = [0, 1, 2, 4, 3, 5, 6];

/// F309's `DATA_005126`: the console record lit for each section (F6's 9
/// through F12's 3).
pub const STATUS_BUTTONS: [usize; 7] = [9, 8, 7, 5, 6, 4, 3];

/// A click on F294's cargo page (its hot areas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoClick {
    /// The Refuel button at (120,15) (action `'r'`).
    Refuel,
    /// The jettison button beside the listed item (actions `0x80 + i`).
    Jettison(usize),
}

/// F294's hot areas for a page listing `items` commodities: the refuel button
/// (120..128, 15..23) and each item's button 12..4 pixels left of its name,
/// rows of 10 from y 35, twelve to a column at x 18 and 106. Edges inclusive.
pub fn cargo_click(x: i32, y: i32, items: usize) -> Option<CargoClick> {
    if (120..=128).contains(&x) && (15..=23).contains(&y) {
        return Some(CargoClick::Refuel);
    }
    (0..items.min(24)).find_map(|index| {
        let column = if index < 12 { 18 } else { 106 };
        let top = 35 + (index % 12) as i32 * 10;
        ((column - 12..=column - 4).contains(&x) && (top..=top + 8).contains(&y))
            .then_some(CargoClick::Jettison(index))
    })
}

/// F308: the page after `current` that has something to show, wrapping to
/// the equipment page. The profile always follows the equipment page.
pub fn next_status_section(current: u8, sections: &[bool; 7]) -> u8 {
    (current.saturating_add(1)..7)
        .find(|section| sections[usize::from(*section)])
        .unwrap_or(0)
}

/// The ship library's issue page (F80/F79): the journal's masthead bitmap,
/// F78's wrapped text, F81's `y` and `0xa601`'s "More ..." text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueView {
    pub masthead: u32,
    pub text: Vec<u8>,
    pub y: i32,
    pub more: String,
}

/// What every status page draws under its text: F956's cabin, F288's ship
/// and, on the equipment and cargo pages, F310's cargo/cabins/cash block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatusBackdrop {
    pub hold: Status,
    pub cargo_block: bool,
    pub ship: u32,
    /// The game clock (65536 a second) for the model's animation.
    pub tics: u32,
    /// The time the status screen has been up (65536 a second): F293 turns
    /// the model only while it is.
    pub spin: u32,
    pub gear: u16,
}

/// What F439's View page draws this frame: the page (texts, model) and the
/// clock for the spin and the animation.
pub struct ShipViewFrame {
    pub page: crate::ship_view::Page,
    /// The game clock (65536 a second) for the model's animation.
    pub tics: u32,
    /// The time the page has been up (65536 a second): F562 turns the model by
    /// it, the status page's spin.
    pub spin: u32,
}

pub struct Screen {
    pub pixels: Vec<u32>,
    frame: Vec<u32>,
    /// `CABIN0..5.BMP`, the backdrops of the station and status screens.
    /// A missing one falls back to `CABIN0`.
    cabins: Vec<Vec<u32>>,
    /// F955's choice for the player's ship (`DATA_007224`).
    cabin_index: usize,
    panel: IndexedImage,
    font: BitmapFont,
    palette: [u32; 128],
    indexed: Box<[u8; 64000]>,
    /// The flight view's background: the original's space colour
    /// (`DATA_008815..817`, set by the Graphic Detail page), RGB.
    pub sky: u32,
    /// `DATA_008812`, the "Level of Shape Detail" the planet surfaces are
    /// generated at. The executed boot's default "High" is 1; the preference
    /// row is stored less one (see [`crate::prefs::SHAPE_DETAIL`]).
    pub surface_detail: i32,
    /// Build the runtime palette once per model render instead of once per frame
    /// (the port's earlier behaviour). Off by default; the before/after comparison in
    /// `docs/notes/palette.md` section 6 and its test switch it on.
    pub palette_build_per_object: bool,
    /// F1877's planet-atmosphere background (`DATA_008814..8817`) from the last
    /// frame's scene. The original clears the next upper view to it and leaves
    /// the stars out while it is set; `None` uses [`Screen::sky`].
    surface_sky: Option<u32>,
    /// The original's "Background stars" option; off leaves the sky empty.
    pub stars_visible: bool,
    /// The original's "Lock external view to ship" option (camera offset axis).
    pub lock_external: bool,
    /// The Others page's "Space dust, clouds etc": off leaves F1035's dust clouds out.
    pub dust_visible: bool,
    /// F1751's detail words (`DATA_008812/8813`) from the Graphics page's "Level of Shape
    /// Detail" radio (`prefs::shape_detail_words`); the default High reads `[1, 1]`.
    pub shape_detail: [i32; 2],
    /// Classic: the flight view's objects' shapes with their F1777 depth keys, in allocation
    /// order, while the original's one queue is being filled (see [`Self::flush_object_queue`]).
    object_queue: Option<Vec<(u32, Shape)>>,
    /// The system's bodies as discs (set by the caller each frame).
    pub discs: Vec<Disc>,
    /// The system's stations (set by the caller each frame, in system flight).
    pub stations: Vec<SystemStation>,
    port_pads: Option<ffe_core::pads::Pads>,
    /// The system's NPC ships (set by the caller each frame).
    pub ships: Vec<SystemShip>,
    /// The system's short-lived visuals: F939 hit particles, F942 explosions
    /// and F926 debris (set by the caller each frame).
    pub effects: Vec<SystemEffect>,
    /// The missiles in flight (F941 objects), drawn by the same object renderer.
    pub missiles: Vec<SystemMissile>,
    /// The NPC ships' laser shots this frame (F1776 for their guns).
    pub ship_beams: Vec<ShipBeam>,
    /// The combat target (`DATA_007247`) is set: F1111 draws F1114's circle at
    /// the view centre.
    pub missile_lock: bool,
    /// The F1111 blink (`DATA_007252`): the circle flashes about once a second.
    pub missile_blink: bool,
    /// Direction toward the local star in flight space (`None` outside a system):
    /// ships, stations and lift objects are lit by it instead of the original's
    /// fixed default light. Planet discs carry their own light.
    pub sun: Option<[f64; 3]>,
    /// The market the docked screen shows; the prototype's one-item screen without it.
    pub market: Option<MarketView>,
    /// The docked services/shipyard/BBS screen of this frame.
    pub dock: Option<DockView>,
    /// The ship library's issue page of this frame (F79/F80), drawn in
    /// place of `dock`.
    pub issue: Option<IssueView>,
    /// F439's View page of a hull: the model spinning in the window at the
    /// left and the text blocks, drawn over the one-row ship list in `dock`.
    pub ship_view: Option<ShipViewFrame>,
    /// F343's services board on a starport pad (`0x30`): the same screen as the
    /// docked view, but drawn over the landed flight view. The console, scanner
    /// and message clock stay live; the flight HUD text is covered like the
    /// source's station screen. The ship's mode stays `Flight`.
    pub pad_services: bool,
    /// The lift scene to draw this frame, in a docking or launching sequence.
    pub lift: Option<LiftView>,
    /// The view is from inside a station (a launch before the ship is in
    /// flight): the port's own status block and map buttons are left out.
    pub station_view: bool,
    /// The system and the station the player is at, for the docked screen and the top bar.
    pub system: String,
    pub station: String,
    /// The locked-on target and its range, for a body or an NPC ship.
    pub lock: String,
    /// Where the locked body shows in the view (set by the caller each frame).
    pub target_mark: Option<TargetMark>,
    /// F274/F275's scanner contacts and the player's frame, when in flight.
    pub scanner: Option<scanner::View>,
    /// F1100's targeting tunnels for the locked target (`DATA_009024`/`025`).
    pub tunnels: Option<targeter::Tunnels>,
    /// F1111's blinking auto-targetter cursor (`DATA_007246`/`247`).
    pub targeter: Option<targeter::Cursor>,
    /// `DATA_007252`: the game second's parity, which blinks the targetter.
    pub blink: bool,
    /// The names the labels toggle draws over the view (the original's F1108).
    pub labels: Vec<BodyLabel>,
    /// F1109's readouts for the combat computer view (6); the caller fills
    /// them only in that view.
    pub combat_readouts: Vec<crate::combat_computer::Readout>,
    /// The galaxy map to draw instead of the flight view, when it is open.
    pub galaxy: Option<MapScene>,
    /// The enhanced (flat) galaxy map, drawn instead of [`Self::galaxy`] when
    /// the enhancement is on.
    pub enhanced_galaxy: Option<EnhancedGalaxy>,
    /// The original's 3D system map, drawn in place of the galaxy map.
    pub system_map: Option<SystemMapScene>,
    /// The galaxy map's console selection (`F834`/`F840`), applied on top of
    /// the flight page while [`Screen::galaxy`] is set.
    pub map_console: crate::console::MapConsole,
    /// Whether a body is locked, for the autopilot buttons.
    pub has_lock: bool,
    /// The four mount lasers (`ship+0xd2..+0xd5`), for F1111's gunsight rule.
    pub lasers: [u8; 4],
    /// The beam to draw this frame (F1776's model 0x9c at the gun), when a shot fired.
    pub beam: Option<crate::combat::LaserBeam>,
    /// `enhancement-13`: draw the first-person laser as a fixed bottom-to-reticle
    /// sight line instead of the beam from the gun mount.
    pub fixed_laser: bool,
    /// The energy pool (`+0x11e`) and heat (`+0xea`) F270's first two bars show.
    pub energy: u32,
    pub heat: u16,
    /// The drive byte (`+0xd0`) the energy bar scales by.
    pub drive: u8,
    /// F1100's target range text, when a target is locked.
    pub target_hud: Option<TargetHud>,
    /// F1099's target shields/hull text, when the target is a ship with that state.
    pub target_damage: Option<TargetDamage>,
    /// `enhancement-9`: the selected NPC's relative speed line, after the hull
    /// and shields. `None` (the original path) with the toggle off.
    pub target_speed: Option<String>,
    /// `enhancement-10`: the NPC targeting list's rows. Empty unless the
    /// enhancement is on and the list is open.
    pub combat_labels: Vec<crate::combat_computer::CombatLabel>,
    /// `enhancement-10`: draw the port-only NPC targeting button this frame.
    pub npc_button: bool,
    /// `enhancement-10`: draw the NPC targeting list this frame.
    pub npc_list_open: bool,
    /// `enhancement-10`: the ship selected in the list, for the row highlight.
    pub npc_list_selected: Option<u32>,
    /// `enhancement-10`: the embedded button artwork, decoded once.
    npc_icon: Option<crate::combat_computer::NpcIcon>,
    npc_icon_pressed: Option<crate::combat_computer::NpcIcon>,
    /// `enhancement-11`: the off-view target arrow, when the toggle is on.
    pub edge_indicator: Option<targeter::EdgeIndicator>,
    /// `enhancement-7`: draw F312's full medal list (all set bits, in two
    /// columns) instead of the port's first three.
    pub mission_fixes: bool,
    /// F1099's player text: shields, hull, Autopilot, Set/Act/Alt and the turret
    /// angles. The caller sets it each frame.
    pub hud: PlayerHud,
    /// The ship's equipment/status bytes the console lamps and equipment page
    /// read (`ship+0xc6..`, `player+0xe8..`).
    pub systems: ShipSystems,
    /// The message log the console's text window shows (`F271/F272/F273`).
    pub messages: MessageLog,
    /// Draw the log this frame. The navigation list / places screens can hide
    /// it without touching the ring, as the original's console does.
    pub messages_visible: bool,
    /// F312's F2 profile page, when it is open. The original draws it over the
    /// upper view; the console stays below.
    pub profile: Option<ffe_core::status::Profile>,
    /// F294's page: each commodity carried and its tonnes.
    pub cargo_inventory: Option<Vec<(String, u16)>>,
    /// F295's passenger and small items roster.
    pub roster: Option<Vec<RosterRow>>,
    /// F316's outstanding contracts.
    pub contracts: Option<Vec<ContractRow>>,
    /// F317's mining equipment list.
    pub mining_list: Option<Vec<MiningRow>>,
    /// The cabin, model and cargo block under any status page.
    pub status_backdrop: Option<StatusBackdrop>,
    /// F2's page 0, F283's ship equipment status (drawn over the view).
    pub equipment_status: Option<EquipmentStatus>,
    /// The game date and time header `F272` writes before the entries.
    pub clock: String,
    /// The console's mutable `DATA_005100` state, derived from the view and ship
    /// each frame and drawn in table order.
    pub console: Console,
    /// F270's three bar bitmaps (primary 275, 274, 273), when cached.
    gauges: [Option<IndexedImage>; 3],
    /// The original console view icons (secondary bitmaps 63..81), when cached.
    icons: Vec<Option<IndexedImage>>,
    /// The navigation list's marker, primary bitmap 0x31.
    marker: Option<IndexedImage>,
    /// The primary bitmaps F851's body diagram blits (ids 11 to 42).
    diagram: Vec<Option<IndexedImage>>,
    /// The docked screen's row button and arrows: primary bitmaps 0x2e, 0x2f
    /// and 0x30 (F328/F330).
    dock_icons: [Option<IndexedImage>; 3],
    /// The journals' masthead bitmaps (primary 0..5), F80's.
    mastheads: Vec<Option<IndexedImage>>,
    /// The SysMenu's on/off icon bitmaps (primary 0x2c/0x2d), when cached.
    prefs_on: Option<IndexedImage>,
    prefs_off: Option<IndexedImage>,
    /// F962's primary bitmap 0x42 (the "FIRST ENCOUNTERS" strip), cache v2+.
    intro_bitmap: Option<IndexedImage>,
    /// The raw credits and F1398 width table (cache v27).
    intro_text: Option<IntroText>,
    /// One-shot report for model 176's undefined-stack child basis (see
    /// `docs/notes/animation.md` section 1.2).
    intro_error_reported: bool,
    /// F1066's hyperspace view: the transit's animation word (model 0xa5's
    /// global 0) while a jump is under way; the caller sets it each frame.
    pub hyperspace: Option<u16>,
    jitter: Jitter,
}

/// The F1476 stream the flight scene's jittered vertices draw from (the
/// flickering ends of thruster plumes). The original runs one stream across all
/// drawn objects and frames; here it advances once per flight step, so a paused
/// picture holds still, and it never feeds the simulation (KNOWN_ISSUES).
#[derive(Debug, Clone, Copy)]
struct Jitter {
    /// Flight step the current frame's start state belongs to.
    tick: Option<u64>,
    /// State the current flight step's first object draws from.
    start: [u32; 2],
    /// State after the last drawn object, in draw order.
    cursor: [u32; 2],
}

fn indexed_bitmap(cache: &VerifiedCache, id: &str) -> Option<IndexedImage> {
    let record = cache.manifest().files.iter().find(|r| r.id == id)?;
    match record.asset {
        AssetKind::IndexedBitmap {
            width,
            height,
            offset_x,
            offset_y,
            masked_zero,
            ..
        } => IndexedImage::new(
            width,
            height,
            [offset_x, offset_y],
            masked_zero,
            cache.asset(id)?,
        )
        .ok(),
        _ => None,
    }
}

impl Default for Jitter {
    fn default() -> Self {
        Self {
            tick: None,
            start: JITTER_SEED,
            cursor: JITTER_SEED,
        }
    }
}

impl Screen {
    pub fn new(cache: &VerifiedCache) -> Result<Self, String> {
        let asset = |id| {
            cache.asset(id).ok_or_else(|| {
                format!("The game content lacks {id}; check the game files in the launcher.")
            })
        };
        let panel_id = "base:bitmap/primary/043";
        let record = cache
            .manifest()
            .files
            .iter()
            .find(|record| record.id == panel_id)
            .ok_or("Cache lacks the original console panel.")?;
        let panel = match record.asset {
            AssetKind::IndexedBitmap {
                width: 320,
                height: 43,
                offset_x,
                offset_y,
                masked_zero,
                ..
            } => IndexedImage::new(320, 43, [offset_x, offset_y], masked_zero, asset(panel_id)?)?,
            _ => return Err("Invalid original console panel descriptor.".to_owned()),
        };
        let mut screen = Self::from_assets(
            asset("base:cockpit/cabin0")?,
            asset("base:font/main")?,
            panel,
            palette_rgb6(asset("base:palette/low")?)?,
        )?;
        screen.port_pads = cache
            .asset("base:gameplay/pads")
            .and_then(|bytes| ffe_core::pads::Pads::from_bytes(bytes).ok());
        for index in 1..6 {
            let cabin = cache
                .asset(&format!("base:cockpit/cabin{index}"))
                .and_then(|png| decode_cabin(png).ok())
                .unwrap_or_else(|| screen.cabins[0].clone());
            screen.cabins.push(cabin);
        }
        // F962's primary bitmap 0x42 and the raw credit/width assets (cache v27).
        screen.intro_bitmap = indexed_bitmap(cache, "base:bitmap/primary/066");
        screen.intro_text = IntroText::new(cache).ok();
        // enhancement-10: the port-only NPC targeting icon, compiled in.
        screen.npc_icon = crate::combat_computer::npc_icon().ok();
        screen.npc_icon_pressed = crate::combat_computer::npc_icon_pressed().ok();
        // Console bitmap ids 96 and up are the secondary table; the table has 251
        // entries, so the equipment page's 0x3a (268) and the lamps (up to 287)
        // are loaded too.
        screen.icons = (0..=0x15au32)
            .map(|id| {
                let index = id.checked_sub(96)?;
                let asset_id = format!("base:bitmap/secondary/{index:03}");
                let record = cache.manifest().files.iter().find(|r| r.id == asset_id)?;
                match record.asset {
                    AssetKind::IndexedBitmap {
                        width,
                        height,
                        offset_x,
                        offset_y,
                        masked_zero,
                        ..
                    } => IndexedImage::new(
                        width,
                        height,
                        [offset_x, offset_y],
                        masked_zero,
                        cache.asset(&asset_id)?,
                    )
                    .ok(),
                    _ => None,
                }
            })
            .collect();
        // The navigation list's 10x10 marker button (bitmap 0x31, primary table).
        screen.marker = indexed_bitmap(cache, crate::console::NAV_MARKER_BITMAP);
        screen.diagram = (0..=42)
            .map(|id| indexed_bitmap(cache, &format!("base:bitmap/primary/{id:03}")))
            .collect();
        screen.mastheads = (0..6)
            .map(|index| indexed_bitmap(cache, &format!("base:bitmap/primary/{index:03}")))
            .collect();
        screen.dock_icons = std::array::from_fn(|k| {
            let asset_id = format!("base:bitmap/primary/{:03}", 0x2e + k);
            let record = cache.manifest().files.iter().find(|r| r.id == asset_id)?;
            match record.asset {
                AssetKind::IndexedBitmap {
                    width,
                    height,
                    offset_x,
                    offset_y,
                    masked_zero,
                    ..
                } => IndexedImage::new(
                    width,
                    height,
                    [offset_x, offset_y],
                    masked_zero,
                    cache.asset(&asset_id)?,
                )
                .ok(),
                _ => None,
            }
        });
        // F270's three bars: bitmap ids 275 (energy), 274 (heat) and 273 (the
        // third value, DATA_008921, which this build leaves at zero). Ids 96 and
        // up are the secondary table.
        screen.gauges = crate::combat::GAUGE_BITMAPS.map(|id| {
            let index = id.checked_sub(96)?;
            let asset_id = format!("base:bitmap/secondary/{index:03}");
            let record = cache.manifest().files.iter().find(|r| r.id == asset_id)?;
            match record.asset {
                AssetKind::IndexedBitmap {
                    width,
                    height,
                    offset_x,
                    offset_y,
                    masked_zero,
                    ..
                } => IndexedImage::new(
                    width,
                    height,
                    [offset_x, offset_y],
                    masked_zero,
                    cache.asset(&asset_id)?,
                )
                .ok(),
                _ => None,
            }
        });
        // The SysMenu's own icon bitmaps: primary 0x2c ("on") and 0x2d
        // ("off"), 11x10 masked, drawn at the hot area's origin.
        let primary = |id: &str| -> Option<IndexedImage> {
            let asset_id = format!("base:bitmap/primary/{id}");
            let record = cache.manifest().files.iter().find(|r| r.id == asset_id)?;
            match record.asset {
                AssetKind::IndexedBitmap {
                    width,
                    height,
                    offset_x,
                    offset_y,
                    masked_zero,
                    ..
                } => IndexedImage::new(
                    width,
                    height,
                    [offset_x, offset_y],
                    masked_zero,
                    cache.asset(&asset_id)?,
                )
                .ok(),
                _ => None,
            }
        };
        screen.prefs_on = primary("044");
        screen.prefs_off = primary("045");
        Ok(screen)
    }

    /// The console view button under logical pixel (x, y), if any (`DATA_007684`
    /// hot areas), or the F1 cycle area.
    pub fn view_button(x: i32, y: i32) -> Option<ViewButton> {
        match crate::console::click(x, y)? {
            crate::console::Click::Front => Some(ViewButton::Select(ViewMode::Front)),
            crate::console::Click::Rear => Some(ViewButton::Select(ViewMode::Rear)),
            crate::console::Click::External => Some(ViewButton::Select(ViewMode::External)),
            crate::console::Click::Turret => Some(ViewButton::Turret),
            crate::console::Click::CycleView => Some(ViewButton::Cycle),
            _ => None,
        }
    }

    /// Any console click under logical pixel (x, y): `DATA_007684`'s hot areas.
    /// The port's map/autopilot buttons are retired for the original HUD.
    pub fn console_click(x: i32, y: i32) -> Option<crate::console::Click> {
        crate::console::click(x, y)
    }

    /// The body under logical pixel (x, y): of `bodies` (an index, its centre and radius in
    /// flight units), the one whose screen position is nearest the click and within its
    /// pick radius.
    pub fn pick(
        state: &Flight,
        views: &ViewState,
        scene: Option<&ModelViewer>,
        bodies: &[(usize, [f64; 3], f64)],
        (x, y): (i32, i32),
    ) -> Option<usize> {
        bodies
            .iter()
            .filter_map(|(index, centre, radius)| {
                let (sx, sy, reach) = Self::body_screen(state, views, scene, *centre, *radius)?;
                let miss = f64::from((sx - x).pow(2) + (sy - y).pow(2)).sqrt();
                (miss <= f64::from(reach)).then_some((*index, miss))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    /// Where a body shows in the view `state` and `views` give: its screen position and a
    /// pick radius in pixels (the body's own size, at least 10), when it is in front of
    /// the eye and on the view. `centre` and `radius` are in flight units.
    pub fn body_screen(
        state: &Flight,
        views: &ViewState,
        scene: Option<&ModelViewer>,
        centre: [f64; 3],
        radius: f64,
    ) -> Option<(i32, i32, i32)> {
        let (sx, sy, pixels) = Self::body_projection_pixels(state, views, scene, centre, radius)?;
        ((-20.0..340.0).contains(&sx) && (-20.0..178.0).contains(&sy)).then_some((
            sx as i32,
            sy as i32,
            pixels as i32,
        ))
    }

    /// F1100's target screen words (`object+0x98/+0x9a`) without F989's view
    /// bounds: where the body projects even when it is outside the 320x158 view.
    /// `None` when it is behind the eye. The range readout uses these to pick
    /// the off-view arrow form.
    pub fn body_projection(
        state: &Flight,
        views: &ViewState,
        scene: Option<&ModelViewer>,
        centre: [f64; 3],
    ) -> Option<(i32, i32)> {
        let (sx, sy, _) = Self::body_projection_pixels(state, views, scene, centre, 0.0)?;
        Some((sx as i32, sy as i32))
    }

    /// `centre` (flight units) in the view camera's frame: x right, y up,
    /// z forward, whether or not it is in front.
    pub fn camera_point(
        state: &Flight,
        views: &ViewState,
        scene: Option<&ModelViewer>,
        centre: [f64; 3],
    ) -> [f64; 3] {
        let stats = scene.and_then(|scene| scene.ship_stats(state.ship));
        let scale = scene.and_then(model_scale).unwrap_or(DEFAULT_MODEL_SCALE);
        Camera::new(state, views, stats.as_ref(), scale).point(centre)
    }

    fn body_projection_pixels(
        state: &Flight,
        views: &ViewState,
        scene: Option<&ModelViewer>,
        centre: [f64; 3],
        radius: f64,
    ) -> Option<(f64, f64, f64)> {
        let stats = scene.and_then(|scene| scene.ship_stats(state.ship));
        let scale = scene.and_then(model_scale).unwrap_or(DEFAULT_MODEL_SCALE);
        let camera = Camera::new(state, views, stats.as_ref(), scale);
        let [x, y, z] = camera.point(centre);
        if z <= 2.0 {
            return None;
        }
        let (sx, sy) = (160.0 + 256.0 * x / z, 79.0 - 256.0 * y / z);
        let pixels = (radius * 256.0 / z).clamp(10.0, 120.0);
        Some((sx, sy, pixels))
    }

    /// The logical 320x200 pixel under a mouse position in window client pixels
    /// (`Window::get_unscaled_mouse_pos`), for a client area of `size` showing the
    /// WIDTH x HEIGHT buffer with minifb's aspect-ratio stretch. `None` on the
    /// letterbox bars or outside the window.
    pub fn logical_point(x: f32, y: f32, size: (usize, usize)) -> Option<(i32, i32)> {
        let (window_width, window_height) = (size.0 as i32, size.1 as i32);
        if window_width <= 0 || window_height <= 0 {
            return None;
        }
        // As minifb's WM_PAINT places the picture.
        let aspect = WIDTH as f32 / HEIGHT as f32;
        let (mut width, mut height, mut left, mut top) = (window_width, window_height, 0, 0);
        if aspect > window_width as f32 / window_height as f32 {
            height = (window_width as f32 / aspect) as i32;
            top = (window_height - height) / 2;
        } else {
            width = (window_height as f32 * aspect) as i32;
            left = (window_width - width) / 2;
        }
        let (px, py) = (x - left as f32, y - top as f32);
        if width <= 0 || height <= 0 || px < 0.0 || py < 0.0 {
            return None;
        }
        let (lx, ly) = (
            (px * LOGICAL_WIDTH as f32 / width as f32) as i32,
            (py * LOGICAL_HEIGHT as f32 / height as f32) as i32,
        );
        (lx < LOGICAL_WIDTH as i32 && ly < LOGICAL_HEIGHT as i32).then_some((lx, ly))
    }

    /// F955: the cabin backdrop for the player's ship, chosen by its hull
    /// mass (`DATA_007222`'s bounds 32, 75, 300 and 1000 t); model `0x34`
    /// always gets `CABIN5`.
    pub fn select_cabin(&mut self, model: u32, mass: Option<u16>) {
        const BOUNDS: [i32; 4] = [0x20, 0x4b, 0x12c, 0x3e8];
        let index = if model == 0x34 {
            5
        } else {
            let mass = i32::from(mass.unwrap_or(0) as i16);
            BOUNDS.iter().take_while(|bound| mass >= **bound).count()
        };
        self.cabin_index = index.min(self.cabins.len() - 1);
    }

    pub fn from_assets(
        cabin_png: &[u8],
        font: &[u8],
        panel: IndexedImage,
        palette: [u32; 128],
    ) -> Result<Self, String> {
        let font = BitmapFont::new(font)?;
        let cabin = decode_cabin(cabin_png)?;
        Ok(Self {
            pixels: vec![0; WIDTH * HEIGHT],
            frame: vec![0; LOGICAL_WIDTH * LOGICAL_HEIGHT],
            cabins: vec![cabin],
            cabin_index: 0,
            panel,
            font,
            palette,
            indexed: Box::new([0; 64000]),
            // `FUNC_000219_SysMenuInit`'s default: "Light Blue" (4), drawn as
            // the captured quantised palette entry (`#0c0c4c`).
            sky: prefs::space_colour_rgb(4),
            surface_detail: 1,
            palette_build_per_object: false,
            surface_sky: None,
            stars_visible: true,
            lock_external: true,
            dust_visible: true,
            shape_detail: [1, 1],
            object_queue: None,
            discs: Vec::new(),
            stations: Vec::new(),
            port_pads: None,
            ships: Vec::new(),
            effects: Vec::new(),
            missiles: Vec::new(),
            ship_beams: Vec::new(),
            missile_lock: false,
            missile_blink: false,
            sun: None,
            market: None,
            dock: None,
            issue: None,
            ship_view: None,
            mastheads: Vec::new(),
            pad_services: false,
            lift: None,
            station_view: false,
            target_mark: None,
            scanner: None,
            tunnels: None,
            targeter: None,
            blink: false,
            labels: Vec::new(),
            combat_readouts: Vec::new(),
            galaxy: None,
            enhanced_galaxy: None,
            system_map: None,
            map_console: crate::console::MapConsole::default(),
            has_lock: false,
            lasers: [0; 4],
            beam: None,
            fixed_laser: false,
            energy: 0,
            heat: 0,
            drive: 0,
            target_hud: None,
            target_damage: None,
            target_speed: None,
            combat_labels: Vec::new(),
            npc_button: false,
            npc_list_open: false,
            npc_list_selected: None,
            npc_icon: None,
            npc_icon_pressed: None,
            edge_indicator: None,
            mission_fixes: false,
            hud: PlayerHud::default(),
            systems: ShipSystems::default(),
            messages: MessageLog::new(),
            messages_visible: true,
            profile: None,
            cargo_inventory: None,
            roster: None,
            contracts: None,
            mining_list: None,
            status_backdrop: None,
            equipment_status: None,
            clock: String::new(),
            console: Console::new(),
            gauges: [None, None, None],
            system: String::new(),
            station: String::new(),
            lock: String::new(),
            icons: Vec::new(),
            marker: None,
            diagram: Vec::new(),
            dock_icons: [None, None, None],
            prefs_on: None,
            prefs_off: None,
            intro_bitmap: None,
            intro_text: None,
            intro_error_reported: false,
            hyperspace: None,
            jitter: Jitter::default(),
        })
    }

    fn pixel(&mut self, x: i32, y: i32, colour: u8) {
        if (0..320).contains(&x) && (0..200).contains(&y) {
            self.frame[y as usize * 320 + x as usize] = self.palette[usize::from(colour)];
        }
    }

    /// F270's three bars at y161: the energy pool scaled by the drive, the laser
    /// heat, and the third value (always zero in this build). Each is clipped
    /// from the top at `0xad - (value * 12 >> 16)`.
    fn draw_gauges(&mut self) {
        let values = [
            ffe_core::laser::energy_gauge(self.energy, self.drive),
            u32::from(self.heat),
            0,
        ];
        for (index, value) in values.into_iter().enumerate() {
            if let Some(icon) = self.gauges[index].as_ref() {
                let ymin = 0xad - ffe_core::laser::gauge_height(value);
                icon.blit_clipped(&mut self.indexed, [249, 242, 237][index], 161, ymin);
            }
        }
    }

    /// F1776's visible beam: model 0x9c (156) at the gun, oriented along the
    /// mount's axes.
    fn draw_beam(
        &mut self,
        scene: &ModelViewer,
        state: &Flight,
        beam: &crate::combat::LaserBeam,
        camera: &Camera,
    ) {
        let scale = model_scale(scene).unwrap_or(DEFAULT_MODEL_SCALE);
        let rows = beam.axes.map(|axis| camera.direction(axis));
        // F1776 copies the firing object (`F1533`, 0x152 bytes) and changes only
        // the model to 0x9c. The copy's `+0x7e` is the object's extra render
        // shift, which is 0 for the player (read from the executed original at
        // the shot, `docs/KNOWN_ISSUES.md`), not the ship's model scale, so the
        // beam is drawn at model 156's *own* scale 17, where its vertices are in
        // position units and the ray ends on the hit point. Drawing it at the
        // ship's scale 8 made the beam 512 times too small: a short stub that
        // pointed wherever the ship did. At the right scale the Eagle's front
        // view shows the original's large wedge from the crosshair to the foot
        // of the view, and the Saker's a one-pixel ray.
        let view_origin = camera.point(beam.origin);
        let origin = view_origin.map(|v| v * scale);
        let light = self.scene_light(scene, camera);
        let mut inputs = AnimationInputs::original(flight_tics(state), 0, &scene.shades());
        inputs.jitter = self.jitter.cursor;
        // F1776 stores the mount's laser type at `+0xa4` (global 4), which
        // picks the flash and ray; F931 left the shot's length at `+0xe6`
        // (global 37). The ray's far end is the gun plus 0x40 << 17 (the
        // model's own scale, F931's 0x7fffff range) times length / 0x10000,
        // so a hit's `distance >> 7` ends it on the target.
        inputs.globals[4] = u16::from(beam.code.unwrap_or(self.lasers[beam.mount]));
        inputs.globals[37] = beam.length;
        if let Ok(frame) = scene.render_scene_with_scale(
            crate::combat::BEAM_MODEL,
            None,
            rows,
            origin,
            light,
            &inputs,
        ) {
            self.jitter.cursor = frame.jitter;
            self.draw_frame(scene, &frame);
        }
    }

    /// F1111's fixed gunsight: secondary bitmap 1 (id 97) at (129,48), drawn
    /// only in the views F1099 does not skip (not external or combat) and only
    /// when that view's mount has a laser (`ship+0xd2+view`).
    fn gunsight_mount(&self, view: ViewMode) -> Option<usize> {
        let mount = match view {
            ViewMode::Front => 0,
            ViewMode::Rear => 1,
            ViewMode::TopTurret => 2,
            ViewMode::BottomTurret => 3,
            // F1099 skips F1111 in external, the missile view and the combat
            // computer (the missile view jumps over the whole HUD block).
            ViewMode::External | ViewMode::Missile | ViewMode::Combat => return None,
        };
        (self.lasers[mount] != 0).then_some(mount)
    }

    fn draw_gunsight(&mut self, view: ViewMode) {
        if self.gunsight_mount(view).is_none() {
            return;
        }
        if let Some(Some(icon)) = self.icons.get(crate::combat::GUNSIGHT_BITMAP as usize) {
            icon.blit_colours(&mut self.frame, &self.palette, 129, 48);
            // The bitmap's colour 15 is its line; 9 and 12 only soften its edges.
        }
        self.draw_targeter_cursor(view);
        // F1111: the combat target (`DATA_007247`) draws F1114's circle at
        // (0x9f,0x4f) with F1612's radius word 0xc (halved to 6) in colour
        // 0x10, while F1113's blink is on (`ffe.asm:81967-81991`).
        if self.missile_lock && self.missile_blink {
            self.circle(0x9f, 0x4f, 0x0c, targeter::COMBAT_COLOUR);
        }
    }

    /// F1111's auto-targetter cursor, drawn after the gunsight: `DATA_007246`'s
    /// navigation box or `DATA_007247`'s combat circle, both only while
    /// `DATA_007252`'s blink phase is on.
    fn draw_targeter_cursor(&mut self, view: ViewMode) {
        if !self.blink || self.gunsight_mount(view).is_none() {
            return;
        }
        let Some(cursor) = self.targeter else {
            return;
        };
        match cursor {
            targeter::Cursor::Navigation => {
                let value = self.palette[usize::from(targeter::NAVIGATION_COLOUR)];
                let ((x, y), half) = targeter::CURSOR_BOX;
                for (from, to) in targeter::box_edges(x, y, half) {
                    self.line_value(from, to, value);
                }
            }
            targeter::Cursor::Combat => {
                let value = self.palette[usize::from(targeter::COMBAT_COLOUR)];
                let ((x, y), radius) = targeter::CURSOR_CIRCLE;
                for (px, py) in targeter::circle((x, y), radius) {
                    if (0..320).contains(&px) {
                        self.frame[py as usize * 320 + px as usize] = value;
                    }
                }
            }
        }
    }

    /// F1100/F1113's targeting tunnels: four nested squares on the line from the
    /// view centre to the locked target, in the target's colour.
    fn draw_target_tunnels(&mut self) {
        let Some(tunnels) = self.tunnels else {
            return;
        };
        let value = self.palette[usize::from(tunnels.colour)];
        for boxed in targeter::tunnels(tunnels) {
            for (from, to) in targeter::box_edges(boxed.x, boxed.y, boxed.half) {
                self.line_value(from, to, value);
            }
        }
    }

    /// F272's scanner orientation overlay: secondary 254..261 at (101,175),
    /// selected by `(DATA_008933 >> 9) & 7`. The console panel's rings are the
    /// background; the contacts are drawn over this globe.
    fn draw_scanner_overlay(&mut self) {
        let orientation = self.scanner.as_ref().map_or(0, |view| view.orientation & 7);
        if let Some(Some(overlay)) = self.icons.get(254 + orientation) {
            overlay.blit(&mut self.indexed, 101, 175);
        }
    }

    /// F274/F275's contacts. The console is composed before the view, so the
    /// stalks are painted into the finished frame here: an up stalk may cross
    /// y 158 into the view, which the original allows while the ship module is
    /// active (`DATA_008630` negative, clip flag 0).
    fn draw_scanner_contacts(&mut self) {
        // F272 only calls F274 when `ship+0xca & 4`: the scanner is fitted.
        if self.systems.ca & 4 == 0 {
            return;
        }
        let Some(view) = self.scanner.clone() else {
            return;
        };
        if std::env::var_os("FFE_DBG_SCAN").is_some() {
            eprintln!(
                "scan pos={:?} frame={:?} axes={:?}",
                view.position, view.frame, view.axes
            );
            for c in &view.contacts {
                eprintln!(
                    "  contact {:?} vis={} mark={:?}",
                    c,
                    scanner::visible(c, view.frame),
                    view.mark(c)
                );
            }
        }
        for mark in view.marks() {
            self.draw_scan_mark(mark);
        }
    }

    /// F1569's stalk: two 2-pixel-wide bars above (positive `stalk`, F1570) or
    /// below (negative, F1571) the contact. The downward bars use F1628, which
    /// leaves the scanner grid's colours 0x28..0x2b untouched.
    fn draw_scan_mark(&mut self, mark: scanner::Mark) {
        let (x, y) = (mark.x, mark.y);
        // Both stalk directions clamp the bottom to row 198 (0xc6).
        let bottom = |value: i32| value.min(0xc6);
        if mark.stalk >= 0 {
            let (top, low) = (y - mark.stalk - 1, bottom(y));
            self.scan_box(x - 1, top, x, low, mark.colour, false);
            self.scan_box(x + 1, top, x + 2, low, mark.colour, false);
        } else {
            let (top, low) = (y, bottom(y - mark.stalk + 1));
            self.scan_box(x - 1, top, x, low, mark.colour, true);
            self.scan_box(x + 1, top, x + 2, low, mark.colour, true);
        }
    }

    /// F1627/F1628's box fill: inclusive, clipped to the logical screen, with
    /// F1628's grid mask when `masked`.
    fn scan_box(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, colour: u8, masked: bool) {
        if y0 > y1 {
            return;
        }
        let value = self.palette[usize::from(colour)];
        for y in y0.max(0)..=y1.min(199) {
            for x in x0.max(0)..=x1.min(319) {
                let at = y as usize * 320 + x as usize;
                if masked && self.indexed[at] & 0xfc == 0x28 {
                    continue;
                }
                self.frame[at] = value;
            }
        }
    }

    /// F1112's velocity marker: bitmap 98 at the velocity direction, offset by
    /// (-19,-18), in the front view only.
    fn draw_velocity_marker(&mut self, state: &Flight, camera: &Camera) {
        let Some((x, y)) = crate::combat::velocity_marker(camera, state.velocity) else {
            return;
        };
        if let Some(Some(icon)) = self
            .icons
            .get(crate::combat::VELOCITY_MARKER_BITMAP as usize)
        {
            icon.blit_colours(&mut self.frame, &self.palette, x - 19, y - 18);
        }
    }

    /// F1100's range text, and F1099's target shields/hull (when a ship target
    /// carries them). The on-view number sits at the target minus 5; the original
    /// string's leading newline moves it one 10-row line down, so the glyphs are
    /// at `x-5, y+5`. Off-view targets use the caller's fixed spot ((50,50) for
    /// the navigation target, (50,60) for combat) and the Behind / Out of Range
    /// / arrow strings.
    fn draw_target_hud(&mut self) {
        if let Some(target) = self.target_hud {
            let form = targeter::range_form(target.behind, target.offscreen, target.x, target.y);
            // F1100's fixed spot: the navigation caller's (50,50), the combat
            // caller's (50,60). The colour is the same selector F1099 uses.
            let fixed = if target.colour == targeter::NAVIGATION_COLOUR {
                (50, 50)
            } else {
                (50, 60)
            };
            let square = self.npc_button
                && target.colour == targeter::COMBAT_COLOUR
                && matches!(form, targeter::RangeForm::OnView);
            let (x, y) = if matches!(form, targeter::RangeForm::OnView) {
                // The leading newline in 0x8604/05 moves the text down 10 rows;
                // the Combat Targeting Computer's square moves it below its corners.
                (
                    target.x - 5,
                    target.y - 5 + 10 + if square { NPC_SQUARE_HALF - 2 } else { 0 },
                )
            } else {
                fixed
            };
            let text = targeter::range_text(form, target.value, target.au);
            self.text_bytes(x.max(0) as usize, y.max(0) as usize, &text, target.colour);
            // The Combat Targeting Computer's target square: a red square round
            // the combat target while it is in the view, drawn as four corners.
            if square {
                let value = self.palette[usize::from(targeter::COMBAT_COLOUR)];
                let (half, arm) = (NPC_SQUARE_HALF, NPC_SQUARE_ARM);
                for (sx, sy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let corner = (target.x + sx * half, target.y + sy * half);
                    self.line_value(corner, (corner.0 - sx * arm, corner.1), value);
                    self.line_value(corner, (corner.0, corner.1 - sy * arm), value);
                }
            }
        }
        if let Some(damage) = self.target_damage {
            let hull = damage.hull;
            let mut lines = vec![format!("Hull: {}.{}%", hull / 10, hull % 10)];
            match damage.shields {
                Some(shields) => {
                    lines.push(format!("Shields: {shields}.0%"));
                    lines.push(format!("Shield Strength {}", damage.shield_strength));
                }
                None => lines.push("Shields: None".to_owned()),
            }
            for (index, line) in lines.iter().enumerate() {
                self.text(235, 1 + index * 10, line, 15);
            }
            // enhancement-9: the relative-speed line follows the hull/shields.
            let speed = self.target_speed.clone();
            if let Some(speed) = speed {
                self.text(235, 1 + lines.len() * 10, &speed, 15);
            }
        }
    }

    /// Planets and moons as shaded discs, the farthest first (a stand-in for the
    /// surface renderer: size, position and phase are right, the surface is not).
    fn draw_discs(&mut self, camera: &Camera, viewer: Option<&ModelViewer>) {
        let mut order: Vec<(f64, Disc)> = self
            .discs
            .iter()
            .map(|d| (camera.point(d.centre)[2], *d))
            .collect();
        order.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (z, disc) in order {
            let light = disc.light;
            // F1680's set-up: past the model's far class nothing is drawn; from its dot class
            // the body is a dot (a plus-shaped glint for a planet or moon, a particle for a
            // star), not a model a hundredth of a pixel across; otherwise the instance runs
            // with F1680's own shifted position and scales.
            if let (Some((model, seed)), Some(viewer)) = (disc.model, viewer) {
                if let Some(descriptor) = viewer.object_descriptor(model) {
                    match f1680_body(camera, disc.centre, &descriptor) {
                        ffe_core::model_animation::Draw::Far { .. } => continue,
                        ffe_core::model_animation::Draw::Culled { .. } => continue,
                        ffe_core::model_animation::Draw::Dot {
                            screen,
                            depth,
                            radius,
                            ..
                        } => {
                            let classes = viewer.object_model(model);
                            let (Some(screen), Some(classes)) = (screen, classes) else {
                                continue;
                            };
                            match viewer.object_dot_frame(classes, screen, radius, depth) {
                                Ok(Some(frame)) => {
                                    self.draw_frame(viewer, &frame);
                                    continue;
                                }
                                Ok(None) => continue,
                                // A shaded sphere the port cannot draw: the model instead.
                                Err(_) => {}
                            }
                        }
                        ffe_core::model_animation::Draw::Model { instance, .. } => {
                            viewer.surface_seed.set(seed);
                            // The object's `+0xa0` words: global 2 >> 14 picks the ring
                            // (model 134 calls child 314 only when it is non-zero; Saturn's
                            // seed low word 0xffa1 gives 3).
                            let mut inputs = AnimationInputs::test(0);
                            inputs.globals[2] = seed as u16;
                            inputs.globals[3] = (seed >> 16) as u16;
                            let rows = disc.orientation.map(|axis| camera.direction(axis));
                            let light = camera.direction(light);
                            // F1680's instance: the shifted view position (`+0x4`), the
                            // operand scale (`+0x148`, lowered once the precision shift
                            // exceeds the descriptor's offset) and the key scale (`+0x130`).
                            // F1750 subtracts the key scale from every child's scale, so
                            // passing the descriptor offset alone made a ring child `2^e`
                            // too large; folding the shift into the matrix instead had the
                            // same effect and also broke F1777's depth key.
                            if let Ok(frame) = viewer.render_scene_with_instance_scale(
                                model,
                                instance.instance.operand_scale.max(0) as u32,
                                instance.instance.key_scale,
                                rows,
                                instance.instance.position.map(f64::from),
                                light,
                                &inputs,
                            ) {
                                self.draw_frame(viewer, &frame);
                                continue;
                            }
                        }
                    }
                }
            }
            // A model without a descriptor (or one the view space refused): keep the
            // port's descriptor-less fallback, which infers the model-to-world scale
            // from the surface axes and shrinks a distant matrix to fit.
            if let (Some((model, seed)), Some(viewer)) = (disc.model, viewer) {
                // The planet's radius in the model's units is its surface axes' length;
                // one render at the origin reads it (the model's vertices reach farther).
                if viewer.find(model).is_some() {
                    let radius = viewer.probe_surface_extent(model);
                    if radius > 0.0 {
                        let factor = viewer
                            .world_units_per_model_unit(model)
                            .map(|unit| crate::original_flight::UNITS_PER_FLIGHT_UNIT / unit)
                            .unwrap_or(radius / disc.radius);
                        let mut position = camera.point(disc.centre).map(|v| v * factor);
                        viewer.surface_seed.set(seed);
                        // The object's `+0xa0` words: global 2 >> 14 picks the ring
                        // (model 134 calls child 314 only when it is non-zero; Saturn's
                        // seed low word 0xffa1 gives 3). The old test input left it 0,
                        // so no planet ever drew a ring.
                        let mut inputs = AnimationInputs::test(0);
                        inputs.globals[2] = seed as u16;
                        inputs.globals[3] = (seed >> 16) as u16;
                        // F1680's instance +0x130 is the descriptor scale offset, which
                        // F1750 subtracts from the ring child's scale. With 0 the child
                        // shifted by 31 and its geometry overflowed.
                        let key_scale = viewer.model_scale_offset(model).unwrap_or(0);
                        let light = camera.direction(light);
                        // The view space is 32-bit: a distant planet is drawn through a
                        // matrix shrunk by a power of two (the position shrinks with it),
                        // which leaves the picture as it was. The terrain's 32-bit
                        // arithmetic keeps the planet's radius and distance well inside the
                        // range the original runs it in (radii of 2^20 to 2^26).
                        let reach = position.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
                        let shrink = (reach / 1.0e9)
                            .max(radius / 16_777_216.0)
                            .max(1.0)
                            .log2()
                            .ceil()
                            .min(16.0);
                        let shrunk = 0.5_f64.powf(shrink);
                        position = position.map(|v| v * shrunk);
                        let rows = disc
                            .orientation
                            .map(|axis| camera.direction(axis).map(|v| v * shrunk));
                        // The shrink is already in the matrix, which every child call
                        // inherits. The instance key scale must stay F1680's: adding the
                        // shrink to it would shrink the ring child a second time (F1750
                        // subtracts it from the child scale) and collapse the rings.
                        if let Ok(frame) = viewer.render_scene_with_instance(
                            model, rows, position, light, &inputs, key_scale,
                        ) {
                            self.draw_frame(viewer, &frame);
                            continue;
                        }
                    }
                }
            }
            // The Classic disc is an exterior view.
            if z <= 0.0 || z <= disc.radius.max(2.0) {
                continue;
            }
            let ahead = camera.point(disc.centre);
            let [cx, cy, _] = ahead;
            let scale = 256.0 / z;
            let radius = disc.radius * scale;
            if radius < 0.6 {
                self.pixel(
                    (160.0 + cx * scale) as i32,
                    (79.0 - cy * scale) as i32,
                    disc.colours.0,
                );
                continue;
            }
            let light = camera.direction(light);
            let (x0, y0) = (160.0 + cx * scale, 79.0 - cy * scale);
            let top = (y0 - radius).floor().max(0.0) as i32;
            let bottom = (y0 + radius).ceil().min(157.0) as i32;
            for y in top..=bottom {
                let dy = (f64::from(y) + 0.5 - y0) / radius;
                if dy.abs() > 1.0 {
                    continue;
                }
                let half = (1.0 - dy * dy).sqrt() * radius;
                let left = (x0 - half).round().max(0.0) as i32;
                let right = (x0 + half).round().min(319.0) as i32;
                for x in left..=right {
                    let dx = (f64::from(x) + 0.5 - x0) / radius;
                    let depth = (1.0 - dx * dx - dy * dy).max(0.0).sqrt();
                    // The sphere normal in view space (y is up on screen).
                    let lit = dx * light[0] - dy * light[1] + depth * light[2];
                    let colour = if disc.star || lit > 0.0 {
                        disc.colours.0
                    } else {
                        disc.colours.1
                    };
                    self.pixel(x, y, colour);
                }
            }
        }
    }

    fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, colour: u8) {
        for row in y..y.saturating_add(height).min(200) {
            for col in x..x.saturating_add(width).min(320) {
                self.frame[row * 320 + col] = self.palette[usize::from(colour)];
            }
        }
    }

    fn line(&mut self, start: (i32, i32), end: (i32, i32), colour: u8) {
        let value = self.palette[usize::from(colour)];
        self.line_value(start, end, value);
    }

    /// `FUNC_001612_DrawCircle`, used by F1111's combat target mark: `size` is
    /// the diameter word (it is halved, so F1111's 0xc is a radius of 6) and the
    /// rows are those of [`ffe_core::model_dot::circle`].
    fn circle(&mut self, x: i32, y: i32, size: i32, colour: u8) {
        let value = self.palette[usize::from(colour)];
        for span in ffe_core::model_dot::circle(x, y, size) {
            for px in span.x..span.x + span.width {
                if (0..320).contains(&px) && (0..158).contains(&span.y) {
                    self.frame[span.y as usize * 320 + px as usize] = value;
                }
            }
        }
    }

    /// Bresenham in the 320x158 view, bounded to 4096 steps.
    fn line_value(&mut self, start: (i32, i32), end: (i32, i32), value: u32) {
        let (mut x, mut y) = start;
        let (end_x, end_y) = end;
        let dx = (end_x - x).abs();
        let dy = -(end_y - y).abs();
        let sx = if x < end_x { 1 } else { -1 };
        let sy = if y < end_y { 1 } else { -1 };
        let mut error = dx + dy;
        for _ in 0..4096 {
            if (0..VIEW_HEIGHT as i32).contains(&y) && (0..320).contains(&x) {
                self.frame[y as usize * 320 + x as usize] = value;
            }
            if x == end_x && y == end_y {
                break;
            }
            let twice = 2 * error;
            if twice >= dy {
                error += dy;
                x += sx;
            }
            if twice <= dx {
                error += dx;
                y += sy;
            }
        }
    }

    /// Even-odd scanline fill at pixel centres, clipped to the 320x158 view.
    /// A triangle with the colour blended from its corners (barycentric), clipped to
    /// the view.
    fn gouraud(&mut self, p: [(i32, i32); 3], c: [[u8; 3]; 3]) {
        let (x0, y0) = (f64::from(p[0].0), f64::from(p[0].1));
        let area = (f64::from(p[1].0) - x0) * (f64::from(p[2].1) - y0)
            - (f64::from(p[2].0) - x0) * (f64::from(p[1].1) - y0);
        if area == 0.0 {
            return;
        }
        let top = p.iter().map(|q| q.1).min().unwrap_or(0).max(0);
        let bottom = p
            .iter()
            .map(|q| q.1)
            .max()
            .unwrap_or(0)
            .min(VIEW_HEIGHT as i32 - 1);
        let left = p.iter().map(|q| q.0).min().unwrap_or(0).max(0);
        let right = p.iter().map(|q| q.0).max().unwrap_or(0).min(319);
        for y in top..=bottom {
            for x in left..=right {
                let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                let edge = |a: (i32, i32), b: (i32, i32)| {
                    (f64::from(b.0) - f64::from(a.0)) * (py - f64::from(a.1))
                        - (f64::from(b.1) - f64::from(a.1)) * (px - f64::from(a.0))
                };
                let w = [
                    edge(p[1], p[2]) / area,
                    edge(p[2], p[0]) / area,
                    edge(p[0], p[1]) / area,
                ];
                if w.iter().any(|v| *v < -1e-9) {
                    continue;
                }
                let channel = |k: usize| {
                    (w[0] * f64::from(c[0][k])
                        + w[1] * f64::from(c[1][k])
                        + w[2] * f64::from(c[2][k]))
                    .round()
                    .clamp(0.0, 255.0) as u32
                };
                self.frame[y as usize * 320 + x as usize] =
                    channel(0) << 16 | channel(1) << 8 | channel(2);
            }
        }
    }

    fn fill_value(&mut self, points: &[(i32, i32)], value: u32) {
        let Some(top) = points.iter().map(|p| p.1).min() else {
            return;
        };
        let bottom = points.iter().map(|p| p.1).max().unwrap_or(top);
        let mut crossings = Vec::with_capacity(points.len());
        for y in top.max(0)..=bottom.min(VIEW_HEIGHT as i32 - 1) {
            let centre = f64::from(y) + 0.5;
            crossings.clear();
            for (i, a) in points.iter().enumerate() {
                let b = points[(i + 1) % points.len()];
                let (ay, by) = (f64::from(a.1), f64::from(b.1));
                if (ay <= centre) != (by <= centre) {
                    let t = (centre - ay) / (by - ay);
                    crossings.push(f64::from(a.0) + t * f64::from(b.0 - a.0));
                }
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.chunks_exact(2) {
                let start = (pair[0] - 0.5).ceil().max(0.0) as i32;
                let end = ((pair[1] - 0.5).floor() as i32).min(319);
                for x in start..=end {
                    self.frame[y as usize * 320 + x as usize] = value;
                }
            }
        }
        // Edges as well, so thin and edge-on faces stay visible.
        for (i, a) in points.iter().enumerate() {
            self.line_value(*a, points[(i + 1) % points.len()], value);
        }
    }

    fn text(&mut self, x: usize, y: usize, message: &str, colour: u8) {
        self.text_screen(x, y as i32, message, colour);
    }

    /// F1584's text writer (`FUNC_001587_TextWriteInternal` + `F1598_DrawChar`)
    /// at a signed y, clipped to the logical screen: glyph ink is clipped to
    /// x 0..320 and y 0..200, and no glyph is started at x >= 0x13c (F1587's
    /// right-hand limit, which keeps DrawChar's row writes on the buffer).
    fn text_screen(&mut self, x: usize, y: i32, message: &str, colour: u8) {
        let bytes: Vec<u8> = message
            .chars()
            .map(|character| {
                if character.is_ascii() && !character.is_ascii_control() {
                    character as u8
                } else {
                    b'?'
                }
            })
            .collect();
        self.text_bytes_rows(x, y, &bytes, colour, 0, 200);
    }

    /// [`Self::text_screen`] with F1586's y clip (`top..bottom`).
    fn text_bytes_rows(
        &mut self,
        x: usize,
        y: i32,
        message: &[u8],
        colour: u8,
        top: i32,
        bottom: i32,
    ) {
        let mut cursor = x as i32;
        for &code in message {
            // F1587 stops calling DrawChar once the pen reaches x 0x13c.
            if cursor >= 0x13c {
                break;
            }
            if let Some(glyph) = self.font.glyph(code) {
                let value = self.palette[usize::from(colour)];
                for (col, row) in glyph.pixels() {
                    let px = cursor + col as i32;
                    let py = y + row as i32;
                    if (0..320).contains(&px) && (top..bottom).contains(&py) && py < 200 {
                        self.frame[py as usize * 320 + px as usize] = value;
                    }
                }
                cursor += glyph.advance as i32;
            }
        }
    }

    fn text_bytes(&mut self, x: usize, y: usize, message: &[u8], colour: u8) {
        self.text_bytes_rows(x, y as i32, message, colour, 0, 200);
    }

    fn present(&mut self) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                self.pixels[y * WIDTH + x] = self.frame[(y * 200 / HEIGHT) * 320 + x * 320 / WIDTH];
            }
        }
    }

    pub fn logical_pixels(&self) -> &[u32] {
        &self.frame
    }

    /// The 320x200 frame to draw over after [`Screen::draw`] (a CD video);
    /// [`Screen::recompose`] then refreshes the presented copy.
    pub fn logical_pixels_mut(&mut self) -> &mut [u32] {
        &mut self.frame
    }

    /// Recompose the presented 640x480 copy from the logical frame.
    pub fn recompose(&mut self) {
        self.present();
    }

    /// The main font's advances by code from 32, for fitting a real font to it.
    pub fn font_advances(&self) -> Vec<u8> {
        (32..=u8::MAX)
            .map_while(|code| self.font.advance(code))
            .map(|advance| advance as u8)
            .collect()
    }

    /// The six cabin backdrops (320x158 RGB each), in F955's order.
    pub fn cabin_pixels(&self) -> &[Vec<u32>] {
        &self.cabins
    }

    /// The console panel (primary bitmap 43) as the frame shows it below the view: 320x42 RGB.
    pub fn panel_pixels(&self) -> Vec<u32> {
        let mut frame = vec![0; LOGICAL_WIDTH * LOGICAL_HEIGHT];
        self.panel
            .blit_rgb(&mut frame, &self.palette, 0, VIEW_HEIGHT as i32);
        frame.split_off(VIEW_HEIGHT * LOGICAL_WIDTH)
    }

    /// The expanded fixed low palette, for tests that check a drawn colour.
    pub fn palette(&self) -> &[u32; 128] {
        &self.palette
    }

    /// The labels the original's F989/F1108 would draw this frame.
    pub fn label_positions(&self, camera: &Camera) -> Vec<(String, i32, i32)> {
        body_label_positions(&self.labels, camera)
    }

    /// The main font's width of `text` in pixels, for the combat label's click
    /// box (F1584's glyph advances).
    pub fn text_width(&self, text: &str) -> usize {
        self.font.width(text).unwrap_or(0)
    }

    /// F1109: each ship's range, hull/shields and type in amber below it.
    fn draw_combat_readouts(&mut self, camera: &Camera) {
        let readouts = std::mem::take(&mut self.combat_readouts);
        for readout in &readouts {
            let Some((x, y)) = camera.project(readout.centre, true) else {
                continue;
            };
            let Some((x, y)) = crate::combat_computer::text_origin(x, y) else {
                continue;
            };
            for (row, line) in readout.lines.iter().enumerate() {
                self.text_screen(
                    x.max(0) as usize,
                    y + (row * crate::messages::LINE_HEIGHT) as i32,
                    line,
                    crate::combat_computer::COLOUR,
                );
            }
        }
        self.combat_readouts = readouts;
    }

    fn draw_body_labels(&mut self, camera: &Camera) {
        for (name, x, y) in self.label_positions(camera) {
            self.text_screen(x as usize, y, &name, 15);
        }
    }

    /// The Combat Targeting Computer's list, drawn as the navigation computer's
    /// places list is (`draw_places`): marker bitmap 0x31 and a name fifteen
    /// pixels to its right, nine rows in each of two columns, over the flight
    /// view with no panel behind it. The name is in the ship's status colour
    /// (red hostile, blue police, green otherwise); the ship the player picked
    /// has a red square for its marker.
    fn draw_npc_list(&mut self) {
        use crate::combat_computer::{list_cell, row_label, LIST_ROWS};
        let rows = self.combat_labels.clone();
        if rows.is_empty() {
            self.text(10, 5, "No ships in range.", 0x11);
        }
        for (index, label) in rows.iter().take(LIST_ROWS).enumerate() {
            let Some((x, y)) = list_cell(index) else {
                break;
            };
            if self.npc_list_selected == Some(label.ship) {
                self.rect(x as usize, y as usize, 10, 10, targeter::COMBAT_COLOUR);
            } else if let Some(marker) = &self.marker {
                marker.blit_colours(&mut self.frame, &self.palette, x, y);
            }
            let text = row_label(label, index).to_ascii_uppercase();
            self.text((x + 15) as usize, y as usize, &text, label.colour.palette());
        }
    }

    /// `enhancement-10`: the port-only NPC targeting button, drawn from the
    /// embedded PNG over the last rows of the flight view.
    fn draw_npc_button(&mut self) {
        // Pressed while the list is open.
        let icon = if self.npc_list_open {
            self.npc_icon_pressed
                .clone()
                .or_else(|| self.npc_icon.clone())
        } else {
            self.npc_icon.clone()
        };
        let Some(icon) = icon else {
            return;
        };
        let (origin_x, origin_y, _, _) = crate::combat_computer::NPC_BUTTON;
        for y in 0..icon.height {
            for x in 0..icon.width {
                let [red, green, blue, alpha] = icon.pixels[(y * icon.width + x) as usize];
                if alpha == 0 {
                    continue;
                }
                let (px, py) = (origin_x + x as i32, origin_y + y as i32);
                if !(0..320).contains(&px) || !(0..200).contains(&py) {
                    continue;
                }
                let index = py as usize * 320 + px as usize;
                let dst = self.frame[index];
                let blend = |source: u8, channel: u32| {
                    (u32::from(source) * u32::from(alpha)
                        + ((dst >> channel) & 0xff) * (255 - u32::from(alpha)))
                        / 255
                };
                self.frame[index] = blend(red, 16) << 16 | blend(green, 8) << 8 | blend(blue, 0);
            }
        }
    }

    /// `enhancement-11`: the off-view target arrow, a solid arrow with a dark
    /// outline, smoothed by sampling each pixel 4x4 times so its slanted edges
    /// do not stair-step.
    fn draw_edge_indicator(&mut self) {
        let Some(indicator) = self.edge_indicator else {
            return;
        };
        let fill = self.palette[usize::from(indicator.colour)];
        let outline = self.palette[0];
        let points = indicator.points;
        // Whether (x, y) is inside the arrow (even-odd).
        let inside = |x: f64, y: f64| {
            let mut crossings = false;
            for index in 0..points.len() {
                let (a, b) = (points[index], points[(index + 1) % points.len()]);
                if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
                    crossings = !crossings;
                }
            }
            crossings
        };
        // The distance from (x, y) to the arrow's outline.
        let distance = |x: f64, y: f64| {
            (0..points.len())
                .map(|index| {
                    let (a, b) = (points[index], points[(index + 1) % points.len()]);
                    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                    let along =
                        (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                    ((x - a.0 - dx * along).powi(2) + (y - a.1 - dy * along).powi(2)).sqrt()
                })
                .fold(f64::MAX, f64::min)
        };
        let (min_x, max_x, min_y, max_y) = points.iter().fold(
            (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
            |(a, b, c, d), (x, y)| (a.min(*x), b.max(*x), c.min(*y), d.max(*y)),
        );
        let (x0, x1) = (
            (min_x.floor() as i32 - 2).max(0),
            (max_x.ceil() as i32 + 2).min(319),
        );
        let (y0, y1) = (
            (min_y.floor() as i32 - 2).max(0),
            (max_y.ceil() as i32 + 2).min(VIEW_HEIGHT as i32 - 1),
        );
        const SAMPLES: i32 = 4;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (mut solid, mut edge) = (0.0_f64, 0.0_f64);
                for sy in 0..SAMPLES {
                    for sx in 0..SAMPLES {
                        let px = f64::from(x) + (f64::from(sx) + 0.5) / f64::from(SAMPLES);
                        let py = f64::from(y) + (f64::from(sy) + 0.5) / f64::from(SAMPLES);
                        let near = distance(px, py);
                        // The outline is a ring about a pixel wide round the arrow's edge.
                        if near < 0.9 {
                            edge += 1.0;
                        } else if inside(px, py) {
                            solid += 1.0;
                        }
                    }
                }
                let total = f64::from(SAMPLES * SAMPLES);
                let (solid, edge) = (solid / total, edge / total);
                if solid + edge <= 0.0 {
                    continue;
                }
                let at = y as usize * 320 + x as usize;
                let under = self.frame[at];
                let mix = |channel: u32| {
                    let base = f64::from((under >> channel) & 0xff);
                    let ink = f64::from((fill >> channel) & 0xff);
                    let dark = f64::from((outline >> channel) & 0xff);
                    // The ring is 80% opaque so it reads on any background; the
                    // inside half of it is the arrow's own colour over dark.
                    let ring = edge * 0.8;
                    let after_ring = base * (1.0 - ring) + dark * ring;
                    (after_ring * (1.0 - solid) + ink * solid).round() as u32
                };
                self.frame[at] = mix(16) << 16 | mix(8) << 8 | mix(0);
            }
        }
    }

    /// `text`, but only the glyph rows inside `top..bottom` (F1586's clip). Used
    /// by the message window, whose text is clipped to the console (ymin 158).
    fn text_clipped(&mut self, x: usize, y: i32, message: &str, colour: u8, top: i32, bottom: i32) {
        let bytes: Vec<u8> = message
            .chars()
            .map(|character| {
                if character.is_ascii() && !character.is_ascii_control() {
                    character as u8
                } else {
                    b'?'
                }
            })
            .collect();
        self.text_bytes_rows(x, y, &bytes, colour, top, bottom);
    }

    /// `F273`'s Page Up/Down (actions 0xf/0xe): `older` slides the log up
    /// toward older lines, otherwise down toward the newest.
    pub fn scroll_messages(&mut self, older: bool) -> bool {
        if older {
            let lines = self.messages.lines(&self.font, &self.clock).len();
            self.messages.scroll_older(lines)
        } else {
            self.messages.scroll_newer()
        }
    }

    /// One `F273` frame of the message log with `tics` of `DATA_008799`.
    pub fn tick_messages(&mut self, tics: u32) {
        self.messages.tick(tics, &self.font);
    }

    /// `F272`'s message window at x 196, y 159: the date/time header, the newest
    /// entry and the older dated entries, 8-pixel lines, the text one pixel left
    /// and up of its colour-0 shadow, clipped to the console (y 158..199). `F273`'s scroll
    /// moves the whole block up from the header (`ffe.asm:21075-21097`).
    /// F312's profile page: the title at (30, 5), the two-column status block at
    /// (2, 20) with F1399's `0xfd` tab stops (114 and 150), the current
    /// jurisdiction's record at (4, 75), and medals/permits at (4, 132). The
    /// positions and strings are from `DATA_005136`/`DATA_005189` and the
    /// executed original's frame `docs/evidence/status/original-profile.png`.
    /// F283's ship equipment status page over F956's cabin backdrop, every
    /// string with F1399's drop shadow: F318's title (`0x9c04` and the ship's
    /// name) at (8,5) in `DATA_008673`'s green; F282's `0x9c12` block at (5,15)
    /// in 10-pixel lines with the `0xfd` tab stops at 68 and 130; the fitted
    /// general equipment in two columns at x 5 and 96 from y 47 (9-pixel lines,
    /// colour 0x11, 0x10 when damaged); the gun mounts from (5,125) in 10-pixel
    /// lines (`0x9c0c`, `0x9c0d`'s turret pair); F310's `0x9c3b` cargo, cabins
    /// and cash at (190,138); and F288's spinning ship model in F957's viewport
    /// x 189..317, y 2..130. The positions and colours are the executed
    /// original's F2 frame.
    fn draw_equipment_status(&mut self, page: &EquipmentStatus) {
        const TEXT: u8 = 0x0f;
        self.shadow_text(8, 5, &page.title, STATUS_TITLE);
        for (row, line) in page.status.iter().enumerate() {
            let y = 15 + row as i32 * 10;
            for (column, part) in line.split('\t').enumerate() {
                let x = [5, 68, 130][column.min(2)];
                self.shadow_text(x, y, part, TEXT);
            }
        }
        // F283: the scroll arrows (bitmap 0x30 while more rows follow, 0x2f
        // once scrolled), then each column's visible rows from y 47.
        let (shown, per_column, last) = equipment_rows(page.equipment.len());
        let scroll = page.scroll.min(last);
        let palette = self.palette;
        if last > scroll {
            if let Some(Some(icon)) = self.dock_icons.get(2) {
                icon.blit_rgb(&mut self.frame, &palette, 0x14, 0x70);
            }
        }
        if scroll > 0 {
            if let Some(Some(icon)) = self.dock_icons.get(1) {
                icon.blit_rgb(&mut self.frame, &palette, 0x20, 0x70);
            }
        }
        for (index, (name, damaged)) in page.equipment.iter().enumerate() {
            let (x, row) = if index < per_column {
                (5, index)
            } else {
                (0x60, index - per_column)
            };
            if row < scroll || row >= scroll + shown {
                continue;
            }
            let y = 47 + (row - scroll) as i32 * 9;
            self.shadow_text(x, y, name, if *damaged { 0x10 } else { 0x11 });
        }
        for (row, (mount, laser, tab)) in page.lasers.iter().enumerate() {
            let y = 125 + row as i32 * 10;
            self.shadow_text(5, y, &format!("{mount}:"), TEXT);
            self.shadow_text(*tab as i32, y, laser, TEXT);
        }
    }

    /// F956's cabin, F288's turning ship and F310's block for any status page.
    fn draw_status_backdrop(&mut self, backdrop: &StatusBackdrop, scene: Option<&ModelViewer>) {
        self.frame[..320 * VIEW_HEIGHT]
            .copy_from_slice(&self.cabins[self.cabin_index][..320 * VIEW_HEIGHT]);
        if let Some(scene) = scene {
            self.draw_spinning_model(scene, backdrop);
        }
        if backdrop.cargo_block {
            self.draw_status_hold(&backdrop.hold);
        }
    }

    /// F295's roster: F318's `0x9c06` title at (30,5) and its headings at
    /// (6,25) with the `0xfd` tab stops 72, 100 and 162 ("Client", "Group",
    /// "Destination", "To pay"), then up to thirteen contracts from (2,35) in
    /// 10-pixel lines: the client, the group at 80 (a passenger's count) or
    /// 72 ("Parcel"; "-" at 84 for a military package), the destination at 100
    /// and the payment at 162.
    fn draw_roster(&mut self, rows: &[RosterRow]) {
        const TEXT: u8 = 0x0f;
        self.shadow_text(30, 5, "PASSENGER AND SMALL ITEMS ROSTER", STATUS_TITLE);
        for (x, heading) in [
            (6, "Client"),
            (72, "Group"),
            (100, "Destination"),
            (162, "To pay"),
        ] {
            self.shadow_text(x, 25, heading, STATUS_TITLE);
        }
        for (index, row) in rows.iter().take(13).enumerate() {
            let y = 35 + index as i32 * 10;
            self.shadow_text(2, y, &row.client, TEXT);
            let group_x = match row.group.as_str() {
                "Parcel" => 72,
                "-" => 84,
                _ => 80,
            };
            let cursor = 2 + self.font.width(&row.client).unwrap_or(0) as i32;
            self.shadow_text(cursor.max(group_x), y, &row.group, TEXT);
            self.shadow_text(100, y, &row.destination, TEXT);
            let pay = format!("${}", row.reward);
            self.shadow_text(162, y, &pay, TEXT);
        }
    }

    /// F317's mining equipment list: F318's `0x9c0a` title, then up to twelve
    /// machines from (24,35) in 10-pixel lines, each behind the marker bitmap
    /// 0x31 at (x-18,y-1): the find, the system at the 0x28 tab, the planet
    /// and the hopper at the 0x46 tab.
    fn draw_mining_list(&mut self, rows: &[MiningRow]) {
        const TEXT: u8 = 0x0f;
        self.shadow_text(30, 5, "MINING EQUIPMENT LIST", STATUS_TITLE);
        for (index, row) in rows.iter().take(12).enumerate() {
            let y = 0x23 + index as i32 * 10;
            if let Some(marker) = &self.marker {
                marker.blit_colours(&mut self.frame, &self.palette, 0x18 - 0x12, y - 1);
            }
            self.shadow_text(0x18, y, &row.status, TEXT);
            self.shadow_text(0x18 + 0x50, y, &row.system, TEXT);
            self.shadow_text(0x18 + 0x50 + 0x46, y, &row.planet, TEXT);
            self.shadow_text(0x18 + 0x50 + 0x46 + 0x5a, y, &row.hopper, TEXT);
        }
    }

    /// F316's outstanding contracts: F318's `0x9c09` title tabbed to x 120,
    /// its headings at (2,15) with the tab stops 124, 202, 232 and 292
    /// ("Contract", "Location", "Time", "Other Info", "To Pay"), then each
    /// contract from y 27 in 24-pixel entries: the job wrapped to 120 pixels
    /// at x 2 (`0x9c1e`), the system and station at 124 (`0x9c21`), the
    /// deadline at 202 and the payment at 292 (`0x9c22`/`0x9c28`).
    fn draw_contracts(&mut self, rows: &[ContractRow]) {
        const TEXT: u8 = 0x0f;
        self.shadow_text(120, 5, "OUTSTANDING CONTRACTS", STATUS_TITLE);
        for (x, heading) in [
            (2, "Contract"),
            (124, "Location"),
            (202, "Time"),
            (232, "Other Info"),
            (292, "To Pay"),
        ] {
            self.shadow_text(x, 15, heading, STATUS_TITLE);
        }
        for (index, row) in rows.iter().take(5).enumerate() {
            let y = 27 + index as i32 * 24;
            let mut line = String::new();
            let mut line_y = y;
            let wrap = if row.wide { 0x13b } else { 120 };
            for word in row.contract.split(' ') {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if self.font.width(&candidate).unwrap_or(0) > wrap && !line.is_empty() {
                    self.shadow_text(2, line_y, &line, TEXT);
                    line_y += 10;
                    line = word.to_owned();
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                self.shadow_text(2, line_y, &line, TEXT);
            }
            if row.wide {
                continue;
            }
            self.shadow_text(124, y, &format!("{},", row.system), TEXT);
            self.shadow_text(124, y + 10, &row.station, TEXT);
            self.shadow_text(202, y, &row.time, TEXT);
            self.shadow_text(292, y, &format!("${}", row.reward), TEXT);
        }
    }

    /// F294's cargo inventory: F318's `0x9c05` title at (30,5) with its
    /// "Refuel" heading at (140,15) beside F1621's button (bitmap 0x31) at
    /// (120,15), the two "Jettison" headings at (2,25) and (88,25), and each
    /// commodity carried (`DATA_008896`, up to 24) with its jettison button:
    /// twelve to a column at x 18 and 106 from y 35 in 10-pixel lines, the
    /// tonnage at the `0x9c13`/`0x9c14` tab stops 74 and 162.
    fn draw_cargo_inventory(&mut self, items: &[(String, u16)]) {
        const TEXT: u8 = 0x0f;
        self.shadow_text(30, 5, "CARGO INVENTORY", STATUS_TITLE);
        self.shadow_text(140, 15, "Refuel", STATUS_TITLE);
        self.shadow_text(2, 25, "Jettison", STATUS_TITLE);
        self.shadow_text(88, 25, "Jettison", STATUS_TITLE);
        if let Some(marker) = &self.marker {
            marker.blit_colours(&mut self.frame, &self.palette, 120, 15);
        }
        for (index, (name, tonnes)) in items.iter().take(24).enumerate() {
            let (x, tab) = if index < 12 { (18, 74) } else { (106, 162) };
            let y = 35 + (index % 12) as i32 * 10;
            if let Some(marker) = &self.marker {
                marker.blit_colours(&mut self.frame, &self.palette, x - 12, y - 1);
            }
            self.shadow_text(x, y, name, TEXT);
            let cursor = x as usize + self.font.width(name).unwrap_or(0);
            self.shadow_text(cursor.max(tab) as i32, y, &format!("{tonnes}t"), TEXT);
        }
    }

    /// F284 + F288 + F293: the player's ship turning in the status viewport.
    /// The view position is from the statistics' `+0xc` word `w`:
    /// `(w·90000 >> 8, w·11000 >> 8, w·1000)`; F293 turns F1675's two angles
    /// by `DATA_009122 >> 3` and `>> 4` a frame (994 per 20 ms), so one full
    /// turn of the first takes about 10.5 s.
    fn draw_spinning_model(&mut self, scene: &ModelViewer, backdrop: &StatusBackdrop) {
        let model = backdrop.ship;
        let Some(stats) = scene.ship_stats(model) else {
            return;
        };
        let w = f64::from(stats.shield_strength);
        let origin = [
            (w * 90_000.0 / 256.0).floor(),
            (w * 11_000.0 / 256.0).floor(),
            w * 1000.0,
        ];
        // `spin` is 65536 per second; F293 adds 994·50/8 and 994·50/16 a second.
        let seconds = f64::from(backdrop.spin) / 65_536.0;
        let a = (seconds * 994.0 * 50.0 / 8.0) as i64 as i32;
        let b = (seconds * 994.0 * 50.0 / 16.0) as i64 as i32;
        let matrix = scene.matrix_yzt(a & 0xffff, b & 0xffff);
        let rows = std::array::from_fn(|k| {
            std::array::from_fn(|j| f64::from(matrix[3 * k + j]) / 2_147_483_648.0)
        });
        let inputs = AnimationInputs::original(backdrop.tics, backdrop.gear, &scene.shades());
        if let Ok(frame) =
            scene.render_scene_with(model, rows, origin, scene.light_direction(), &inputs)
        {
            self.draw_shapes(scene, &frame.shapes);
        }
    }

    /// F439/F559/F562: the hull turning in the window at the left (placed by the
    /// statistics' `+0xc` word `w` at `(-93000 w / 256, -1000 w / 256, 1000 w)`,
    /// the status page's spin), F561's mark at (4,131)-(4,134) (its scale bar's
    /// length word is never set, so it has none), and the text blocks, tabs and
    /// colours as F1397 leaves them for F1399.
    fn draw_ship_view(&mut self, view: &ShipViewFrame, scene: Option<&ModelViewer>) {
        if let Some(scene) = scene {
            let w = i64::from(view.page.shield);
            // `imul`, then `+0xff` when negative and `sar 8`: a division by 256
            // truncating towards zero.
            let div = |value: i64| (value / 256) as f64;
            let origin = [div(w * -93_000), div(-(w * 1000)), (w * 1000) as f64];
            let seconds = f64::from(view.spin) / 65_536.0;
            let a = (seconds * 994.0 * 50.0 / 8.0) as i64 as i32;
            let b = (seconds * 994.0 * 50.0 / 16.0) as i64 as i32;
            let matrix = scene.matrix_yzt(a & 0xffff, b & 0xffff);
            let rows = std::array::from_fn(|k| {
                std::array::from_fn(|j| f64::from(matrix[3 * k + j]) / 2_147_483_648.0)
            });
            let inputs = AnimationInputs::original(view.tics, 0, &scene.shades());
            if let Ok(frame) = scene.render_scene_with(
                view.page.model,
                rows,
                origin,
                scene.light_direction(),
                &inputs,
            ) {
                self.draw_shapes(scene, &frame.shapes);
            }
        }
        self.line((4, 131), (4, 134), 15);
        for text in &view.page.texts {
            self.draw_view_text(text.x, text.y, &text.bytes);
        }
    }

    /// F1399's pen for an expanded string: `0x0d` starts a 10-pixel line at the
    /// line origin, `0xfd XX` moves the pen to `2 * XX`, `0xfe XX YY` to
    /// (`2 * XX`, `YY`) and makes that x the line origin, `0x01 XX` selects
    /// `DATA_007776[XX]`'s colour. Text is in the BBS colour (`DATA_005286`, 0x7d).
    fn draw_view_text(&mut self, x: i32, y: i32, bytes: &[u8]) {
        // DATA_007776's first entries: white, red, red, 0x5f, 0x11, white, 0x12, white, 0x43.
        const COLOURS: [u8; 9] = [0x0f, 0x10, 0x10, 0x5f, 0x11, 0x0f, 0x12, 0x0f, 0x43];
        let (mut pen_x, mut pen_y, mut origin) = (x, y, x);
        let mut colour = 0x7d;
        let mut run = String::new();
        let mut index = 0;
        let flush =
            |screen: &mut Self, run: &mut String, pen_x: &mut i32, pen_y: i32, colour: u8| {
                if !run.is_empty() {
                    screen.shadow_text(*pen_x, pen_y, run, colour);
                    *pen_x += screen.font.width(run).unwrap_or(0) as i32;
                    run.clear();
                }
            };
        while index < bytes.len() {
            let byte = bytes[index];
            match byte {
                0x0d => {
                    flush(self, &mut run, &mut pen_x, pen_y, colour);
                    pen_x = origin;
                    pen_y += 10;
                    index += 1;
                }
                0xfd | 0x1e => {
                    flush(self, &mut run, &mut pen_x, pen_y, colour);
                    pen_x = i32::from(bytes.get(index + 1).copied().unwrap_or(0)) * 2;
                    index += 2;
                }
                0xfe | 0x1f => {
                    flush(self, &mut run, &mut pen_x, pen_y, colour);
                    pen_x = i32::from(bytes.get(index + 1).copied().unwrap_or(0)) * 2;
                    pen_y = i32::from(bytes.get(index + 2).copied().unwrap_or(0));
                    origin = pen_x;
                    index += 3;
                }
                0x01 => {
                    flush(self, &mut run, &mut pen_x, pen_y, colour);
                    colour = bytes
                        .get(index + 1)
                        .and_then(|selector| COLOURS.get(usize::from(*selector)))
                        .copied()
                        .unwrap_or(colour);
                    index += 2;
                }
                byte if byte >= 0x20 => {
                    run.push(char::from(byte));
                    index += 1;
                }
                _ => index += 1,
            }
        }
        flush(self, &mut run, &mut pen_x, pen_y, colour);
    }

    fn draw_profile(&mut self, profile: &ffe_core::status::Profile) {
        const TEXT: u8 = 0x0f;
        // F1399's 0xfd tab: the pen moves on to the stop, never back.
        let tabbed = |screen: &mut Self, x: i32, y: i32, label: &str, tab: i32, value: &str| {
            screen.shadow_text(x, y, label, TEXT);
            let cursor = x + screen.font.width(label).unwrap_or(0) as i32;
            screen.shadow_text(cursor.max(tab), y, value, TEXT);
        };
        self.shadow_text(
            30,
            5,
            &format!("PROFILE FOR COMMANDER {}", profile.commander),
            STATUS_TITLE,
        );
        self.shadow_text(2, 20, "MILITARY STANDING", TEXT);
        self.shadow_text(114, 20, "LEGAL STATUS", TEXT);
        tabbed(self, 2, 30, "Federal Rank: ", 52, profile.federal_rank);
        tabbed(self, 114, 30, "Federal:", 150, profile.federal_status);
        tabbed(self, 2, 40, "Imperial Title: ", 52, profile.imperial_title);
        tabbed(self, 114, 40, "Imperial:", 150, profile.imperial_status);
        tabbed(self, 2, 50, "Elite Rating:", 52, profile.elite_rating);
        tabbed(self, 114, 50, "Interpol:", 150, profile.interpol_status);
        self.shadow_text(
            2,
            65,
            &format!("{} CRIMINAL RECORD", profile.heading()),
            TEXT,
        );
        for (index, offence) in profile.record.iter().take(10).enumerate() {
            let (x, row) = if index < 5 {
                (4, index)
            } else {
                (108, index - 5)
            };
            self.shadow_text(x, 75 + row as i32 * 9, offence, TEXT);
        }
        self.shadow_text(2, 123, "MEDALS AND PERMITS", TEXT);
        if self.mission_fixes {
            // enhancement-7 / F312: every set bit, four rows a column.
            for (index, medal) in profile.medals.iter().enumerate() {
                let (x, y) = ffe_core::status::medal_position(index);
                self.shadow_text(x, y, medal, TEXT);
            }
        } else {
            for (index, medal) in profile.medals.iter().take(3).enumerate() {
                self.shadow_text(4, 132 + index as i32 * 9, medal, TEXT);
            }
        }
    }

    fn draw_messages(&mut self) {
        if !self.messages_visible {
            return;
        }
        let lines = self.messages.lines(&self.font, &self.clock);
        if lines.is_empty() {
            return;
        }
        let scroll = self.messages.scroll();
        for (index, line) in lines.iter().enumerate() {
            let y = crate::messages::TEXT_Y as i32 - scroll
                + (index * crate::messages::LINE_HEIGHT) as i32;
            if y + crate::messages::LINE_HEIGHT as i32 <= crate::messages::CLIP_TOP as i32 {
                continue;
            }
            if y >= crate::messages::CLIP_BOTTOM as i32 {
                break;
            }
            let colour = if line.header { 17 } else { HUD_COLOUR };
            // F272's first pass: colour 0 at (0xc4, y), the shadow.
            self.text_clipped(
                crate::messages::TEXT_X,
                y,
                &line.text,
                0,
                crate::messages::CLIP_TOP as i32,
                crate::messages::CLIP_BOTTOM as i32,
            );
            // The second: the text's own colours at (0xc3, y - 1). The original's warning
            // strings carry `0x01 0x02 WARNING: 0x01 0x03 text`: selector 2 is DATA_007778's
            // 0x10 (red) and selector 3 DATA_007779's 0x5f in F272's second pass.
            let prefix = if line.header {
                None
            } else {
                ["WARNING: ", "ATTENTION: "]
                    .into_iter()
                    .find(|prefix| line.text.starts_with(prefix))
            };
            if let Some(prefix) = prefix {
                let width = self.font.width(prefix).unwrap_or(0);
                self.text_clipped(
                    crate::messages::SHADOW_X,
                    y - 1,
                    prefix,
                    0x10,
                    crate::messages::CLIP_TOP as i32,
                    crate::messages::CLIP_BOTTOM as i32,
                );
                self.text_clipped(
                    crate::messages::SHADOW_X + width,
                    y - 1,
                    &line.text[prefix.len()..],
                    0x5f,
                    crate::messages::CLIP_TOP as i32,
                    crate::messages::CLIP_BOTTOM as i32,
                );
            } else {
                self.text_clipped(
                    crate::messages::SHADOW_X,
                    y - 1,
                    &line.text,
                    colour,
                    crate::messages::CLIP_TOP as i32,
                    crate::messages::CLIP_BOTTOM as i32,
                );
            }
        }
    }

    /// The player's `F1099` text at its original spots. Shields and hull are only
    /// drawn under the original's conditions; the Autopilot line only while the
    /// autopilot flies; the Set/Act/Alt block ending on row 149 and the turret
    /// angles at (5,76) in turret views.
    fn draw_player_hud(&mut self, view: ViewMode) {
        // F1099 jumps to JUMP_004541 in the missile view: only the combat
        // target's range text is drawn there.
        if view == ViewMode::Missile {
            return;
        }
        let hud = self.hud.clone();
        if let Some(text) = hud.shield_text() {
            self.hud_text(SHIELDS_POS, &text, HUD_COLOUR);
        }
        if let Some(text) = hud.hull_text() {
            self.hud_text(HULL_POS, &text, HUD_COLOUR);
        }
        if hud.autopilot {
            self.hud_text(AUTOPILOT_POS, "Autopilot", HUD_COLOUR);
        }
        // F1099's `0x8620`..`0x8623`: the strings' own position codes put
        // both columns' last line on row 149, ten rows a line.
        for (x, y, line) in hud.speed_layout(&self.font) {
            self.hud_text((x, y), &line, HUD_COLOUR);
        }
        if view.is_turret() {
            if let Some(lines) = hud.turret_lines() {
                for (index, line) in lines.iter().enumerate() {
                    self.hud_text(
                        (TURRET_POS.0, TURRET_POS.1 + index * crate::hud::LINE_HEIGHT),
                        line,
                        TURRET_COLOUR,
                    );
                }
            }
        }
    }

    /// F1399's text: F1589 draws every glyph through F1588, colour 0 one pixel
    /// right and down, then the glyph in its colour.
    fn hud_text(&mut self, (x, y): (usize, usize), message: &str, colour: u8) {
        self.shadow_text(x as i32, y as i32, message, colour);
    }

    /// F961 + F962 at one `GameTics`: clear to F961's F1575 background, draw
    /// model 177/176 through F1682/F1683, blit primary bitmap 0x42 at y 158,
    /// then the applicable credits (and the version line once V latches it).
    /// The background and the model's unfixed colours resolve through the
    /// runtime upper palette (`F1326`/`F1327`/`F1330`).
    pub fn draw_intro(
        &mut self,
        scene: Option<&ModelViewer>,
        tics: u32,
        version: bool,
    ) -> Result<(), String> {
        use crate::front_end::{IntroTimeline, PRIMARY_BITMAP_Y, VERSION_LINE};
        let intro = IntroTimeline::at(tics);
        let background = pack(intro.background) & 0xfff;
        if let Some(viewer) = scene {
            viewer.set_background(background);
            // F959 gives the intro object (`DATA_009127`) the surface seed
            // `+0xa0 = 0x789a3456` (ffe.asm:71595). F1876 generates the patch
            // grid from it, so the planet's terrain is that object's, not the
            // zero-filled default.
            viewer
                .surface_seed
                .set(crate::front_end::INTRO_SURFACE_SEED);
            // `DATA_008812` is 1 by the time the intro loop runs (the executed
            // boot's default Shape Detail); the scene was wrongly built at 2,
            // whose finer subdivision aliased the band textures into speckle.
            viewer.set_surface_detail(self.surface_detail);
        }
        let model = match scene {
            Some(viewer) if intro.model != 0 => {
                // F959's DATA_007235/DATA_007236: object origin (0, 0, 0x14820)
                // and the 0x7f000000 row scale (127/128), with the F961 animation
                // word as global 0, not a raw tic count.
                let rows = [
                    [127.0 / 128.0, 0.0, 0.0],
                    [0.0, 127.0 / 128.0, 0.0],
                    [0.0, 0.0, 127.0 / 128.0],
                ];
                let inputs = AnimationInputs::original(
                    u32::from(intro.animation) << 10,
                    0,
                    &viewer.shades(),
                );
                let origin = [0.0, 0.0, 0x14820 as f64];
                let frame = viewer.render_scene_with(
                    intro.model,
                    rows,
                    origin,
                    viewer.light_direction(),
                    &inputs,
                )?;
                if let Some(error) = frame.children_error.as_deref() {
                    // Model 176's late animation calls a child frame that copies
                    // a basis F1750 leaves as stale stack (animation.md §1.2).
                    // The original would draw whatever that stack held; the port
                    // cannot reproduce it, so report once and keep the credits.
                    if !self.intro_error_reported {
                        eprintln!("Intro model {}: {error}", intro.model);
                        self.intro_error_reported = true;
                    }
                }
                Some((frame, rows, origin))
            }
            Some(viewer) => {
                // The loop point draws no model but F961 still clears to the
                // last F1575 colour; build it with the accumulated cube.
                viewer.prepare_palette();
                None
            }
            None => None,
        };
        // F961's clear (F1618) uses the palette entry F1575 resolved.
        let clear = scene
            .and_then(|viewer| viewer.palette_rgb(background))
            .map(|[r, g, b]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
            .unwrap_or(self.palette[0]);
        self.frame[..LOGICAL_WIDTH * LOGICAL_HEIGHT].fill(clear);
        if let (Some(viewer), Some((frame, _, _))) = (scene, model) {
            self.draw_shapes(viewer, &frame.shapes);
        }
        if intro.bitmap {
            let bitmap = self
                .intro_bitmap
                .as_ref()
                .ok_or("Cache lacks base:bitmap/primary/066 (F962's bitmap 0x42).")?;
            let (width, _) = bitmap.size();
            let x = 160 - width as i32 / 2;
            let palette = self.palette;
            bitmap.blit_rgb(&mut self.frame, &palette, x, PRIMARY_BITMAP_Y);
        }
        let text = self
            .intro_text
            .as_ref()
            .ok_or("Cache lacks the intro credits/widths (cache v27).")?;
        for line in &intro.lines {
            text.draw(line, &self.font, &self.palette, &mut self.frame)?;
        }
        if version && tics >= crate::front_end::INTRO_INPUT_FROM {
            text.draw(&VERSION_LINE, &self.font, &self.palette, &mut self.frame)?;
        }
        self.present();
        Ok(())
    }

    pub fn draw(
        &mut self,
        state: &Flight,
        views: ViewState,
        scene: Option<&ModelViewer>,
        _modern: bool,
        // The port's notices are not drawn: the original's console shows only
        // F272's log and F1099's texts. Prompts and game events reach the log.
        _message: &str,
    ) {
        self.indexed.fill(0);
        self.panel.blit(&mut self.indexed, 0, 158);
        let dying = state
            .death
            .is_some_and(|death| death.phase == crate::death::DeathPhase::Exploding);
        let mut views = views;
        views.lock_external = self.lock_external;
        let view = views.mode;
        // The original registers the effective background (space colour, or the
        // previous frame's haze) with F1326 before building the palette.
        let background = self.surface_sky.unwrap_or(self.sky);
        if let Some(scene) = scene {
            scene.set_background(
                pack([
                    (((background >> 16) & 0xff) as u8) >> 4,
                    (((background >> 8) & 0xff) as u8) >> 4,
                    ((background & 0xff) as u8) >> 4,
                ]) & 0xfff,
            );
            scene.set_surface_detail(self.surface_detail);
        }
        let stats = scene.and_then(|scene| scene.ship_stats(state.ship));
        let turrets = stats.is_some_and(|stats| stats.gun_mounts > 2);
        // Docked, the status screen (F2) puts the flight console back, as the
        // executed original does.
        let flight_console =
            state.mode != Mode::Docked || self.pad_services || self.status_backdrop.is_some();
        if flight_console {
            // F272's order: primary 43 panel, the scanner globe (secondary
            // 254..261), then F261's buttons, then F270's gauges. F274's
            // contacts come last, after the view (see `draw_scanner_contacts`).
            self.draw_scanner_overlay();
            // F261: the visible `DATA_005100` records in index order. The page
            // (screen buttons, meter, view row, lamps or the F1120 equipment
            // page) is derived first; nothing outside the table is drawn.
            let mut systems = self.systems;
            if turrets {
                systems.mounts = systems.mounts.max(3);
            }
            self.console.apply_flight(view, &views.console, &systems);
            if self.galaxy.is_some() || self.enhanced_galaxy.is_some() || self.system_map.is_some()
            {
                // F834/F840: the map's page replaces the view row while the
                // screen buttons, meter, scanner and lamps stay visible.
                self.console.apply_map(&self.map_console);
            }
        } else {
            self.console.apply_station(&views.console);
        }
        // Common screen controls are present on station pages as well as flight.
        for (_, button) in self.console.drawable() {
            let Some(id) = button.image() else {
                continue;
            };
            if let Some(Some(icon)) = self.icons.get(usize::from(id)) {
                icon.blit(&mut self.indexed, button.x, button.y);
            }
        }
        if state.mode != Mode::Docked || self.status_backdrop.is_some() {
            self.draw_gauges();
        }
        // The lift stages leave the view as the last frame left it (below), so
        // only the console rows come from the indexed layer then.
        let lift_alone = self.lift.as_ref().is_some_and(|lift| !lift.world);
        let first = if lift_alone { 320 * VIEW_HEIGHT } else { 0 };
        for (pixel, index) in self.frame[first..].iter_mut().zip(&self.indexed[first..]) {
            *pixel = self.palette[usize::from(*index)];
        }
        // The original clears the upper view to the palette entry nearest its
        // space colour (`FUNC_001618_ClearUpperBuf`); the port draws the captured
        // quantised colour (`prefs::space_colour_rgb`, `DATA_008815..817`) and
        // registers the same `(0,0,index)` request with the runtime palette, so
        // the models and FX around it are quantised against it.
        // The lift stages' views (F1069-F1087) draw their objects straight
        // through F1682 and never run F1052, so nothing clears the view: what
        // the lift does not cover keeps the previous frame (the closed door
        // shows through the interior's open end while the lift travels).
        if (state.mode != Mode::Docked || self.pad_services) && !lift_alone {
            // F1052 clears with the background colour the previous frame's scene
            // left in `DATA_008814..8817`; a planet's haze overrides the space
            // colour until the eye is out of it again.
            self.frame[..320 * VIEW_HEIGHT].fill(self.surface_sky.unwrap_or(self.sky));
        }
        if let Some(page) = self.system_map.take() {
            self.draw_system_map(&page, scene);
            self.present();
            return;
        }
        if let Some(page) = self.enhanced_galaxy.take() {
            self.draw_enhanced_galaxy(&page);
            self.present();
            return;
        }
        if let Some(map) = self.galaxy.take().filter(|_| scene.is_some()) {
            self.draw_galaxy(scene.unwrap(), &map);
            self.present();
            return;
        }
        if state.mode == Mode::Docked {
            self.frame[..320 * 158].copy_from_slice(&self.cabins[self.cabin_index]);
            if let Some(issue) = self.issue.take() {
                self.dock = None;
                self.draw_issue(&issue);
            } else if let Some(dock) = self.dock.take() {
                self.draw_dock(&dock);
                if let Some(view) = self.ship_view.take() {
                    self.draw_ship_view(&view, scene);
                }
            } else if self.market.is_some() {
                self.draw_market();
            }
        } else if self.pad_services {
            // F343 over the landed view (`0x30`): the station screen covers the
            // 3-D view, while the console/scanner page and the message clock
            // above it stay live. The ship's mode is still `Flight`, so closing
            // the board with F1 (F1003) reveals the same landed view and model.
            self.frame[..320 * 158].copy_from_slice(&self.cabins[self.cabin_index]);
            if let Some(issue) = self.issue.take() {
                self.dock = None;
                self.draw_issue(&issue);
            } else if let Some(dock) = self.dock.take() {
                self.draw_dock(&dock);
            } else if self.market.is_some() {
                self.draw_market();
            }
        } else if let (Some(word), Some(viewer)) = (self.hyperspace.take(), scene) {
            // F1066: the tunnel replaces the view while the ship is in mode 4.
            // F1575 requests (0, 0, 4) for the clear; model 0xa5 stands at F959's
            // object origin (0, 0, 0x14820) with F1093's 0x7f000000 rows and the
            // transit's progress as global 0.
            let colour = pack([0, 0, 4]) & 0xfff;
            viewer.set_background(colour);
            let clear = viewer
                .palette_rgb(colour)
                .map(|[r, g, b]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
                .unwrap_or(0);
            self.frame[..320 * VIEW_HEIGHT].fill(clear);
            let rows = [
                [127.0 / 128.0, 0.0, 0.0],
                [0.0, 127.0 / 128.0, 0.0],
                [0.0, 0.0, 127.0 / 128.0],
            ];
            let mut inputs = AnimationInputs::original(0, 0, &viewer.shades());
            inputs.globals[0] = word;
            if let Ok(frame) = viewer.render_scene_with(
                HYPERSPACE_MODEL,
                rows,
                [0.0, 0.0, 0x14820 as f64],
                viewer.light_direction(),
                &inputs,
            ) {
                self.draw_frame(viewer, &frame);
            }
        } else {
            let scale = scene.and_then(model_scale).unwrap_or(DEFAULT_MODEL_SCALE);
            let camera = Camera::new(state, &views, stats.as_ref(), scale);
            // A new flight step continues the stream where the last frame left it;
            // redrawing the same step replays it.
            if self.jitter.tick != Some(state.ticks) {
                self.jitter.tick = Some(state.ticks);
                self.jitter.start = self.jitter.cursor;
            }
            self.jitter.cursor = self.jitter.start;
            let world = self.lift.as_ref().is_none_or(|lift| lift.world);
            // The original registers every object's colours, builds the palette once
            // (F1330) and then draws the frame (docs/notes/palette.md section 6): the
            // objects are run twice, first only to register, then drawn.
            let eye = stats.map_or([0, 0], |s| s.eye);
            if let Some(scene) = scene.filter(|_| !self.palette_build_per_object) {
                let saved = (
                    self.frame.clone(),
                    self.jitter,
                    self.object_queue.clone(),
                    self.ship_beams.clone(),
                    self.surface_sky,
                );
                scene.begin_registration();
                self.draw_world_pass(state, view, Some(scene), &camera, world, eye);
                scene.finish_registration();
                (
                    self.frame,
                    self.jitter,
                    self.object_queue,
                    self.ship_beams,
                    self.surface_sky,
                ) = saved;
            }
            self.draw_world_pass(state, view, scene, &camera, world, eye);
            if let Some(scene) = scene.filter(|_| !self.palette_build_per_object) {
                scene.end_frame_palette();
            }
            // Text labels (`l`): the original's F989 walks the live objects and
            // F1108 draws each name at its screen position (colour 15).
            // F1099 draws the labels (F1108) only outside view 6, where F1109's
            // readouts take their place.
            if world && views.console.labels && view != ViewMode::Combat {
                self.draw_body_labels(&camera);
            }
            if world && view == ViewMode::Combat {
                self.draw_combat_readouts(&camera);
            }
            // F1104's death hold shows the external view alone: it clears the
            // whole screen once (F1577) and nothing redraws the console, the
            // HUD texts or the scanner (the executed probe's frames).
            if !dying {
                // F1100's range and F1099's target shields/hull, then the
                // player's own shields, hull, Autopilot and Set/Act/Alt.
                self.draw_target_hud();
                self.draw_player_hud(view);
                // F1100's targeting tunnels (F1113/F1114), then F1111's fixed
                // gunsight (bitmap 97) and its blinking targetter cursor, then
                // F1112's velocity marker (bitmap 98, front view only).
                self.draw_target_tunnels();
                self.draw_gunsight(view);
                if view == ViewMode::Front {
                    self.draw_velocity_marker(state, &camera);
                }
                // enhancement-11: the off-view target arrow.
                self.draw_edge_indicator();
                // F274/F275's scanner contacts own the console's scanner area
                // and may reach over its top row into the view.
                self.draw_scanner_contacts();
            }
            // enhancement-10: the port-only NPC targeting list and button.
            // Both only draw in the flight view; the list covers the HUD too,
            // and the button stays on top so it can close the list.
            if self.npc_list_open {
                self.draw_npc_list();
            }
            if self.npc_button {
                self.draw_npc_button();
            }
        }
        // The original HUD now owns the top rows and the lower left: the port's
        // top bar, controls button, SPEED line and status/map/autopilot block are
        // gone (they covered the shields/hull rows, the Autopilot and Set/Act
        // lines, and the lift's lower rows).
        if state.mode == Mode::Flight || self.status_backdrop.is_some() {
            if let Some(backdrop) = self.status_backdrop.clone() {
                self.draw_status_backdrop(&backdrop, scene);
            }
            if let Some(page) = self.equipment_status.clone() {
                self.draw_equipment_status(&page);
            }
            if let Some(profile) = self.profile.clone() {
                self.draw_profile(&profile);
            }
            if let Some(items) = self.cargo_inventory.clone() {
                self.draw_cargo_inventory(&items);
            }
            if let Some(rows) = self.roster.clone() {
                self.draw_roster(&rows);
            }
            if let Some(rows) = self.contracts.clone() {
                self.draw_contracts(&rows);
            }
            if let Some(rows) = self.mining_list.clone() {
                self.draw_mining_list(&rows);
            }
        }
        if state.mode == Mode::Docked {
            // The docked screens put the game's console message in the console's
            // message area at the lower right, not over the menu. F272's green
            // date/time header stays live under it (the source's station screen
            // shows the clock advancing).
            self.draw_messages();
        } else {
            // F272's text window: date/time header, ten entries, 124-pixel wrap,
            // shadow at (195, y-1) and text at (196, y), clipped to y 158..199.
            if self.profile.is_none() && !dying {
                // The pad services board keeps it, so the green clock stays visible
                // over the console (F272 runs in the station screens too).
                self.draw_messages();
            }
        }
        if dying {
            self.frame[320 * VIEW_HEIGHT..].fill(0);
        }
        self.present();
    }

    /// One pass over the flight view's objects: the background stars, the planets,
    /// stations, ships and effects, the lift and the beam. `Screen::draw` runs it
    /// twice per frame, registering colours and then drawing (see
    /// [`ModelViewer::begin_registration`]).
    fn draw_world_pass(
        &mut self,
        state: &Flight,
        view: ViewMode,
        scene: Option<&ModelViewer>,
        camera: &Camera,
        world: bool,
        eye: [i16; 2],
    ) {
        let camera = *camera;
        // F1052 gates the background on stars and the preceding frame's haze.
        // DOS F1775 (file 0x51cac) transposes the camera basis through F1665
        // (0x4ac60), then F1683 (0x4c164) copies (0, 0, 0x200) unchanged.
        if world && self.stars_visible && self.surface_sky.is_none() {
            if let Some(scene) = scene {
                let world_axes = self
                    .discs
                    .iter()
                    .find(|disc| disc.star)
                    .map_or(camera.axes, |star| star.orientation);
                let rows = world_axes.map(|axis| camera.direction(axis));
                let origin = camera
                    .direction(camera.axes[2])
                    .map(|component| component * BACKGROUND_DEPTH);
                let mut inputs = AnimationInputs::original(0, 0, &scene.shades());
                inputs.depth_detail = true;
                inputs.detail = self.shape_detail;
                if let Ok(frame) = scene.render_scene_with(
                    BACKGROUND_MODEL,
                    rows,
                    origin,
                    scene.light_direction(),
                    &inputs,
                ) {
                    self.draw_frame(scene, &frame);
                }
            }
        }
        if world {
            // F1052 sets `DATA_008814` to 0 before F1536 draws the objects;
            // the scene's planets set it again when their haze holds the eye.
            if let Some(scene) = scene {
                scene.clear_haze_sky();
            }
            // F1536 queues every object of the frame, the planets and
            // the star with the stations and ships, and F1789 draws the
            // one queue by depth key: the setting star at Hope draws over
            // the starport's buildings, as in the original's start frame.
            if state.system_flight && scene.is_some() {
                self.object_queue = Some(Vec::new());
            }
            self.draw_discs(&camera, scene);
            self.surface_sky = scene.and_then(|scene| scene.haze_sky()).map(|colour| {
                u32::from(colour[0]) << 16 | u32::from(colour[1]) << 8 | u32::from(colour[2])
            });
        } else {
            self.surface_sky = None;
        }
        // FUNC_001054 hides the player's own ship only in the front view.
        // Painter's order between the two objects: the farther one first.
        let own_ship = state.death.is_none() && view != ViewMode::Front;
        if state.system_flight && world {
            if let Some(scene) = scene {
                self.draw_system_stations(scene, state, &camera, own_ship);
            }
        }
        let ship_first = own_ship
            && world
            && !state.system_flight
            && distance(camera.position, state.position) > distance(camera.position, [0.0; 3]);
        if ship_first {
            if let Some(scene) = scene {
                self.draw_ship(scene, state, &camera);
            }
        }
        // In a system the prototype's station at the origin is not there.
        let drawn = !world
            || state.system_flight
            || scene.is_some_and(|scene| self.draw_station(scene, state, &camera));
        if !ship_first && own_ship && world && !state.system_flight {
            if let Some(scene) = scene {
                self.draw_ship(scene, state, &camera);
            }
        }
        // Placeholder wireframe when the cache has no drawable model 74.
        let vertices = [
            [-40.0, -40.0, -40.0],
            [40.0, -40.0, -40.0],
            [40.0, 40.0, -40.0],
            [-40.0, 40.0, -40.0],
            [-40.0, -40.0, 40.0],
            [40.0, -40.0, 40.0],
            [40.0, 40.0, 40.0],
            [-40.0, 40.0, 40.0],
        ];
        for (a, b) in [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            if drawn {
                break;
            }
            if let (Some(a), Some(b)) = (
                camera.project(vertices[a], true),
                camera.project(vertices[b], true),
            ) {
                self.line(a, b, 12);
            }
        }
        for (a, b) in [
            ([-14.0, -12.0, -41.0], [14.0, -12.0, -41.0]),
            ([14.0, -12.0, -41.0], [14.0, 12.0, -41.0]),
            ([14.0, 12.0, -41.0], [-14.0, 12.0, -41.0]),
            ([-14.0, 12.0, -41.0], [-14.0, -12.0, -41.0]),
        ] {
            if drawn {
                break;
            }
            if let (Some(a), Some(b)) = (camera.project(a, true), camera.project(b, true)) {
                self.line(a, b, if state.docking_requested { 17 } else { 18 });
            }
        }
        if let (Some(lift), Some(scene)) = (self.lift.take(), scene) {
            self.draw_lift(scene, &lift, &camera, eye);
            self.lift = Some(lift);
        }
        if let Some(beam) = self.beam {
            if beam_visible(view) {
                if self.fixed_laser && first_person_view(view) {
                    // enhancement-13: a fixed sight line from the foot of the
                    // view to the reticle, the Saker's look, instead of the beam
                    // from the gun mount.
                    self.line((160, 156), (160, 79), 0x5f);
                } else if let Some(scene) = scene {
                    self.draw_beam(scene, state, &beam, &camera);
                }
            }
        }
    }

    /// The Escape preference pages: `FUNC_000225_SysMenuDisplayNonFile`'s
    /// 320x158 upper view over the console. Background palette index 3 is the
    /// original's clear colour for (3,3,3) (`DATA_009222`), the icons are
    /// `0x2c`/`0x2d`, and every rectangle is `DATA_004903`..`DATA_004905`.
    /// A bullet marks the port's keyboard selection (the original is mouse-only).
    pub fn draw_prefs(&mut self, prefs: &Prefs) {
        // The original only clears the upper view; the console below it stays
        // visible (`FUNC_000225` calls `FUNC_001618_ClearUpperBuf`).
        self.indexed.fill(0);
        self.panel.blit(&mut self.indexed, 0, VIEW_HEIGHT as i32);
        for y in VIEW_HEIGHT..LOGICAL_HEIGHT {
            for x in 0..LOGICAL_WIDTH {
                let index = self.indexed[y * LOGICAL_WIDTH + x];
                self.frame[y * LOGICAL_WIDTH + x] = self.palette[usize::from(index)];
            }
        }
        self.rect(0, 0, 320, VIEW_HEIGHT, 3);
        let rows = prefs.rows();
        for row in &rows {
            let on = match row.mark {
                Mark::Action => row.applied,
                Mark::Radio(on) | Mark::Toggle(on) => on,
            };
            self.prefs_icon(row.x, row.y, on);
            let colour = if row.applied { row.colour } else { 8 };
            self.shadow_text(row.x + 16, row.y, row.label, colour);
        }
        for text in prefs.texts() {
            self.shadow_text(text.x, text.y, text.text, text.colour);
        }
        if prefs.keyboard {
            if let Some(row) = rows.get(prefs.selected) {
                self.prefs_bullet(row.x - 9, row.y + 2);
            }
        }
        self.present();
    }

    /// A pick-a-commander list (Load, Delete, confirmations) drawn like the
    /// Escape preference pages: the same palette-3 upper view over the console,
    /// the 0x2c/0x2d icons, shadowed labels, a heading and sub-heading in the
    /// pages' colours, and the keyboard bullet. `rows` are (label, selected).
    pub fn draw_pref_list(
        &mut self,
        heading: &str,
        subheading: &str,
        rows: &[(String, bool)],
        footer: &str,
    ) {
        self.indexed.fill(0);
        self.panel.blit(&mut self.indexed, 0, VIEW_HEIGHT as i32);
        for y in VIEW_HEIGHT..LOGICAL_HEIGHT {
            for x in 0..LOGICAL_WIDTH {
                let index = self.indexed[y * LOGICAL_WIDTH + x];
                self.frame[y * LOGICAL_WIDTH + x] = self.palette[usize::from(index)];
            }
        }
        self.rect(0, 0, 320, VIEW_HEIGHT, 3);
        self.shadow_text(32, 8, heading, 17);
        self.shadow_text(32, 18, subheading, 16);
        for (index, (label, selected)) in rows.iter().enumerate() {
            let y = 28 + 10 * index as i32;
            self.prefs_icon(32, y, *selected);
            let text: String = label.chars().take(30).collect();
            self.shadow_text(48, y, &text, 15);
            if *selected {
                self.prefs_bullet(23, y + 2);
            }
        }
        self.shadow_text(32, 140, footer, 16);
        self.present();
    }

    /// The save menu's name entry: the preference list's page with one text
    /// box. `name` is what the player types; `suffix` (`.json`) follows it inside
    /// the box in the sub-heading's red and is never part of the edit. `caret`
    /// draws the insertion bar after the name.
    pub fn draw_pref_textbox(
        &mut self,
        heading: &str,
        subheading: &str,
        name: &str,
        suffix: &str,
        caret: bool,
        footer: &str,
    ) {
        self.draw_pref_list(heading, subheading, &[], footer);
        let (x, y, width, height) = (32usize, 30usize, 256usize, 14usize);
        self.rect(x, y, width, height, 15);
        self.rect(x + 1, y + 1, width - 2, height - 2, 0);
        let width_of = |screen: &Self, text: &str| -> usize {
            text.bytes()
                .map(|code| screen.font.glyph(code).map_or(0, |glyph| glyph.advance))
                .sum()
        };
        let text_y = y + 3;
        self.shadow_text(x as i32 + 4, text_y as i32, name, 15);
        let after_name = x + 4 + width_of(self, name);
        if caret {
            self.rect(after_name + 1, y + 3, 1, 8, 18);
        }
        self.shadow_text(after_name as i32 + 4, text_y as i32, suffix, 16);
        self.present();
    }

    /// `0x2c` (on) and `0x2d` (off): a red or grey square with a dark border,
    /// as the menu icon bitmap draws it at the hot area's origin.
    fn prefs_icon(&mut self, x: i32, y: i32, on: bool) {
        let bitmap = if on {
            self.prefs_on.as_ref()
        } else {
            self.prefs_off.as_ref()
        };
        if let Some(bitmap) = bitmap {
            bitmap.blit_rgb(&mut self.frame, &self.palette, x, y);
            return;
        }
        // Fallback for a cache without the primary bitmaps: the same 11x10
        // pattern, hand-drawn (the on icon's captured pixel values).
        let (edge, body, shade) = if on { (16u8, 29u8, 26u8) } else { (8, 0, 8) };
        let (x, y) = (x.max(0) as usize, y.max(0) as usize);
        self.rect(x + 2, y + 1, 7, 1, edge);
        self.rect(x + 2, y + 2, 1, 5, edge);
        self.rect(x + 3, y + 2, 6, 5, body);
        self.rect(x + 9, y + 2, 1, 5, shade);
        self.rect(x + 2, y + 7, 7, 1, body);
        self.rect(x + 3, y + 8, 6, 1, shade);
    }

    /// FFE text with its one-pixel black drop shadow.
    /// The keyboard selection's round 4x4 bullet with a drop shadow, its top
    /// left at (x, y).
    fn prefs_bullet(&mut self, x: i32, y: i32) {
        for (dx, dy, colour) in [(1, 1, 0u8), (0, 0, 18)] {
            let (x, y) = ((x + dx) as usize, (y + dy) as usize);
            self.rect(x + 1, y, 2, 1, colour);
            self.rect(x, y + 1, 4, 2, colour);
            self.rect(x + 1, y + 3, 2, 1, colour);
        }
    }

    fn shadow_text(&mut self, x: i32, y: i32, message: &str, colour: u8) {
        if x < 0 || y < 0 {
            return;
        }
        self.text(x as usize + 1, y as usize + 1, message, 0);
        self.text(x as usize, y as usize, message, colour);
    }

    /// The navigation map: a scrolling list at the left and a plot of the bodies
    /// around the ship (log scale, the ship at the centre) at the right.
    /// The list row of the map under logical pixel (x, y), as `draw_map` lays them out.
    pub fn map_row(selected: usize, count: usize, x: i32, y: i32) -> Option<usize> {
        let rows = 11usize;
        if !(10..=190).contains(&x) || y < 21 {
            return None;
        }
        let row = ((y - 21) / 11) as usize;
        let first = selected
            .saturating_sub(rows / 2)
            .min(count.saturating_sub(rows));
        (row < rows && first + row < count).then_some(first + row)
    }

    /// The row of the navigation list under logical pixel (x, y): the original's two
    /// columns of nine, 15 pixels apart from y 5, the second column from x 230.
    pub fn nav_row(x: i32, y: i32) -> Option<usize> {
        if !(5..140).contains(&y) || !(10..310).contains(&x) {
            return None;
        }
        let row = ((y - 5) / 15) as usize;
        let column = usize::from(x >= 230);
        (row < 9).then_some(column * 9 + row)
    }

    /// The top-left corner of F1099's list entry `index`: nine rows of fifteen
    /// pixels down the first column from (10, 5), then the second from x 230
    /// (ffe.asm 80515-80516, 80575-80579). `None` past the original's eighteen
    /// marker hot areas (`0xd1..=0xe2`).
    pub fn place_cell(index: usize) -> Option<(i32, i32)> {
        const COLUMNS: [i32; 2] = [10, 230];
        const ROWS: usize = 9;
        (index < COLUMNS.len() * ROWS)
            .then(|| (COLUMNS[index / ROWS], 5 + 15 * (index % ROWS) as i32))
    }

    /// The original navigation computer's marker under (x, y): F1099 adds a
    /// hot area at the marker's top-left corner, 10 pixels wide and high
    /// (ffe.asm 80536-80551), in the first column at x 10..=20 and the second
    /// at x 230..=240, nine rows from y 5 every 15 pixels.
    pub fn place_row(x: i32, y: i32) -> Option<usize> {
        (0..18).find(|index| {
            Self::place_cell(*index)
                .is_some_and(|(cx, cy)| (cx..=cx + 10).contains(&x) && (cy..=cy + 10).contains(&y))
        })
    }

    /// The navigation computer: a marker square and a name per body, nine rows in two
    /// columns; `rows` are (name, selected, locked on).
    pub fn draw_nav_list(&mut self, rows: &[(String, bool, bool)], detail: &str) {
        self.rect(0, 0, 320, 158, 1);
        for (k, (name, selected, locked)) in rows.iter().enumerate() {
            let (x, y) = (if k < 9 { 10 } else { 230 }, 5 + 15 * (k % 9));
            let colour = if *locked {
                17
            } else if *selected {
                18
            } else {
                12
            };
            self.rect(x, y, 10, 10, colour);
            let room = if k < 9 { 28 } else { 10 };
            let text: String = name.chars().take(room).collect();
            self.text(
                x + 15,
                y + 1,
                &text.to_ascii_uppercase(),
                if *selected { 18 } else { 15 },
            );
        }
        self.rect(0, 142, 320, 1, 15);
        self.text(5, 146, detail, 18);
    }

    /// The original navigation computer's list (F1099): the marker bitmap 0x31 and
    /// the name fifteen pixels to its right, nine rows of fifteen pixels from y 5,
    /// then a second column from x 230. It is drawn over the flight view (the
    /// original calls F1100 first and F1099 then draws only the markers and names),
    /// with no title, selection, distance or background. The rows all lie above
    /// y 134, so F272's message window (y 159..199) and F1099's own Set/Act text
    /// (y 148) cannot cover them.
    pub fn draw_places(&mut self, rows: &[String]) {
        for (index, name) in rows.iter().take(18).enumerate() {
            let Some((x, y)) = Self::place_cell(index) else {
                break;
            };
            if let Some(marker) = &self.marker {
                marker.blit_colours(&mut self.frame, &self.palette, x, y);
            }
            self.text((x + 15) as usize, y as usize, name, 0x11);
        }
        self.present();
    }

    pub fn draw_map(
        &mut self,
        title: &str,
        lines: &[String],
        selected: usize,
        points: &[(f32, f32, crate::navigation::Kind, bool)],
        status: &str,
    ) {
        use crate::navigation::Kind;
        self.rect(4, 4, 312, 152, 1);
        self.text(10, 8, title, 18);
        let rows = 11usize;
        let first = selected
            .saturating_sub(rows / 2)
            .min(lines.len().saturating_sub(rows));
        for (row, line) in lines.iter().skip(first).take(rows).enumerate() {
            let colour = if first + row == selected { 18 } else { 15 };
            let clipped: String = line.chars().take(30).collect();
            self.text(10, 22 + row * 11, &clipped, colour);
        }
        // The plot: a frame, the ship at the centre and a dot per body.
        let (cx, cy, half) = (244usize, 80usize, 66.0f32);
        self.rect(cx - 68, cy - 68, 136, 1, 15);
        self.rect(cx - 68, cy + 67, 136, 1, 15);
        self.rect(cx - 68, cy - 68, 1, 136, 15);
        self.rect(cx + 67, cy - 68, 1, 136, 15);
        self.rect(cx - 1, cy - 1, 3, 3, 12);
        for (x, y, kind, chosen) in points {
            let colour = if *chosen {
                18
            } else {
                match kind {
                    Kind::Star => 14,
                    Kind::Planet => 10,
                    Kind::Moon => 9,
                    Kind::Station | Kind::Starport => 13,
                }
            };
            let size = if matches!(kind, Kind::Star) { 3 } else { 2 };
            let px = (cx as f32 + x * half).round() as usize;
            let py = (cy as f32 + y * half).round() as usize;
            self.rect(
                px.saturating_sub(1),
                py.saturating_sub(1),
                size,
                size,
                colour,
            );
        }
        self.text(10, 146, status, 15);
        self.present();
    }

    /// Test support for the whole-frame scene oracle (`v0.48-14`): clear the
    /// 320x200 logical frame to black and draw an already depth-sorted shape
    /// list through the same rasterisers the flight view uses. Not called by
    /// the game.
    #[doc(hidden)]
    pub fn scene_frame_clear(&mut self) {
        self.frame.fill(0);
    }

    /// Test support: the whole-frame draw pass after [`Self::scene_frame_clear`].
    #[doc(hidden)]
    pub fn scene_frame_draw(&mut self, viewer: &ModelViewer, shapes: &[Shape]) {
        self.draw_shapes_full(viewer, shapes);
    }

    /// One object of the flight view. The original's F1680 only queues an object's
    /// primitives; F1789 sorts the whole frame's queue by depth key and draws it, so while
    /// that queue is open the shapes go into it with their keys.
    fn draw_frame(&mut self, viewer: &ModelViewer, frame: &Frame) {
        if let Some(queue) = self.object_queue.as_mut() {
            let mut key = u32::MAX;
            for (index, shape) in frame.shapes.iter().enumerate() {
                key = frame.keys.get(index).copied().unwrap_or(key);
                queue.push((key, shape.clone()));
            }
            return;
        }
        self.draw_shapes(viewer, &frame.shapes);
    }

    /// F6: every imported model executed by the verified port, in original order.
    /// Draw executed-model shapes in order into the 320x158 view.
    fn draw_shapes(&mut self, viewer: &ModelViewer, shapes: &[Shape]) {
        // The executor's view height: 158, or 200 while F917's tombstone raises it.
        let rows = ffe_core::model_execute::view_height().clamp(0, LOGICAL_HEIGHT as i32);
        self.draw_shapes_in(viewer, shapes, rows as usize);
    }

    /// The same drawing without the flight view's y=158 clip, for the game-over
    /// tombstone, whose slab reaches the bottom of the 320x200 logical screen.
    fn draw_shapes_full(&mut self, viewer: &ModelViewer, shapes: &[Shape]) {
        self.draw_shapes_in(viewer, shapes, LOGICAL_HEIGHT);
    }

    fn draw_shapes_in(&mut self, viewer: &ModelViewer, shapes: &[Shape], height: usize) {
        let value = |[r, g, b]: [u8; 3]| (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        for shape in shapes {
            match shape {
                Shape::Fill(points, rgb) => self.fill_value(points, value(*rgb)),
                Shape::Gouraud(points, colours) => {
                    if let ([a, b, c], [ca, cb, cc]) = (&points[..], &colours[..]) {
                        self.gouraud([*a, *b, *c], [*ca, *cb, *cc]);
                    }
                }
                Shape::Polygon(edges, rgb) => {
                    // F1560 keeps rows in 0..158 and F1600 clips x when an edge
                    // reaches off the view, so every pixel lands in the view.
                    let frame = &mut self.frame;
                    let _ = fill_polygon(edges, |x, y| {
                        if (0..320).contains(&x) && (0..height as i32).contains(&y) {
                            frame[y as usize * 320 + x as usize] = value(*rgb);
                        }
                    });
                }
                Shape::Flat(points, rgb) => {
                    // F1604/F1607 on an on-view entry: every pixel lands in the view.
                    let frame = &mut self.frame;
                    let p: Vec<[i16; 2]> =
                        points.iter().map(|&(x, y)| [x as i16, y as i16]).collect();
                    let plot = |x: i32, y: i32| frame[y as usize * 320 + x as usize] = value(*rgb);
                    if let [a, b, c] = p[..] {
                        fill_triangle([a, b, c], plot);
                    } else if let [a, b, c, d] = p[..] {
                        fill_quad([a, b, c, d], plot);
                    }
                }
                Shape::Line(a, b, rgb) => {
                    let on_view = |(x, y): (i32, i32)| {
                        (0..320).contains(&x) && (0..height as i32).contains(&y)
                    };
                    if on_view(*a) && on_view(*b) {
                        // F1616 on end points F1704 has already put on the view.
                        let frame = &mut self.frame;
                        let point = |(x, y): (i32, i32)| [x as i16, y as i16];
                        draw_line(point(*a), point(*b), |x, y| {
                            frame[y as usize * 320 + x as usize] = value(*rgb);
                        });
                    } else {
                        // Stand-in shapes (two-point faces): bounded Bresenham.
                        self.line_value(*a, *b, value(*rgb));
                    }
                }
                Shape::Curve(points, rgb) => {
                    // F1546 can leave a clipped end off the view; the original then
                    // draws outside it (over the console), which is dropped here.
                    let frame = &mut self.frame;
                    let _ = draw_curve(*points, |x, y| {
                        if (0..320).contains(&x) && (0..height as i32).contains(&y) {
                            frame[y as usize * 320 + x as usize] = value(*rgb);
                        }
                    });
                }
                Shape::Spans(spans, rgb) => {
                    let frame = &mut self.frame;
                    for [x, y, width] in spans {
                        if (0..height as i32).contains(y) {
                            for column in *x..*x + *width {
                                if (0..320).contains(&column) {
                                    frame[*y as usize * 320 + column as usize] = value(*rgb);
                                }
                            }
                        }
                    }
                }
                Shape::Point((x, y), rgb) => {
                    if (0..VIEW_HEIGHT as i32).contains(y) && (0..320).contains(x) {
                        self.frame[*y as usize * 320 + *x as usize] = value(*rgb);
                    }
                }
                Shape::Textured(textured) | Shape::Glow(textured) => {
                    let frame = &mut self.frame;
                    // The rasteriser clips to x 0..320, y 0..158 itself.
                    viewer.rasterise(textured, |x, y, rgb| {
                        frame[y as usize * 320 + x as usize] = value(rgb);
                    });
                }
                Shape::TexturedUv(textured, uv) => {
                    let frame = &mut self.frame;
                    viewer.rasterise_uv(textured, uv, |x, y, rgb| {
                        frame[y as usize * 320 + x as usize] = value(rgb);
                    });
                }
                Shape::Label((x, y), text) => {
                    if *x >= 0 && *y >= 0 {
                        self.text_bytes(*x as usize, *y as usize, text, 15);
                    }
                }
            }
        }
    }

    /// Greedy word wrap at `width` logical pixels, using the same font metrics
    /// F1398 uses. A word longer than the line is kept whole (the original's
    /// wrap does the same).
    fn wrap_text(&self, message: &str, width: i32) -> Vec<String> {
        station::wrap_words(message, width, |text| self.font.width(text).unwrap_or(0))
    }

    /// The market screen as F536/F538 draw it: the stockmarket screen's column
    /// header at y 2, thirteen ten-pixel rows from y 15 (the Buy and Sell
    /// buttons at x 124 and 136), and the original's cargo/cash/cabins line.
    fn draw_market(&mut self) {
        let Some(view) = self.market.take() else {
            return;
        };
        let rows: Vec<DockRow> = view
            .rows
            .iter()
            .map(|row| DockRow {
                main: row.name.clone(),
                detail: None,
                columns: vec![
                    (
                        station::MARKET_PRICE_X,
                        format!("${}", crate::trade::credits(row.price)),
                        0x7d,
                    ),
                    (station::MARKET_STOCK_X, format!("{}t", row.stock), 0x7d),
                    (
                        station::MARKET_CARGO_X,
                        if row.held > 0 {
                            format!("{}t", row.held)
                        } else {
                            "-".to_owned()
                        },
                        0x2b,
                    ),
                ],
                fitted: false,
            })
            .collect();
        let visible = station::layout(StationScreen::Market).visible();
        let first = view.scroll.min(rows.len().saturating_sub(visible));
        let dock = DockView {
            screen: StationScreen::Market,
            title: if view.black_market {
                view.station.clone()
            } else {
                format!("{} Stockmarket", view.station)
            },
            rows: rows[first..].to_vec(),
            status: Status {
                cargo_used: view.used,
                cargo_capacity: view.capacity,
                cash: view.cash,
                ..Status::default()
            },
            scroll: first,
            selected: view.selected,
            total: view.rows.len(),
        };
        self.draw_dock(&dock);
        self.market = Some(view);
    }

    /// The docked services, shipyard and BBS screens (F327/F329/F330/F558).
    /// The layout constants and hot areas live in [`crate::station`].
    /// A glyph's advance in the main font (F1598's `+9`), `None` without one.
    pub fn glyph_advance(&self, code: u8) -> Option<i32> {
        self.font.advance(code).map(|advance| advance as i32)
    }

    /// F80 and F79: F956's cabin (already drawn), the journal's masthead
    /// centred at y 2, the up arrow and "More ..." while the text is
    /// scrolled, F1585's text from (10, y) clipped to 0x28..0x8c, and the
    /// down arrow and "More ..." while text is below the page.
    fn draw_issue(&mut self, view: &IssueView) {
        use crate::journal_page::{self, COLOUR, DOWN_ARROW, MORE, UP_ARROW};
        let palette = self.palette;
        if let Some(Some(masthead)) = self.mastheads.get(view.masthead as usize) {
            let x = (320 - masthead.width() as i32) / 2;
            masthead.blit_rgb(&mut self.frame, &palette, x, journal_page::MASTHEAD_Y);
        }
        if view.y < journal_page::TOP {
            if let Some(Some(icon)) = self.dock_icons.get(1) {
                icon.blit_rgb(&mut self.frame, &palette, UP_ARROW.1, UP_ARROW.2);
            }
            self.text(MORE.1 as usize, MORE.2 as usize, &view.more, COLOUR);
        }
        let page = journal_page::layout(&view.text, view.y, |code| self.glyph_advance(code));
        for glyph in &page.glyphs {
            self.text_bytes_rows(
                glyph.x as usize,
                glyph.y,
                &[glyph.code],
                COLOUR,
                journal_page::TOP,
                journal_page::BOTTOM,
            );
        }
        if page.more_below {
            if let Some(Some(icon)) = self.dock_icons.get(2) {
                icon.blit_rgb(&mut self.frame, &palette, DOWN_ARROW.1, DOWN_ARROW.2);
            }
            self.text(MORE.1 as usize, MORE.2 as usize, &view.more, COLOUR);
        }
    }

    fn draw_dock(&mut self, view: &DockView) {
        let layout = station::layout(view.screen);
        self.text(4, 6, &view.title, 0x7d);
        for &(x, text) in station::header(view.screen) {
            self.text(x as usize, 2, text, 0x7e);
        }
        let pitch = layout.pitch;
        // F327/F329: the entry text wraps at `0x140 - 3 - x` and the next row's
        // top moves by the larger of the screen pitch and F329's
        // `10 * StringDrawWrapShadow lines`, so a long ten-pixel reply occupies
        // two (or more) lines and pushes the rows below it down.
        let width = 0x140 - 3 - layout.entry_x;
        let paragraph = matches!(view.screen, StationScreen::Journal | StationScreen::Comms);
        let palette = self.palette;
        let mut y = station::LIST_TOP;
        let mut drawn = 0;
        for row in &view.rows {
            if y + pitch >= station::LIST_BOTTOM {
                break;
            }
            // F328: an 8x9 button at x 124 (and x 136 for the two-button screens).
            // On Upgrades F329 draws the Remove button only when F383 set the
            // row's fitted bit (F392).
            for button in 0..layout.buttons {
                if button == 1 && view.screen == StationScreen::Upgrades && !row.fitted {
                    continue;
                }
                let x = 124 + button * 12;
                if let Some(Some(icon)) = self.dock_icons.first() {
                    icon.blit_rgb(&mut self.frame, &palette, x, y);
                }
            }
            if paragraph {
                // The journal/comms page is a paragraph list: each entry wraps
                // across the 320-pixel width (x 4..317) in three 10-pixel
                // lines, as F1399's journal viewer does.
                for (line, text) in self.wrap_text(&row.main, 312).iter().take(3).enumerate() {
                    self.text(
                        layout.entry_x as usize,
                        (y + 1 + 10 * line as i32) as usize,
                        text,
                        0x7d,
                    );
                }
                y += pitch;
            } else {
                let lines = self.wrap_text(&row.main, width);
                for (line, text) in lines.iter().enumerate() {
                    self.text(
                        layout.entry_x as usize,
                        (y + 1 + 10 * line as i32) as usize,
                        text,
                        0x7d,
                    );
                }
                if let Some(detail) = &row.detail {
                    self.text(
                        layout.entry_x as usize,
                        (y + 1 + 10 * i32::try_from(lines.len()).unwrap_or(1)) as usize,
                        detail,
                        0x7d,
                    );
                }
                for (x, text, colour) in &row.columns {
                    self.text(*x as usize, (y + 1) as usize, text, *colour);
                }
                y += (10 * i32::try_from(lines.len()).unwrap_or(1)).max(pitch);
            }
            drawn += 1;
        }
        // F330: the down arrow at (124, 147) while entries remain, the up arrow
        // at (136, 147) once the window is scrolled, and "More..." at (148, 148).
        if view.total > view.scroll + drawn {
            if let Some(Some(icon)) = self.dock_icons.get(2) {
                icon.blit_rgb(&mut self.frame, &palette, 124, 147);
            }
            self.text(148, 148, "More...", 0x7d);
        }
        if view.scroll > 0 {
            if let Some(Some(icon)) = self.dock_icons.get(1) {
                icon.blit_rgb(&mut self.frame, &palette, 136, 147);
            }
        }
        // F566: the "Go Back..." button and text.
        if layout.back {
            if let Some(Some(icon)) = self.dock_icons.first() {
                icon.blit_rgb(&mut self.frame, &palette, 200, 147);
            }
            self.text(210, 147, "Go Back...", 0x7d);
        }
        // F558's cargo/cash/cabins line (0x98e8). The journal/comms text pages
        // use the space, so they omit it.
        if matches!(view.screen, StationScreen::Journal | StationScreen::Comms) {
            return;
        }
        self.draw_cargo_status(&view.status);
    }

    /// F558's cargo/cabins/cash block (`0x98e8`) at the station's lower
    /// left: blue labels (0x43), white values.
    fn draw_cargo_status(&mut self, status: &Status) {
        let money = format!("${}", crate::trade::credits(status.cash));
        let money_x = (station::STATUS_CASH_RIGHT as usize)
            .saturating_sub(self.font.width(&money).unwrap_or(0));
        let label = station::STATUS_LABEL_X as usize;
        let value = station::STATUS_VALUE_X as usize;
        let (row_1, row_2) = (
            station::STATUS_ROW_1 as usize,
            station::STATUS_ROW_2 as usize,
        );
        self.text(label, row_1, "Cargo", 0x43);
        self.text(
            value,
            row_1,
            &format!("{} : {}", status.cargo_used, status.cargo_capacity),
            0x0f,
        );
        self.text(station::STATUS_CASH_LABEL_X as usize, row_1, "Cash", 0x43);
        self.text(label, row_2, "Cabins", 0x43);
        self.text(
            value,
            row_2,
            &format!("{} : {}", status.cabins_used, status.cabins_total),
            0x0f,
        );
        self.text(money_x, row_2, &money, 0x0f);
    }

    /// F310's `0x9c3b` at (190,138), with F1399's drop shadow and its
    /// absolute `0xfd` tab stops: the used tonnes/cabins at 234, the totals
    /// at 252, "Cash" at 272 and the cash at 264, in 10-pixel lines.
    fn draw_status_hold(&mut self, status: &Status) {
        const LABEL: u8 = 0x43;
        const VALUE: u8 = 0x0f;
        let rows = [
            ("Cargo Space", status.cargo_used, status.cargo_capacity, 138),
            ("Cabins", status.cabins_used, status.cabins_total, 148),
        ];
        for (label, used, total, y) in rows {
            self.shadow_text(190, y, label, LABEL);
            let used = format!("{used} :");
            self.shadow_text(234, y, &used, VALUE);
            let cursor = 234 + self.font.width(&used).unwrap_or(0) as i32;
            self.shadow_text(cursor.max(252), y, &total.to_string(), VALUE);
        }
        self.shadow_text(272, 138, "Cash", LABEL);
        let money = format!("${}", crate::trade::credits(status.cash));
        self.shadow_text(264, 148, &money, VALUE);
    }

    /// The station as the original model 74, executed by the verified port and
    /// placed by the synthetic flight camera. False when it cannot be drawn.
    /// The light ships, stations and lift objects are drawn with: the local
    /// star's direction in system flight, else the original's fixed F960 light.
    fn scene_light(&self, scene: &ModelViewer, camera: &Camera) -> [f64; 3] {
        match self.sun {
            // `sun` points toward the star; the original's light vector (F1535,
            // F1481's `dot < 0` test) points from the star toward the observer.
            Some(direction) => camera.direction(direction.map(|value| -value)),
            None => camera.direction(scene.light_direction()),
        }
    }

    fn draw_station(&mut self, scene: &ModelViewer, state: &Flight, camera: &Camera) -> bool {
        let Some(scale) = model_scale(scene) else {
            return false;
        };
        // Station rotation (docs/notes/animation.md): F866 spins the station
        // about its own z (docking) axis at the generated rate; the flight's
        // 50 Hz ticks are 994 game tics each from the new-game GameTics.
        let tics = flight_tics(state);
        let spin = scene.station_rows(tics, STATION_SPEED);
        let axes: [[f64; 3]; 3] = std::array::from_fn(|k| {
            std::array::from_fn(|i| (0..3).map(|j| spin[k][j] * STATION_AXES[j][i]).sum())
        });
        let rows = axes.map(|axis| camera.direction(axis));
        let origin = camera.point([0.0; 3]).map(|v| v * scale);
        // The original default light (F960), fixed in flight space.
        let light = self.scene_light(scene, camera);
        let mut inputs = AnimationInputs::original(tics, 0, &scene.shades());
        inputs.jitter = self.jitter.cursor;
        match scene.render_scene_with(STATION_MODEL, rows, origin, light, &inputs) {
            Ok(frame) => {
                self.jitter.cursor = frame.jitter;
                self.draw_frame(scene, &frame);
                true
            }
            Err(_) => false,
        }
    }

    /// The lift models of a docking or launching stage, the farthest first, in view space.
    fn draw_lift(&mut self, scene: &ModelViewer, lift: &LiftView, camera: &Camera, eye: [i16; 2]) {
        let mut objects = lift.objects.clone();
        objects.sort_by_key(|object| std::cmp::Reverse(object.depth));
        // The lift is inside the station: the original's fixed default light,
        // not the local star (which leaves the door faces black as it closes).
        let light = camera.direction(scene.light_direction());
        for object in objects {
            let position = ffe_core::lift::view_position(&object, eye).map(f64::from);
            let rows = scene.lift_rows(object.angle);
            let mut inputs = AnimationInputs::original(lift.tics, object.word, &scene.shades());
            inputs.jitter = self.jitter.cursor;
            inputs.globals[2] = lift.seed as u16;
            inputs.globals[3] = (lift.seed >> 16) as u16;
            inputs.globals[4] = u16::from(!lift.starport);
            let r = scene.render_scene_with(object.model, rows, position, light, &inputs);
            if let Ok(frame) = r {
                self.jitter.cursor = frame.jitter;
                self.draw_frame(scene, &frame);
            }
        }
    }

    /// The system's stations, the farthest first, with the player's own ship (when
    /// it shows) in painter's order among them.
    fn draw_system_stations(
        &mut self,
        scene: &ModelViewer,
        state: &Flight,
        camera: &Camera,
        own_ship: bool,
    ) {
        // Classic draws the objects as the original does: each only queues its primitives,
        // and one sort by depth key draws them, so a ship hides the dust behind it and a
        // station hides the ships behind it, whichever class is drawn first.
        self.object_queue.get_or_insert_with(Vec::new);
        self.queue_system_objects(scene, state, camera, own_ship);
        self.flush_object_queue(scene);
    }

    /// F1789: draw the queued shapes farthest first. The sort is F1788's: descending unsigned
    /// key, equal keys in allocation order (a stable sort of the queue).
    fn flush_object_queue(&mut self, viewer: &ModelViewer) {
        if let Some(mut queue) = self.object_queue.take() {
            queue.sort_by_key(|entry| std::cmp::Reverse(entry.0));
            let shapes: Vec<Shape> = queue.into_iter().map(|(_, shape)| shape).collect();
            self.draw_shapes(viewer, &shapes);
        }
    }

    fn queue_system_objects(
        &mut self,
        scene: &ModelViewer,
        state: &Flight,
        camera: &Camera,
        own_ship: bool,
    ) {
        let mut order: Vec<(f64, SystemStation)> = self
            .stations
            .iter()
            .map(|s| (distance(camera.position, s.centre), *s))
            .collect();
        order.sort_by(|a, b| b.0.total_cmp(&a.0));
        let ship_range = distance(camera.position, state.position);
        let mut ship_drawn = !own_ship;
        for (range, station) in order {
            if !ship_drawn && range < ship_range {
                self.draw_ship(scene, state, camera);
                ship_drawn = true;
            }
            self.draw_system_station(scene, &station, camera);
        }
        if !ship_drawn {
            self.draw_ship(scene, state, camera);
        }
        self.draw_system_ships(scene, state, camera);
        self.draw_system_effects(scene, state, camera);
    }

    /// F1680's far, cull and dot decisions for one object of the flight view, before its
    /// model runs. The original's frame loop (F1536/F1537) sends every object through
    /// F1680, not only the planets: past its model's far class (descriptor `+0x20`) the
    /// object is not drawn at all, F1686's bounding-sphere cull drops it, and from its
    /// dot class (`+0x28`) it is under a pixel across, so F1700 draws its glint (a lit
    /// body) or particle (an unlit one) instead of a model. `true` when the caller must
    /// draw the model; `false` when F1680 skipped the object or F1700 drew its dot.
    /// A cache without the descriptor tables keeps the old model drawing.
    fn f1680_draws_model(
        &mut self,
        viewer: &ModelViewer,
        camera: &Camera,
        centre: [f64; 3],
        model: u32,
    ) -> bool {
        use ffe_core::model_animation::Draw;
        let Some(descriptor) = viewer.object_descriptor(model) else {
            return true;
        };
        match f1680_body(camera, centre, &descriptor) {
            Draw::Far { .. } | Draw::Culled { .. } => false,
            Draw::Dot {
                screen,
                depth,
                radius,
                ..
            } => {
                // F1479 could not project the centre: F1700's screen clip discards it.
                let (Some(screen), Some(classes)) = (screen, viewer.object_model(model)) else {
                    return false;
                };
                match viewer.object_dot_frame(classes, screen, radius, depth) {
                    Ok(Some(frame)) => {
                        self.draw_frame(viewer, &frame);
                        false
                    }
                    Ok(None) => false,
                    // F1700's lit shaded sphere (nine pixels across or more at the dot
                    // class) is not ported: the model stands in for it.
                    Err(_) => true,
                }
            }
            Draw::Model { .. } => true,
        }
    }

    /// The system's NPC ships, farthest first. A ship whose shields absorbed a
    /// hit also draws F1536's model 0x9d (157) over itself; a destroyed ship
    /// draws no model (F942's effect in `draw_system_effects` replaces it).
    fn draw_system_ships(&mut self, scene: &ModelViewer, state: &Flight, camera: &Camera) {
        let Some(scale) = model_scale(scene) else {
            return;
        };
        let mut order = self.ships.clone();
        order.extend(self.missiles.iter().map(|missile| SystemShip {
            model: missile.model,
            centre: missile.centre,
            axes: missile.axes,
            flash: false,
            dead: false,
        }));
        order.sort_by(|a, b| {
            distance(camera.position, b.centre).total_cmp(&distance(camera.position, a.centre))
        });
        let light = self.scene_light(scene, camera);
        for ship in order {
            let origin = camera.point(ship.centre).map(|v| v * scale);
            if origin.iter().any(|value| value.abs() > STATION_DRAW_RANGE) {
                continue;
            }
            // F1680: a ship past its model's far class is not drawn and one past its
            // dot class is an F1700 glint; F1686 culls it when the sphere is outside
            // the view. A destroyed ship's slot is freed (F924), so it is left to
            // F942's effect in `draw_system_effects`.
            if !ship.dead && !self.f1680_draws_model(scene, camera, ship.centre, ship.model) {
                continue;
            }
            let rows = ship.axes.map(|axis| camera.direction(axis));
            if !ship.dead {
                let mut inputs = AnimationInputs::original(flight_tics(state), 0, &scene.shades());
                inputs.jitter = self.jitter.cursor;
                if let Ok(frame) = scene.render_scene_with(ship.model, rows, origin, light, &inputs)
                {
                    self.jitter.cursor = frame.jitter;
                    self.draw_frame(scene, &frame);
                }
            }
            if ship.flash {
                let mut inputs = AnimationInputs::original(flight_tics(state), 0, &scene.shades());
                inputs.jitter = self.jitter.cursor;
                let ship_scale = scene.find(ship.model).map(|model| model.scale);
                if let Ok(frame) = scene.render_scene_with_scale(
                    crate::combat::SHIELD_FLASH_MODEL,
                    ship_scale,
                    rows,
                    origin,
                    light,
                    &inputs,
                ) {
                    self.jitter.cursor = frame.jitter;
                    self.draw_frame(scene, &frame);
                }
            }
        }
        // F1776 for each NPC whose gun fired: model 0x9c at the gun, drawn after the
        // ships as the player's own beam is.
        for shot in std::mem::take(&mut self.ship_beams) {
            let stats = scene.ship_stats(shot.model);
            let beam = crate::combat::npc_beam(
                shot.centre,
                shot.axes,
                stats.as_ref(),
                shot.code,
                shot.length,
            );
            let origin = camera.point(beam.origin).map(|v| v * scale);
            if origin.iter().any(|value| value.abs() > STATION_DRAW_RANGE) {
                continue;
            }
            self.draw_beam(scene, state, &beam, camera);
        }
    }

    /// The system's short-lived effects, farthest first: F939's hit particle,
    /// F942's explosion and F926's debris. The explosion passes FFP 1.0 as the
    /// object's global word 4, the value model 155's fragment branches select
    /// its twelve shapes for (see `ffe_core::effects`).
    fn draw_system_effects(&mut self, scene: &ModelViewer, state: &Flight, camera: &Camera) {
        let Some(scale) = model_scale(scene) else {
            return;
        };
        let mut order = self.effects.clone();
        // F1035's dust motes are not drawn from the player's death through the
        // explosion and the death camera (`docs/DEVIATIONS.md`, 2026-10-29).
        if state.death.is_some() || !self.dust_visible {
            order.retain(|effect| effect.model != ffe_core::dust::MODEL);
        }
        order.sort_by(|a, b| {
            distance(camera.position, b.centre).total_cmp(&distance(camera.position, a.centre))
        });
        let light = self.scene_light(scene, camera);
        for effect in order {
            let mut origin = camera.point(effect.centre).map(|v| v * scale);
            let mut effect_scale = effect.scale;
            // F1680's shift, which F1777's depth key puts back (`+0x130`).
            let mut key_scale = 0;
            // F1680 (`ffe.asm:101900-102027`) keeps any distance inside the 32-bit view space:
            // `shift = max(msb - 0x1c, 0)` of the relative position is taken off the position
            // first, then `e = desc[+0x18] + obj[+0x7e] - shift` either lowers the scale (`e`
            // negative: `+0x148 = max(desc[+0x14] + e, 0)`) or shifts the position right by `e`
            // (scale unchanged). Either way size over depth is `2^(scale + bias)` times the
            // model's own. Dust needs both halves: the cloud's `+0x7e` word is the F1035 jitter
            // plus the speed class and the time-acceleration shift less 7, so a fast ship's
            // clouds are many powers of two bigger (`+0x7e` up to about 25), and under time
            // acceleration F1517 leads them `2^shift` times as far ahead. `effect.scale` is
            // `desc[+0x14] + obj[+0x7e]`; model 0x6d's `desc[+0x18]` is 0.
            if effect.model == ffe_core::dust::MODEL {
                if let Some(total) = effect_scale {
                    let base = scene.find(effect.model).map_or(total, |model| model.scale);
                    let bias = total as i32 - base as i32;
                    let reach = origin.iter().fold(0.0f64, |far, v| far.max(v.abs()));
                    let msb = if reach >= 1.0 {
                        reach.log2().floor() as i32
                    } else {
                        0
                    };
                    let shift = (msb - 0x1c).max(0);
                    let e = bias - shift;
                    let moved = shift + e.max(0);
                    origin = origin.map(|v| v / 2.0f64.powi(moved));
                    effect_scale = Some((base as i32 + e.min(0)).max(0) as u32);
                    // `+0x130 = desc[+0x18] - e` for a negative `e`, else `desc[+0x18]` (0):
                    // the depth key and every child model subtract it from their scale.
                    key_scale = (-e).max(0);
                }
            }
            if origin.iter().any(|value| value.abs() > STATION_DRAW_RANGE) {
                continue;
            }
            let rows = effect.axes.map(|axis| camera.direction(axis));
            let mut inputs = AnimationInputs::original(flight_tics(state), 0, &scene.shades());
            // Model 0x6d's chains are detail tiers chosen by F1751 from the cloud's depth: a
            // far cloud draws only its first dots, a near one all of them.
            inputs.depth_detail = effect.model == ffe_core::dust::MODEL;
            inputs.detail = self.shape_detail;
            inputs.globals[ffe_core::effects::EXPLOSION_MASS_GLOBAL] = effect.timer;
            // F942 writes `+0x9e = 0xffff` (global 1) on the explosion: model
            // 155 scales its fragments by it (`global1 >> 7`), so a zero word
            // collapses every fragment onto the centre pixel.
            if effect.size != 0 {
                inputs.globals[1] = effect.size;
            }
            inputs.jitter = self.jitter.cursor;
            if let Ok(frame) = scene.render_scene_with_scale_key(
                effect.model,
                effect_scale,
                key_scale,
                rows,
                origin,
                light,
                &inputs,
            ) {
                if !frame.shapes.is_empty() {
                    self.jitter.cursor = frame.jitter;
                    self.draw_frame(scene, &frame);
                }
            }
        }
    }

    /// One station of the system: F866's spin about its z (docking) axis applied to
    /// the object's rows, lit from the system's star at the origin.
    fn draw_system_station(
        &mut self,
        scene: &ModelViewer,
        station: &SystemStation,
        camera: &Camera,
    ) -> bool {
        let Some(scale) = model_scale(scene) else {
            return false;
        };
        let origin = camera.point(station.centre).map(|v| v * scale);
        if origin.iter().any(|v| v.abs() > STATION_DRAW_RANGE) {
            return false;
        }
        // F1680 applies to the stations too: model 74's dot class is 16 (about 537 km),
        // so a station farther than that is a glint, and past its far class (24, about
        // 137,000 km) it is not drawn. Docking range is far inside the model's class.
        if !self.f1680_draws_model(scene, camera, station.centre, station.model) {
            return false;
        }
        let spin = scene.station_rows(station.tics, station.speed);
        let axes: [[f64; 3]; 3] = std::array::from_fn(|k| {
            std::array::from_fn(|i| (0..3).map(|j| spin[k][j] * station.orientation[j][i]).sum())
        });
        let rows = axes.map(|axis| camera.direction(axis));
        // One frame-wide light from the local star (F1535), not a per-station
        // direction; the sign convention is `scene_light`'s.
        let light = self.scene_light(scene, camera);
        let mut inputs = AnimationInputs::original(station.tics, station.door, &scene.shades());
        inputs.globals[2] = station.seed as u16;
        inputs.globals[3] = (station.seed >> 16) as u16;
        inputs.jitter = self.jitter.cursor;
        match scene.render_scene_with(station.model, rows, origin, light, &inputs) {
            Ok(frame) => {
                self.jitter.cursor = frame.jitter;
                self.draw_frame(scene, &frame);
                true
            }
            Err(_) => false,
        }
    }

    /// The player's ship (its own model, at the station's scale) seen by `camera`.
    fn draw_ship(&mut self, scene: &ModelViewer, state: &Flight, camera: &Camera) -> bool {
        let Some(scale) = model_scale(scene) else {
            return false;
        };
        let rows = ship_axes(state).map(|axis| camera.direction(axis));
        let origin = camera.point(state.position).map(|v| v * scale);
        let light = self.scene_light(scene, camera);
        // The original inputs over flight time, global 1 the undercarriage, global 15 the
        // object's z thrust word the engine glows read: the throttle scaled to the
        // ship's own limits (F600). Without statistics (a cache before v12), full
        // thrust maps to 4095, where ship models stop lengthening the plume.
        let mut inputs = AnimationInputs::original(flight_tics(state), state.gear, &scene.shades());
        inputs.globals[THRUST_GLOBAL] = match scene.ship_stats(state.ship) {
            Some(stats) => stats.thrust_word(state.thrust),
            None => (state.thrust.max(0.0) * 4095.0) as u16,
        };
        // The plume's far end is a jittered vertex: it flickers as the stream runs.
        inputs.jitter = self.jitter.cursor;
        match scene.render_scene_with(state.ship, rows, origin, light, &inputs) {
            Ok(frame) => {
                self.jitter.cursor = frame.jitter;
                self.draw_frame(scene, &frame);
                true
            }
            Err(_) => false,
        }
    }

    /// The galaxy map (F840, F828, F863): the cabin art behind the scene, the sector and fuel
    /// lines with a shadow at the lower left, and the date and the star's text in the console.
    fn draw_galaxy(&mut self, viewer: &ModelViewer, map: &MapScene) {
        self.frame[..320 * VIEW_HEIGHT]
            .copy_from_slice(&self.cabins[self.cabin_index][..320 * VIEW_HEIGHT]);
        let shadowed = |screen: &mut Self, x: usize, y: usize, value: &str| {
            screen.text(x + 1, y + 1, value, 0);
            screen.text(x, y, value, 15);
        };
        if let Some(page) = &map.page {
            self.draw_map_page(page);
        } else {
            self.draw_shapes(viewer, &map.shapes);
            shadowed(self, 6, 147, &map.sector);
            if let Some(fuel) = &map.fuel {
                shadowed(self, 6, 137, fuel);
            }
        }
        // The console text: the date, then the description wrapped to 120 pixels.
        self.text(195, 164, &map.date, 17);
        let mut y = 173;
        for paragraph in map.info.split('\r') {
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let trial = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if self.font.width(&trial).is_ok_and(|width| width > 120) && !line.is_empty() {
                    self.text(195, y, &line, 15);
                    y += 8;
                    line = word.to_owned();
                } else {
                    line = trial;
                }
            }
            self.text(195, y, &line, 15);
            y += 8;
        }
    }

    /// The enhanced (flat) galaxy map: the stars as faction- or danger-coloured
    /// dots, the reach circle, the route with its unflyable legs in red and its
    /// strand warnings, the lines home and to Sol, the system-name labels, the
    /// colour legend, the controls line and the console text, over the cabin.
    fn draw_enhanced_galaxy(&mut self, scene: &EnhancedGalaxy) {
        use crate::enhanced_map as em;
        // A dark, slightly blue space (a little lighter toward the bottom) under a grid, in
        // place of the cabin's art, as Elite Dangerous' galaxy map has.
        for y in 0..VIEW_HEIGHT {
            let rise = (y * 12 / VIEW_HEIGHT) as u32;
            let colour = ((2 + rise / 3) << 16) | ((5 + rise / 2) << 8) | (11 + rise);
            self.frame[y * 320..(y + 1) * 320].fill(colour);
        }
        for line in &scene.grid {
            let colour = if line.major { 0x1a2a46 } else { 0x0d1626 };
            if line.vertical {
                if (0..320).contains(&line.at) {
                    for y in 0..VIEW_HEIGHT {
                        self.frame[y * 320 + line.at as usize] = colour;
                    }
                }
            } else if (0..VIEW_HEIGHT as i32).contains(&line.at) {
                let row = line.at as usize * 320;
                self.frame[row..row + 320].fill(colour);
            }
        }
        let shadowed = |screen: &mut Self, x: usize, y: usize, value: &str, colour: u8| {
            screen.text(x + 1, y + 1, value, 0);
            screen.text(x, y, value, colour);
        };
        // Darken the strips the legend, the controls line and the notes sit on, so
        // they read over the busiest part of the starfield.
        self.dim_rect(0, 0, 320, 24, 45);
        let notes = 1 + usize::from(scene.route_summary.is_some());
        let notes_top = 158 - 10 * notes - 2;
        self.dim_rect(0, notes_top, 200, 158, 45);
        // The drive's reach circle about the home system.
        self.circle(
            scene.home.0,
            scene.home.1,
            scene.range_radius.max(1) * 2,
            em::REACH_COLOUR,
        );
        // The route line from home through each waypoint (red where the leg
        // cannot be flown), and the waypoint rings.
        for (index, pair) in scene.route.windows(2).enumerate() {
            let ok = scene.leg_ok.get(index).copied().unwrap_or(true);
            self.line(
                pair[0],
                pair[1],
                if ok {
                    em::ROUTE_COLOUR
                } else {
                    em::THREAT_HIGH
                },
            );
        }
        for (index, point) in scene.route.iter().skip(1).enumerate() {
            let ok = scene.leg_ok.get(index).copied().unwrap_or(true);
            self.circle(
                point.0,
                point.1,
                12,
                if ok {
                    em::ROUTE_COLOUR
                } else {
                    em::THREAT_HIGH
                },
            );
        }
        // Warning triangles on waypoints that would strand the player.
        for (x, y) in &scene.warnings {
            let amber = self.palette[0x5f];
            self.fill_value(&[(*x, *y - 5), (*x - 4, *y + 3), (*x + 4, *y + 3)], amber);
        }
        // The hovered star gets a thin white ring (it can be clicked), the
        // selected one a yellow ring, both under their dots.
        if let Some((x, y, size)) = scene.hovered {
            self.circle(x, y, (size + 14).max(18), 15);
        }
        if let Some((x, y, size)) = scene.selected {
            self.circle(x, y, (size + 8).max(12), 18);
        }
        // Stars: solid dots (a dot's `size` is its radius).
        for (x, y, size, colour) in &scene.dots {
            self.fill_disc(*x, *y, (*size).max(1), *colour);
        }
        // System-name labels, shadowed for readability.
        for (x, y, name) in &scene.labels {
            if name.is_empty() {
                continue;
            }
            let (x, y) = ((*x + 3).max(0) as usize, (*y - 4).max(0) as usize);
            // Not under the legend and controls line, nor the notes in the corner.
            if y < 24 || (x < 200 && y + 8 > notes_top) {
                continue;
            }
            self.text(x + 1, y + 1, name, 0);
            self.text(x, y, name, 15);
        }
        // The line from the middle of the view to the player's own system, with
        // an arrowhead at its end, and "You" with the distance when it is off view.
        let (cx, cy) = (160, VIEW_HEIGHT as i32 / 2);
        let pointer = scene.home_pointer;
        // Where the "You" label went when the line home runs off the view, so the
        // Sol label can keep clear of it.
        let mut home_label: Option<(i32, i32, i32)> = None;
        if (pointer.tip.0 - cx).abs() + (pointer.tip.1 - cy).abs() > 10 {
            self.line((cx, cy), pointer.tip, em::HOME_COLOUR);
            self.arrow_head((cx, cy), pointer.tip, 6, em::HOME_COLOUR);
        }
        if pointer.on_view {
            self.circle(scene.home.0, scene.home.1, 14, em::HOME_COLOUR);
            shadowed(
                self,
                (scene.home.0 + 8).clamp(0, 300) as usize,
                (scene.home.1 + 4).clamp(0, 148) as usize,
                "You",
                em::HOME_COLOUR,
            );
        } else {
            let label = format!(
                "You {}.{:02} ly",
                pointer.distance / 100,
                pointer.distance % 100
            );
            let width = self.font.width(&label).unwrap_or(0) as i32;
            let (lx, ly) = (
                (pointer.tip.0 - width / 2).clamp(2, 318 - width),
                (pointer.tip.1 + if pointer.tip.1 < cy { 6 } else { -12 }).clamp(2, 148),
            );
            home_label = Some((lx, ly, width));
            shadowed(self, lx as usize, ly as usize, &label, em::HOME_COLOUR);
        }
        // The Sol direction pointer (yellow); no distance text.
        let (sx, sy, _) = scene.sol;
        if (0..320).contains(&sx) && (0..VIEW_HEIGHT as i32).contains(&sy) {
            self.circle(sx, sy, 6, 18);
            self.text(
                sx.saturating_add(4) as usize,
                sy.saturating_sub(4) as usize,
                "Sol",
                18,
            );
        } else if sx != cx || sy != cy {
            // Off view: the line stops at the same inset edge as the line home.
            let (ex, ey) = em::edge_point((sx, sy));
            self.line((cx, cy), (ex, ey), 18);
            self.arrow_head((cx, cy), (ex, ey), 6, 18);
            let width = self.font.width("Sol").unwrap_or(0) as i32;
            let lx = (ex - width / 2).clamp(2, 318 - width);
            let mut ly = (ey + if ey < cy { 6 } else { -12 }).clamp(2, 148);
            // The two lines can run the same way: stack the labels, not overprint them.
            if let Some((hx, hy, hw)) = home_label {
                if (lx - hx).abs() < hw.max(width) + 2 && (ly - hy).abs() < 10 {
                    ly = if hy >= 14 { hy - 10 } else { hy + 10 };
                }
            }
            shadowed(self, lx as usize, ly as usize, "Sol", 18);
        }
        // The colour legend (each name in its colour) and the controls line.
        let mut lx = 6usize;
        for (colour, name) in &scene.legend {
            shadowed(self, lx, 4, name, *colour);
            lx += self.font.width(name).unwrap_or(0) + 8;
        }
        shadowed(self, 6, 14, &scene.hint, 15);
        // The left text block, from the bottom up: the sector, then the route summary.
        let mut block: Vec<(&str, u8)> = vec![(scene.sector.as_str(), 15)];
        if let Some((route, colour)) = &scene.route_summary {
            block.push((route, *colour));
        }
        for (row, (line, colour)) in block.into_iter().enumerate() {
            shadowed(self, 6, 147 - 10 * row, line, colour);
        }
        // What hovering a star says, in a panel at the right of the view (at the left
        // when the star is over there, so the panel never covers it).
        if !scene.hover.is_empty() {
            const PANEL_W: usize = 118;
            const PANEL_TOP: usize = 26;
            const PANEL_BOTTOM: usize = 154;
            let on_left = scene
                .hovered
                .is_some_and(|(x, _, _)| x >= 320 - PANEL_W as i32 - 8);
            let left = if on_left { 4 } else { 320 - PANEL_W - 4 };
            let mut rows: Vec<(String, u8)> = Vec::new();
            for (paragraph, colour) in &scene.hover {
                let mut line = String::new();
                for word in paragraph.split(' ') {
                    let trial = if line.is_empty() {
                        word.to_owned()
                    } else {
                        format!("{line} {word}")
                    };
                    if self
                        .font
                        .width(&trial)
                        .is_ok_and(|width| width > PANEL_W - 6)
                        && !line.is_empty()
                    {
                        rows.push((std::mem::take(&mut line), *colour));
                        line = word.to_owned();
                    } else {
                        line = trial;
                    }
                }
                rows.push((line, *colour));
            }
            let height = (rows.len() * 9 + 6).min(PANEL_BOTTOM - PANEL_TOP);
            self.dim_rect(left, PANEL_TOP, left + PANEL_W, PANEL_TOP + height, 30);
            for (row, (line, colour)) in rows.iter().enumerate() {
                let y = PANEL_TOP + 3 + 9 * row;
                if y + 8 > PANEL_BOTTOM {
                    break;
                }
                shadowed(self, left + 3, y, line, *colour);
            }
        }
        // The console's date, as the original's map shows it.
        self.text(195, 164, &scene.date, 17);
    }

    /// A solid disc of `radius` pixels about (`x`, `y`), clipped to the view.
    fn fill_disc(&mut self, x: i32, y: i32, radius: i32, colour: u8) {
        let value = self.palette[usize::from(colour)];
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius + radius / 2 {
                    let (px, py) = (x + dx, y + dy);
                    if (0..320).contains(&px) && (0..VIEW_HEIGHT as i32).contains(&py) {
                        self.frame[py as usize * 320 + px as usize] = value;
                    }
                }
            }
        }
    }

    /// Darken the view's rectangle `x0..x1` by `y0..y1` to `keep` per cent of its
    /// brightness.
    fn dim_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, keep: u32) {
        for y in y0..y1.min(VIEW_HEIGHT) {
            for x in x0..x1.min(320) {
                let pixel = self.frame[y * 320 + x];
                let scale = |shift: u32| (((pixel >> shift) & 0xff) * keep / 100) << shift;
                self.frame[y * 320 + x] = scale(16) | scale(8) | scale(0);
            }
        }
    }

    /// A small solid arrowhead at `tip`, pointing away from `from`.
    fn arrow_head(&mut self, from: (i32, i32), tip: (i32, i32), size: i32, colour: u8) {
        let (dx, dy) = (f64::from(tip.0 - from.0), f64::from(tip.1 - from.1));
        let length = (dx * dx + dy * dy).sqrt();
        if length < 1.0 {
            return;
        }
        let (ux, uy) = (dx / length, dy / length);
        let size = f64::from(size);
        let back = |along: f64, across: f64| {
            (
                (f64::from(tip.0) - ux * along - uy * across).round() as i32,
                (f64::from(tip.1) - uy * along + ux * across).round() as i32,
            )
        };
        let value = self.palette[usize::from(colour)];
        self.fill_value(
            &[tip, back(size, size * 0.5), back(size, -size * 0.5)],
            value,
        );
    }

    /// The system map (F977): the orbit curves, the bodies' dots and names, the
    /// time point at (10, 148) and the console's date, over the cabin art.
    fn draw_system_map(&mut self, page: &SystemMapScene, viewer: Option<&ModelViewer>) {
        let frame = &page.frame;
        self.frame[..320 * VIEW_HEIGHT]
            .copy_from_slice(&self.cabins[self.cabin_index][..320 * VIEW_HEIGHT]);
        let height = crate::system_map::VIEW_HEIGHT;
        let inside = |x: i32, y: i32| (0..320).contains(&x) && (0..height).contains(&y);
        for orbit in &frame.orbits {
            for pair in orbit.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if (a.0 < -200 && b.0 < -200)
                    || (a.0 > 520 && b.0 > 520)
                    || (a.1 < -200 && b.1 < -200)
                    || (a.1 > 360 && b.1 > 360)
                {
                    continue;
                }
                self.clipped_line(a, b, crate::system_map::GREEN, height);
            }
        }
        for dot in &frame.dots {
            // F1680's far object: the original's sprite for the model's class.
            if let (Some(sprite), Some(viewer)) = (dot.sprite, viewer) {
                if let Some(classes) = viewer.object_model(sprite.model) {
                    if let Ok(drawn) =
                        viewer.object_dot_frame(classes, sprite.screen, sprite.radius, sprite.depth)
                    {
                        if let Some(drawn) = drawn {
                            self.draw_frame(viewer, &drawn);
                        }
                        continue;
                    }
                }
            }
            let radius = dot.radius;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy <= radius * radius + radius / 2
                        && inside(dot.x + dx, dot.y + dy)
                    {
                        self.pixel(dot.x + dx, dot.y + dy, dot.colour);
                    }
                }
            }
        }
        // F978's marker on the picked body: a yellow box about its dot.
        for dot in frame.dots.iter().filter(|dot| dot.marked) {
            let r = dot.radius + 3;
            let (x0, y0, x1, y1) = (dot.x - r, dot.y - r, dot.x + r, dot.y + r);
            for (a, b) in [
                ((x0, y0), (x1, y0)),
                ((x1, y0), (x1, y1)),
                ((x1, y1), (x0, y1)),
                ((x0, y1), (x0, y0)),
            ] {
                self.clipped_line(a, b, crate::system_map::YELLOW, height);
            }
        }
        for label in &frame.labels {
            let colour = if label.marked {
                crate::system_map::YELLOW
            } else {
                crate::system_map::WHITE
            };
            self.shadow_text(label.x, label.y, &label.text, colour);
        }
        for (row, line) in frame.details.iter().rev().enumerate() {
            self.shadow_text(10, 138 - 10 * row as i32, line, crate::system_map::YELLOW);
        }
        self.text(10, 148, &frame.time_point, 15);
        self.text(195, 164, &page.date, 17);
    }

    /// A line cut to the first `height` rows of the view.
    fn clipped_line(&mut self, start: (i32, i32), end: (i32, i32), colour: u8, height: i32) {
        // Liang-Barsky against the view's rectangle.
        let (x0, y0) = (f64::from(start.0), f64::from(start.1));
        let (dx, dy) = (f64::from(end.0 - start.0), f64::from(end.1 - start.1));
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (p, q) in [
            (-dx, x0),
            (dx, 319.0 - x0),
            (-dy, y0),
            (dy, f64::from(height - 1) - y0),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    return;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
        if t0 > t1 {
            return;
        }
        let at = |t: f64| ((x0 + t * dx).round() as i32, (y0 + t * dy).round() as i32);
        self.line(at(t0), at(t1), colour);
    }

    /// F842's page over the cabin: F851's body sprites (each `BlitIndex`ed at the
    /// pen less its record's offsets, the bitmap's own offset added) and the texts.
    fn draw_map_page(&mut self, page: &crate::map_pages::Page) {
        let palette = self.palette;
        if let Some(sky) = &page.background {
            for (pixel, index) in self.frame.iter_mut().zip(sky) {
                *pixel = palette[usize::from(*index)];
            }
        }
        for placed in &page.sprites {
            if let Some(Some(image)) = self.diagram.get(usize::from(placed.bitmap)) {
                image.blit_rgb(&mut self.frame, &palette, placed.x, placed.y);
            }
        }
        for text in &page.texts {
            self.draw_page_text(text);
        }
    }

    /// F1399/F1589 for a page string: the pen starts at (x, y); `0x0d` starts a
    /// new 10-pixel line, `0xfd XX` moves the pen to `2 * XX`, `0xfe XX YY` to
    /// (`2 * XX`, `YY`), `0x01 XX` (a colour) is skipped, and a word that would
    /// pass the wrap width starts a new line (F1398).
    fn draw_page_text(&mut self, text: &crate::map_pages::PageText) {
        let (mut x, mut y) = (text.x, text.y);
        let mut run = String::new();
        let mut index = 0;
        let bytes = &text.bytes;
        let width = |screen: &Self, value: &str| screen.font.width(value).unwrap_or(0) as i32;
        let flush = |screen: &mut Self, run: &mut String, x: &mut i32, y: &mut i32| {
            for (n, word) in run.split_inclusive(' ').enumerate() {
                let word_width = width(screen, word);
                let trimmed = width(screen, word.trim_end());
                // F1398 breaks as soon as the width left drops to zero.
                if text.wrap > 0 && *x > text.x && *x - text.x + trimmed >= text.wrap {
                    *x = text.x;
                    *y += 10;
                }
                let _ = n;
                screen.shadow_text(*x, *y, word, text.colour);
                *x += word_width;
            }
            run.clear();
        };
        while index < bytes.len() {
            let byte = bytes[index];
            match byte {
                0x0d => {
                    flush(self, &mut run, &mut x, &mut y);
                    x = text.x;
                    y += 10;
                    index += 1;
                }
                0xfd | 0x1e => {
                    flush(self, &mut run, &mut x, &mut y);
                    x = i32::from(bytes.get(index + 1).copied().unwrap_or(0)) * 2;
                    index += 2;
                }
                0xfe | 0x1f => {
                    flush(self, &mut run, &mut x, &mut y);
                    x = i32::from(bytes.get(index + 1).copied().unwrap_or(0)) * 2;
                    y = i32::from(bytes.get(index + 2).copied().unwrap_or(0));
                    index += 3;
                }
                0x01 => index += 2,
                byte if byte >= 0x20 => {
                    run.push(char::from(byte));
                    index += 1;
                }
                _ => index += 1,
            }
        }
        flush(self, &mut run, &mut x, &mut y);
    }

    /// The black frame `F1104`/`F915` leave after clearing the buffer.
    pub fn clear_for_game_over(&mut self) {
        self.frame[..320 * VIEW_HEIGHT].fill(0x00_00_00_00);
        self.present();
    }

    /// The tombstone screen (`F917`): a black 320x200 frame with the spinning
    /// model and its engraved FF9C text. `remaining` is F917's loop counter.
    ///
    /// F917 builds a stand-alone display object (`F1683`) at the origin,
    /// `remaining` deep along +z (clamped to `TOMBSTONE_NEAREST`) and calls
    /// `F1675_MatBuildYZT(matrix, 0, remaining >> 1)`. The second argument is 0
    /// and the third is the angle, so the slab yaws about its vertical axis; its
    /// countdown also brings it closer for the first 215,000 tics. The light is
    /// `DATA_007201..3` (0x5a82, 0, 0x5a82). F1683 copies the object matrix
    /// straight into the camera frame, so no camera rotation is applied.
    pub fn draw_game_over(
        &mut self,
        viewer: &mut ModelViewer,
        model: u32,
        remaining: i64,
    ) -> Result<(), String> {
        self.frame[..320 * LOGICAL_HEIGHT].fill(0x00_00_00_00);
        // F917 draws on the full 320x200 logical screen, not the 320x158 flight
        // view: the tombstone's lower text would otherwise be clipped.
        let _full_screen = ffe_core::model_execute::ViewHeightGuard::full_screen();
        let matrix = viewer.matrix_yzt(0, (remaining >> 1) as i32);
        let rows: [[f64; 3]; 3] = std::array::from_fn(|k| {
            std::array::from_fn(|j| f64::from(matrix[3 * k + j]) / 2_147_483_648.0)
        });
        let distance = remaining.max(crate::gameover::TOMBSTONE_NEAREST) as f64;
        let frame = viewer.render_scene(
            model,
            rows,
            [0.0, 0.0, distance],
            crate::gameover::TOMBSTONE_LIGHT,
        )?;
        self.draw_shapes_full(viewer, &frame.shapes);
        self.present();
        Ok(())
    }

    pub fn draw_model_viewer(&mut self, viewer: &ModelViewer) {
        // Dark blue backdrop so black (unlit) faces stay visible as silhouettes.
        self.frame[..320 * VIEW_HEIGHT].fill(0x00_00_10_28);
        let model = viewer.model();
        let frame = viewer.render();
        let status = match &frame {
            Ok(frame) => {
                self.draw_shapes(viewer, &frame.shapes);
                if let Some(error) = &frame.children_error {
                    self.text(3, 23, "PARTS NOT SHOWN YET:", 12);
                    let words: String = error.to_ascii_uppercase().chars().take(60).collect();
                    self.text(3, 34, &words, 12);
                }
                let mut status = format!("{} SHAPES", frame.shapes.len());
                if frame.skipped > 0 {
                    status = format!(
                        "{} SHAPES, {} NOT DRAWN YET",
                        frame.shapes.len(),
                        frame.skipped
                    );
                }
                if frame.texts > 0 {
                    status.push_str(&format!(", {} TEXT", frame.texts));
                }
                status
            }
            Err(error) => {
                // Shown on screen, not logged: this runs every frame.
                self.text(3, 23, "CANNOT SHOW THIS MODEL YET:", 12);
                let words: String = error.to_ascii_uppercase().chars().take(60).collect();
                self.text(3, 34, &words, 12);
                "NOT DRAWN".to_owned()
            }
        };
        self.text(
            3,
            1,
            &format!(
                "MODEL VIEWER {}/{}  MODEL {}  F12 BACK",
                viewer.selected() + 1,
                viewer.len(),
                model.index,
            ),
            18,
        );
        self.text(3, 12, &status, 15);
        // The animation inputs: the test constant, or the original inputs over
        // the viewer's clock (global 0 = tics >> 10, global 1 = gear/door word).
        let (tics, door, rate) = viewer.clock();
        let engines = match viewer.thrust() {
            Some(thrust) => format!("ENGINES {thrust}"),
            None => "ENGINES OFF".to_owned(),
        };
        let inputs = match viewer.animation() {
            Some(value) => format!("TEST INPUT {value}  {engines}"),
            None => format!(
                "TIME {:04X}  DOOR {door:04X}  X{rate}  {engines}",
                ffe_core::model_animation::time_word(tics)
            ),
        };
        self.text(3, 126, &inputs, 15);
        self.text(3, 137, "PGUP/PGDN MODEL  ARROWS TURN  +/- ZOOM", 15);
        self.text(3, 148, "0/5/9 TEST C CLOCK G GEAR T RATE E ENGINES", 18);
        self.present();
    }

    pub fn draw_model_lab(&mut self, lab: &ModelLab) -> Result<(), String> {
        self.rect(0, 0, 320, 158, 0);
        self.text(3, 1, "STATION MODEL LAB - GAME PAUSED", 18);
        let model = lab.model();
        self.text(
            3,
            12,
            &format!(
                "MODEL {} (0x{:02X})  {} VERTICES  SCALE {}",
                model.index, model.index, model.count, model.scale
            ),
            15,
        );
        let points = lab.points()?;
        let normals = lab.normals()?;
        match (lab.view(), model.program()) {
            (LabView::Points, _) => {
                self.text(
                    3,
                    23,
                    if model.program().is_some() {
                        "POINTS - GREEN = COMPUTED"
                    } else {
                        "POINTS ONLY - CACHE V4 ADDS FACE RECORDS"
                    },
                    12,
                );
                for (x, y, computed) in points {
                    let colour = if computed { 17 } else { 15 };
                    self.pixel(x, y, colour);
                    self.pixel(x + 1, y, colour);
                }
            }
            (view, Some(program)) => {
                self.text(
                    3,
                    23,
                    if view == LabView::Normals {
                        "NORMAL FILTER - NO BRANCH EXECUTION"
                    } else {
                        "DECLARED FACES - NOT A MODEL EXECUTION"
                    },
                    12,
                );
                for (index, face) in program.faces.iter().enumerate() {
                    if view == LabView::Face && lab.selected_face().is_none_or(|(i, _)| i != index)
                    {
                        continue;
                    }
                    if view == LabView::Normals
                        && normals
                            .as_ref()
                            .ok_or("Normal view requires cache v5.")?
                            .get(usize::from(face.normal))
                            .ok_or("Face normal outside imported table.")?
                            .culled
                    {
                        continue;
                    }
                    let colour = if face.reflected { 17 } else { 15 };
                    for edge in 0..face.vertices.len() {
                        let (x1, y1, _) = points[usize::from(face.vertices[edge])];
                        let (x2, y2, _) =
                            points[usize::from(face.vertices[(edge + 1) % face.vertices.len()])];
                        self.line((x1, y1), (x2, y2), colour);
                    }
                }
            }
            _ => return Err("Surface view has no validated program.".to_owned()),
        }
        let detail = if lab.view() == LabView::Face {
            let (index, face) = lab.selected_face().ok_or("No selected surface.")?;
            format!(
                "FACE {} AT {:03X} MAT {:04X} N {}",
                index + 1,
                face.offset,
                face.material,
                face.normal
            )
        } else if lab.view() == LabView::Normals {
            let count = normals
                .as_ref()
                .ok_or("Missing normal evaluation.")?
                .iter()
                .filter(|n| !n.culled)
                .count();
            format!(
                "{} NORMALS FRONT  TAB VIEW  INPUT {}",
                count,
                lab.animation()
            )
        } else {
            format!("INPUT {}   TAB VIEW   ,/. FACE", lab.animation())
        };
        self.text(3, 115, "NO FULL COMMANDS / SUBMODELS / SHADING", 12);
        self.text(3, 126, &detail, 17);
        self.text(3, 137, "1/2 MODEL  ARROWS ROTATE  0/5/9 TEST INPUT", 15);
        self.text(3, 148, "SHIFT+F12 RETURN - NO FLIGHT OR SAVE CHANGES", 18);
        self.present();
        Ok(())
    }
}

/// The labels the original's F989/F1108 would draw: each object's name at its
/// screen position, skipping the ones F989's bounds reject (x <= 0,
/// x + 3 >= 316, y <= 0, y - 4 >= 149) and anything behind the eye. F1108
/// writes the name at the object's projected point plus `(+4, -4)`: F989 hands
/// the text writer `x + 3, y - 4`, and F1108 adds the final `+1` to x. The y
/// may be negative near the top edge; F1584 clips those rows.
pub fn body_label_positions(labels: &[BodyLabel], camera: &Camera) -> Vec<(String, i32, i32)> {
    labels
        .iter()
        .filter_map(|label| {
            let (x, y) = camera.project(label.centre, true)?;
            (x > 0 && x + 3 < 0x13c && y > 0 && y - 4 < 0x95)
                .then(|| (label.name.clone(), x + 4, y - 4))
        })
        .collect()
}

/// Original model table slot drawn as the flight station.
const STATION_MODEL: u32 = 74;
/// Flight-space radius the station model is scaled to (the old placeholder size).
const STATION_RADIUS: f64 = 40.0;
/// Station object axes in flight space: model 74's docking port is at its +z end,
/// turned to face the approach from -z.
const STATION_AXES: [[f64; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
/// F866 rotation speed of a generated station with model 74 (F892 case 2,
/// template `DATA_008795`: hand-coded Abraham Lincoln, speed 10).
const STATION_SPEED: i8 = ffe_core::model_animation::ORBITAL_CITY_SPEED;

/// The game clock the flight view shows: the new-game `GameTics` plus 994 tics
/// (20 ms at acceleration 1) per 50 Hz flight step.
pub fn flight_tics(state: &Flight) -> u32 {
    ffe_core::model_animation::START_TICS
        .wrapping_add((state.ticks as u32).wrapping_mul(ffe_core::model_animation::TICS_PER_FRAME))
}

/// F1776 submits model 0x9c in cockpit, turret, external and combat views. The
/// camera clips its geometry; do not suppress the whole shot by muzzle depth.
fn beam_visible(view: ViewMode) -> bool {
    matches!(
        view,
        ViewMode::Front
            | ViewMode::Rear
            | ViewMode::TopTurret
            | ViewMode::BottomTurret
            | ViewMode::External
            | ViewMode::Combat
    )
}

/// The views `enhancement-13`'s fixed laser sight replaces: the first-person
/// cockpit views (front, rear and the two turrets), not the external view.
fn first_person_view(view: ViewMode) -> bool {
    matches!(
        view,
        ViewMode::Front | ViewMode::Rear | ViewMode::TopTurret | ViewMode::BottomTurret
    )
}

/// Model units per flight unit: station 74's extent fills the old 40-unit
/// placeholder (docking and safety distances unchanged), and every other model
/// shares that scale, so ships keep their true size relative to the station.
pub fn model_scale(scene: &ModelViewer) -> Option<f64> {
    scene
        .find(STATION_MODEL)
        .and_then(|m| m.radius())
        .map(|radius| radius / STATION_RADIUS)
}

/// Model units per flight unit when the cache has no station model.
const DEFAULT_MODEL_SCALE: f64 = 9700.0;

/// A console button click: select a view (F6/F7/F9), the turret (F8) or cycle
/// (F1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewButton {
    Select(ViewMode),
    Turret,
    Cycle,
}

/// A model's bounding radius in flight units, at the shared model scale.
pub fn model_radius(scene: &ModelViewer, model: u32) -> Option<f64> {
    Some(scene.find(model)?.radius()? / model_scale(scene)?)
}

/// F1680's set-up for a body in the camera's frame of reference: the distance class
/// against the model's far and dot classes, the precision shift, and the instance's
/// position/operand/key scales. The body is at `centre` (flight units, the camera's
/// space); the object's own scale (`+0x7e`) is 0 for a system body.
fn f1680_body(
    camera: &Camera,
    centre: [f64; 3],
    descriptor: &ffe_core::model_animation::Descriptor,
) -> ffe_core::model_animation::Draw {
    use ffe_core::model_animation::{setup_object, Camera as ObjectCamera, ObjectState};
    let unit = crate::original_flight::UNITS_PER_FLIGHT_UNIT;
    let q31 = |v: f64| (v * 2_147_483_648.0).clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
    let matrix: [i32; 9] = std::array::from_fn(|k| q31(camera.axes[k / 3][k % 3]));
    let object = ObjectState {
        position: centre.map(|v| (v * unit) as i64),
        ..ObjectState::default()
    };
    let camera = ObjectCamera {
        matrix,
        position: camera.position.map(|v| (v * unit) as i64),
    };
    setup_object(&object, &camera, descriptor, [0; 3], &[[0; 3]; 8])
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// One `CABINn.BMP`, exported as a 320x158 PNG, as RGB pixels.
fn decode_cabin(png: &[u8]) -> Result<Vec<u32>, String> {
    let mut decoder = png::Decoder::new(Cursor::new(png));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("Cabin PNG: {e}"))?;
    if reader.info().width != 320
        || reader.info().height != 158
        || reader.output_buffer_size() > 320 * 158 * 4
    {
        return Err("Expected a bounded 320x158 cabin PNG.".to_owned());
    }
    let mut output = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut output)
        .map_err(|e| format!("Cabin pixels: {e}"))?;
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err("Cabin PNG did not expand to RGB/RGBA.".to_owned()),
    };
    Ok(output[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
        .collect())
}

#[cfg(test)]
mod cargo_page_tests {
    use super::{cargo_click, equipment_rows, equipment_scroll_click, CargoClick};

    #[test]
    fn f283_scrolls_a_long_equipment_list() {
        assert_eq!(equipment_rows(16), (8, 8, 0));
        // 21 items: two columns of 11, seven rows showing, four rows to scroll.
        assert_eq!(equipment_rows(21), (7, 11, 4));
        assert_eq!(equipment_scroll_click(24, 115), Some(1));
        assert_eq!(equipment_scroll_click(36, 112), Some(-1));
        assert_eq!(equipment_scroll_click(36, 124), None);
    }

    #[test]
    fn f294_hot_areas() {
        assert_eq!(cargo_click(124, 19, 0), Some(CargoClick::Refuel));
        // Item 0's button is 12..4 pixels left of x 18 on row 35.
        assert_eq!(cargo_click(6, 35, 1), Some(CargoClick::Jettison(0)));
        assert_eq!(cargo_click(14, 43, 1), Some(CargoClick::Jettison(0)));
        assert_eq!(cargo_click(15, 35, 1), None);
        // Row 2 of the first column and the second column's first item.
        assert_eq!(cargo_click(10, 56, 3), Some(CargoClick::Jettison(2)));
        assert_eq!(cargo_click(98, 36, 13), Some(CargoClick::Jettison(12)));
        assert_eq!(cargo_click(98, 36, 12), None);
    }
}

#[cfg(test)]
mod beam_view_tests {
    use super::*;

    #[test]
    fn muzzle_model_is_submitted_in_source_laser_views() {
        for view in [
            ViewMode::Front,
            ViewMode::Rear,
            ViewMode::TopTurret,
            ViewMode::BottomTurret,
            ViewMode::External,
            ViewMode::Combat,
        ] {
            assert!(beam_visible(view), "missing muzzle model in {view:?}");
        }
        assert!(!beam_visible(ViewMode::Missile));
    }
}

#[cfg(test)]
mod f1680_scale_tests {
    use super::*;
    use ffe_core::model_animation::{Descriptor, Draw};

    /// Hope's surface descriptor: vertex scale 22, scale offset 4 (16 mm per
    /// model unit), with far/dot classes well outside these ranges.
    fn planet() -> Descriptor {
        Descriptor {
            scale: 22,
            scale_offset: 4,
            radius: 0x4000,
            far: 0x3f,
            dot: 0x30,
        }
    }

    fn instance_at(centre_mm: f64) -> (i32, i32, [i32; 3]) {
        let unit = crate::original_flight::UNITS_PER_FLIGHT_UNIT;
        let camera = Camera {
            position: [0.0; 3],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        match f1680_body(&camera, [0.0, 0.0, centre_mm / unit], &planet()) {
            Draw::Model { instance, .. } => (
                instance.instance.operand_scale,
                instance.instance.key_scale,
                instance.instance.position,
            ),
            other => panic!("expected a model, got {other:?}"),
        }
    }

    /// F1680's `e = desc[+0x18] + obj[+0x7e] - shift`: while `shift` is at or
    /// below the descriptor's offset the model keeps its scale and the position
    /// absorbs the whole offset; past that the operand scale falls by `e` and the
    /// key scale rises by the same amount.
    #[test]
    fn f1680_lowers_the_scale_and_raises_the_key_past_the_precision_shift() {
        // 100 km: shift 0, e = 4. The flight-unit round trip truncates one mm.
        let (operand, key, position) = instance_at(100_000_000.0);
        assert_eq!((operand, key), (22, 4));
        assert!((position[2] - 6_250_000).abs() <= 1, "{position:?}");

        // 1,000 km: shift 1, e = 3, so the whole offset still applies.
        let (operand, key, position) = instance_at(1_000_000_000.0);
        assert_eq!((operand, key), (22, 4));
        assert!((position[2] - 62_500_000).abs() <= 1, "{position:?}");

        // 10,000 km: shift 5, e = -1, so the operand scale drops a bit and the
        // key scale takes it back.
        let (operand, key, position) = instance_at(10_000_000_000.0);
        assert_eq!((operand, key), (21, 5));
        assert!((position[2] - 312_500_000).abs() <= 1, "{position:?}");

        // 20,000 km: shift 6, e = -2.
        let (operand, key, position) = instance_at(20_000_000_000.0);
        assert_eq!((operand, key), (20, 6));
        assert!((position[2] - 312_500_000).abs() <= 1, "{position:?}");
    }

    /// The pair `(+0x148, +0x130)` exists so F1777's depth key stays the object's
    /// true depth. F1680's two shifts truncate at the shifted position's
    /// granularity, so the key is the plain `P >> 3` within a few units; using the
    /// descriptor offset alone (the old `shrink` path) halves it at 10,000 km.
    #[test]
    fn f1680_keeps_the_depth_key_at_the_true_depth() {
        for distance_mm in [100_000_000.0, 1_000_000_000.0, 10_000_000_000.0] {
            let (_, key, position) = instance_at(distance_mm);
            let shifted = ffe_core::model_depth::key(position, key);
            let plain = (distance_mm / 8.0) as u32;
            assert!(
                shifted.abs_diff(plain) <= 4,
                "at {distance_mm} mm: {shifted} vs {plain}"
            );
        }
        let (_, _, position) = instance_at(10_000_000_000.0);
        let descriptor_only = ffe_core::model_depth::key(position, planet().scale_offset);
        let source = ffe_core::model_depth::key(position, 5);
        assert_eq!(source, descriptor_only * 2);
    }
}

#[cfg(test)]
mod f1680_transition_tests {
    use super::*;
    use ffe_core::model_animation::{Descriptor, Draw};

    /// A ship-sized descriptor: vertex scale 8, dot class 10, far class 19 and a
    /// 16 m cull radius. The classes are the distance class `max(msb - 0xd, 0)`, so
    /// class 10 starts at 2^23 mm (8.39 km) and class 19 at 2^32 mm (4,295 km).
    fn ship() -> Descriptor {
        Descriptor {
            scale: 8,
            scale_offset: 0,
            radius: 0x40,
            far: 19,
            dot: 10,
        }
    }

    fn at(centre_mm: [f64; 3]) -> Draw {
        let unit = crate::original_flight::UNITS_PER_FLIGHT_UNIT;
        let camera = Camera {
            position: [0.0; 3],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        f1680_body(&camera, centre_mm.map(|v| v / unit), &ship())
    }

    /// F1680's distance class crosses the descriptor's classes exactly where the
    /// original does: the model runs below the dot class, the body is an F1700 dot
    /// from it (F1680's `+0x28`) and nothing is drawn from the far class.
    #[test]
    fn f1680_switches_to_a_dot_at_its_dot_class_and_stops_at_its_far_class() {
        // 4.19 km (2^22 mm): class 9, still the model.
        assert!(matches!(
            at([0.0, 0.0, 4.194_304e6]),
            Draw::Model { distance: 9, .. }
        ));
        // 8.39 km (2^23 mm): class 10, the first dot.
        assert!(matches!(
            at([0.0, 0.0, 8.388_608e6]),
            Draw::Dot { distance: 10, .. }
        ));
        // 4,295 km (2^32 mm): class 19, past the far class.
        assert!(matches!(
            at([0.0, 0.0, 4.294_967_296e9]),
            Draw::Far { distance: 19, .. }
        ));
    }

    /// F1686's cull is part of the same decision and runs before the dot test: a body
    /// whose bounding sphere is wholly outside the view is dropped, so a ship at the
    /// screen edge never runs its model.
    #[test]
    fn f1686_culls_a_body_whose_sphere_is_outside_the_view() {
        // 1 km ahead but 100 km to the side: outside the 45-degree planes.
        assert!(matches!(at([1.0e8, 0.0, 1.0e6]), Draw::Culled { .. }));
        // Behind the eye: `z + radius < 0x40`.
        assert!(matches!(at([0.0, 0.0, -1.0e6]), Draw::Culled { .. }));
    }
}
