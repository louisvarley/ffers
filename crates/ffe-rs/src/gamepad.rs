//! Game controller input (a port addition; the DOS original read a PC
//! joystick through its own driver). A connected pad is read each frame and
//! turned into the keys of the active control preset, so every screen and
//! the original flight model see ordinary key presses:
//!
//! | pad | action | Modern key | Classic key |
//! |---|---|---|---|
//! | left stick up / down | pitch down / up | Up / Down | Z / A |
//! | left stick left / right | yaw | Left / Right | `,` / `.` |
//! | right trigger / left trigger | faster / slower | W / S | Enter / right Shift |
//! | right / left bumper | roll right / left | E / Q | `]` / `[` |
//! | south (A) | fire | Space | Space |
//! | east (B) | launch a missile | M | M |
//! | west (X) | combat target | T | T |
//! | north (Y) | next view | F1 | F1 |
//! | d-pad up / down | time faster / slower | `+` / `-` keys | same |
//! | start | the Escape pages | Escape | Escape |
//! | select | the galaxy map | G | G |
//!
//! The reading is pure ([`Pad`] to keys) so it is tested without a device;
//! only Windows builds open a real pad (the `gilrs` backend needs udev on
//! Linux, which this project does not require).

use minifb::Key;

use crate::controls::{CustomControls, Input, Pad as PadInput};
use crate::input::Preset;

/// A stick deflection that counts as a key held.
pub const DEAD_ZONE: f32 = 0.35;

/// One frame of a pad: sticks in -1..1 (`y` up positive), triggers 0..1 and
/// the buttons held.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pad {
    pub stick_x: f32,
    pub stick_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub left_bumper: bool,
    pub right_bumper: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub start: bool,
    pub select: bool,
    /// The right stick, and the buttons the Custom scheme can bind (the presets leave them).
    pub right_x: f32,
    pub right_y: f32,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub left_thumb: bool,
    pub right_thumb: bool,
}

/// The standard controller inputs `pad` holds, for the Custom scheme (a stick or a trigger
/// counts as held past [`DEAD_ZONE`]).
pub fn pad_inputs(pad: &Pad) -> Vec<Input> {
    let mut out = Vec::new();
    let mut push = |held: bool, input: PadInput| {
        if held {
            out.push(Input::Pad(input));
        }
    };
    push(pad.south, PadInput::South);
    push(pad.east, PadInput::East);
    push(pad.west, PadInput::West);
    push(pad.north, PadInput::North);
    push(pad.left_bumper, PadInput::LeftBumper);
    push(pad.right_bumper, PadInput::RightBumper);
    push(pad.left_trigger > DEAD_ZONE, PadInput::LeftTrigger);
    push(pad.right_trigger > DEAD_ZONE, PadInput::RightTrigger);
    push(pad.select, PadInput::Select);
    push(pad.start, PadInput::Start);
    push(pad.left_thumb, PadInput::LeftThumb);
    push(pad.right_thumb, PadInput::RightThumb);
    push(pad.dpad_up, PadInput::DPadUp);
    push(pad.dpad_down, PadInput::DPadDown);
    push(pad.dpad_left, PadInput::DPadLeft);
    push(pad.dpad_right, PadInput::DPadRight);
    push(pad.stick_y > DEAD_ZONE, PadInput::LeftStickUp);
    push(pad.stick_y < -DEAD_ZONE, PadInput::LeftStickDown);
    push(pad.stick_x < -DEAD_ZONE, PadInput::LeftStickLeft);
    push(pad.stick_x > DEAD_ZONE, PadInput::LeftStickRight);
    push(pad.right_y > DEAD_ZONE, PadInput::RightStickUp);
    push(pad.right_y < -DEAD_ZONE, PadInput::RightStickDown);
    push(pad.right_x < -DEAD_ZONE, PadInput::RightStickLeft);
    push(pad.right_x > DEAD_ZONE, PadInput::RightStickRight);
    out
}

