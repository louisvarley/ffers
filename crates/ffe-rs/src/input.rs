//! Flight input: the held-key state, the two control presets and how each
//! action is routed to them.
//!
//! The presets are the port's own choice, exposed on the Escape pages' `Port
//! Options` page and on F3 ([`crate::prefs`]). They are deliberately disjoint:
//!
//! | action | Modern | Classic (the original's scheme) |
//! |---|---|---|
//! | raise / lower thrust | W / S | Enter / Shift |
//! | pitch | Up / Down | A / Z |
//! | yaw | Left / Right | `.` / `,` |
//! | roll (original flight) | E / Q | `]` / `[` |
//! | steer by mouse | - | hold the right button and move |
//!
//! `docs/CONTROLS.md` has the full table, including the actions both presets
//! share (fire, missiles, targets, autopilot, views and so on). The routing is
//! pure so the presets can be tested without a window.
use minifb::Key;

/// The character a key types in the save menu's name box: letters (capitals
/// with Shift), digits, space, `-` and (with Shift) `_`. `None` for every other
/// key, so nothing outside [`crate::saves::save_name_error`]'s alphabet can be typed.
pub fn save_name_char(key: Key, shift: bool) -> Option<char> {
    let letters = "abcdefghijklmnopqrstuvwxyz";
    let letter_keys = [
        Key::A,
        Key::B,
        Key::C,
        Key::D,
        Key::E,
        Key::F,
        Key::G,
        Key::H,
        Key::I,
        Key::J,
        Key::K,
        Key::L,
        Key::M,
        Key::N,
        Key::O,
        Key::P,
        Key::Q,
        Key::R,
        Key::S,
        Key::T,
        Key::U,
        Key::V,
        Key::W,
        Key::X,
        Key::Y,
        Key::Z,
    ];
    if let Some(index) = letter_keys.iter().position(|candidate| *candidate == key) {
        let letter = letters.as_bytes()[index] as char;
        return Some(if shift {
            letter.to_ascii_uppercase()
        } else {
            letter
        });
    }
    let digit_keys = [
        Key::Key0,
        Key::Key1,
        Key::Key2,
        Key::Key3,
        Key::Key4,
        Key::Key5,
        Key::Key6,
        Key::Key7,
        Key::Key8,
        Key::Key9,
    ];
    let pad_keys = [
        Key::NumPad0,
        Key::NumPad1,
        Key::NumPad2,
        Key::NumPad3,
        Key::NumPad4,
        Key::NumPad5,
        Key::NumPad6,
        Key::NumPad7,
        Key::NumPad8,
        Key::NumPad9,
    ];
    if let Some(digit) = digit_keys.iter().position(|candidate| *candidate == key) {
        return (!shift).then(|| char::from(b'0' + digit as u8));
    }
    if let Some(digit) = pad_keys.iter().position(|candidate| *candidate == key) {
        return Some(char::from(b'0' + digit as u8));
    }
    match key {
        Key::Space => Some(' '),
        Key::Minus => Some(if shift { '_' } else { '-' }),
        Key::NumPadMinus => Some('-'),
        _ => None,
    }
}

use crate::camera::ViewMode;
use crate::flight::Controls as FlightControls;
use crate::original_flight::Keys as OriginalKeys;

/// The original's view keys (`DATA_007688` actions `0xf1`..`0xfa`). F1 cycles;
/// F6-F10 select a view directly. F11 is the combat computer, not a view the
/// port has, and F12 is the escape capsule, so neither is a debug key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewCommand {
    Cycle,
    Front,
    Rear,
    Turret,
    External,
    Missile,
    /// F11 (action 0xfb): the combat computer view.
    Combat,
}

/// Map one flight key to its source view action. The caller applies the
/// equipment/view gates for the missile view.
pub fn view_command(key: Key) -> Option<ViewCommand> {
    match key {
        Key::F1 => Some(ViewCommand::Cycle),
        Key::F6 => Some(ViewCommand::Front),
        Key::F7 => Some(ViewCommand::Rear),
        Key::F8 => Some(ViewCommand::Turret),
        Key::F9 => Some(ViewCommand::External),
        Key::F10 => Some(ViewCommand::Missile),
        Key::F11 => Some(ViewCommand::Combat),
        _ => None,
    }
}

/// The source galaxy-map key. F3 is F1054's map action (`docs/CONTROLS.md`);
/// the port also accepts G as its long-standing shortcut. The control preset is
/// chosen only on `Escape` -> `Port Options`, never from a flight key.
pub fn galaxy_map_key(key: Key) -> bool {
    matches!(key, Key::F3 | Key::G)
}

/// A named flight action for the `bind.<action>=<physical key>` lines of
/// `saves/config.cfg`. The vocabulary covers the documented flight controls in
/// `docs/CONTROLS.md`.
///
/// Naming the action rather than one preset's key lets the same entry follow the
/// active preset: [`Action::keys`] lists the game key the action uses in both
/// presets, so `bind.pitch_up=NumPad8` pitches up with Modern's Up and Classic's
/// A. The two presets' steering and thrust sets are deliberately disjoint, so
/// delivering both keys cannot double an axis. Entries the parser does not
/// recognise are ignored, exactly like an unknown key name.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Action {
    ThrustUp,
    ThrustDown,
    PitchUp,
    PitchDown,
    YawLeft,
    YawRight,
    RollLeft,
    RollRight,
    Brake,
    Fire,
    Missile,
    Ecm,
    Chaff,
    Target,
    Autopilot,
    StopAutopilot,
    Docking,
    Launch,
    SystemMap,
    Navigation,
    GalaxyMap,
    Hyperspace,
    Pause,
    Status,
    ViewCycle,
    ViewFront,
    ViewRear,
    ViewTurret,
    ViewExternal,
    ViewMissile,
    CombatView,
    EscapeCapsule,
    MapCentre,
}

