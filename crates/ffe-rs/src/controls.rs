//! The control schemes, and the Custom one's bindings.
//!
//! The launcher's Settings > Controls offers three schemes. **Classic** and **Modern** are the
//! two fixed presets (`docs/CONTROLS.md`). **Custom** reads like Modern but every action has
//! bindings of its own: keyboard keys, game controller buttons and sticks, and the raw buttons
//! and axes of a joystick. A new Custom scheme starts with everything Modern has, and is saved
//! in `saves/config.cfg` as `custom.<action>=<input>|<input>` lines (see [`Input::to_text`]).
//!
//! Custom works by *delivering game keys*: while a bound input is held, the game sees the key
//! the action uses in the Modern preset ([`game_keys`]), exactly as if it had been typed. While
//! flying, the default keys of every action are taken away, so an action you moved to other keys
//! no longer answers to its old ones; the menus, the docked screens and the maps keep the normal
//! keyboard.

use crate::input::Action;
use crate::session::{key_from, key_name};
use minifb::Key;
use std::collections::BTreeMap;

/// How many bindings one action can have.
pub const MAX_PER_ACTION: usize = 3;

/// A button, trigger or stick direction of a standard game controller, as the controller
/// library names them (so an Xbox, a PlayStation or a Switch pad all fit the same names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pad {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,
}

/// Every pad input: its variant, its name in `config.cfg` and what the launcher calls it.
pub const PADS: [(Pad, &str, &str); 24] = [
    (Pad::South, "South", "A / Cross"),
    (Pad::East, "East", "B / Circle"),
    (Pad::West, "West", "X / Square"),
    (Pad::North, "North", "Y / Triangle"),
    (Pad::LeftBumper, "LeftBumper", "LB / L1"),
    (Pad::RightBumper, "RightBumper", "RB / R1"),
    (Pad::LeftTrigger, "LeftTrigger", "LT / L2"),
    (Pad::RightTrigger, "RightTrigger", "RT / R2"),
    (Pad::Select, "Select", "View / Back"),
    (Pad::Start, "Start", "Menu / Start"),
    (Pad::LeftThumb, "LeftThumb", "Left stick press"),
    (Pad::RightThumb, "RightThumb", "Right stick press"),
    (Pad::DPadUp, "DPadUp", "D-pad up"),
    (Pad::DPadDown, "DPadDown", "D-pad down"),
    (Pad::DPadLeft, "DPadLeft", "D-pad left"),
    (Pad::DPadRight, "DPadRight", "D-pad right"),
    (Pad::LeftStickUp, "LeftStickUp", "Left stick up"),
    (Pad::LeftStickDown, "LeftStickDown", "Left stick down"),
    (Pad::LeftStickLeft, "LeftStickLeft", "Left stick left"),
    (Pad::LeftStickRight, "LeftStickRight", "Left stick right"),
    (Pad::RightStickUp, "RightStickUp", "Right stick up"),
    (Pad::RightStickDown, "RightStickDown", "Right stick down"),
    (Pad::RightStickLeft, "RightStickLeft", "Right stick left"),
    (Pad::RightStickRight, "RightStickRight", "Right stick right"),
];

impl Pad {
    pub fn name(self) -> &'static str {
        PADS.iter()
            .find(|(pad, _, _)| *pad == self)
            .map_or("", |(_, name, _)| name)
    }

    pub fn label(self) -> &'static str {
        PADS.iter()
            .find(|(pad, _, _)| *pad == self)
            .map_or("", |(_, _, label)| label)
    }

    pub fn from_name(name: &str) -> Option<Self> {
        PADS.iter()
            .find(|(_, candidate, _)| *candidate == name)
            .map(|(pad, _, _)| *pad)
    }
}

/// One thing a player can press or push: a keyboard key, a standard game controller input, or a
/// raw joystick button or axis (for the devices a controller library has no layout for).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    Pad(Pad),
    /// A raw joystick button, by the device's own code.
    JoyButton(u32),
    /// A raw joystick axis, by the device's own code; `true` is the positive direction.
    JoyAxis(u32, bool),
}

impl Input {
    /// The text in `config.cfg`: `Key:Up`, `Pad:South`, `Joy:Button7` or `Joy:Axis3+`.
    pub fn to_text(self) -> String {
        match self {
            Self::Key(key) => format!("Key:{}", key_name(key)),
            Self::Pad(pad) => format!("Pad:{}", pad.name()),
            Self::JoyButton(code) => format!("Joy:Button{code}"),
            Self::JoyAxis(code, positive) => {
                format!("Joy:Axis{code}{}", if positive { '+' } else { '-' })
            }
        }
    }

