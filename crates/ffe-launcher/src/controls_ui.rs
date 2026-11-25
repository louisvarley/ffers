//! Settings > Controls: the control scheme (Classic, Modern or Custom), its keys listed, and for
//! Custom a binding editor for the keyboard, game controllers and joysticks.
//!
//! The scheme and the Custom bindings are saved in `saves/config.cfg` (`controls=` and
//! `custom.<action>=` lines, see `ffe_rs::controls`), the file the game reads. A new Custom
//! scheme starts with everything Modern has.

use super::*;
use ffe_rs::controls::{self, CustomControls, Input};
use ffe_rs::input::Action;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scheme {
    Classic,
    Modern,
    Custom,
}

fn scheme_of(config: &Config) -> Scheme {
    if config.custom {
        Scheme::Custom
    } else if config.modern {
        Scheme::Modern
    } else {
        Scheme::Classic
    }
}

fn set_scheme(config: &mut Config, scheme: Scheme) {
    // Custom reads the Modern keys, with its own bindings, so `modern` is set for it too.
    config.modern = scheme != Scheme::Classic;
    config.custom = scheme == Scheme::Custom;
}

/// The minifb key name of an egui key, for the keys a game binding can be (egui reports no
/// bare Shift, Ctrl or Alt, and no left/right: the editor has buttons for those).
fn key_name(key: egui::Key) -> Option<&'static str> {
    use egui::Key::*;
    Some(match key {
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Tab => "Tab",
        Backspace => "Backspace",
        Enter => "Enter",
        Space => "Space",
        Insert => "Insert",
        Delete => "Delete",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        Comma => "Comma",
        Period => "Period",
        Semicolon => "Semicolon",
        Slash => "Slash",
        Backslash => "Backslash",
        Minus => "Minus",
        Equals => "Equal",
        OpenBracket => "LeftBracket",
        CloseBracket => "RightBracket",
        Backtick => "Backquote",
        Quote => "Apostrophe",
        Num0 => "Key0",
        Num1 => "Key1",
        Num2 => "Key2",
        Num3 => "Key3",
        Num4 => "Key4",
        Num5 => "Key5",
        Num6 => "Key6",
        Num7 => "Key7",
        Num8 => "Key8",
        Num9 => "Key9",
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        _ => return None,
    })
}

const SCHEME_ABOUT: [(Scheme, &str); 3] = [
    (Scheme::Classic, super::CLASSIC_CONTROLS),
    (Scheme::Modern, super::MODERN_CONTROLS),
    (
        Scheme::Custom,
        "Your own: keys, controller buttons and sticks, and joystick buttons and axes for \
         every action. It starts as a copy of Modern. The mouse does not steer.",
    ),
];

impl App {
    pub(super) fn controls_tab(&mut self, ui: &mut egui::Ui) {
        let mut scheme = scheme_of(&self.config);
        let about = SCHEME_ABOUT
            .iter()
            .find(|(candidate, _)| *candidate == scheme)
            .map_or("", |(_, about)| about);
        section(ui, "Control scheme");
        option_row(ui, "Controls", about, |ui| {
            choice(
                ui,
                &mut scheme,
                &[
                    (Scheme::Classic, "Classic"),
                    (Scheme::Modern, "Modern"),
                    (Scheme::Custom, "Custom"),
                ],
            );
        });
        if scheme != scheme_of(&self.config) {
            set_scheme(&mut self.config, scheme);
            self.listening = None;
        }
        match scheme {
            Scheme::Classic => readonly_list(ui, false),
            Scheme::Modern => readonly_list(ui, true),
            Scheme::Custom => self.custom_editor(ui),
        }
    }