impl Action {
    /// Every accepted action name, in documentation order. Names are lowercase
    /// `snake_case` so they cannot collide with minifb's key names (`A`, `Up`,
    /// `Key0`, ...).
    pub const NAMES: [(&'static str, Action); 33] = [
        ("thrust_up", Action::ThrustUp),
        ("thrust_down", Action::ThrustDown),
        ("pitch_up", Action::PitchUp),
        ("pitch_down", Action::PitchDown),
        ("yaw_left", Action::YawLeft),
        ("yaw_right", Action::YawRight),
        ("roll_left", Action::RollLeft),
        ("roll_right", Action::RollRight),
        ("brake", Action::Brake),
        ("fire", Action::Fire),
        ("missile", Action::Missile),
        ("ecm", Action::Ecm),
        ("chaff", Action::Chaff),
        ("target", Action::Target),
        ("autopilot", Action::Autopilot),
        ("stop_autopilot", Action::StopAutopilot),
        ("docking", Action::Docking),
        ("launch", Action::Launch),
        ("system_map", Action::SystemMap),
        ("navigation", Action::Navigation),
        ("galaxy_map", Action::GalaxyMap),
        ("hyperspace", Action::Hyperspace),
        ("pause", Action::Pause),
        ("status", Action::Status),
        ("view_cycle", Action::ViewCycle),
        ("view_front", Action::ViewFront),
        ("view_rear", Action::ViewRear),
        ("view_turret", Action::ViewTurret),
        ("view_external", Action::ViewExternal),
        ("view_missile", Action::ViewMissile),
        ("combat_view", Action::CombatView),
        ("escape_capsule", Action::EscapeCapsule),
        ("map_centre", Action::MapCentre),
    ];

    /// The action an entry names, or `None` for an unknown name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, action)| *action)
    }

    /// The name written in `config.cfg`. Every variant is in [`Action::NAMES`].
    pub fn name(self) -> &'static str {
        Self::NAMES
            .iter()
            .find(|(_, action)| *action == self)
            .map(|(name, _)| *name)
            .unwrap_or("")
    }

    /// The game keys this action uses in either preset (`docs/CONTROLS.md`).
    /// Most actions share one key; the thrust and steering actions list the
    /// Modern and Classic keys, which the presets read disjointly.
    pub fn keys(self) -> &'static [Key] {
        use Key::*;
        match self {
            Self::ThrustUp => &[W, Enter],
            Self::ThrustDown => &[S, RightShift],
            Self::PitchUp => &[Up, A],
            Self::PitchDown => &[Down, Z],
            Self::YawLeft => &[Left, Comma],
            Self::YawRight => &[Right, Period],
            Self::RollLeft => &[Q, LeftBracket],
            Self::RollRight => &[E, RightBracket],
            Self::Brake => &[B],
            Self::Fire => &[Space],
            Self::Missile => &[M],
            // `E` is also Modern's roll-right key; the action delivers the key
            // the game already reads for the ECM, exactly as pressing it does.
            Self::Ecm => &[E, Semicolon, Insert],
            Self::Chaff => &[C],
            Self::Target => &[T],
            Self::Autopilot => &[K],
            Self::StopAutopilot => &[Backspace],
            Self::Docking => &[D],
            // Tab launches from a pad or the surface and opens the system map in
            // flight, as `docs/CONTROLS.md` records.
            Self::Launch => &[L, Tab],
            Self::SystemMap => &[Tab],
            Self::Navigation => &[N],
            Self::GalaxyMap => &[F3, G],
            Self::Hyperspace => &[H],
            Self::Pause => &[P],
            Self::Status => &[F2],
            Self::ViewCycle => &[F1],
            Self::ViewFront => &[F6],
            Self::ViewRear => &[F7],
            Self::ViewTurret => &[F8],
            Self::ViewExternal => &[F9],
            Self::ViewMissile => &[F10],
            Self::CombatView => &[F11],
            Self::EscapeCapsule => &[X, F12],
            Self::MapCentre => &[C],
        }
    }
}

/// The port's developer-only controls. None are bound in a player build: the
/// caller passes `--dev` or `FFE_DEV=1` to enable them, and every one uses a
/// Ctrl chord (or O) so no source key is taken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DevCommand {
    Screenshot,
    ModelViewer,
    ModelLab,
    PrototypeFlight,
}

pub fn dev_command(enabled: bool, key: Key, ctrl: bool, shift: bool) -> Option<DevCommand> {
    if !enabled {
        return None;
    }
    match key {
        Key::F11 if ctrl && !shift => Some(DevCommand::Screenshot),
        Key::F12 if ctrl && shift => Some(DevCommand::ModelLab),
        Key::F12 if ctrl => Some(DevCommand::ModelViewer),
        Key::O if !ctrl && !shift => Some(DevCommand::PrototypeFlight),
        _ => None,
    }
}

