//! The launcher's own choices (`saves/launcher.cfg`): where the game files
//! are, and how to start the Classic game. The game's settings
//! live in `saves/config.cfg` (`ffe_rs::settings`), which the launcher edits
//! through the game's own reader and writer.

use std::path::{Path, PathBuf};

pub const LAUNCHER_FILE: &str = "launcher.cfg";
const MAX_BYTES: u64 = 16 * 1024;

/// The start points the game accepts (`--start`), with their names.
/// The code is the game's `--start` type (F998's arms; the list index is the code).
pub const STARTS: [(u8, &str, &str); 4] = [
    (
        0,
        "Gateway (Hope)",
        "The default start: a Saker Mk III on the pad at Old Blackelk, on Hope in the Gateway system.",
    ),
    (
        1,
        "Ross 154",
        "An Eagle on the pad of a starport in the Ross 154 system.",
    ),
    (
        2,
        "Sol (Mars)",
        "An Eagle on the pad of a starport on Mars, in the Sol system.",
    ),
    (
        3,
        "Lave Station",
        "Docked at Lave Station in the Lave system.",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    pub game: PathBuf,
    pub start: u8,
    pub skip_intro: bool,
    pub record: bool,
    /// A folder of the CD videos chosen by the player; `None` uses the game
    /// folder's own `DATA` folder when it has one.
    pub videos: Option<PathBuf>,
    /// Look for a newer release on GitHub when the launcher starts.
    pub check_updates: bool,
}

/// Where the videos are and how many clips (AVI files) are there.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Videos {
    /// The folder looked in (a chosen one even when it holds no videos).
    pub folder: Option<PathBuf>,
    pub count: usize,
    /// The player chose the folder (rather than the game's own `DATA`).
    pub custom: bool,
}

impl Videos {
    pub fn found(&self) -> bool {
        self.count > 0
    }
}

/// The videos for `game`: the player's `custom` folder (or the `DATA` folder
/// inside it) when set, else the game folder's own `DATA` folder.
pub fn resolve_videos(game: &Path, custom: Option<&Path>) -> Videos {
    let folder = match custom {
        Some(path) => Some(ffe_rs::fmv::video_folder(path)),
        None => ffe_rs::fmv::data_folder(game),
    };
    let count = folder.as_deref().map_or(0, ffe_rs::fmv::count_avi);
    Videos {
        folder,
        count,
        custom: custom.is_some(),
    }
}

impl Launcher {
    pub fn defaults(root: &Path) -> Self {
        Self {
            game: root.join("game"),
            start: 0,
            skip_intro: false,
            record: false,
            videos: None,
            check_updates: true,
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::from("# ffe-rs launcher\nversion=1\n");
        out.push_str(&format!("game={}\n", self.game.display()));
        out.push_str(&format!("start={}\n", self.start));
        out.push_str(&format!("skip_intro={}\n", on_off(self.skip_intro)));
        out.push_str(&format!("record={}\n", on_off(self.record)));
        out.push_str(&format!("check_updates={}\n", on_off(self.check_updates)));
        if let Some(videos) = &self.videos {
            out.push_str(&format!("videos={}\n", videos.display()));
        }
        out
    }

    /// Read `text` over `self`; unknown or malformed lines are ignored.
    pub fn read(&mut self, text: &str) {
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "game" if !value.is_empty() => self.game = PathBuf::from(value),
                "start" => {
                    if let Ok(start) = value.parse::<u8>() {
                        if STARTS.iter().any(|(code, _, _)| *code == start) {
                            self.start = start;
                        }
                    }
                }
                "skip_intro" => self.skip_intro = value == "on",
                "record" => self.record = value == "on",
                "check_updates" => self.check_updates = value != "off",
                "videos" => {
                    self.videos = (!value.is_empty()).then(|| PathBuf::from(value));
                }
                _ => {}
            }
        }
    }

    pub fn load(saves: &Path, root: &Path) -> Self {
        let mut launcher = Self::defaults(root);
        let path = saves.join(LAUNCHER_FILE);
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.is_file() && meta.len() <= MAX_BYTES {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    launcher.read(&text);
                }
            }
        }
        if detect_game(&launcher.game).is_none() {
            if let Some(game) = detect_game(root) {
                launcher.game = game;
            }
        }
        launcher
    }

    pub fn save(&self, saves: &Path) -> Result<(), String> {
        std::fs::create_dir_all(saves).map_err(|e| e.to_string())?;
        let path = saves.join(LAUNCHER_FILE);
        let temporary = saves.join(format!("{LAUNCHER_FILE}.tmp"));
        std::fs::write(&temporary, self.to_text()).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

/// The folder that holds `saves` and `recordings`: the nearest of the
/// launcher's folder and four parents with saved launcher settings, a
/// `game\FIRSTENC.EXE` (the checkout's layout, `target\release\` below it)
/// or the workspace's `Cargo.toml`, else the launcher's own folder. The game
/// searches the same way.
pub fn find_root() -> PathBuf {
    // An AppImage runs from a read-only mount that vanishes when it exits: the player's `game`,
    // `saves` and `music.bin` belong beside the `.AppImage` file itself (`$APPIMAGE`).
    let beside = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .and_then(|image| image.parent().map(Path::to_path_buf))
        .filter(|dir| dir.is_dir())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    // A macOS app bundle must not be written into: keep the player's files beside the `.app` when
    // that folder already holds them, else in `~/Library/Application Support/ffe-rs`.
    if let Some(app_folder) = std::env::current_exe()
        .ok()
        .and_then(|exe| bundle_parent(&exe))
    {
        if has_player_files(&app_folder) {
            return app_folder;
        }
        if let Some(home) = std::env::var_os("HOME") {
            let support = Path::new(&home).join("Library/Application Support/ffe-rs");
            if std::fs::create_dir_all(&support).is_ok() {
                return support;
            }
        }
    }
    root_from(&beside)
}

/// For a program inside `Name.app/Contents/MacOS/`, the folder that holds `Name.app`.
pub fn bundle_parent(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    (macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension()? == "app")
        .then(|| app.parent().map(Path::to_path_buf))?
}

/// Whether `dir` already holds a game folder or the launcher's saved settings.
fn has_player_files(dir: &Path) -> bool {
    dir.join("saves").join(LAUNCHER_FILE).is_file()
        || dir.join("game").join("FIRSTENC.EXE").is_file()
}

pub fn root_from(beside: &Path) -> PathBuf {
    let candidates = || beside.ancestors().take(5);
    candidates()
        .find(|dir| dir.join("saves").join(LAUNCHER_FILE).is_file())
        .or_else(|| candidates().find(|dir| dir.join("game").join("FIRSTENC.EXE").is_file()))
        .or_else(|| {
            candidates().find(|dir| dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir())
        })
        .map_or_else(|| beside.to_path_buf(), Path::to_path_buf)
}

/// The game executable beside the launcher.
pub fn game_executable() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let name = if cfg!(windows) {
        "ffe-rs.exe"
    } else {
        "ffe-rs"
    };
    let path = dir.join(name);
    path.is_file().then_some(path)
}

