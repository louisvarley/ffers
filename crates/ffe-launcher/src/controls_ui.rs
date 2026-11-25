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

/// The chip's colours by device: keyboard, controller, joystick.
const CHIP_FILL: [Color32; 3] = [
    Color32::from_rgb(24, 46, 38),
    Color32::from_rgb(16, 44, 62),
    Color32::from_rgb(62, 46, 16),
];

/// One binding as a labelled chip (its device by colour), with a cross to say a click removes
/// it. A clash with another action gets an amber outline and says so on hover.
fn chip(ui: &mut egui::Ui, input: Input, clashes: &[Action]) -> egui::Response {
    let text = RichText::new(format!("{}  \u{d7}", input.label()))
        .font(draw::font(16.0, draw::REGULAR))
        .color(CREAM);
    let stroke = if clashes.is_empty() {
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(215, 222, 210, 90))
    } else {
        Stroke::new(2.0_f32, WARNING)
    };
    let response = ui.add(
        egui::Button::new(text)
            .fill(CHIP_FILL[usize::from(input.device()).min(2)])
            .stroke(stroke)
            .corner_radius(CornerRadius::same(7)),
    );
    let mut hint = String::from("Click to remove this binding");
    if !clashes.is_empty() {
        let names: Vec<&str> = clashes.iter().map(|a| controls::action_label(*a)).collect();
        hint = format!("Also used for: {}.\n{hint}", names.join(", "));
    }
    response.on_hover_text(hint)
}

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
            self.confirm_reset = false;
        }
        match scheme {
            Scheme::Classic => readonly_list(ui, false),
            Scheme::Modern => readonly_list(ui, true),
            Scheme::Custom => self.custom_editor(ui),
        }
    }

    /// Take in whatever was pressed while a binding is awaited: a key, or a controller or
    /// joystick input. Escape cancels.
    fn hear_binding(&mut self, ctx: &egui::Context) -> bool {
        if self.pads.is_none() {
            self.pads = Some(ffe_rs::gamepad::Gamepads::new());
        }
        let mut captured: Option<Input> = None;
        let controller = self.pads.as_mut().is_some_and(|pads| {
            let seen = pads.take_capture();
            if self.listening.is_some() {
                captured = seen;
            }
            pads.available()
        });
        if self.listening.is_some() {
            // Controllers send no repaint of their own: keep polling while waiting.
            ctx.request_repaint_after(Duration::from_millis(40));
            if captured.is_none() {
                for event in ctx.input(|input| input.events.clone()) {
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
        }
        if let (Some(action), Some(input)) = (self.listening, captured) {
            self.config.custom_controls.add(action, input);
            self.listening = None;
        }
        controller
    }

    /// The Custom bindings, by group: each action's inputs as removable chips, an Add button
    /// that waits for a press, and a reset where the action differs from Modern's.
    fn custom_editor(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let controller = self.hear_binding(&ctx);

        section(ui, "Your controls");
        ui.label(
            RichText::new(
                "Give each action a key, a controller button or stick, or a joystick control.",
            )
            .font(draw::font(18.0, draw::REGULAR))
            .color(CREAM),
        );
        ui.label(
            RichText::new(
                "Click Add beside an action, then press what you want to use. Click a binding to \
                 remove it. Each action holds up to three. While flying, your bindings replace the \
                 default keys; menus and docked screens keep the normal keyboard.",
            )
            .color(DIM)
            .italics(),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(if controller {
                "Controller support is on: connect a pad or joystick and press its buttons to bind them."
            } else {
                "Controller support is not available on this system, so only keys can be bound."
            })
            .color(if controller { VERIFIED } else { WARNING }),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.control_filter)
                    .hint_text("Find an action or a key...")
                    .desired_width(260.0),
            );
            if !self.control_filter.is_empty() && ui.button("Clear").clicked() {
                self.control_filter.clear();
            }
            ui.add_space(16.0);
            if self.confirm_reset {
                ui.label(RichText::new("Put every control back to Modern?").color(WARNING));
                if ui.add(pill("Yes, reset all", true)).clicked() {
                    self.config.custom_controls = CustomControls::modern();
                    self.confirm_reset = false;
                }
                if ui.add(pill("No", false)).clicked() {
                    self.confirm_reset = false;
                }
            } else if ui
                .add_enabled(
                    self.config.custom_controls.changed(),
                    egui::Button::new("Reset all to Modern"),
                )
                .on_hover_text("Puts every binding back to what Modern has.")
                .clicked()
            {
                self.confirm_reset = true;
            }
        });

        let needle = self.control_filter.trim().to_lowercase();
        let mut shown = 0;
        for (group, actions) in controls::GROUPS {
            let rows: Vec<Action> = actions
                .iter()
                .copied()
                .filter(|action| self.action_matches(*action, &needle))
                .collect();
            if rows.is_empty() {
                continue;
            }
            shown += rows.len();
            section(ui, group);
            egui::Grid::new(("custom_group", group))
                .num_columns(3)
                .spacing([18.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    for action in rows {
                        self.binding_row(ui, action);
                        ui.end_row();
                    }
                });
        }
        if shown == 0 {
            ui.add_space(8.0);
            ui.label(
                RichText::new("No action or binding matches that.")
                    .color(DIM)
                    .italics(),
            );
        }
        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "Fixed, not rebindable: Escape opens the options pages (Start on a controller), \
                 6-0 set the speed of time (the D-pad up and down), U works the undercarriage, \
                 F5 saves and Shift+F5 loads.",
            )
            .color(DIM)
            .italics(),
        );
        self.bind_prompt(&ctx, controller);
    }

    /// Whether `action` is listed for the filter text (lower case): by its name or by one of
    /// its bindings.
    fn action_matches(&self, action: Action, needle: &str) -> bool {
        needle.is_empty()
            || controls::action_label(action)
                .to_lowercase()
                .contains(needle)
            || self
                .config
                .custom_controls
                .get(action)
                .iter()
                .any(|input| input.label().to_lowercase().contains(needle))
    }

    /// One action's row: its name, its bindings and the buttons.
    fn binding_row(&mut self, ui: &mut egui::Ui, action: Action) {
        let listening = self.listening == Some(action);
        let inputs: Vec<Input> = self.config.custom_controls.get(action).to_vec();
        let clashes: Vec<Vec<Action>> = inputs
            .iter()
            .map(|input| self.config.custom_controls.also_used_by(action, *input))
            .collect();
        let differs = inputs != CustomControls::defaults_for(action);
        let (mut remove, mut start, mut reset) = (None, false, false);
        ui.add_sized(
            [210.0, 28.0],
            egui::Label::new(
                RichText::new(controls::action_label(action))
                    .font(draw::font(18.0, draw::BOLD))
                    .color(CREAM),
            ),
        );
        ui.horizontal(|ui| {
            for (index, input) in inputs.iter().enumerate() {
                if chip(ui, *input, &clashes[index]).clicked() {
                    remove = Some(index);
                }
            }
            if inputs.is_empty() {
                ui.label(RichText::new("not bound").color(DIM).italics());
            }
            if inputs.len() < controls::MAX_PER_ACTION
                && ui
                    .add(pill("+ Add", listening))
                    .on_hover_text("Press a key, controller button or joystick control to bind it")
                    .clicked()
            {
                start = true;
            }
        });
        if differs {
            if ui
                .small_button("Reset")
                .on_hover_text("Back to Modern's bindings for this action")
                .clicked()
            {
                reset = true;
            }
        } else {
            ui.label("");
        }
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

    /// The prompt while a binding is awaited: it says what to press and offers the modifier
    /// keys, which the page cannot hear on their own.
    fn bind_prompt(&mut self, ctx: &egui::Context, controller: bool) {
        let Some(action) = self.listening else {
            return;
        };
        let mut chosen: Option<Input> = None;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("bind_prompt")).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.label(
                RichText::new(format!("Binding: {}", controls::action_label(action)))
                    .font(draw::font(26.0, draw::BOLD))
                    .color(CREAM),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(if controller {
                    "Press a key, a controller button, push a stick or trigger, or press a \
                     joystick button or move its axis."
                } else {
                    "Press the key you want to use."
                })
                .font(draw::font(19.0, draw::REGULAR))
                .color(WARNING),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new("Shift, Ctrl and Alt cannot be heard on their own: choose one here.")
                    .color(DIM),
            );
            ui.horizontal_wrapped(|ui| {
                for (label, name) in [
                    ("Left Shift", "LeftShift"),
                    ("Right Shift", "RightShift"),
                    ("Left Ctrl", "LeftCtrl"),
                    ("Right Ctrl", "RightCtrl"),
                    ("Left Alt", "LeftAlt"),
                    ("Right Alt", "RightAlt"),
                ] {
                    if ui.button(label).clicked() {
                        chosen = Input::parse(&format!("Key:{name}"));
                    }
                }
            });
            ui.add_space(14.0);
            if ui.add(pill("Cancel (Esc)", false)).clicked() {
                cancel = true;
            }
        });
        if let Some(input) = chosen {
            self.config.custom_controls.add(action, input);
            self.listening = None;
        } else if cancel {
            self.listening = None;
        }
    }
}