#[derive(Default)]
pub struct HeldControls {
    held: Vec<Key>,
    blocked: Vec<Key>,
}

impl HeldControls {
    pub fn update(&mut self, down: &[Key], pressed: &[Key], enabled: bool) {
        self.held.retain(|key| down.contains(key));
        self.blocked.retain(|key| down.contains(key));
        if !enabled {
            self.disarm(down);
            return;
        }
        for key in pressed {
            if down.contains(key) && !self.blocked.contains(key) && !self.held.contains(key) {
                self.held.push(*key);
            }
        }
    }

    pub fn disarm(&mut self, down: &[Key]) {
        self.held.clear();
        self.blocked = down.to_vec();
    }

    pub fn contains(&self, key: &Key) -> bool {
        self.held.contains(key)
    }

    /// The keys latched as held this frame, for the preset routing.
    pub fn keys(&self) -> &[Key] {
        &self.held
    }
}

/// Which control preset is active. `Prefs::modern` stores the choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    Modern,
    Classic,
}

impl Preset {
    pub fn from_modern(modern: bool) -> Self {
        if modern {
            Self::Modern
        } else {
            Self::Classic
        }
    }

    pub fn modern(self) -> bool {
        self == Self::Modern
    }
}

/// The part of the view that decides who owns the steering input. Turrets aim
/// with the steering keys; the external camera owns the arrow keys.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum View {
    #[default]
    Ship,
    Turret,
    External,
}

impl View {
    pub fn of(mode: ViewMode) -> Self {
        if mode.is_turret() {
            Self::Turret
        } else if mode == ViewMode::External {
            Self::External
        } else {
            Self::Ship
        }
    }
}

/// Mouse movement since the previous frame, in logical 320x200 pixels, with
/// `dx` right and `dy` down. `held` is the right button: the original flies only
/// while it is down.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MouseSteer {
    pub held: bool,
    pub dx: i32,
    pub dy: i32,
}

/// Pixels of movement for a full-prototype-flight deflection.
const MOUSE_FULL_DEFLECTION: f64 = 50.0;
/// The original F1038 scale: one mouse count is one pixel here, and F1038
/// shifts each count left six.
const MOUSE_UNITS_PER_PIXEL: i32 = 64;

/// A relative mouse counter in the original's `DATA_009188` form: x in the low
/// word and y in the high word. It only moves while the right button is held,
/// and is cleared as soon as it is released so a later press starts at zero.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MouseAccumulator {
    x: i32,
    y: i32,
}