/// `yyyymmdd-hhmmss` (UTC) for a recording's file name.
pub fn stamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_round_trips() {
        let root = Path::new("/games/ffe");
        let mut launcher = Launcher::defaults(root);
        launcher.start = 1;
        launcher.skip_intro = true;
        launcher.record = false;
        launcher.check_updates = false;
        launcher.videos = Some(PathBuf::from("/discs/elite3/DATA"));
        let mut read = Launcher::defaults(root);
        read.read(&launcher.to_text());
        assert_eq!(read, launcher);
    }

    #[test]
    fn an_app_bundles_files_belong_outside_it() {
        assert_eq!(
            bundle_parent(Path::new(
                "/Applications/ffers.app/Contents/MacOS/ffe-launcher"
            )),
            Some(PathBuf::from("/Applications"))
        );
        assert_eq!(bundle_parent(Path::new("/games/ffe/ffe-launcher")), None);
        assert_eq!(
            bundle_parent(Path::new("/x/Contents/MacOS/ffe-launcher")),
            None
        );
    }

    #[test]
    fn every_listed_start_round_trips_and_codes_are_list_indices() {
        for (index, (code, _, _)) in STARTS.iter().enumerate() {
            assert_eq!(usize::from(*code), index);
            let mut read = Launcher::defaults(Path::new("/games/ffe"));
            read.read(&format!("start={code}\n"));
            assert_eq!(read.start, *code);
        }
        let mut read = Launcher::defaults(Path::new("/games/ffe"));
        read.read("start=4\n");
        assert_eq!(read.start, 0, "types 4 and 5 repeat the default arm");
    }

    #[test]
    fn videos_come_from_the_games_data_folder_unless_one_is_chosen() {
        let game = tempfile::tempdir().unwrap();
        let none = resolve_videos(game.path(), None);
        assert!(!none.found() && none.folder.is_none() && !none.custom);
        let data = game.path().join("DATA");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("BB1.AVI"), b"x").unwrap();
        std::fs::write(data.join("BB1.BMP"), b"x").unwrap();
        let auto = resolve_videos(game.path(), None);
        assert_eq!((auto.count, auto.custom), (1, false));
        assert_eq!(auto.folder, Some(data));
        // A chosen disc root resolves to its DATA folder and wins.
        let disc = tempfile::tempdir().unwrap();
        std::fs::create_dir(disc.path().join("DATA")).unwrap();
        std::fs::write(disc.path().join("DATA").join("G1.AVI"), b"x").unwrap();
        std::fs::write(disc.path().join("DATA").join("G2.AVI"), b"x").unwrap();
        let chosen = resolve_videos(game.path(), Some(disc.path()));
        assert_eq!((chosen.count, chosen.custom), (2, true));
        // A chosen folder with no videos is kept, and not found.
        let empty = tempfile::tempdir().unwrap();
        let missing = resolve_videos(game.path(), Some(empty.path()));
        assert!(!missing.found() && missing.custom);
        assert_eq!(missing.folder, Some(empty.path().to_path_buf()));
    }

    #[test]
    fn bad_lines_keep_the_defaults() {
        let root = Path::new("/games/ffe");
        let mut read = Launcher::defaults(root);
        read.read("start=9\nnonsense\nrecord=maybe\n");
        assert_eq!(read.start, 0);
        assert!(!read.record);
    }

    #[test]
    fn the_root_is_the_checkout_without_saved_settings() {
        let dir = tempfile::tempdir().unwrap();
        let release = dir.path().join("target").join("release");
        std::fs::create_dir_all(&release).unwrap();
        // A plain folder: the launcher's own.
        assert_eq!(root_from(&release), release);
        // The checkout's game folder.
        std::fs::create_dir_all(dir.path().join("game")).unwrap();
        std::fs::write(dir.path().join("game").join("FIRSTENC.EXE"), b"x").unwrap();
        assert_eq!(root_from(&release), dir.path());
        // Saved launcher settings win.
        std::fs::create_dir_all(release.join("saves")).unwrap();
        std::fs::write(release.join("saves").join(LAUNCHER_FILE), b"x").unwrap();
        assert_eq!(root_from(&release), release);
    }

    #[test]
    fn stamps_are_utc_dates() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_793_667_723), "20261103-010203");
    }
}