    /// The Custom bindings: each action's inputs as removable chips, a button to add one (it
    /// listens for a key, a controller or a joystick), and a reset.
    fn custom_editor(&mut self, ui: &mut egui::Ui) {
        if self.pads.is_none() {
            self.pads = Some(ffe_rs::gamepad::Gamepads::new());
        }
        let ctx = ui.ctx().clone();
        // Keep polling the controllers (they send no repaint of their own) while listening.
        if self.listening.is_some() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
        let mut captured: Option<Input> = None;
        let controller = self.pads.as_mut().is_some_and(|pads| {
            let seen = pads.take_capture();
            if self.listening.is_some() {
                captured = seen;
            }
            pads.available()
        });
        if self.listening.is_some() && captured.is_none() {
            let events = ctx.input(|input| input.events.clone());
            for event in events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } = event
                {
                    if key == egui::Key::Escape {
                        self.listening = None;
                        break;
                    }
                    if let Some(input) =
                        key_name(key).and_then(|name| Input::parse(&format!("Key:{name}")))
                    {
                        captured = Some(input);
                        break;
                    }
                }
            }
        }
        if let (Some(action), Some(input)) = (self.listening, captured) {
            self.config.custom_controls.add(action, input);
            self.listening = None;
        }

        section(ui, "Your controls");
        ui.label(
            RichText::new(if controller {
                "Click + beside an action, then press a key, a controller button or stick, or a \
                 joystick button or axis. Each action holds up to three. While flying, the \
                 default keys of every action are replaced by yours; the menus and the docked \
                 screens keep the normal keyboard."
            } else {
                "Click + beside an action, then press a key. (No controller support is running \
                 here, so controllers and joysticks cannot be bound.) Each action holds up to \
                 three bindings. While flying, the default keys are replaced by yours; the \
                 menus and the docked screens keep the normal keyboard."
            })
            .color(DIM)
            .italics(),
        );
        ui.add_space(4.0);
        if ui
            .button("Reset every control to Modern")
            .on_hover_text("Puts all the bindings back to what Modern has.")
            .clicked()
        {
            self.config.custom_controls = CustomControls::modern();
            self.listening = None;
        }
        if let Some(action) = self.listening {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Binding \"{}\": press a control now.",
                        controls::action_label(action)
                    ))
                    .font(draw::font(18.0, draw::BOLD))
                    .color(WARNING),
                );
                for (label, name) in [
                    ("Left Shift", "LeftShift"),
                    ("Right Shift", "RightShift"),
                    ("Left Ctrl", "LeftCtrl"),
                    ("Right Ctrl", "RightCtrl"),
                    ("Left Alt", "LeftAlt"),
                    ("Right Alt", "RightAlt"),
                ] {
                    if ui.button(label).clicked() {
                        if let Some(input) = Input::parse(&format!("Key:{name}")) {
                            self.config.custom_controls.add(action, input);
                        }
                        self.listening = None;
                    }
                }
                if ui.button("Cancel (Esc)").clicked() {
                    self.listening = None;
                }
            });
        }
        ui.add_space(6.0);
        for (_, action) in Action::NAMES {
            self.binding_row(ui, action);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Fixed, not rebindable: Escape opens the options pages (Start on a controller), \
                 6-0 set the speed of time (the D-pad up and down), U works the undercarriage, \
                 F5 saves and Shift+F5 loads.",
            )
            .color(DIM)
            .italics(),
        );
    }

    fn binding_row(&mut self, ui: &mut egui::Ui, action: Action) {
        let listening = self.listening == Some(action);
        let inputs: Vec<Input> = self.config.custom_controls.get(action).to_vec();
        let mut remove = None;
        let mut start = false;
        let mut reset = false;
        ui.horizontal_wrapped(|ui| {
            ui.add_sized(
                [190.0, 24.0],
                egui::Label::new(
                    RichText::new(controls::action_label(action))
                        .font(draw::font(17.0, draw::BOLD))
                        .color(CREAM),
                ),
            );
            for (index, input) in inputs.iter().enumerate() {
                if ui
                    .add(pill(&format!("{}  \u{d7}", input.label()), false))
                    .on_hover_text("Click to remove this binding")
                    .clicked()
                {
                    remove = Some(index);
                }
            }
            if inputs.is_empty() {
                ui.label(RichText::new("not bound").color(DIM).italics());
            }
            if inputs.len() < controls::MAX_PER_ACTION
                && ui
                    .add(pill(if listening { "..." } else { "+" }, listening))
                    .on_hover_text("Bind a key, controller button or joystick control")
                    .clicked()
            {
                start = !listening;
            }
            if ui
                .small_button("Reset")
                .on_hover_text("Back to Modern's bindings for this action")
                .clicked()
            {
                reset = true;
            }
        });
        if let Some(index) = remove {
            self.config.custom_controls.remove(action, index);
        }
        if reset {
            self.config.custom_controls.reset(action);
        }
        if start {
            // Anything pressed before the click is not the binding.
            if let Some(pads) = self.pads.as_mut() {
                let _ = pads.take_capture();
            }
            self.listening = Some(action);
        }
    }
}