/// The keys `pad` holds for `preset`.
pub fn keys(pad: &Pad, preset: Preset) -> Vec<Key> {
    let modern = preset == Preset::Modern;
    let mut out = Vec::new();
    let mut push = |held: bool, key: Key| {
        if held && !out.contains(&key) {
            out.push(key);
        }
    };
    let (up, down, left, right, faster, slower, roll_left, roll_right) = if modern {
        (
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::W,
            Key::S,
            Key::Q,
            Key::E,
        )
    } else {
        (
            Key::Z,
            Key::A,
            Key::Comma,
            Key::Period,
            Key::Enter,
            Key::RightShift,
            Key::LeftBracket,
            Key::RightBracket,
        )
    };
    push(pad.stick_y > DEAD_ZONE, up);
    push(pad.stick_y < -DEAD_ZONE, down);
    push(pad.stick_x < -DEAD_ZONE, left);
    push(pad.stick_x > DEAD_ZONE, right);
    push(pad.right_trigger > DEAD_ZONE, faster);
    push(pad.left_trigger > DEAD_ZONE, slower);
    push(pad.left_bumper, roll_left);
    push(pad.right_bumper, roll_right);
    push(pad.south, Key::Space);
    push(pad.east, Key::M);
    push(pad.west, Key::T);
    push(pad.north, Key::F1);
    push(pad.dpad_up, Key::Equal);
    push(pad.dpad_down, Key::Minus);
    push(pad.start, Key::Escape);
    push(pad.select, Key::G);
    out
}

/// The connected pads, when the platform has a backend.
pub struct Gamepads {
    #[cfg(windows)]
    gilrs: Option<gilrs::Gilrs>,
    held: Vec<Key>,
    /// The inputs the Custom scheme saw held on the last frame (for the first frame of a press).
    before: Vec<Input>,
    /// Raw joystick buttons held, by the device's code.
    raw_buttons: Vec<u32>,
    /// Raw joystick axes, by the device's code, with their last value.
    raw_axes: Vec<(u32, f32)>,
    /// The first input pressed since [`Gamepads::take_capture`] last asked (for the launcher's
    /// "press a control to bind it").
    capture: Option<Input>,
}

impl Default for Gamepads {
    fn default() -> Self {
        Self::new()
    }
}

/// How far a raw joystick axis must move to count as held or as a binding.
pub const RAW_AXIS_ZONE: f32 = 0.5;

