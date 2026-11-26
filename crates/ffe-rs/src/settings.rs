//! Persisted port preferences: the control preset, the presentation choices and
//! the remaining applied preference pages, in one versioned `config.cfg`.
//!
//! The original keeps its preferences in the commander file; the port has not
//! imported that block. `config.cfg` is a port-native text file with an explicit
//! `version` line so future fields can be added without guessing. `controls.cfg`
//! and `display.cfg` are the version-1 sidecars written before this file
//! existed; they are still read as a migration path and are no longer written.
//!
//! Loading never fails startup: a missing, malformed, oversized or symlinked
//! file leaves the caller's defaults. Saving is atomic (temporary file, sync,
//! rename) and reports errors to the caller.
//!
//! The `bind.*` keys remap the keyboard: `bind.<game key>=<physical key>` with
//! minifb key names (`bind.A=Up` makes Up act as A), or
//! `bind.<action>=<physical key>` with the action names of
//! [`crate::input::Action`] (`bind.pitch_up=NumPad8`). Entries that do not name
//! a known key or action are kept unchanged and have no effect. The launcher's
//! Settings > Controls page edits the separate `controls=custom` scheme
//! (`custom.<action>=` lines, [`crate::controls`]); `docs/CONTROLS.md` lists the names.
//!
//! The optional enhancements (`docs/notes/enhancements.md`) are
//! `enhancement.<name>=on|off` lines, all off unless a line turns one on. An
//! unknown `enhancement.*` name is ignored and a known one with a bad value
//! keeps its default, so a damaged line never resets the rest of the file.

use crate::controls::CustomControls;
use crate::display::DisplaySettings;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

/// The combined configuration file.
pub const CONFIG_FILE: &str = "config.cfg";
/// The current configuration version.
pub const CONFIG_VERSION: u32 = 2;
/// The legacy controls sidecar (read-only migration).
pub const CONTROLS_FILE: &str = "controls.cfg";
/// The legacy display sidecar (read-only migration).
pub const DISPLAY_FILE: &str = "display.cfg";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// The optional enhancements from `docs/notes/enhancements.md`, in item order.
/// Every one is off by default: with the whole block off the game keeps the
/// original behaviour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Enhancements {
    /// `enhancement-1`: turrets turn while time is paused.
    pub turrets_paused: bool,
    /// `enhancement-2`: a missile ignores its launcher's collision on launch.
    pub missile_launch: bool,
    /// `enhancement-3`: the autopilot offsets its waypoint around bodies.
    pub autopilot_waypoint: bool,
    /// `enhancement-4`: autopilot braking uses retro thrust and gravity.
    pub autopilot_braking: bool,
    /// `enhancement-5`: the autopilot collides against every nearby body.
    pub autopilot_collisions: bool,
    /// `enhancement-6`: missiles use swept (continuous) collision.
    pub missile_swept: bool,
    /// `enhancement-7`: the mission fixes from the known-issues list.
    pub mission_fixes: bool,
    /// `enhancement-8`: ECM can disable NN500 missiles.
    pub ecm_nn500: bool,
    /// `enhancement-9`: the selected NPC's relative speed is shown.
    pub npc_speed: bool,
    /// `enhancement-10`: combat-mode labels list and select NPCs.
    pub npc_labels: bool,
    /// `enhancement-11`: an edge indicator points at an off-screen target.
    pub edge_indicator: bool,
    /// `enhancement-12`: the enhanced (flat) galaxy map with multi-step routes.
    pub enhanced_galaxy_map: bool,
    /// `enhancement-13`: first-person lasers draw as a fixed bottom-to-reticle
    /// sight line (the Saker look) instead of the beam from the gun mount.
    pub laser_sight: bool,
    /// `enhancement-15`: hyperspace shows the stars and dust streaking past
    /// instead of F1066's tunnel.
    pub hyperspace_streaks: bool,
    /// `enhancement-16`: the velocity marker in every first-person view, with a
    /// retrograde marker opposite it.
    pub velocity_markers: bool,
    /// `enhancement-17`: the active mission's destination, jumps and time left on
    /// the HUD, and its destination marked on the enhanced galaxy map.
    pub mission_tracker: bool,
    /// `enhancement-18`: an automatic save to its own slot on every docking.
    pub autosave: bool,
    /// `enhancement-19`: the market shows each price against the galactic average.
    pub market_average: bool,
}