/// Classic's or Modern's keys, by group (they cannot be changed here: choose Custom for that).
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
    for (group, actions) in controls::GROUPS {
        section(ui, group);
        egui::Grid::new((
            if modern {
                "modern_keys"
            } else {
                "classic_keys"
            },
            group,
        ))
        .num_columns(3)
        .spacing([28.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            for heading in ["Action", "Keyboard", "Controller"] {
                ui.label(
                    RichText::new(heading)
                        .font(draw::font(15.0, draw::BOLD))
                        .color(DIM),
                );
            }
            ui.end_row();
            for action in actions {
                ui.label(RichText::new(controls::action_label(*action)).color(CREAM));
                let keys: Vec<String> = controls::preset_keys(*action, modern)
                    .into_iter()
                    .map(controls::key_label)
                    .collect();
                ui.label(RichText::new(keys.join("  /  ")).color(CREAM));
                let pad = controls::default_pad(*action).map_or("", |pad| pad.label());
                ui.label(RichText::new(pad).color(DIM));
                ui.end_row();
            }
            if !modern && group == "Flying" {
                ui.label(RichText::new("Steer the ship").color(CREAM));
                ui.label(
                    RichText::new("Hold the right mouse button and move the mouse").color(CREAM),
                );
                ui.label("");
                ui.end_row();
            }
        });
    }
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