    /// Parse [`Self::to_text`]; `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(name) = text.strip_prefix("Key:") {
            return key_from(name).map(Self::Key);
        }
        if let Some(name) = text.strip_prefix("Pad:") {
            return Pad::from_name(name).map(Self::Pad);
        }
        let joy = text.strip_prefix("Joy:")?;
        if let Some(code) = joy.strip_prefix("Button") {
            return code.parse().ok().map(Self::JoyButton);
        }
        let axis = joy.strip_prefix("Axis")?;
        let (code, positive) = match axis.chars().last()? {
            '+' => (&axis[..axis.len() - 1], true),
            '-' => (&axis[..axis.len() - 1], false),
            _ => return None,
        };
        code.parse().ok().map(|code| Self::JoyAxis(code, positive))
    }

    /// What the launcher shows for it.
    pub fn label(self) -> String {
        match self {
            Self::Key(key) => key_label(key),
            Self::Pad(pad) => format!("Pad: {}", pad.label()),
            Self::JoyButton(code) => format!("Joystick button {code}"),
            Self::JoyAxis(code, positive) => {
                format!(
                    "Joystick axis {code} ({})",
                    if positive { "+" } else { "-" }
                )
            }
        }
    }

    /// Which device it comes from, for the launcher's icons: 0 keyboard, 1 controller,
    /// 2 joystick.
    pub fn device(self) -> u8 {
        match self {
            Self::Key(_) => 0,
            Self::Pad(_) => 1,
            Self::JoyButton(_) | Self::JoyAxis(..) => 2,
        }
    }
}

/// A key as a player reads it: letters and digits as they are, the rest spelled out.
pub fn key_label(key: Key) -> String {
    use Key::*;
    let name = match key {
        Key0 => "0",
        Key1 => "1",
        Key2 => "2",
        Key3 => "3",
        Key4 => "4",
        Key5 => "5",
        Key6 => "6",
        Key7 => "7",
        Key8 => "8",
        Key9 => "9",
        Apostrophe => "'",
        Backquote => "`",
        Backslash => "\\",
        Comma => ",",
        Equal => "=",
        LeftBracket => "[",
        RightBracket => "]",
        Minus => "-",
        Period => ".",
        Semicolon => ";",
        Slash => "/",
        Up => "Up arrow",
        Down => "Down arrow",
        Left => "Left arrow",
        Right => "Right arrow",
        LeftShift => "Left Shift",
        RightShift => "Right Shift",
        LeftCtrl => "Left Ctrl",
        RightCtrl => "Right Ctrl",
        LeftAlt => "Left Alt",
        RightAlt => "Right Alt",
        PageUp => "Page Up",
        PageDown => "Page Down",
        NumPad0 => "Num 0",
        NumPad1 => "Num 1",
        NumPad2 => "Num 2",
        NumPad3 => "Num 3",
        NumPad4 => "Num 4",
        NumPad5 => "Num 5",
        NumPad6 => "Num 6",
        NumPad7 => "Num 7",
        NumPad8 => "Num 8",
        NumPad9 => "Num 9",
        NumPadDot => "Num .",
        NumPadSlash => "Num /",
        NumPadAsterisk => "Num *",
        NumPadMinus => "Num -",
        NumPadPlus => "Num +",
        NumPadEnter => "Num Enter",
        _ => return key_name(key),
    };
    name.to_owned()
}

/// The eight flight axes: the actions whose keys are held rather than pressed.
pub fn is_axis(action: Action) -> bool {
    matches!(
        action,
        Action::ThrustUp
            | Action::ThrustDown
            | Action::PitchUp
            | Action::PitchDown
            | Action::YawLeft
            | Action::YawRight
            | Action::RollLeft
            | Action::RollRight
    )
}

/// The game key(s) a bound action delivers: the key the action uses in the Modern preset.
/// E.C.M. delivers `;` rather than `E`, because Modern's roll-right key is `E` and a roll must
/// not fire the E.C.M.
pub fn game_keys(action: Action) -> &'static [Key] {
    match action {
        Action::Ecm => &[Key::Semicolon],
        other => &other.keys()[..1],
    }
}