impl Gamepads {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            gilrs: gilrs::Gilrs::new().ok(),
            held: Vec::new(),
            before: Vec::new(),
            raw_buttons: Vec::new(),
            raw_axes: Vec::new(),
            capture: None,
        }
    }

    /// Whether a controller backend is running (the launcher says so when it is not).
    pub fn available(&self) -> bool {
        #[cfg(windows)]
        {
            self.gilrs.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// Take in the controller events since the last call: the raw joystick state, and the first
    /// press worth binding.
    fn pump(&mut self) {
        #[cfg(windows)]
        {
            use gilrs::EventType;
            let Some(gilrs) = self.gilrs.as_mut() else {
                return;
            };
            while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
                match event {
                    EventType::ButtonPressed(button, code) => {
                        let code = code.into_u32();
                        if !self.raw_buttons.contains(&code) {
                            self.raw_buttons.push(code);
                        }
                        self.capture.get_or_insert(button_input(button, code));
                    }
                    EventType::ButtonReleased(_, code) => {
                        let code = code.into_u32();
                        self.raw_buttons.retain(|held| *held != code);
                    }
                    // A trigger that reports as a button value rather than a press.
                    EventType::ButtonChanged(button, value, _) if value > 0.6 => {
                        if let Some(pad) = trigger_input(button) {
                            self.capture.get_or_insert(Input::Pad(pad));
                        }
                    }
                    EventType::AxisChanged(axis, value, code) => {
                        let code = code.into_u32();
                        match self.raw_axes.iter_mut().find(|(known, _)| *known == code) {
                            Some(entry) => entry.1 = value,
                            None => self.raw_axes.push((code, value)),
                        }
                        if value.abs() > 0.6 {
                            if let Some(input) = axis_input(axis, value, code) {
                                self.capture.get_or_insert(input);
                            }
                        }
                    }
                    EventType::Disconnected => {
                        self.raw_buttons.clear();
                        self.raw_axes.clear();
                    }
                    _ => {}
                }
            }
        }
    }

    /// The first controller or joystick input pressed since the last call, once, for binding.
    pub fn take_capture(&mut self) -> Option<Input> {
        self.pump();
        self.capture.take()
    }

    /// The first connected pad's state this frame, if any.
    pub fn read(&mut self) -> Option<Pad> {
        #[cfg(windows)]
        {
            use gilrs::{Axis, Button};
            self.pump();
            let gilrs = self.gilrs.as_mut()?;
            let (_, pad) = gilrs.gamepads().next()?;
            let trigger = |button| pad.button_data(button).map_or(0.0, |data| data.value());
            Some(Pad {
                stick_x: pad.value(Axis::LeftStickX),
                stick_y: pad.value(Axis::LeftStickY),
                left_trigger: trigger(Button::LeftTrigger2),
                right_trigger: trigger(Button::RightTrigger2),
                left_bumper: pad.is_pressed(Button::LeftTrigger),
                right_bumper: pad.is_pressed(Button::RightTrigger),
                south: pad.is_pressed(Button::South),
                east: pad.is_pressed(Button::East),
                west: pad.is_pressed(Button::West),
                north: pad.is_pressed(Button::North),
                dpad_up: pad.is_pressed(Button::DPadUp),
                dpad_down: pad.is_pressed(Button::DPadDown),
                start: pad.is_pressed(Button::Start),
                select: pad.is_pressed(Button::Select),
                right_x: pad.value(Axis::RightStickX),
                right_y: pad.value(Axis::RightStickY),
                dpad_left: pad.is_pressed(Button::DPadLeft),
                dpad_right: pad.is_pressed(Button::DPadRight),
                left_thumb: pad.is_pressed(Button::LeftThumb),
                right_thumb: pad.is_pressed(Button::RightThumb),
            })
        }
        #[cfg(not(windows))]
        None
    }

    /// Every controller and joystick input held now, for the Custom scheme: the standard
    /// controller's buttons and sticks by name, and the raw joystick buttons and axes.
    pub fn inputs(&mut self) -> Vec<Input> {
        let mut inputs = self.read().map_or_else(Vec::new, |pad| pad_inputs(&pad));
        for code in &self.raw_buttons {
            inputs.push(Input::JoyButton(*code));
        }
        for (code, value) in &self.raw_axes {
            if value.abs() > RAW_AXIS_ZONE {
                inputs.push(Input::JoyAxis(*code, *value > 0.0));
            }
        }
        inputs
    }

    /// Add this frame's pad keys to the frame's `down` list and, for keys not
    /// held last frame, its `pressed` list.
    pub fn apply(&mut self, preset: Preset, down: &mut Vec<Key>, pressed: &mut Vec<Key>) {
        let now = self.read().map_or_else(Vec::new, |pad| keys(&pad, preset));
        self.apply_keys(now, preset, down, pressed);
    }

    /// The Custom scheme's turn at a frame: the keys its bindings deliver (from the keyboard
    /// and from the controller and joystick) join `down`, `pressed` and `repeated`; and, while
    /// `flying`, the default keys of every action are first taken away, so an action moved to
    /// other keys no longer answers to its old ones. Menus and the docked screens keep the
    /// normal keyboard.
    pub fn apply_custom(
        &mut self,
        custom: &CustomControls,
        flying: bool,
        down: &mut Vec<Key>,
        pressed: &mut Vec<Key>,
        repeated: &mut Vec<Key>,
    ) {
        let now = self.inputs();
        deliver_custom(custom, flying, &now, &self.before, down, pressed, repeated);
        self.before = now;
    }

    fn apply_keys(
        &mut self,
        now: Vec<Key>,
        preset: Preset,
        down: &mut Vec<Key>,
        pressed: &mut Vec<Key>,
    ) {
        let roll = roll_keys(preset);
        for key in &now {
            if !down.contains(key) {
                down.push(*key);
            }
            // The bumpers only roll the ship, which reads the held keys. They are
            // never a key press: Modern's roll-right key is E, and a press of E
            // fires the E.C.M. (its sound, or "No E.C.M. system fitted"), so every
            // tap of RB set it off.
            if roll.contains(key) {
                continue;
            }
            if !self.held.contains(key) && !pressed.contains(key) {
                pressed.push(*key);
            }
        }
        self.held = now;
    }
}

/// The Custom scheme applied to one frame's key lists (see [`Gamepads::apply_custom`]).
pub fn deliver_custom(
    custom: &CustomControls,
    flying: bool,
    pad_now: &[Input],
    pad_before: &[Input],
    down: &mut Vec<Key>,
    pressed: &mut Vec<Key>,
    repeated: &mut Vec<Key>,
) {
    let (held, fresh) = custom.deliver(down, pressed, pad_now, pad_before);
    if flying {
        let defaults = crate::controls::default_keys();
        down.retain(|key| !defaults.contains(key));
        pressed.retain(|key| !defaults.contains(key));
        repeated.retain(|key| !defaults.contains(key));
    }
    for key in held {
        if !down.contains(&key) {
            down.push(key);
        }
    }
    for key in fresh {
        if !pressed.contains(&key) {
            pressed.push(key);
        }
        if !repeated.contains(&key) {
            repeated.push(key);
        }
    }
    // The pad buttons no action is bound to keep their fixed jobs: Start opens the Escape
    // pages and the D-pad speeds time up and down.
    for (pad, key) in [
        (PadInput::Start, Key::Escape),
        (PadInput::DPadUp, Key::Equal),
        (PadInput::DPadDown, Key::Minus),
    ] {
        let input = Input::Pad(pad);
        if custom.uses(input) || !pad_now.contains(&input) {
            continue;
        }
        if !down.contains(&key) {
            down.push(key);
        }
        if !pad_before.contains(&input) {
            if !pressed.contains(&key) {
                pressed.push(key);
            }
            if !repeated.contains(&key) {
                repeated.push(key);
            }
        }
    }
}