impl MouseAccumulator {
    pub fn update(&mut self, mouse: MouseSteer) {
        if mouse.held {
            self.x = self.x.wrapping_add(mouse.dx);
            self.y = self.y.wrapping_add(mouse.dy);
        } else {
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        self.x = 0;
        self.y = 0;
    }

    /// `DATA_009188`: x in the low word, y in the high word.
    pub fn packed(self) -> u32 {
        (self.x as u16 as u32) | ((self.y as u16 as u32) << 16)
    }
}

/// Whether the original's flight module (module 3) would be the active one, so
/// F1037 reads the set-speed and steering keys (`DATA_008630 < 0`). The F2
/// status screen (module 0x0e) and the map (module 2: the galaxy page, and the
/// port's system map that stands in for its system page) replace it. The
/// navigation computer (`n`) and the F5 engines page are drawn by the flight
/// module itself and leave it active.
pub fn flight_module_active(status_page: bool, map: crate::navigation::View) -> bool {
    use crate::navigation::View as Map;
    !status_page && matches!(map, Map::Closed | Map::Places)
}

fn down(held: &[Key], key: Key) -> bool {
    held.contains(&key)
}

fn axis(held: &[Key], positive: Key, negative: Key) -> f64 {
    f64::from(u8::from(down(held, positive))) - f64::from(u8::from(down(held, negative)))
}

/// The original's slow-down key: F1037 reads `DATA_007694`, the key state of
/// scancode 0x36, right Shift. Left Shift (0x2a, `DATA_007693`) only selects
/// the shifted half of the action table.
fn slow_down(held: &[Key]) -> bool {
    down(held, Key::RightShift)
}

/// A held-key axis as -1, 0 or 1, and a mouse deflection in the same range.
fn axis_with_mouse(held: &[Key], positive: Key, negative: Key, delta: i32) -> f64 {
    (axis(held, positive, negative) + (f64::from(delta) / MOUSE_FULL_DEFLECTION).clamp(-1.0, 1.0))
        .clamp(-1.0, 1.0)
}

fn flip(value: f64, reverse: bool) -> f64 {
    if reverse {
        -value
    } else {
        value
    }
}

/// The prototype flight's stick for this frame. `reverse` is the original's
/// "Reverse left-right controls" and "Reverse up-down controls"
/// (`prefs.others[3]` and `[4]`).
pub fn flight_controls(
    preset: Preset,
    view: View,
    held: &[Key],
    mouse: MouseSteer,
    reverse: [bool; 2],
) -> FlightControls {
    let steering = view == View::Ship || view == View::External;
    let mouse = if preset == Preset::Classic && steering && mouse.held {
        mouse
    } else {
        MouseSteer::default()
    };
    let (thrust, yaw, pitch) = match preset {
        Preset::Modern => (
            axis(held, Key::W, Key::S),
            if steering {
                axis(held, Key::Right, Key::Left)
            } else {
                0.0
            },
            if steering {
                axis(held, Key::Up, Key::Down)
            } else {
                0.0
            },
        ),
        Preset::Classic => (
            f64::from(u8::from(down(held, Key::Enter))) - f64::from(u8::from(slow_down(held))),
            if steering {
                axis_with_mouse(held, Key::Period, Key::Comma, mouse.dx)
            } else {
                0.0
            },
            if steering {
                axis_with_mouse(held, Key::A, Key::Z, -mouse.dy)
            } else {
                0.0
            },
        ),
    };
    FlightControls {
        thrust,
        yaw: flip(yaw, reverse[0]),
        pitch: flip(pitch, reverse[1]),
        // The port's brake (the original has none); X is F1054's escape capsule.
        brake: down(held, Key::B),
    }
}

/// The original flight model's keys for this frame, including the mouse word
/// while the Classic scheme holds the right button.
pub fn original_keys(
    preset: Preset,
    view: View,
    held: &[Key],
    mouse: MouseAccumulator,
    mouse_held: bool,
    reverse: [bool; 2],
) -> OriginalKeys {
    let scancodes = held.iter().filter_map(|key| scancode(*key)).collect();
    let mut keys = match preset {
        Preset::Modern => OriginalKeys {
            faster: down(held, Key::W),
            slower: down(held, Key::S),
            pitch_down: down(held, Key::Up),
            pitch_up: down(held, Key::Down),
            left: down(held, Key::Left),
            right: down(held, Key::Right),
            roll_left: down(held, Key::Q),
            roll_right: down(held, Key::E),
            scancodes,
            ..OriginalKeys::default()
        },
        Preset::Classic => OriginalKeys {
            faster: down(held, Key::Enter),
            slower: slow_down(held),
            pitch_down: down(held, Key::Z),
            pitch_up: down(held, Key::A),
            left: down(held, Key::Comma),
            right: down(held, Key::Period),
            roll_left: down(held, Key::LeftBracket),
            roll_right: down(held, Key::RightBracket),
            mouse: mouse.packed(),
            mouse_enabled: mouse_held,
            mouse_flip: reverse,
            scancodes,
        },
    };
    if reverse[0] {
        std::mem::swap(&mut keys.left, &mut keys.right);
    }
    if reverse[1] {
        std::mem::swap(&mut keys.pitch_down, &mut keys.pitch_up);
    }
    // In the external view the arrows are the camera's, so Modern does not also
    // steer the ship with them. Classic keeps its own steering keys there.
    if preset == Preset::Modern && view == View::External {
        keys.left = false;
        keys.right = false;
        keys.pitch_down = false;
        keys.pitch_up = false;
        keys.roll_left = false;
        keys.roll_right = false;
    }
    keys
}

/// The original's scancode for a key: the index JJFFE's platform layer gives
/// it (`pKeybConvTable` in `win32/w32input.cpp`, the DOS set-1 code, with the
/// extended keys at their DirectInput codes). The flight code's key
/// accumulators ([`ffe_core::keyboard`]) track keys by it. Keys the original's
/// table leaves out (Pause, F13-F15) have none.
pub fn scancode(key: Key) -> Option<u8> {
    use Key::*;
    Some(match key {
        Escape => 0x01,
        Key1 => 0x02,
        Key2 => 0x03,
        Key3 => 0x04,
        Key4 => 0x05,
        Key5 => 0x06,
        Key6 => 0x07,
        Key7 => 0x08,
        Key8 => 0x09,
        Key9 => 0x0a,
        Key0 => 0x0b,
        Minus => 0x0c,
        Equal => 0x0d,
        Backspace => 0x0e,
        Tab => 0x0f,
        Q => 0x10,
        W => 0x11,
        E => 0x12,
        R => 0x13,
        T => 0x14,
        Y => 0x15,
        U => 0x16,
        I => 0x17,
        O => 0x18,
        P => 0x19,
        LeftBracket => 0x1a,
        RightBracket => 0x1b,
        Enter => 0x1c,
        LeftCtrl => 0x1d,
        A => 0x1e,
        S => 0x1f,
        D => 0x20,
        F => 0x21,
        G => 0x22,
        H => 0x23,
        J => 0x24,
        K => 0x25,
        L => 0x26,
        Semicolon => 0x27,
        Apostrophe => 0x28,
        Backquote => 0x29,
        LeftShift => 0x2a,
        Backslash => 0x2b,
        Z => 0x2c,
        X => 0x2d,
        C => 0x2e,
        V => 0x2f,
        B => 0x30,
        N => 0x31,
        M => 0x32,
        Comma => 0x33,
        Period => 0x34,
        Slash => 0x35,
        RightShift => 0x36,
        NumPadAsterisk => 0x37,
        LeftAlt => 0x38,
        Space => 0x39,
        CapsLock => 0x3a,
        F1 => 0x3b,
        F2 => 0x3c,
        F3 => 0x3d,
        F4 => 0x3e,
        F5 => 0x3f,
        F6 => 0x40,
        F7 => 0x41,
        F8 => 0x42,
        F9 => 0x43,
        F10 => 0x44,
        NumLock => 0x45,
        ScrollLock => 0x46,
        NumPad7 => 0x47,
        NumPad8 => 0x48,
        NumPad9 => 0x49,
        NumPadMinus => 0x4a,
        NumPad4 => 0x4b,
        NumPad5 => 0x4c,
        NumPad6 => 0x4d,
        NumPadPlus => 0x4e,
        NumPad1 => 0x4f,
        NumPad2 => 0x50,
        NumPad3 => 0x51,
        NumPad0 => 0x52,
        NumPadDot => 0x53,
        F11 => 0x57,
        F12 => 0x58,
        NumPadEnter => 0x9c,
        RightCtrl => 0x9d,
        NumPadSlash => 0xb5,
        RightAlt => 0xb8,
        Home => 0xc7,
        Up => 0xc8,
        PageUp => 0xc9,
        Left => 0xcb,
        Right => 0xcd,
        End => 0xcf,
        Down => 0xd0,
        PageDown => 0xd1,
        Insert => 0xd2,
        Delete => 0xd3,
        LeftSuper => 0xdb,
        RightSuper => 0xdc,
        Menu => 0xdd,
        _ => return None,
    })
}

/// The prototype turret's steering keys, in `(pitch up, pitch down, yaw left,
/// yaw right)` order. The original model routes them through
/// [`original_keys`]; this is for the synthetic turret view.
pub fn turret_keys(preset: Preset) -> [Key; 4] {
    match preset {
        Preset::Modern => [Key::Up, Key::Down, Key::Left, Key::Right],
        Preset::Classic => [Key::A, Key::Z, Key::Comma, Key::Period],
    }
}

/// The turret's mouse aim for one frame, as F1038's counts (left six).
pub fn turret_mouse(preset: Preset, mouse: MouseSteer) -> (i32, i32) {
    if preset != Preset::Classic || !mouse.held {
        return (0, 0);
    }
    (
        -mouse.dy * MOUSE_UNITS_PER_PIXEL,
        -mouse.dx * MOUSE_UNITS_PER_PIXEL,
    )
}

#[cfg(test)]
mod tests {
    use super::save_name_char;