/// A label for each action, as the launcher lists them.
pub fn action_label(action: Action) -> &'static str {
    match action {
        Action::ThrustUp => "Speed up",
        Action::ThrustDown => "Slow down",
        Action::PitchUp => "Pitch up",
        Action::PitchDown => "Pitch down",
        Action::YawLeft => "Yaw left",
        Action::YawRight => "Yaw right",
        Action::RollLeft => "Roll left",
        Action::RollRight => "Roll right",
        Action::Brake => "Brake",
        Action::Fire => "Fire lasers",
        Action::Missile => "Launch missile",
        Action::Ecm => "E.C.M.",
        Action::Chaff => "Chaff / centre map",
        Action::Target => "Next target",
        Action::Autopilot => "Autopilot",
        Action::StopAutopilot => "Stop autopilot",
        Action::Docking => "Docking clearance",
        Action::Launch => "Launch / labels",
        Action::Navigation => "Navigation computer",
        Action::GalaxyMap => "Galaxy map",
        Action::Hyperspace => "Hyperspace jump",
        Action::Pause => "Pause",
        Action::Status => "Status page",
        Action::ViewCycle => "Cycle views",
        Action::ViewFront => "Front view",
        Action::ViewRear => "Rear view",
        Action::ViewTurret => "Turret view",
        Action::ViewExternal => "External view",
        Action::ViewMissile => "Missile view",
        Action::CombatView => "Combat view",
        Action::EscapeCapsule => "Escape capsule",
        Action::MapCentre => "Centre the map",
    }
}

/// The actions in the groups the launcher lists them under, in order. Every action is in
/// exactly one group.
pub const GROUPS: [(&str, &[Action]); 5] = [
    (
        "Flying",
        &[
            Action::ThrustUp,
            Action::ThrustDown,
            Action::PitchUp,
            Action::PitchDown,
            Action::YawLeft,
            Action::YawRight,
            Action::RollLeft,
            Action::RollRight,
            Action::Brake,
        ],
    ),
    (
        "Weapons",
        &[Action::Fire, Action::Missile, Action::Ecm, Action::Chaff],
    ),
    (
        "Targets, autopilot and docking",
        &[
            Action::Target,
            Action::Autopilot,
            Action::StopAutopilot,
            Action::Docking,
            Action::Launch,
            Action::Hyperspace,
            Action::Navigation,
        ],
    ),
    (
        "Views and maps",
        &[
            Action::ViewCycle,
            Action::ViewFront,
            Action::ViewRear,
            Action::ViewTurret,
            Action::ViewExternal,
            Action::ViewMissile,
            Action::CombatView,
            Action::GalaxyMap,
            Action::MapCentre,
            Action::Status,
        ],
    ),
    ("Other", &[Action::Pause, Action::EscapeCapsule]),
];

/// The controller input the default (Classic and Modern) layout gives an action, if any.
pub fn default_pad(action: Action) -> Option<Pad> {
    Some(match action {
        Action::ThrustUp => Pad::RightTrigger,
        Action::ThrustDown => Pad::LeftTrigger,
        Action::PitchUp => Pad::LeftStickUp,
        Action::PitchDown => Pad::LeftStickDown,
        Action::YawLeft => Pad::LeftStickLeft,
        Action::YawRight => Pad::LeftStickRight,
        Action::RollLeft => Pad::LeftBumper,
        Action::RollRight => Pad::RightBumper,
        Action::Fire => Pad::South,
        Action::Missile => Pad::East,
        Action::Target => Pad::West,
        Action::ViewCycle => Pad::North,
        Action::GalaxyMap => Pad::Select,
        _ => return None,
    })
}

/// The keys an action has in a preset, as the Settings page lists them.
pub fn preset_keys(action: Action, modern: bool) -> Vec<Key> {
    if is_axis(action) {
        let keys = action.keys();
        return vec![keys[usize::from(!modern)]];
    }
    if action == Action::Ecm && modern {
        // `E` is Modern's roll key too; the E.C.M. answers to `;` and Insert there.
        return vec![Key::Semicolon, Key::Insert];
    }
    action.keys().to_vec()
}

/// Every key some action uses by default in either preset: the keys Custom takes away while
/// flying (an action you rebound must not also answer to its old keys).
pub fn default_keys() -> Vec<Key> {
    let mut keys = Vec::new();
    for (_, action) in Action::NAMES {
        for key in action.keys() {
            if !keys.contains(key) {
                keys.push(*key);
            }
        }
    }
    keys
}

/// The Custom scheme's bindings: each action's list of inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomControls {
    map: BTreeMap<String, Vec<Input>>,
}