impl Enhancements {
    /// The `enhancement.<name>` key suffixes, in item order.
    pub const KEYS: [&'static str; 18] = [
        "turrets_paused",
        "missile_launch",
        "autopilot_waypoint",
        "autopilot_braking",
        "autopilot_collisions",
        "missile_swept",
        "mission_fixes",
        "ecm_nn500",
        "npc_speed",
        "npc_labels",
        "edge_indicator",
        "enhanced_galaxy_map",
        "laser_sight",
        "hyperspace_streaks",
        "velocity_markers",
        "mission_tracker",
        "autosave",
        "market_average",
    ];

    /// The field at `index` in [`Self::KEYS`] order.
    pub fn slot(&mut self, index: usize) -> Option<&mut bool> {
        Some(match index {
            0 => &mut self.turrets_paused,
            1 => &mut self.missile_launch,
            2 => &mut self.autopilot_waypoint,
            3 => &mut self.autopilot_braking,
            4 => &mut self.autopilot_collisions,
            5 => &mut self.missile_swept,
            6 => &mut self.mission_fixes,
            7 => &mut self.ecm_nn500,
            8 => &mut self.npc_speed,
            9 => &mut self.npc_labels,
            10 => &mut self.edge_indicator,
            11 => &mut self.enhanced_galaxy_map,
            12 => &mut self.laser_sight,
            13 => &mut self.hyperspace_streaks,
            14 => &mut self.velocity_markers,
            15 => &mut self.mission_tracker,
            16 => &mut self.autosave,
            17 => &mut self.market_average,
            _ => return None,
        })
    }

    /// The `enhancement.<name>=on|off` lines, in item order.
    pub fn to_config(self) -> String {
        let mut out = String::new();
        for (name, on) in Self::KEYS.iter().zip(self.values()) {
            out.push_str(&format!("enhancement.{name}={}\n", on_off(on)));
        }
        out
    }

    /// Read one `enhancement.<name>` value. Unknown names and bad values leave
    /// the setting at its default instead of rejecting the whole file.
    pub fn set(&mut self, name: &str, value: &str) {
        let Some(index) = Self::KEYS.iter().position(|key| *key == name) else {
            return;
        };
        if let Some(on) = parse_on_off(value) {
            if let Some(slot) = self.slot(index) {
                *slot = on;
            }
        }
    }

    /// Every field in [`Self::KEYS`] order.
    fn values(&self) -> [bool; 18] {
        [
            self.turrets_paused,
            self.missile_launch,
            self.autopilot_waypoint,
            self.autopilot_braking,
            self.autopilot_collisions,
            self.missile_swept,
            self.mission_fixes,
            self.ecm_nn500,
            self.npc_speed,
            self.npc_labels,
            self.edge_indicator,
            self.enhanced_galaxy_map,
            self.laser_sight,
            self.hyperspace_streaks,
            self.velocity_markers,
            self.mission_tracker,
            self.autosave,
            self.market_average,
        ]
    }
}

/// Every persisted preference. `remap` holds the `bind.*` lines as written;
/// [`crate::session::bindings`] validates and applies them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The Modern flight keys are read (true for Modern and for Custom, which reads like it).
    pub modern: bool,
    /// The Custom control scheme is selected (`controls=custom`); `modern` is then true too.
    pub custom: bool,
    /// The Custom scheme's bindings, kept whichever scheme is selected.
    pub custom_controls: CustomControls,
    pub display: DisplaySettings,
    /// Start in the borderless full-screen window (the launcher's Display
    /// setting). Alt+Enter and the in-game toggle stay session-only.
    pub fullscreen: bool,
    pub space_colour: usize,
    pub shape_detail: usize,
    pub others: [bool; 10],
    pub maps: [bool; 5],
    pub sound: [bool; 10],
    /// Port addition: the Docking Options page's indices.
    pub station_approach: usize,
    pub starport_approach: usize,
    /// The optional enhancements; all off by default.
    pub enhancements: Enhancements,
    pub remap: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let prefs = crate::prefs::Prefs::default();
        Self {
            modern: prefs.modern,
            custom: false,
            custom_controls: CustomControls::modern(),
            display: prefs.display,
            fullscreen: false,
            space_colour: prefs.space_colour,
            shape_detail: prefs.shape_detail,
            others: prefs.others,
            maps: prefs.maps,
            sound: prefs.sound,
            station_approach: prefs.station_approach,
            starport_approach: prefs.starport_approach,
            enhancements: Enhancements::default(),
            remap: BTreeMap::new(),
        }
    }
}