    #[test]
    fn the_save_name_box_types_only_the_allowed_characters() {
        assert_eq!(save_name_char(Key::A, false), Some('a'));
        assert_eq!(save_name_char(Key::Z, true), Some('Z'));
        assert_eq!(save_name_char(Key::Key7, false), Some('7'));
        assert_eq!(save_name_char(Key::Key7, true), None);
        assert_eq!(save_name_char(Key::NumPad3, false), Some('3'));
        assert_eq!(save_name_char(Key::Space, false), Some(' '));
        assert_eq!(save_name_char(Key::Minus, false), Some('-'));
        assert_eq!(save_name_char(Key::Minus, true), Some('_'));
        for key in [
            Key::Period,
            Key::Slash,
            Key::Backslash,
            Key::Enter,
            Key::F5,
            Key::Tab,
        ] {
            assert_eq!(save_name_char(key, false), None, "{key:?}");
            assert_eq!(save_name_char(key, true), None, "{key:?}");
        }
    }

    use super::*;
    use crate::prefs::Prefs;

    #[test]
    fn the_flight_keys_run_only_while_the_flight_module_shows() {
        use crate::navigation::View as Map;
        assert!(flight_module_active(false, Map::Closed));
        // The navigation computer is part of the flight module.
        assert!(flight_module_active(false, Map::Places));
        assert!(!flight_module_active(false, Map::Galaxy));
        assert!(!flight_module_active(false, Map::System));
        assert!(!flight_module_active(true, Map::Closed));
    }

    const MODERN: [Key; 4] = [Key::W, Key::Left, Key::Up, Key::E];
    const CLASSIC: [Key; 9] = [
        Key::Enter,
        Key::LeftShift,
        Key::RightShift,
        Key::Comma,
        Key::A,
        Key::LeftBracket,
        Key::Z,
        Key::Period,
        Key::RightBracket,
    ];

    fn controls(preset: Preset, held: &[Key]) -> FlightControls {
        flight_controls(preset, View::Ship, held, MouseSteer::default(), [false; 2])
    }

    #[test]
    fn intro_space_must_be_released_before_a_new_press_can_fire() {
        let mut held = HeldControls::default();
        held.disarm(&[Key::Space]);
        held.update(&[Key::Space], &[Key::Space], true);
        assert!(!held.contains(&Key::Space));
        held.update(&[], &[], true);
        held.update(&[Key::Space], &[Key::Space], true);
        assert!(held.contains(&Key::Space));
        held.update(&[Key::Space], &[], true);
        assert!(held.contains(&Key::Space));
    }

    #[test]
    fn held_control_requires_a_real_press_and_clears_on_release() {
        let mut input = HeldControls::default();
        input.update(&[Key::W], &[], true);
        assert!(!input.contains(&Key::W));
        input.update(&[Key::W], &[Key::W], true);
        assert!(input.contains(&Key::W));
        input.update(&[Key::W], &[], true);
        assert!(input.contains(&Key::W));
        input.update(&[], &[], true);
        assert!(!input.contains(&Key::W));
    }

    #[test]
    fn disabled_controls_stay_blocked_until_released_even_if_reported_as_pressed() {
        let mut input = HeldControls::default();
        input.update(&[Key::W], &[Key::W], true);
        input.update(&[Key::W, Key::Right], &[Key::Right], false);
        input.update(&[Key::W, Key::Right], &[Key::W, Key::Right], true);
        assert!(!input.contains(&Key::W));
        assert!(!input.contains(&Key::Right));
        input.update(&[Key::Right], &[], true);
        input.update(&[Key::W, Key::Right], &[Key::W], true);
        assert!(input.contains(&Key::W));
        assert!(!input.contains(&Key::Right));
    }