impl Default for CustomControls {
    fn default() -> Self {
        Self::modern()
    }
}

impl CustomControls {
    /// Everything the Modern preset has: its keys, and the controller layout.
    pub fn modern() -> Self {
        let mut map = BTreeMap::new();
        for (name, action) in Action::NAMES {
            map.insert(name.to_owned(), Self::defaults_for(action));
        }
        Self { map }
    }

    /// One action's Modern bindings.
    pub fn defaults_for(action: Action) -> Vec<Input> {
        let mut inputs: Vec<Input> = preset_keys(action, true)
            .into_iter()
            .map(Input::Key)
            .collect();
        inputs.extend(default_pad(action).map(Input::Pad));
        inputs.truncate(MAX_PER_ACTION);
        inputs
    }

    /// The inputs bound to `action`.
    pub fn get(&self, action: Action) -> &[Input] {
        self.map.get(action.name()).map_or(&[], Vec::as_slice)
    }

    /// Bind one more input to `action`: `false` when it has [`MAX_PER_ACTION`] already or the
    /// input is bound to it. The same input can serve several actions (Modern's `C` is chaff
    /// and the map centre).
    pub fn add(&mut self, action: Action, input: Input) -> bool {
        let list = self.map.entry(action.name().to_owned()).or_default();
        if list.len() >= MAX_PER_ACTION || list.contains(&input) {
            return false;
        }
        list.push(input);
        true
    }

    /// Unbind the input at `index` of `action`'s list.
    pub fn remove(&mut self, action: Action, index: usize) {
        if let Some(list) = self.map.get_mut(action.name()) {
            if index < list.len() {
                list.remove(index);
            }
        }
    }

    /// Put `action` back to its Modern bindings.
    pub fn reset(&mut self, action: Action) {
        self.map
            .insert(action.name().to_owned(), Self::defaults_for(action));
    }

    /// Whether any binding of any action is not the Modern default.
    pub fn changed(&self) -> bool {
        *self != Self::modern()
    }

    /// The `custom.<action>=<input>|<input>` lines, in the actions' order. An action with
    /// nothing bound is written as `none` so that it stays unbound.
    pub fn to_config(&self) -> String {
        let mut out = String::new();
        for (name, action) in Action::NAMES {
            let inputs = self.get(action);
            let value = if inputs.is_empty() {
                "none".to_owned()
            } else {
                inputs
                    .iter()
                    .map(|input| input.to_text())
                    .collect::<Vec<_>>()
                    .join("|")
            };
            out.push_str(&format!("custom.{name}={value}\n"));
        }
        out
    }

    /// Read one `custom.<action>` line's value. An unknown action is ignored, and so is an
    /// input that does not parse, so a damaged entry never loses the rest of the file.
    pub fn set_from_config(&mut self, name: &str, value: &str) {
        let Some(action) = Action::from_name(name) else {
            return;
        };
        let mut inputs = Vec::new();
        if value.trim() != "none" {
            for text in value.split('|') {
                if let Some(input) = Input::parse(text) {
                    if !inputs.contains(&input) && inputs.len() < MAX_PER_ACTION {
                        inputs.push(input);
                    }
                }
            }
        }
        self.map.insert(action.name().to_owned(), inputs);
    }

    /// The game keys to hold and to press this frame: for each action with an input held, the
    /// game keys it delivers ([`game_keys`]). `down` and `pressed` are the keyboard's keys held
    /// and newly pressed; `pad_now` and `pad_before` the controller and joystick inputs held
    /// this frame and the last.
    pub fn deliver(
        &self,
        down: &[Key],
        pressed: &[Key],
        pad_now: &[Input],
        pad_before: &[Input],
    ) -> (Vec<Key>, Vec<Key>) {
        let mut held = Vec::new();
        let mut fresh = Vec::new();
        for (_, action) in Action::NAMES {
            let inputs = self.get(action);
            let is_held = inputs.iter().any(|input| match input {
                Input::Key(key) => down.contains(key),
                other => pad_now.contains(other),
            });
            if !is_held {
                continue;
            }
            let is_fresh = inputs.iter().any(|input| match input {
                Input::Key(key) => pressed.contains(key),
                other => pad_now.contains(other) && !pad_before.contains(other),
            });
            for key in game_keys(action) {
                if !held.contains(key) {
                    held.push(*key);
                }
                if is_fresh && !fresh.contains(key) {
                    fresh.push(*key);
                }
            }
        }
        (held, fresh)
    }