impl Config {
    /// The file's text, in a stable order.
    pub fn to_config(&self) -> String {
        let mut out = String::new();
        out.push_str("# ffe-rs preferences; version 2 adds the preference pages.\n");
        out.push_str(&format!("version={CONFIG_VERSION}\n"));
        out.push_str(&format!(
            "controls={}\n",
            if self.custom {
                "custom"
            } else if self.modern {
                "modern"
            } else {
                "classic"
            }
        ));
        out.push_str(&self.custom_controls.to_config());
        out.push_str(&format!("space_colour={}\n", self.space_colour.min(4)));
        out.push_str(&format!("shape_detail={}\n", self.shape_detail.min(3)));
        for (index, value) in self.others.iter().enumerate() {
            out.push_str(&format!("other_{index}={}\n", on_off(*value)));
        }
        for (index, value) in self.maps.iter().enumerate() {
            out.push_str(&format!("map_{index}={}\n", on_off(*value)));
        }
        for (index, value) in self.sound.iter().enumerate() {
            out.push_str(&format!("sound_{index}={}\n", on_off(*value)));
        }
        out.push_str(&format!("station_approach={}\n", self.station_approach));
        out.push_str(&format!("starport_approach={}\n", self.starport_approach));
        out.push_str(&self.display.to_config());
        out.push_str(&format!(
            "fullscreen={}
",
            on_off(self.fullscreen)
        ));
        out.push_str(&self.enhancements.to_config());
        for (action, key) in &self.remap {
            out.push_str(&format!("bind.{action}={key}\n"));
        }
        out
    }

    /// Parse the file. Unknown keys are ignored for forward compatibility; a
    /// known key with a bad value, a missing/unsupported version or a malformed
    /// line returns `None` so the caller keeps its defaults.
    pub fn from_config(text: &str) -> Option<Self> {
        let mut config = Self::default();
        let mut version = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "version" => {
                    let parsed: u32 = value.parse().ok()?;
                    if !(1..=CONFIG_VERSION).contains(&parsed) {
                        return None;
                    }
                    version = Some(parsed);
                }
                "controls" => match value {
                    "modern" => (config.modern, config.custom) = (true, false),
                    "classic" => (config.modern, config.custom) = (false, false),
                    // Custom reads like Modern, with its own bindings.
                    "custom" => (config.modern, config.custom) = (true, true),
                    _ => return None,
                },
                // The Custom scheme's bindings (`crate::controls`): a damaged entry is dropped
                // and never loses the rest of the file.
                _ if key.starts_with("custom.") => {
                    config
                        .custom_controls
                        .set_from_config(&key["custom.".len()..], value);
                }
                "space_colour" => {
                    let parsed: usize = value.parse().ok()?;
                    if parsed > 4 {
                        return None;
                    }
                    config.space_colour = parsed;
                }
                "station_approach" => {
                    let parsed: usize = value.parse().ok()?;
                    if parsed >= crate::prefs::STATION_APPROACH_KM.len() {
                        return None;
                    }
                    config.station_approach = parsed;
                }
                "starport_approach" => {
                    let parsed: usize = value.parse().ok()?;
                    if parsed >= crate::prefs::STARPORT_APPROACH_KM.len() {
                        return None;
                    }
                    config.starport_approach = parsed;
                }
                // Older builds stored a Modern-graphics font and launcher backend; ignored.
                "font" => {}
                "backend" => {}
                // The retired World Options page (extra traffic, the police and
                // pirate switches, Advanced NPCs and the Improved combat aids,
                // removed 2026-11-11): older files still load, the values are
                // dropped and the original's traffic is used.
                "traffic" | "police" | "pirates" | "advanced_traffic" | "police_pirates"
                | "combat" => {}
                "shape_detail" => {
                    let parsed: usize = value.parse().ok()?;
                    if parsed > 3 {
                        return None;
                    }
                    config.shape_detail = parsed;
                }
                _ if key.starts_with("other_") => {
                    let index: usize = key["other_".len()..].parse().ok()?;
                    *config.others.get_mut(index)? = parse_on_off(value)?;
                }
                _ if key.starts_with("map_") => {
                    let index: usize = key["map_".len()..].parse().ok()?;
                    *config.maps.get_mut(index)? = parse_on_off(value)?;
                }
                _ if key.starts_with("sound_") => {
                    let index: usize = key["sound_".len()..].parse().ok()?;
                    *config.sound.get_mut(index)? = parse_on_off(value)?;
                }
                _ if key.starts_with("bind.") => {
                    let action = &key["bind.".len()..];
                    if action.is_empty() || value.is_empty() {
                        return None;
                    }
                    config.remap.insert(action.to_owned(), value.to_owned());
                }
                _ if key.starts_with("enhancement.") => {
                    config.enhancements.set(&key["enhancement.".len()..], value);
                }
                "fullscreen" => config.fullscreen = parse_on_off(value)?,
                "scale" | "fit" | "filter" | "aspect" => {}
                _ => {}
            }
        }
        version?;
        // `DisplaySettings::from_config` keeps its own defaults for missing keys
        // and rejects a known key with a bad value. It ignores the version line.
        config.display = DisplaySettings::from_config(text)?;
        Some(config)
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