/// The keys the bumpers hold for `preset`: roll left, roll right.
fn roll_keys(preset: Preset) -> [Key; 2] {
    if preset == Preset::Modern {
        [Key::Q, Key::E]
    } else {
        [Key::LeftBracket, Key::RightBracket]
    }
}

/// A controller button as a binding: the standard name when the library knows the button, the
/// device's own code when it does not (a joystick's fire button, for one).
#[cfg(windows)]
fn button_input(button: gilrs::Button, code: u32) -> Input {
    use gilrs::Button;
    let pad = match button {
        Button::South => PadInput::South,
        Button::East => PadInput::East,
        Button::West => PadInput::West,
        Button::North => PadInput::North,
        Button::LeftTrigger => PadInput::LeftBumper,
        Button::RightTrigger => PadInput::RightBumper,
        Button::LeftTrigger2 => PadInput::LeftTrigger,
        Button::RightTrigger2 => PadInput::RightTrigger,
        Button::Select => PadInput::Select,
        Button::Start => PadInput::Start,
        Button::LeftThumb => PadInput::LeftThumb,
        Button::RightThumb => PadInput::RightThumb,
        Button::DPadUp => PadInput::DPadUp,
        Button::DPadDown => PadInput::DPadDown,
        Button::DPadLeft => PadInput::DPadLeft,
        Button::DPadRight => PadInput::DPadRight,
        _ => return Input::JoyButton(code),
    };
    Input::Pad(pad)
}

#[cfg(windows)]
fn trigger_input(button: gilrs::Button) -> Option<PadInput> {
    match button {
        gilrs::Button::LeftTrigger2 => Some(PadInput::LeftTrigger),
        gilrs::Button::RightTrigger2 => Some(PadInput::RightTrigger),
        _ => None,
    }
}