    /// The other actions that `input` is also bound to, for the launcher's clash warning. Two
    /// pairs share a key on purpose (and in Modern): chaff and centring the map are both `C`,
    /// since one is for flight and the other for the map.
    pub fn also_used_by(&self, action: Action, input: Input) -> Vec<Action> {
        Action::NAMES
            .into_iter()
            .map(|(_, other)| other)
            .filter(|other| {
                *other != action
                    && self.get(*other).contains(&input)
                    && !matches!(
                        (action, *other),
                        (Action::Chaff, Action::MapCentre) | (Action::MapCentre, Action::Chaff)
                    )
            })
            .collect()
    }

    /// Whether any action is bound to `input`.
    pub fn uses(&self, input: Input) -> bool {
        self.map.values().any(|inputs| inputs.contains(&input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions() -> impl Iterator<Item = Action> {
        Action::NAMES.into_iter().map(|(_, action)| action)
    }

    #[test]
    fn inputs_round_trip_through_their_text() {
        for input in [
            Input::Key(Key::Up),
            Input::Key(Key::NumPad8),
            Input::Key(Key::LeftShift),
            Input::Pad(Pad::South),
            Input::Pad(Pad::RightStickLeft),
            Input::JoyButton(7),
            Input::JoyAxis(3, true),
            Input::JoyAxis(12, false),
        ] {
            assert_eq!(Input::parse(&input.to_text()), Some(input), "{input:?}");
        }
        for bad in [
            "",
            "Up",
            "Key:Nope",
            "Pad:Nope",
            "Joy:Axis3",
            "Joy:Button",
            "Joy:Axisx+",
        ] {
            assert_eq!(Input::parse(bad), None, "{bad:?}");
        }
        for (pad, name, _) in PADS {
            assert_eq!(Pad::from_name(name), Some(pad));
            assert_eq!(pad.name(), name);
        }
    }

    #[test]
    fn a_new_custom_scheme_has_everything_modern_has() {
        let custom = CustomControls::modern();
        assert!(!custom.changed());
        for action in actions() {
            assert!(
                !custom.get(action).is_empty(),
                "{} has no binding",
                action.name()
            );
            for key in preset_keys(action, true) {
                assert!(
                    custom.get(action).contains(&Input::Key(key)),
                    "{}: {key:?}",
                    action.name()
                );
            }
        }
        // Modern's flight keys, and the controller's layout.
        assert_eq!(
            custom.get(Action::ThrustUp),
            [Input::Key(Key::W), Input::Pad(Pad::RightTrigger)]
        );
        assert_eq!(
            custom.get(Action::PitchUp),
            [Input::Key(Key::Up), Input::Pad(Pad::LeftStickUp)]
        );
        assert!(custom.get(Action::Fire).contains(&Input::Key(Key::Space)));
        assert!(custom.get(Action::Fire).contains(&Input::Pad(Pad::South)));
        // The E.C.M. does not share Modern's roll key.
        assert!(!custom.get(Action::Ecm).contains(&Input::Key(Key::E)));
        assert!(custom.get(Action::RollRight).contains(&Input::Key(Key::E)));
    }

    #[test]
    fn the_scheme_round_trips_through_config_lines_and_survives_damage() {
        let mut custom = CustomControls::modern();
        custom.reset(Action::Fire);
        custom.remove(Action::Fire, 0);
        assert!(custom.add(Action::Fire, Input::Key(Key::F)));
        assert!(custom.add(Action::Fire, Input::JoyButton(4)));
        custom.remove(Action::Missile, 0);
        custom.remove(Action::Missile, 0);
        assert!(custom.get(Action::Missile).is_empty());
        let text = custom.to_config();
        assert!(
            text.contains("custom.fire=Pad:South|Key:F|Joy:Button4\n"),
            "{text}"
        );
        assert!(text.contains("custom.missile=none\n"), "{text}");
        let mut read = CustomControls::modern();
        for line in text.lines() {
            let (key, value) = line.split_once('=').unwrap();
            read.set_from_config(key.strip_prefix("custom.").unwrap(), value);
        }
        assert_eq!(read, custom);
        // A bad input, an unknown action and a repeat are dropped; the good ones stay.
        let mut damaged = CustomControls::modern();
        damaged.set_from_config("fire", "Key:F|Nope|Key:F|Pad:Nope|Key:G");
        damaged.set_from_config("not_an_action", "Key:F");
        assert_eq!(
            damaged.get(Action::Fire),
            [Input::Key(Key::F), Input::Key(Key::G)]
        );
        assert_eq!(
            damaged.get(Action::Pause),
            CustomControls::defaults_for(Action::Pause)
        );
    }

    #[test]
    fn an_action_holds_at_most_three_inputs_without_repeats() {
        let mut custom = CustomControls::modern();
        custom.remove(Action::Pause, 0);
        assert!(custom.add(Action::Pause, Input::Key(Key::F1)));
        assert!(!custom.add(Action::Pause, Input::Key(Key::F1)), "no repeat");
        assert!(custom.add(Action::Pause, Input::Key(Key::F2)));
        assert!(custom.add(Action::Pause, Input::Key(Key::F3)));
        assert!(
            !custom.add(Action::Pause, Input::Key(Key::F4)),
            "three at most"
        );
        assert_eq!(custom.get(Action::Pause).len(), MAX_PER_ACTION);
    }

    #[test]
    fn bound_inputs_deliver_the_actions_game_keys() {
        let mut custom = CustomControls::modern();
        // Fire moves to F and a joystick button; Space no longer fires.
        custom.set_from_config("fire", "Key:F|Joy:Button2");
        let (held, fresh) = custom.deliver(&[Key::F], &[Key::F], &[], &[]);
        assert_eq!((held, fresh), (vec![Key::Space], vec![Key::Space]));
        let (held, _) = custom.deliver(&[Key::Space], &[Key::Space], &[], &[]);
        assert!(held.is_empty(), "the old key is unbound: {held:?}");
        // A joystick button delivers it too, and is a fresh press only on the first frame.
        let button = [Input::JoyButton(2)];
        let (held, fresh) = custom.deliver(&[], &[], &button, &[]);
        assert_eq!((held, fresh), (vec![Key::Space], vec![Key::Space]));
        let (held, fresh) = custom.deliver(&[], &[], &button, &button);
        assert_eq!((held, fresh), (vec![Key::Space], vec![]));
        // The axes deliver Modern's flight keys, and a roll never delivers E.C.M.'s key.
        let (held, _) = custom.deliver(&[Key::E], &[Key::E], &[], &[]);
        assert_eq!(held, vec![Key::E], "roll right");
        assert_eq!(game_keys(Action::Ecm), [Key::Semicolon]);
        assert_eq!(game_keys(Action::PitchUp), [Key::Up]);
        // One key bound to two actions delivers both (Modern's C is chaff and map centre).
        let (held, _) = custom.deliver(&[Key::C], &[Key::C], &[], &[]);
        assert_eq!(held, vec![Key::C]);
    }

    #[test]
    fn every_action_is_in_exactly_one_group() {
        let mut seen = Vec::new();
        for (_, group) in GROUPS {
            for action in group {
                assert!(!seen.contains(action), "{} is in two groups", action.name());
                seen.push(*action);
            }
        }
        for action in actions() {
            assert!(seen.contains(&action), "{} is in no group", action.name());
        }
        assert_eq!(seen.len(), Action::NAMES.len());
    }

    #[test]
    fn a_clash_names_the_other_actions_but_modern_has_none() {
        // Modern's own bindings clash nowhere (chaff and the map centre share C on purpose).
        let custom = CustomControls::modern();
        for action in actions() {
            for input in custom.get(action) {
                assert!(
                    custom.also_used_by(action, *input).is_empty(),
                    "{}: {input:?}",
                    action.name()
                );
            }
        }
        // Binding Space to the pause key as well makes both actions say so.
        let mut custom = custom;
        assert!(custom.add(Action::Pause, Input::Key(Key::Space)));
        assert_eq!(
            custom.also_used_by(Action::Pause, Input::Key(Key::Space)),
            [Action::Fire]
        );
        assert_eq!(
            custom.also_used_by(Action::Fire, Input::Key(Key::Space)),
            [Action::Pause]
        );
    }

    #[test]
    fn the_default_keys_taken_away_in_flight_cover_both_presets() {
        let keys = default_keys();
        for key in [
            Key::W,
            Key::S,
            Key::Up,
            Key::A,
            Key::Z,
            Key::Enter,
            Key::RightShift,
            Key::Space,
            Key::LeftBracket,
            Key::F3,
        ] {
            assert!(keys.contains(&key), "{key:?}");
        }
        // Keys no action uses stay: the speed-up of time, the undercarriage, Escape.
        for key in [Key::Escape, Key::U, Key::Key6, Key::F5] {
            assert!(!keys.contains(&key), "{key:?}");
        }
    }
}