fn parse_on_off(value: &str) -> Option<bool> {
    match value {
        "on" | "true" | "1" | "yes" => Some(true),
        "off" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

/// Read the combined configuration from `directory`.
///
/// `config.cfg` wins. When it is absent, the two version-1 sidecars are read as
/// a migration: `controls.cfg` (`modern`/`classic`) and `display.cfg` (the
/// display keys). A malformed new file is not silently replaced by the old
/// sidecars (that would change the user's last choice); it returns the defaults.
pub fn load_config(directory: &Path) -> Option<Config> {
    let path = directory.join(CONFIG_FILE);
    if path.exists() {
        let text = read_text(&path, MAX_CONFIG_BYTES)?;
        return Config::from_config(&text);
    }
    let mut config = Config::default();
    if let Some(modern) = load_controls(&directory.join(CONTROLS_FILE)) {
        config.modern = modern;
    }
    if let Some(display) = load_display(&directory.join(DISPLAY_FILE)) {
        config.display = display;
    }
    Some(config)
}

/// Write the combined configuration atomically.
pub fn save_config(directory: &Path, config: &Config) -> Result<(), String> {
    save_text(&directory.join(CONFIG_FILE), &config.to_config())
}

/// Read the old controls sidecar: `Some(true)` is Modern, `Some(false)` Classic.
pub fn load_controls(path: &Path) -> Option<bool> {
    let text = read_text(path, 64)?;
    let mut preset = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("preset=") {
            preset = match value.trim() {
                "modern" => Some(true),
                "classic" => Some(false),
                _ => return None,
            };
        } else if line == "modern" {
            preset = Some(true);
        } else if line == "classic" {
            preset = Some(false);
        } else if let Some(value) = line.strip_prefix("version=") {
            let version: u32 = value.trim().parse().ok()?;
            if !(1..=CONFIG_VERSION).contains(&version) {
                return None;
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            return None;
        }
    }
    preset
}

/// Write the legacy controls sidecar in the versioned form. The playable writes
/// `config.cfg`; this is retained for callers that only need the preset.
pub fn save_controls(path: &Path, modern: bool) -> Result<(), String> {
    let text = format!(
        "version={CONFIG_VERSION}\npreset={}\n",
        if modern { "modern" } else { "classic" }
    );
    save_text(path, &text)
}

/// Read the old display sidecar. A missing, malformed, oversized or symlinked
/// file falls back to the caller's default.
pub fn load_display(path: &Path) -> Option<DisplaySettings> {
    let text = read_text(path, 256)?;
    DisplaySettings::from_config(&text)
}

/// Write the legacy display sidecar in the versioned form.
pub fn save_display(path: &Path, settings: DisplaySettings) -> Result<(), String> {
    save_text(
        path,
        &format!("version={CONFIG_VERSION}\n{}", settings.to_config()),
    )
}

fn read_text(path: &Path, limit: u64) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    String::from_utf8(bytes).ok()
}

fn save_text(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", path.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{}: invalid file name", path.display()))?;
    let mut temp = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut attempt = 0u32;
    loop {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(mut file) => {
                let result = file
                    .write_all(text.as_bytes())
                    .and_then(|()| file.sync_all())
                    .map_err(|error| format!("{}: write failed: {error}", temp.display()));
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&temp);
                    return Err(error);
                }
                return match fs::rename(&temp, path) {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        let _ = fs::remove_file(&temp);
                        Err(format!(
                            "{}: cannot replace atomically: {error}",
                            path.display()
                        ))
                    }
                };
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt += 1;
                if attempt > 100 {
                    return Err(format!("{}: cannot make a temporary name", path.display()));
                }
                temp = parent.join(format!(".{name}.{}.{attempt}.tmp", std::process::id()));
            }
            Err(error) => {
                return Err(format!(
                    "{}: cannot create a temporary file: {error}",
                    temp.display()
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_custom_scheme_is_saved_and_starts_as_modern() {
        use crate::controls::Input;
        use crate::input::Action;
        use minifb::Key;
        // A fresh file selects nothing custom and offers Modern's bindings to edit.
        let fresh = Config::default();
        assert!(!fresh.custom && !fresh.custom_controls.changed());
        // Selecting Custom and rebinding fire survives a save and a load.
        let mut config = Config {
            modern: true,
            custom: true,
            ..Config::default()
        };
        config
            .custom_controls
            .set_from_config("fire", "Key:F|Joy:Button2");
        let text = config.to_config();
        assert!(text.contains("controls=custom\n"), "{text}");
        assert!(text.contains("custom.fire=Key:F|Joy:Button2\n"), "{text}");
        let loaded = Config::from_config(&text).unwrap();
        assert_eq!(loaded, config);
        assert!(
            loaded.modern && loaded.custom,
            "Custom reads the Modern keys"
        );
        assert_eq!(
            loaded.custom_controls.get(Action::Fire),
            [Input::Key(Key::F), Input::JoyButton(2)]
        );
        // The bindings are kept when another scheme is selected, and the old schemes still read.
        let classic = Config {
            custom_controls: config.custom_controls.clone(),
            ..Config::default()
        };
        let text = classic.to_config();
        assert!(text.contains("controls=classic\n"), "{text}");
        assert_eq!(Config::from_config(&text).unwrap(), classic);
        assert!(
            Config::from_config("version=2\ncontrols=modern\n")
                .unwrap()
                .modern
        );
        // A damaged binding line costs only that binding.
        let damaged = Config::from_config(
            "version=2\ncontrols=custom\ncustom.fire=Nope|Key:F\ncustom.zzz=Key:F\n",
        )
        .unwrap();
        assert_eq!(
            damaged.custom_controls.get(Action::Fire),
            [Input::Key(Key::F)]
        );
    }

    #[test]
    fn docking_options_round_trip_and_reject_bad_indices() {
        let config = Config {
            station_approach: 3,
            starport_approach: 2,
            ..Config::default()
        };
        let text = config.to_config();
        assert!(text.contains("station_approach=3\nstarport_approach=2\n"));
        assert_eq!(Config::from_config(&text), Some(config));
        assert_eq!(
            Config::from_config("version=2\nstation_approach=4\n"),
            None,
            "only four station distances"
        );
        assert_eq!(
            Config::default().station_approach,
            0,
            "the original's offset"
        );
    }

    #[test]
    fn enhancement_toggles_round_trip_and_damaged_lines_keep_defaults() {
        // All on, written in item order and read back unchanged.
        let mut all_on = Config::default();
        for index in 0..Enhancements::KEYS.len() {
            *all_on.enhancements.slot(index).unwrap() = true;
        }
        let text = all_on.to_config();
        assert_eq!(Config::from_config(&text), Some(all_on.clone()));
        assert!(text.contains("enhancement.turrets_paused=on\n"), "{text}");
        assert!(text.contains("enhancement.edge_indicator=on\n"), "{text}");

        // The default is every item off, and the keys are still written.
        let off = Config::default().to_config();
        assert!(off.contains("enhancement.mission_fixes=off\n"), "{off}");
        let parsed = Config::from_config(&off).unwrap();
        assert_eq!(parsed.enhancements, Enhancements::default());
        assert_eq!(parsed, Config::default());

        // A bad value keeps that item's default without rejecting the file.
        let config = Config::from_config(
            "version=2\nenhancement.missile_swept=maybe\nenhancement.ecm_nn500=on\n",
        )
        .unwrap();
        assert!(!config.enhancements.missile_swept);
        assert!(config.enhancements.ecm_nn500);
        assert!(!config.enhancements.npc_speed);

        // Unknown enhancement names are ignored and never written back.
        let config = Config::from_config("version=2\nenhancement.teleport=on\n").unwrap();
        assert_eq!(config, Config::default());
        assert!(!config.to_config().contains("enhancement.teleport"));
    }

    #[test]
    fn retired_world_options_load_and_are_dropped() {
        let text = "version=2\ntraffic=3\npolice=off\npirates=on\nadvanced_traffic=on\n\
                    police_pirates=off\ncombat=improved\n";
        let config = Config::from_config(text).unwrap();
        assert_eq!(config, Config::default());
        let written = config.to_config();
        for key in [
            "traffic=",
            "police=",
            "pirates=",
            "advanced_traffic=",
            "combat=",
        ] {
            assert!(!written.contains(key), "{key} is no longer written");
        }
    }

    #[test]
    fn old_backend_settings_load_and_are_retired() {
        let config = Config::from_config("version=2\nbackend=dx12\n").unwrap();
        assert!(!config.to_config().contains("backend="));
    }

    #[test]
    fn the_versioned_config_round_trips_every_page() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(load_config(directory.path()), Some(Config::default()));
        let mut config = Config {
            modern: false,
            space_colour: 1,
            shape_detail: 3,
            ..Config::default()
        };
        config.display.scale = 3;
        config.display.filter = crate::display::Filter::Scale2x;
        config.display.scanlines = true;
        config.display.scanline_intensity = 0.9;
        config.fullscreen = true;
        config.others[1] = true;
        config.maps[4] = true;
        config.sound[7] = false;
        config
            .remap
            .insert("pitch_up".to_owned(), "NumPad8".to_owned());
        save_config(directory.path(), &config).unwrap();
        assert_eq!(load_config(directory.path()), Some(config));
        let text = fs::read_to_string(directory.path().join(CONFIG_FILE)).unwrap();
        assert!(text.starts_with("#"), "{text}");
        assert!(text.contains("version=2"), "{text}");
        assert!(text.contains("bind.pitch_up=NumPad8"), "{text}");
    }

    #[test]
    fn the_old_sidecars_migrate_and_are_not_required_afterwards() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(CONTROLS_FILE), "classic\n").unwrap();
        fs::write(
            directory.path().join(DISPLAY_FILE),
            "scale=2\nfit=fill-window\n",
        )
        .unwrap();
        let config = load_config(directory.path()).unwrap();
        assert!(!config.modern);
        assert_eq!(config.display.scale, 2);
        assert_eq!(config.display.fit, crate::display::FitMode::FillWindow);
        save_config(directory.path(), &config).unwrap();
        fs::remove_file(directory.path().join(CONTROLS_FILE)).unwrap();
        fs::remove_file(directory.path().join(DISPLAY_FILE)).unwrap();
        assert_eq!(load_config(directory.path()), Some(config));
    }

    #[test]
    fn malformed_and_future_configs_fall_back_to_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE);
        for text in [
            "version=99\ncontrols=modern\n",
            "version=2\ncontrols=sideways\n",
            "version=2\nscale=9\n",
            "no equals sign\n",
        ] {
            fs::write(&path, text).unwrap();
            assert_eq!(load_config(directory.path()), None, "{text}");
        }
        fs::write(&path, "x".repeat(MAX_CONFIG_BYTES as usize + 1)).unwrap();
        assert_eq!(load_config(directory.path()), None);
    }

    #[test]
    fn the_preset_sidecar_reads_both_forms_and_rejects_junk() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(CONTROLS_FILE);
        assert_eq!(load_controls(&path), None);
        fs::write(&path, "classic\n").unwrap();
        assert_eq!(load_controls(&path), Some(false));
        save_controls(&path, true).unwrap();
        assert_eq!(load_controls(&path), Some(true));
        fs::write(&path, "sideways\n").unwrap();
        assert_eq!(load_controls(&path), None);
        fs::write(&path, "x".repeat(65)).unwrap();
        assert_eq!(load_controls(&path), None);
    }

    #[test]
    fn the_display_sidecar_round_trips_and_bad_files_are_ignored() {
        use crate::display::{Filter, FitMode, PixelAspect};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(DISPLAY_FILE);
        assert_eq!(load_display(&path), None);
        let settings = DisplaySettings {
            scale: 3,
            fit: FitMode::FillWindow,
            filter: Filter::Linear,
            aspect: PixelAspect::Wide,
            scanlines: false,
            scanline_intensity: 0.3,
        };
        save_display(&path, settings).unwrap();
        assert_eq!(load_display(&path), Some(settings));
        fs::write(&path, "scale=9\n").unwrap();
        assert_eq!(load_display(&path), None);
        fs::write(&path, "x".repeat(257)).unwrap();
        assert_eq!(load_display(&path), None);
    }
}