/// An axis pushed past the dead zone as a binding: a named stick direction or trigger, or the
/// device's own axis code with its direction.
#[cfg(windows)]
fn axis_input(axis: gilrs::Axis, value: f32, code: u32) -> Option<Input> {
    use gilrs::Axis;
    let positive = value > 0.0;
    let pad = match (axis, positive) {
        (Axis::LeftStickY, true) => PadInput::LeftStickUp,
        (Axis::LeftStickY, false) => PadInput::LeftStickDown,
        (Axis::LeftStickX, true) => PadInput::LeftStickRight,
        (Axis::LeftStickX, false) => PadInput::LeftStickLeft,
        (Axis::RightStickY, true) => PadInput::RightStickUp,
        (Axis::RightStickY, false) => PadInput::RightStickDown,
        (Axis::RightStickX, true) => PadInput::RightStickRight,
        (Axis::RightStickX, false) => PadInput::RightStickLeft,
        (Axis::DPadY, true) => PadInput::DPadUp,
        (Axis::DPadY, false) => PadInput::DPadDown,
        (Axis::DPadX, true) => PadInput::DPadRight,
        (Axis::DPadX, false) => PadInput::DPadLeft,
        (Axis::LeftZ, true) => PadInput::LeftTrigger,
        (Axis::RightZ, true) => PadInput::RightTrigger,
        (Axis::LeftZ | Axis::RightZ, false) => return None,
        _ => return Some(Input::JoyAxis(code, positive)),
    };
    Some(Input::Pad(pad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Action;

    #[test]
    fn sticks_and_triggers_become_the_presets_keys() {
        let pad = Pad {
            stick_y: 0.9,
            stick_x: -0.5,
            right_trigger: 1.0,
            south: true,
            ..Pad::default()
        };
        assert_eq!(
            keys(&pad, Preset::Modern),
            [Key::Up, Key::Left, Key::W, Key::Space]
        );
        assert_eq!(
            keys(&pad, Preset::Classic),
            [Key::Z, Key::Comma, Key::Enter, Key::Space]
        );
    }

    #[test]
    fn small_deflections_are_ignored() {
        let pad = Pad {
            stick_x: 0.2,
            stick_y: -0.3,
            left_trigger: 0.1,
            ..Pad::default()
        };
        assert!(keys(&pad, Preset::Modern).is_empty());
    }

    #[test]
    fn the_bumpers_roll_the_ship_without_pressing_a_key() {
        // Modern's roll-right key is E, which is also the E.C.M.: a bumper must be
        // held (to roll) but never "pressed" (which would fire it).
        for (preset, left, right) in [
            (Preset::Modern, Key::Q, Key::E),
            (Preset::Classic, Key::LeftBracket, Key::RightBracket),
        ] {
            let pad = Pad {
                left_bumper: true,
                right_bumper: true,
                south: true,
                ..Pad::default()
            };
            let mut pads = Gamepads::new();
            let (mut down, mut pressed) = (Vec::new(), Vec::new());
            pads.apply_keys(keys(&pad, preset), preset, &mut down, &mut pressed);
            assert!(down.contains(&left) && down.contains(&right), "{preset:?}");
            assert_eq!(pressed, [Key::Space], "{preset:?}: only fire is a press");
        }
    }

    #[test]
    fn the_standard_inputs_follow_the_dead_zone() {
        let pad = Pad {
            stick_y: 0.9,
            stick_x: -0.2,
            right_x: 0.8,
            left_trigger: 0.5,
            right_trigger: 0.1,
            dpad_left: true,
            ..Pad::default()
        };
        assert_eq!(
            pad_inputs(&pad),
            [
                Input::Pad(PadInput::LeftTrigger),
                Input::Pad(PadInput::DPadLeft),
                Input::Pad(PadInput::LeftStickUp),
                Input::Pad(PadInput::RightStickRight),
            ]
        );
    }

    #[test]
    fn custom_bindings_replace_the_defaults_while_flying_only() {
        let mut custom = CustomControls::modern();
        custom.set_from_config("fire", "Key:F|Joy:Button2");
        // Flying: F fires (it delivers Space), and the arrows still pitch.
        let (mut down, mut pressed, mut repeated) = (vec![Key::F, Key::Up], vec![Key::F], vec![]);
        deliver_custom(
            &custom,
            true,
            &[],
            &[],
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert!(down.contains(&Key::Space), "F delivers Space");
        assert!(pressed.contains(&Key::Space) && repeated.contains(&Key::Space));
        assert!(down.contains(&Key::Up));
        // The old key alone does nothing in flight, as it is no longer bound...
        let (mut down, mut pressed, mut repeated) = (vec![Key::Space], vec![Key::Space], vec![]);
        deliver_custom(
            &custom,
            true,
            &[],
            &[],
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert!(down.is_empty() && pressed.is_empty(), "{down:?}");
        // ...but in a menu (not flying) it is still the normal keyboard.
        let (mut down, mut pressed, mut repeated) = (vec![Key::Space], vec![Key::Space], vec![]);
        deliver_custom(
            &custom,
            false,
            &[],
            &[],
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert_eq!(down, [Key::Space]);
        // A joystick button fires too, pressed only on its first frame.
        let button = [Input::JoyButton(2)];
        let (mut down, mut pressed, mut repeated) = (vec![], vec![], vec![]);
        deliver_custom(
            &custom,
            true,
            &button,
            &[],
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert_eq!(
            (&down[..], &pressed[..]),
            (&[Key::Space][..], &[Key::Space][..])
        );
        let (mut down, mut pressed, mut repeated) = (vec![], vec![], vec![]);
        deliver_custom(
            &custom,
            true,
            &button,
            &button,
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert_eq!((&down[..], pressed.len()), (&[Key::Space][..], 0));
    }

    #[test]
    fn unbound_pad_buttons_keep_their_fixed_jobs() {
        let mut custom = CustomControls::modern();
        let start = [Input::Pad(PadInput::Start)];
        let (mut down, mut pressed, mut repeated) = (vec![], vec![], vec![]);
        deliver_custom(
            &custom,
            true,
            &start,
            &[],
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert_eq!(
            (&down[..], &pressed[..]),
            (&[Key::Escape][..], &[Key::Escape][..])
        );
        // Bound to an action, Start is that action's and no longer opens the pages.
        assert!(custom.add(Action::Pause, Input::Pad(PadInput::Start)));
        let (mut down, mut pressed, mut repeated) = (vec![], vec![], vec![]);
        deliver_custom(
            &custom,
            true,
            &start,
            &[],
            &mut down,
            &mut pressed,
            &mut repeated,
        );
        assert_eq!(down, [Key::P]);
    }

    #[test]
    fn a_new_press_is_reported_once() {
        let mut pads = Gamepads::new();
        let (mut down, mut pressed) = (Vec::new(), Vec::new());
        // No device here: nothing is added.
        pads.apply(Preset::Modern, &mut down, &mut pressed);
        assert!(down.is_empty() && pressed.is_empty());
    }
}