    #[test]
    fn mode_transition_disarms_all_current_keys_without_blocking_new_keys() {
        let mut input = HeldControls::default();
        input.update(&[Key::W], &[Key::W], true);
        input.disarm(&[Key::W]);
        input.update(&[Key::W, Key::S], &[Key::W, Key::S], true);
        assert!(!input.contains(&Key::W));
        assert!(input.contains(&Key::S));
        input.update(&[], &[Key::W], true);
        assert!(!input.contains(&Key::W));
        input.update(&[Key::W], &[Key::W], true);
        assert!(input.contains(&Key::W));
    }

    #[test]
    fn modern_thrust_and_steering_are_w_s_arrows_and_q_e() {
        assert_eq!(controls(Preset::Modern, &[Key::W]).thrust, 1.0);
        assert_eq!(controls(Preset::Modern, &[Key::S]).thrust, -1.0);
        assert_eq!(controls(Preset::Modern, &[Key::Right]).yaw, 1.0);
        assert_eq!(controls(Preset::Modern, &[Key::Left]).yaw, -1.0);
        assert_eq!(controls(Preset::Modern, &[Key::Up]).pitch, 1.0);
        assert_eq!(controls(Preset::Modern, &[Key::Down]).pitch, -1.0);
        let keys = original_keys(
            Preset::Modern,
            View::Ship,
            &[Key::W, Key::Right, Key::Down, Key::E],
            MouseAccumulator::default(),
            false,
            [false; 2],
        );
        assert!(keys.faster && keys.right && keys.pitch_up && keys.roll_right);
        assert!(!keys.slower && !keys.left && !keys.pitch_down && !keys.roll_left);
        // The external view's arrows are the camera's in Modern.
        let external = original_keys(
            Preset::Modern,
            View::External,
            &[Key::Right, Key::Up],
            MouseAccumulator::default(),
            false,
            [false; 2],
        );
        assert!(!external.left && !external.right);
        assert!(!external.pitch_down && !external.pitch_up);
        // Classic still flies from its own keys in the external view.
        let classic = original_keys(
            Preset::Classic,
            View::External,
            &[Key::Period, Key::A],
            MouseAccumulator::default(),
            false,
            [false; 2],
        );
        assert!(classic.right && classic.pitch_up);
    }

    #[test]
    fn classic_enter_shift_and_the_original_keys_do_not_leak_into_modern() {
        let thrust = controls(Preset::Classic, &[Key::Enter]);
        assert_eq!(thrust.thrust, 1.0);
        // Right Shift lowers the set speed; left Shift does not (F1037 reads
        // only scancode 0x36).
        assert_eq!(controls(Preset::Classic, &[Key::RightShift]).thrust, -1.0);
        assert_eq!(controls(Preset::Classic, &[Key::LeftShift]).thrust, 0.0);
        let held = [Key::LeftShift];
        let keys = original_keys(
            Preset::Classic,
            View::Ship,
            &held,
            MouseAccumulator::default(),
            false,
            [false; 2],
        );
        assert!(!keys.slower);
        assert!(keys.scancodes.contains(0x2a));
        for key in CLASSIC {
            let classic = controls(Preset::Classic, &[key]);
            let modern = controls(Preset::Modern, &[key]);
            assert_eq!(
                modern,
                FlightControls::default(),
                "{key:?} must do nothing in Modern"
            );
            if key == Key::Enter || key == Key::RightShift {
                assert_ne!(classic.thrust, 0.0, "{key:?} thrusts in Classic");
            }
        }
        for key in MODERN {
            assert_eq!(
                controls(Preset::Classic, &[key]),
                FlightControls::default(),
                "{key:?} must do nothing in Classic"
            );
        }
        // Modern's original-flight keys do not include any of the Classic set.
        let keys = original_keys(
            Preset::Modern,
            View::Ship,
            &CLASSIC,
            MouseAccumulator::default(),
            false,
            [false; 2],
        );
        assert!(!keys.faster && !keys.slower && !keys.left && !keys.right);
        assert!(!keys.pitch_down && !keys.pitch_up);
        assert!(!keys.roll_left && !keys.roll_right);
    }

    #[test]
    fn the_right_mouse_button_steers_only_in_classic() {
        let mouse = MouseSteer {
            held: true,
            dx: 25,
            dy: -25,
        };
        let classic = flight_controls(Preset::Classic, View::Ship, &[], mouse, [false; 2]);
        assert!((classic.yaw - 0.5).abs() < 1e-9, "{classic:?}");
        assert!((classic.pitch - 0.5).abs() < 1e-9, "{classic:?}");
        let modern = flight_controls(Preset::Modern, View::Ship, &[], mouse, [false; 2]);
        assert_eq!(modern, FlightControls::default());
        // Released, the button does nothing in Classic either.
        let released = MouseSteer {
            held: false,
            ..mouse
        };
        assert_eq!(
            flight_controls(Preset::Classic, View::Ship, &[], released, [false; 2]),
            FlightControls::default()
        );
        // It steers the ship only: the turret has its own aim.
        assert_eq!(
            flight_controls(Preset::Classic, View::Turret, &[], mouse, [false; 2]),
            FlightControls::default()
        );
        let (pitch, yaw) = turret_mouse(Preset::Classic, mouse);
        assert_eq!((pitch, yaw), (25 * 64, -25 * 64));
        assert_eq!(turret_mouse(Preset::Modern, mouse), (0, 0));
    }