/// Find an installation directly or within a selected extracted release folder.
/// Bounded depth and entry count avoid walking an entire drive or following links.
pub fn detect_game(root: &Path) -> Option<PathBuf> {
    let mut pending = std::collections::VecDeque::from([(root.to_path_buf(), 0)]);
    let mut visited = 0;
    while let Some((folder, depth)) = pending.pop_front() {
        visited += 1;
        if visited > 512 {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        let mut children = Vec::new();
        for entry in entries.filter_map(Result::ok).take(2048) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_file()
                && (entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("FIRSTENC.EXE")
                    || entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("FIRSTENC.DAT"))
            {
                return Some(folder);
            }
            if depth < 3 && kind.is_dir() {
                children.push(entry.path());
            }
        }
        children.sort();
        pending.extend(children.into_iter().map(|path| (path, depth + 1)));
    }
    None
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    #[test]
    fn finds_nested_installations_without_following_unrelated_files() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(detect_game(root.path()), None);
        let install = root.path().join("release").join("FFE");
        std::fs::create_dir_all(&install).unwrap();
        std::fs::write(install.join("firstenc.exe"), b"fixture").unwrap();
        assert_eq!(detect_game(root.path()), Some(install.clone()));
        assert_eq!(detect_game(&install), Some(install.clone()));
        std::fs::rename(install.join("firstenc.exe"), install.join("FIRSTENC.DAT")).unwrap();
        assert_eq!(detect_game(root.path()), Some(install));
    }
}

/// Invalidate generated packs before switching source installations.
/// A failed removal leaves the caller on the previous source directory.
pub fn invalidate_assets(root: &Path, previous: &Path, next: &Path) -> Result<(), String> {
    let identity = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if identity(previous) == identity(next) {
        return Ok(());
    }
    for name in ["music.bin", "assets.bin"] {
        let path = root.join(name);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not remove {}: {error}. Game folder was not changed.",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod asset_invalidation_tests {
    use super::*;
    #[test]
    fn switching_folders_removes_generated_packs_but_rechecking_keeps_them() {
        let root = tempfile::tempdir().unwrap();
        let previous = root.path().join("previous");
        let next = root.path().join("next");
        std::fs::create_dir(&previous).unwrap();
        std::fs::create_dir(&next).unwrap();
        let assets = root.path().join("assets.bin");
        let music = root.path().join("music.bin");
        std::fs::write(&assets, b"old pack").unwrap();
        std::fs::write(&music, b"old music").unwrap();
        invalidate_assets(root.path(), &previous, &previous.join(".")).unwrap();
        assert!(assets.is_file());
        assert!(music.is_file());
        invalidate_assets(root.path(), &previous, &next).unwrap();
        assert!(!assets.exists());
        assert!(!music.exists());
        invalidate_assets(root.path(), &previous, &next).unwrap();
    }
    #[test]
    fn removal_failure_is_reported() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("assets.bin")).unwrap();
        assert!(invalidate_assets(root.path(), Path::new("old"), Path::new("new")).is_err());
    }
}