/// Classic's or Modern's keys, listed (they cannot be changed here: choose Custom for that).
fn readonly_list(ui: &mut egui::Ui, modern: bool) {
    section(
        ui,
        if modern {
            "Modern keys"
        } else {
            "Classic keys"
        },
    );
    ui.label(
        RichText::new(
            "These are fixed. Choose Custom to set your own: it starts as a copy of Modern.",
        )
        .color(DIM)
        .italics(),
    );
    ui.add_space(4.0);
    egui::Grid::new(if modern {
        "modern_keys"
    } else {
        "classic_keys"
    })
    .num_columns(3)
    .spacing([28.0, 6.0])
    .striped(true)
    .show(ui, |ui| {
        for heading in ["Action", "Keyboard", "Controller"] {
            ui.label(
                RichText::new(heading)
                    .font(draw::font(16.0, draw::BOLD))
                    .color(CYAN),
            );
        }
        ui.end_row();
        for (_, action) in Action::NAMES {
            ui.label(RichText::new(controls::action_label(action)).color(CREAM));
            let keys: Vec<String> = controls::preset_keys(action, modern)
                .into_iter()
                .map(controls::key_label)
                .collect();
            ui.label(RichText::new(keys.join("  /  ")).color(CREAM));
            let pad = controls::default_pad(action).map_or("", |pad| pad.label());
            ui.label(RichText::new(pad).color(DIM));
            ui.end_row();
        }
        if !modern {
            ui.label(RichText::new("Steer the ship").color(CREAM));
            ui.label(RichText::new("Hold the right mouse button and move the mouse").color(CREAM));
            ui.label("");
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Also fixed: Escape opens the options pages (Start on a controller), 6-0 set the \
             speed of time (the D-pad up and down), U works the undercarriage, F5 saves and \
             Shift+F5 loads.",
        )
        .color(DIM)
        .italics(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_maps_to_the_two_config_flags() {
        let mut config = Config::default();
        for scheme in [
            Scheme::Modern,
            Scheme::Custom,
            Scheme::Classic,
            Scheme::Custom,
        ] {
            set_scheme(&mut config, scheme);
            assert!(scheme_of(&config) == scheme);
        }
        // Custom reads the Modern keys, as the game does.
        assert!(config.modern && config.custom);
        set_scheme(&mut config, Scheme::Classic);
        assert!(!config.modern && !config.custom);
    }

    #[test]
    fn every_key_the_editor_hears_is_a_key_the_game_knows() {
        use egui::Key::*;
        for key in [
            ArrowUp,
            ArrowDown,
            ArrowLeft,
            ArrowRight,
            Tab,
            Backspace,
            Enter,
            Space,
            Insert,
            Delete,
            Home,
            End,
            PageUp,
            PageDown,
            Comma,
            Period,
            Semicolon,
            Slash,
            Backslash,
            Minus,
            Equals,
            OpenBracket,
            CloseBracket,
            Backtick,
            Quote,
            Num0,
            Num9,
            A,
            Z,
            F1,
            F12,
        ] {
            let name = key_name(key).unwrap_or_else(|| panic!("{key:?} has no name"));
            assert!(
                Input::parse(&format!("Key:{name}")).is_some(),
                "{key:?} -> {name} is not a game key"
            );
        }
        // Escape cancels the listening, so it is never a binding.
        assert!(key_name(Escape).is_none());
    }

    #[test]
    fn every_action_is_listed_with_a_label_and_modern_keys() {
        for (_, action) in Action::NAMES {
            assert!(!controls::action_label(action).is_empty());
            assert!(!controls::preset_keys(action, true).is_empty());
            assert!(!controls::preset_keys(action, false).is_empty());
        }
    }
}