    #[test]
    fn reverse_options_flip_the_keys_and_the_mouse() {
        let mouse = MouseSteer {
            held: true,
            dx: 50,
            dy: 50,
        };
        let normal = flight_controls(Preset::Classic, View::Ship, &[], mouse, [false; 2]);
        let reversed = flight_controls(Preset::Classic, View::Ship, &[], mouse, [true, true]);
        assert_eq!((normal.yaw, normal.pitch), (1.0, -1.0));
        assert_eq!((reversed.yaw, reversed.pitch), (-1.0, 1.0));
        let keys = original_keys(
            Preset::Classic,
            View::Ship,
            &[Key::Comma, Key::Z],
            MouseAccumulator::default(),
            false,
            [true, true],
        );
        // "Reverse left-right" swaps the yaw keys and "up-down" the pitch keys.
        assert!(keys.right && !keys.left);
        assert!(keys.pitch_up && !keys.pitch_down);
        assert_eq!(keys.mouse_flip, [true, true]);
    }

    #[test]
    fn the_mouse_accumulator_moves_only_while_held_and_starts_at_zero() {
        let mut accumulator = MouseAccumulator::default();
        let step = |held, dx, dy| MouseSteer { held, dx, dy };
        accumulator.update(step(true, 3, -2));
        assert_eq!(accumulator.packed(), 0xfffe_0003);
        accumulator.update(step(true, 2, 1));
        assert_eq!(accumulator.packed(), 0xffff_0005);
        // Releasing starts a fresh count so the next press cannot jump.
        accumulator.update(step(false, 100, 100));
        assert_eq!(accumulator.packed(), 0);
        accumulator.update(step(true, 1, 0));
        assert_eq!(accumulator.packed(), 1);
    }

    #[test]
    fn f3_is_the_source_map_key_and_not_a_preset_switch() {
        assert!(galaxy_map_key(Key::F3));
        assert!(galaxy_map_key(Key::G));
        // The source's other screen and view keys keep their own routes.
        for key in [Key::F1, Key::F2, Key::F4, Key::F6, Key::F12] {
            assert!(!galaxy_map_key(key), "{key:?} is not the map key");
        }
    }

    #[test]
    fn the_source_view_keys_are_unchanged_and_help_is_not_a_binding() {
        assert_eq!(view_command(Key::F1), Some(ViewCommand::Cycle));
        assert_eq!(view_command(Key::F6), Some(ViewCommand::Front));
        assert_eq!(view_command(Key::F7), Some(ViewCommand::Rear));
        assert_eq!(view_command(Key::F8), Some(ViewCommand::Turret));
        assert_eq!(view_command(Key::F9), Some(ViewCommand::External));
        assert_eq!(view_command(Key::F10), Some(ViewCommand::Missile));
        assert_eq!(view_command(Key::F11), Some(ViewCommand::Combat));
        // The retired port help key and the debug keys are not view actions.
        assert_eq!(view_command(Key::Slash), None);
        assert_eq!(view_command(Key::F12), None);
    }

    #[test]
    fn developer_commands_require_the_dev_switch_and_never_take_a_source_key() {
        // A player build has no debug bindings at all.
        for (key, ctrl, shift) in [
            (Key::F11, true, false),
            (Key::F12, true, false),
            (Key::F12, true, true),
            (Key::O, false, false),
        ] {
            assert_eq!(dev_command(false, key, ctrl, shift), None);
        }
        // Enabled, the debug chord is Ctrl+F11 / Ctrl+F12 / Ctrl+Shift+F12.
        assert_eq!(
            dev_command(true, Key::F11, true, false),
            Some(DevCommand::Screenshot)
        );
        assert_eq!(
            dev_command(true, Key::F12, true, false),
            Some(DevCommand::ModelViewer)
        );
        assert_eq!(
            dev_command(true, Key::F12, true, true),
            Some(DevCommand::ModelLab)
        );
        assert_eq!(
            dev_command(true, Key::O, false, false),
            Some(DevCommand::PrototypeFlight)
        );
        // Plain F11/F12 stay free for their source actions.
        assert_eq!(dev_command(true, Key::F11, false, false), None);
        assert_eq!(dev_command(true, Key::F12, false, false), None);
        assert_eq!(dev_command(true, Key::F12, false, true), None);
    }

    #[test]
    fn switching_the_prefs_preset_changes_the_route_immediately() {
        let mut prefs = Prefs::default();
        assert_eq!(Preset::from_modern(prefs.modern), Preset::Classic);
        prefs.toggle_modern();
        assert_eq!(Preset::from_modern(prefs.modern), Preset::Modern);
        assert_eq!(
            controls(Preset::from_modern(prefs.modern), &[Key::W]).thrust,
            1.0
        );
        assert_eq!(
            controls(Preset::from_modern(prefs.modern), &[Key::Enter]).thrust,
            0.0
        );
        prefs.toggle_modern();
        assert!(!prefs.modern);
        assert_eq!(
            controls(Preset::from_modern(prefs.modern), &[Key::W]).thrust,
            0.0
        );
        assert_eq!(
            controls(Preset::from_modern(prefs.modern), &[Key::Enter]).thrust,
            1.0
        );
    }

    #[test]
    fn every_action_has_a_unique_name_and_the_presets_game_keys() {
        // The vocabulary is exactly the documented flight controls, so a typo
        // in `config.cfg` is ignored instead of binding something invisible.
        assert_eq!(Action::from_name("thrust_up"), Some(Action::ThrustUp));
        assert_eq!(Action::from_name("pitch_up"), Some(Action::PitchUp));
        assert_eq!(Action::PitchUp.name(), "pitch_up");
        assert_eq!(Action::from_name("PitchUp"), None, "names are lower-case");
        assert_eq!(Action::from_name("help"), None);
        let mut names = std::collections::BTreeSet::new();
        for (name, action) in Action::NAMES {
            assert!(names.insert(name), "{name} is listed twice");
            assert_eq!(Action::from_name(name), Some(action));
            assert_eq!(action.name(), name);
            assert!(!action.keys().is_empty(), "{name} has no key");
            for key in action.keys() {
                assert!(
                    scancode(*key).is_some(),
                    "{name}: {key:?} has no original scancode"
                );
            }
        }
        // The preset axes name the keys each preset actually reads, so one
        // binding works after a preset switch.
        assert_eq!(Action::ThrustUp.keys(), [Key::W, Key::Enter]);
        assert_eq!(Action::ThrustDown.keys(), [Key::S, Key::RightShift]);
        assert_eq!(Action::PitchUp.keys(), [Key::Up, Key::A]);
        assert_eq!(Action::YawRight.keys(), [Key::Right, Key::Period]);
        assert_eq!(Action::RollLeft.keys(), [Key::Q, Key::LeftBracket]);
        assert_eq!(Action::Brake.keys(), [Key::B]);
        // The view actions agree with the source view routing.
        for (action, command) in [
            (Action::ViewCycle, ViewCommand::Cycle),
            (Action::ViewFront, ViewCommand::Front),
            (Action::ViewRear, ViewCommand::Rear),
            (Action::ViewTurret, ViewCommand::Turret),
            (Action::ViewExternal, ViewCommand::External),
            (Action::ViewMissile, ViewCommand::Missile),
            (Action::CombatView, ViewCommand::Combat),
        ] {
            assert_eq!(action.keys().len(), 1, "{action:?}");
            assert_eq!(view_command(action.keys()[0]), Some(command), "{action:?}");
        }
    }

    #[test]
    fn bind_entries_remap_the_delivered_flight_action() {
        use crate::session::{bindings as parse_bindings, Frame};
        use std::collections::BTreeMap;

        // An action name follows the active preset: Z raises the set speed in
        // Modern (W) and in Classic (Enter), and stops acting as itself.
        let mut remap = BTreeMap::new();
        remap.insert("thrust_up".to_owned(), "Z".to_owned());
        let binds = parse_bindings(&remap);
        let mut frame = Frame {
            down: vec![Key::Z],
            pressed: vec![Key::Z],
            ..Frame::default()
        };
        frame.remap(&binds);
        assert_eq!(frame.down, [Key::W, Key::Enter]);
        assert_eq!(frame.pressed, [Key::W, Key::Enter]);
        assert_eq!(controls(Preset::Modern, &frame.down).thrust, 1.0);
        assert_eq!(controls(Preset::Classic, &frame.down).thrust, 1.0);

        // A game-key name remaps exactly that key, as before.
        let mut remap = BTreeMap::new();
        remap.insert("A".to_owned(), "Up".to_owned());
        let binds = parse_bindings(&remap);
        let mut frame = Frame {
            down: vec![Key::Up, Key::W],
            ..Frame::default()
        };
        frame.remap(&binds);
        assert_eq!(frame.down, [Key::A, Key::W]);
        assert_eq!(controls(Preset::Classic, &frame.down).pitch, 1.0);

        // A fire remap reaches the shared action without touching an axis.
        let mut remap = BTreeMap::new();
        remap.insert("fire".to_owned(), "NumPad0".to_owned());
        let binds = parse_bindings(&remap);
        let mut frame = Frame {
            down: vec![Key::NumPad0],
            ..Frame::default()
        };
        frame.remap(&binds);
        assert_eq!(frame.down, [Key::Space]);
        assert_eq!(
            controls(Preset::Modern, &frame.down),
            FlightControls::default()
        );
    }

    #[test]
    fn unknown_bind_actions_and_keys_are_ignored() {
        use crate::session::{bindings as parse_bindings, Frame};
        use std::collections::BTreeMap;

        let mut remap = BTreeMap::new();
        remap.insert("not_an_action".to_owned(), "W".to_owned());
        remap.insert("pitch_up".to_owned(), "NotAKey".to_owned());
        remap.insert("A".to_owned(), "KeyZ".to_owned());
        let binds = parse_bindings(&remap);
        assert!(binds.is_empty(), "{binds:?}");
        let mut frame = Frame {
            down: vec![Key::W, Key::A],
            pressed: vec![Key::A],
            ..Frame::default()
        };
        frame.remap(&binds);
        assert_eq!(frame.down, [Key::W, Key::A]);
        assert_eq!(frame.pressed, [Key::A]);

        // A self-binding is valid but changes nothing.
        let mut remap = BTreeMap::new();
        remap.insert("fire".to_owned(), "Space".to_owned());
        assert!(parse_bindings(&remap).is_empty());
    }
}
