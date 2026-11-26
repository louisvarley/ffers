//! Native, versioned commander saves and the port's save-file helpers.
//!
//! # Port-native format (not DOS/JJFFE compatible)
//!
//! The original writes a binary commander file through `F65`/`F66` and
//! `F170`/`F171` (a version dword, a 284-byte global block, 105 table blobs, 150
//! object records, their payloads and a zero-run-compressed/XOR-scrambled body).
//! The port writes **JSON**: a bounded, validated, human-auditable document
//! named `quick.json` or `commander-NNNNNNNN.json`. The two formats are
//! deliberately not interchangeable. A real commander file is recognised and
//! left untouched, and [`legacy_import`] reads the fields this build can prove
//! from the `F170` body without writing anywhere; see `docs/notes/persistence.md`.
//!
//! Version 2 is the first full save. It adds the commander (name, rank slots and
//! mission slots), the navigator/system state (system, clock, legal record,
//! docking/landing stage) and the port flags that a plain [`Flight`] does not
//! carry. Version 1 (`ffe-rs-experimental`) snapshots are still accepted
//! read-only: they become a version-2 [`SaveGame`] with no navigator state.
//!
//! Every file is read with a size bound and must be a regular, non-symlinked
//! file. Writes go to a temporary file in the same directory, are synced, and
//! are then renamed over the destination, so a failed write cannot truncate the
//! previous save. Numbered saves are still new-only.

use crate::flight::Flight;
use crate::navigation::NavigatorSave;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The native save format's tag.
pub const FORMAT: &str = "ffe-rs-save";
/// The current native save version.
pub const VERSION: u32 = 2;
/// The tag of the development snapshots this module can still read.
pub const LEGACY_FORMAT: &str = "ffe-rs-experimental";
/// The largest accepted save. The navigator state is small; this is deliberately
/// far below any memory-dump size.
pub const MAX_SAVE_BYTES: usize = 1024 * 1024;
/// The quick-save file inside the `saves` directory.
pub const QUICK_SAVE: &str = "quick.json";
/// The automatic save on docking (`enhancement-18`).
pub const AUTO_SAVE: &str = "autosave.json";
/// The stem of the numbered save files.
pub const NUMBERED_PREFIX: &str = "commander-";
/// The original's commander name (`DATA_004963`) used when none is supplied.
pub const DEFAULT_COMMANDER: &str = "Jameson";

/// The source's commander file name limit (`DATA_009117`, 20 bytes) is not a
/// port limit, but keeps imported names sane.
const MAX_COMMANDER_NAME: usize = 32;
const MAX_MISSIONS: usize = 256;

/// The commander fields that live outside [`Flight`].
///
/// `ranks` are the two words `DATA_008912`/`DATA_008913` the galaxy tables use
/// for a system's control; the port does not change them yet. `missions` is a
/// reserved list: the port has no mission engine, so it is empty in play, but it
/// round-trips so a future mission subsystem can use the same file version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commander {
    #[serde(default = "default_commander")]
    pub name: String,
    #[serde(default)]
    pub ranks: [u32; 2],
    #[serde(default)]
    pub missions: Vec<Mission>,
}

impl Default for Commander {
    fn default() -> Self {
        Self {
            name: default_commander(),
            ranks: [0; 2],
            missions: Vec::new(),
        }
    }
}

fn default_commander() -> String {
    DEFAULT_COMMANDER.to_owned()
}

/// A reserved mission record. The port has no mission engine; this shape keeps
/// the save forward-compatible without claiming a source layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mission {
    pub id: u32,
    /// The system code the mission belongs to, when it has one.
    #[serde(default)]
    pub system: Option<u32>,
    #[serde(default)]
    pub completed: bool,
}

/// One complete native save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveGame {
    pub format: String,
    pub version: u32,
    pub commander: Commander,
    /// The flight/player state, kept under the version-1 field name so a v1
    /// snapshot's body can be migrated mechanically.
    pub state: Flight,
    /// The navigator, when the game had one. Version-1 snapshots and the
    /// prototype-only loop have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation: Option<NavigatorSave>,
    /// Whether the original flight model (key O) was driving the ship.
    #[serde(default)]
    pub original_flight: bool,
    /// The console time-acceleration step (0 = 1x .. 4 = 10000x).
    #[serde(default)]
    pub acceleration: usize,
}

impl SaveGame {
    /// A v2 save for a state with no navigator (prototype loop or an old body).
    pub fn from_flight(state: &Flight) -> Self {
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            commander: Commander::default(),
            state: state.clone(),
            navigation: None,
            original_flight: false,
            acceleration: 0,
        }
    }

    /// A complete save.
    pub fn new(commander: Commander, state: Flight, navigation: Option<NavigatorSave>) -> Self {
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            commander,
            state,
            navigation,
            original_flight: false,
            acceleration: 0,
        }
    }

    /// Check every field this module owns. The caller still refuses a save in
    /// the game-over state, which is a main-loop concern rather than a property
    /// of the document.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "Unsupported save format {:?}; expected {FORMAT:?}.",
                self.format
            ));
        }
        if self.version != VERSION {
            return Err(format!(
                "Unsupported save version {}; this build writes version {VERSION}.",
                self.version
            ));
        }
        self.commander.validate()?;
        self.state.validate()?;
        if let Some(navigation) = &self.navigation {
            navigation.validate()?;
        }
        if self.acceleration > 4 {
            return Err("Save has an invalid time-acceleration step.".to_owned());
        }
        Ok(())
    }
}

impl Commander {
    fn validate(&self) -> Result<(), String> {
        if self.name.len() > MAX_COMMANDER_NAME {
            return Err(format!(
                "Commander name is longer than {MAX_COMMANDER_NAME} bytes."
            ));
        }
        if self.name.chars().any(char::is_control) {
            return Err("Commander name contains a control character.".to_owned());
        }
        if self.ranks.iter().any(|rank| *rank > 1_000_000) {
            return Err("Commander rank is out of range.".to_owned());
        }
        if self.missions.len() > MAX_MISSIONS {
            return Err("Save has too many mission records.".to_owned());
        }
        let mut ids = std::collections::BTreeSet::new();
        for mission in &self.missions {
            if mission.id > 1_000_000 || !ids.insert(mission.id) {
                return Err("Save has an invalid or duplicate mission id.".to_owned());
            }
        }
        Ok(())
    }
}

/// A version-1 development snapshot, read-only.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveV1 {
    format: String,
    version: u32,
    state: Flight,
}

/// The version/format probe. It deliberately does not reject unknown fields so
/// a newer file still gets a clear "newer version" message instead of a serde
/// shape error.
#[derive(Deserialize)]
struct Header {
    format: String,
    version: u32,
}

/// Encode a full save. Validation happens before serialization.
pub fn encode_game(save: &SaveGame) -> Result<Vec<u8>, String> {
    save.validate()?;
    let mut bytes =
        serde_json::to_vec_pretty(save).map_err(|error| format!("Cannot encode save: {error}"))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_SAVE_BYTES {
        return Err(format!(
            "Save is {} bytes, larger than the {MAX_SAVE_BYTES}-byte limit.",
            bytes.len()
        ));
    }
    Ok(bytes)
}

/// Object +0xd0 (Combat) is authoritative, including zero after F947.
/// Older versions could leave a stale loadout class after damage or servicing.
fn migrate_drive(state: &mut Flight) {
    state.loadout.drive = state.combat.drive;
}

/// Decode a full save, accepting a version-1 snapshot as a migrated v2 document.
pub fn decode_game(bytes: &[u8]) -> Result<SaveGame, String> {
    if bytes.len() > MAX_SAVE_BYTES {
        return Err(format!("Save exceeds the {MAX_SAVE_BYTES}-byte limit."));
    }
    if bytes.is_empty() {
        return Err("Save is empty.".to_owned());
    }
    if looks_like_legacy_binary(bytes) {
        return Err(legacy_binary_message(bytes));
    }
    let header: Header = serde_json::from_slice(bytes)
        .map_err(|error| format!("Invalid save (not JSON): {error}"))?;
    match (header.format.as_str(), header.version) {
        (FORMAT, VERSION) => {
            let mut save: SaveGame = serde_json::from_slice(bytes)
                .map_err(|error| format!("Invalid save v{VERSION}: {error}"))?;
            migrate_drive(&mut save.state);
            save.validate()?;
            Ok(save)
        }
        (LEGACY_FORMAT, 1) => {
            let old: SaveV1 = serde_json::from_slice(bytes)
                .map_err(|error| format!("Invalid version-1 snapshot: {error}"))?;
            if old.format != LEGACY_FORMAT || old.version != 1 {
                return Err("Invalid version-1 snapshot header.".to_owned());
            }
            let mut state = old.state;
            migrate_drive(&mut state);
            state.validate()?;
            Ok(SaveGame {
                format: FORMAT.to_owned(),
                version: VERSION,
                commander: Commander::default(),
                state,
                navigation: None,
                original_flight: false,
                acceleration: 0,
            })
        }
        (LEGACY_FORMAT, version) => Err(format!(
            "Unsupported version-1 snapshot version {version}; only 1 is known."
        )),
        (FORMAT, version) => Err(format!(
            "Save version {version} is newer than this build's version {VERSION}."
        )),
        (format, _) => Err(format!(
            "Unsupported save format {format:?}; expected {FORMAT:?}."
        )),
    }
}

/// Backwards-compatible v1-style helper: encode a flight state as a full v2 save
/// with the default commander. The playable uses [`encode_game`].
pub fn encode(state: &Flight) -> Result<Vec<u8>, String> {
    encode_game(&SaveGame::from_flight(state))
}

/// Backwards-compatible helper: decode any supported save and return its flight
/// state. The playable uses [`decode_game`] to keep the commander and navigator.
pub fn decode(bytes: &[u8]) -> Result<Flight, String> {
    decode_game(bytes).map(|save| save.state)
}

/// Read and validate a save file. The path must be a regular, non-symlinked file
/// no larger than [`MAX_SAVE_BYTES`].
pub fn load_game(path: &Path) -> Result<SaveGame, String> {
    let bytes = read_bounded(path)?;
    decode_game(&bytes)
}

/// Backwards-compatible file loader returning only the flight state.
pub fn load(path: &Path) -> Result<Flight, String> {
    load_game(path).map(|save| save.state)
}

/// Read at most `MAX_SAVE_BYTES + 1` bytes from a regular file.
fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "{}: a save must be a regular file, not a link or directory.",
            path.display()
        ));
    }
    if metadata.len() > MAX_SAVE_BYTES as u64 {
        return Err(format!(
            "{}: save is {} bytes, larger than the {MAX_SAVE_BYTES}-byte limit.",
            path.display(),
            metadata.len()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_SAVE_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error| format!("{}: cannot read save: {error}", path.display()))?;
    if bytes.len() > MAX_SAVE_BYTES {
        return Err(format!(
            "{}: save exceeds the {MAX_SAVE_BYTES}-byte limit.",
            path.display()
        ));
    }
    Ok(bytes)
}

/// Write `save` to `path` atomically: a sibling temporary file is created,
/// synced and renamed over the destination. On failure the old file remains.
pub fn save_atomic(path: &Path, save: &SaveGame) -> Result<(), String> {
    let bytes = encode_game(save)?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("{}: save has no parent directory", path.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{}: invalid save name", path.display()))?;
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
                    .write_all(&bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|error| format!("{}: save write failed: {error}", temp.display()));
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&temp);
                    return Err(error);
                }
                match fs::rename(&temp, path) {
                    Ok(()) => return Ok(()),
                    Err(error) => {
                        let _ = fs::remove_file(&temp);
                        return Err(format!(
                            "{}: cannot replace the save atomically: {error}",
                            path.display()
                        ));
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt += 1;
                if attempt > 100 {
                    return Err(format!(
                        "{}: cannot make a temporary save name",
                        path.display()
                    ));
                }
                temp = parent.join(format!(".{name}.{}.{attempt}.tmp", std::process::id()));
            }
            Err(error) => {
                return Err(format!(
                    "{}: cannot create a temporary save: {error}",
                    temp.display()
                ));
            }
        }
    }
}

/// The quick-save path inside `directory`.
pub fn quick_path(directory: &Path) -> PathBuf {
    directory.join(QUICK_SAVE)
}

/// Write the quick save, replacing the previous one atomically.
pub fn save_quick(directory: &Path, save: &SaveGame) -> Result<PathBuf, String> {
    let path = quick_path(directory);
    save_atomic(&path, save)?;
    Ok(path)
}

/// The automatic save's path inside `directory` (`enhancement-18`).
pub fn auto_path(directory: &Path) -> PathBuf {
    directory.join(AUTO_SAVE)
}

/// Write the automatic save (`enhancement-18`, on docking), replacing the
/// previous one atomically. It is its own slot: the quick save and the named
/// saves are never touched.
pub fn save_auto(directory: &Path, save: &SaveGame) -> Result<PathBuf, String> {
    let path = auto_path(directory);
    save_atomic(&path, save)?;
    Ok(path)
}

/// Load the quick save.
pub fn load_quick(directory: &Path) -> Result<(PathBuf, SaveGame), String> {
    let path = quick_path(directory);
    let save = load_game(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok((path, save))
}

/// The numbered saves in `directory`, lowest first. Only exact
/// `commander-NNNNNNNN.json` names are considered.
pub fn numbered_saves(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(number) = name
            .strip_prefix(NUMBERED_PREFIX)
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        if number.len() != 8 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        paths.push(entry.path());
    }
    paths.sort();
    Ok(paths)
}

/// Delete only a quick or numbered save discovered in this save directory.
pub fn delete_save(directory: &Path, path: &Path) -> Result<(), String> {
    let allowed = all_saves(directory)?
        .iter()
        .any(|candidate| candidate == path);
    if !allowed {
        return Err(format!("{}: not a commander save", path.display()));
    }
    fs::remove_file(path)
        .map_err(|error| format!("{}: cannot delete save: {error}", path.display()))
}

/// Write a new numbered save without overwriting any existing one. The bytes go
/// to a sibling temporary file and are linked into place, so an interrupted
/// writer cannot leave a partial numbered save.
pub fn save_numbered(directory: &Path, save: &SaveGame) -> Result<PathBuf, String> {
    let next = next_numbered(directory)?;
    let path = directory.join(format!("{NUMBERED_PREFIX}{next:08}.json"));
    write_new_atomic(&path, &encode_game(save)?)?;
    Ok(path)
}

/// The number the next `commander-NNNNNNNN.json` save would take.
fn next_numbered(directory: &Path) -> Result<u32, String> {
    let existing = numbered_saves(directory)?;
    let next = match existing.last() {
        Some(path) => {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.strip_prefix(NUMBERED_PREFIX))
                .ok_or("Invalid numbered save name")?
                .parse::<u32>()
                .map_err(|error| error.to_string())?
                + 1
        }
        None => 1,
    };
    if next > 99_999_999 {
        return Err("Numbered save limit reached.".to_owned());
    }
    Ok(next)
}

/// The longest save name the save menu's text box takes (without `.json`).
pub const MAX_SAVE_NAME: usize = 24;
/// The fixed ending of every save file; the save menu shows it after the box's
/// text and it cannot be edited.
pub const SAVE_EXTENSION: &str = ".json";

/// The name the save menu offers: the next free `commander-NNNNNNNN`.
pub fn default_save_name(directory: &Path) -> String {
    let next = next_numbered(directory).unwrap_or(1);
    format!("{NUMBERED_PREFIX}{next:08}")
}

/// Why `name` (typed without `.json`) cannot be a save name, or `None` when it
/// can. A name is 1 to [`MAX_SAVE_NAME`] letters, digits, spaces, `-` or `_`,
/// with no space at either end, and is not the quick or automatic save's or a Windows
/// device name, so it can only ever name a plain file in the save directory.
pub fn save_name_error(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return Some("Type a name for the save.");
    }
    if name.chars().count() > MAX_SAVE_NAME {
        return Some("That name is too long.");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'))
    {
        return Some("Use letters, digits, spaces, - and _ only.");
    }
    if name.starts_with(' ') || name.ends_with(' ') {
        return Some("A name cannot start or end with a space.");
    }
    let upper = name.to_ascii_uppercase();
    let device = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.len() == 4
            && upper.as_bytes()[3].is_ascii_digit());
    if device || upper == "QUICK" || upper == "AUTOSAVE" {
        return Some("That name is reserved.");
    }
    None
}

/// The path a save named `name` (without `.json`) would have.
pub fn named_path(directory: &Path, name: &str) -> Result<PathBuf, String> {
    if let Some(reason) = save_name_error(name) {
        return Err(reason.to_owned());
    }
    Ok(directory.join(format!("{name}{SAVE_EXTENSION}")))
}

/// Write a new save called `name` (without `.json`). An existing save of that
/// name is never replaced.
pub fn save_named(directory: &Path, name: &str, save: &SaveGame) -> Result<PathBuf, String> {
    let path = named_path(directory, name)?;
    if path.exists() {
        return Err(format!("A save named {name} already exists."));
    }
    write_new_atomic(&path, &encode_game(save)?)?;
    Ok(path)
}

/// Every save the load and delete lists offer: the quick save first, then the
/// rest (numbered or named) newest first.
pub fn all_saves(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", directory.display()))?;
        let name = entry.file_name();
        let Some(stem) = name
            .to_str()
            .and_then(|name| name.strip_suffix(SAVE_EXTENSION))
        else {
            continue;
        };
        if stem.eq_ignore_ascii_case("quick") || save_name_error(stem).is_some() {
            continue;
        }
        let modified = entry.metadata().and_then(|meta| meta.modified()).ok();
        found.push((modified, entry.path()));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut paths: Vec<PathBuf> = found.into_iter().map(|(_, path)| path).collect();
    // The automatic save after the quick save (its name is reserved, so the loop
    // above skipped it).
    let auto = auto_path(directory);
    if auto.is_file() {
        paths.insert(0, auto);
    }
    let quick = quick_path(directory);
    if quick.is_file() {
        paths.insert(0, quick);
    }
    Ok(paths)
}

/// Write a file that must not already exist, via a synced temporary file and a
/// hard link. Unlike rename, the link refuses to replace an existing name on
/// every platform.
fn write_new_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{}: save has no parent directory", path.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{}: invalid save name", path.display()))?;
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
                    .write_all(bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|error| format!("{}: save write failed: {error}", temp.display()));
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&temp);
                    return Err(error);
                }
                let linked = fs::hard_link(&temp, path).map_err(|error| {
                    format!("{}: cannot create the save: {error}", path.display())
                });
                let _ = fs::remove_file(&temp);
                return linked;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt += 1;
                if attempt > 100 {
                    return Err(format!(
                        "{}: cannot make a temporary save name",
                        path.display()
                    ));
                }
                temp = parent.join(format!(".{name}.{}.{attempt}.tmp", std::process::id()));
            }
            Err(error) => {
                return Err(format!(
                    "{}: cannot create a temporary save: {error}",
                    temp.display()
                ));
            }
        }
    }
}

/// Load the quick save, or the highest numbered save when there is no quick
/// save. No silent fallback: a present but invalid quick save is an error.
pub fn load_quick_or_latest(directory: &Path) -> Result<(PathBuf, SaveGame), String> {
    let quick = quick_path(directory);
    if quick.exists() {
        return load_quick(directory);
    }
    let paths = numbered_saves(directory)?;
    let path = paths
        .last()
        .ok_or_else(|| format!("No save in {}; F5 quick-saves.", directory.display()))?;
    let save = load_game(path)?;
    Ok((path.clone(), save))
}

/// Write the old experimental snapshot format's successor: a new-only file at
/// `path` with the default commander. Kept for the prototype loop and callers
/// that only have a [`Flight`].
pub fn save_new(path: &Path, state: &Flight) -> Result<(), String> {
    write_new_atomic(path, &encode_game(&SaveGame::from_flight(state))?)
}

// ---------------------------------------------------------------------------
// Original (F65/F170) commander files: read-only import
// ---------------------------------------------------------------------------
//
// `F65` writes `version`, a 284-byte global block, 105 table blobs (sizes from
// the running build's descriptors), 0x96 object records and their payloads; the
// caller then appends the `F170` body. The blob sizes are build constants, so
// the F65 prefix cannot be re-derived from the source alone. The supplied
// original delimits it as:
//
//   0x0000  4 zero dwords + 5 x 0xc8 paper records + 0x16 bytes (F69's block)
//   0x040e  u32 version 1
//   0x0412  the 284-byte global block
//   0x052e  0x28 bytes of table blobs (the other 104 descriptors are empty)
//   0x0556  0x96 x 0x24 object records (21 present)
//   0x1a6e  the present objects' payloads (560 bytes)
//   0x1c9e  the `F170` body (0x891c bytes plus the checksum)
//
// None of that prefix is needed for the commander fields, and its blob sizes
// differ between builds, so the importer locates the body structurally instead
// of trusting an offset. It validates the body rather than the prefix:
//
// * `F168`/`F169` XOR every 16-bit word with the low word of an accumulator
//   seeded with `0x12345678 + DATA_008669` (0xef0a, set by `F219`) and then do
//   `acc = (acc + (i16)word) * 2`. The accumulator after the last word is the
//   checksum stored in the trailing dword.
// * `F166`/`F167` expand/compress zero runs as `0x00, n` (n = following zeros,
//   at most 255).
// * The body's plain layout is the object array (`0x984a`), the 0x40
//   every-other-dword globals (`DATA_008622..8653`), `0x40` raw bytes
//   (`DATA_008663`), the DOS player block (`0x5535`, `DATA_008804`) and a final
//   string block. JJFFE 2.8a7 uses 0x556a for the player block and `0x51` for
//   the string block; the supplied original's sections are 0x5535 and 135
//   bytes, so the block sizes above are the file's, not JJFFE's reads.

/// Smallest accepted original commander file (`F185` rejects under `0x258`).
pub const LEGACY_MIN_BYTES: usize = 0x258;
/// Largest accepted original commander file (`F185`: `DATA_008670` = 100000).
pub const LEGACY_MAX_BYTES: usize = 0x1_86a0;
/// The first dword of an `F170` body.
pub const LEGACY_BODY_MAGIC: u32 = 0x11;
/// The `F168`/`F169` XOR/checksum seed: `0x12345678 + 0xef0a`.
pub const LEGACY_SCRAMBLE_SEED: u32 = 0x1234_5678 + 0xef0a;

/// The uncompressed object array (`DATA_009133`).
const LEGACY_OBJECT_BYTES: usize = 0x984a;
/// `F170` copies `[DATA_008622 + 8*i + 4]` for every 8-byte slot up to
/// `DATA_008653`; the real distance is 0x200 bytes, so 0x40 dwords.
const LEGACY_SCATTER_DWORDS: usize = 0x40;
/// The raw `0x40` bytes `F170` copies from `DATA_008663`.
const LEGACY_RAW_BYTES: usize = 0x40;
/// The DOS-era player block (`DATA_008804`) size. The DOS release's block is
/// 0x5535 bytes; JJFFE's own `DATA_008804` is 0x556a, so F171's fixed 0x556a
/// read takes the following section's first 0x35 bytes. The file's section
/// boundary is 0x5535 (the text block starts right after it).
pub const LEGACY_PLAYER_BYTES: usize = 0x5535;
/// A lower bound for the final string block (`jjffe-master` writes `0x51`).
const LEGACY_STRING_MIN_BYTES: usize = 0x51;
/// An upper bound for the final string block, to reject runaway input.
const LEGACY_STRING_MAX_BYTES: usize = 0x400;

/// String field offsets inside the final block, as established from the
/// supplied original. Its layout is `3 bytes, 20-byte save name, 6-byte
/// extension, 20-byte commander name, 4 bytes, 17-byte system name, star text`.
const LEGACY_TEXT_SAVE_NAME: usize = 0x03;
const LEGACY_TEXT_SAVE_EXTENSION: usize = 0x17;
const LEGACY_TEXT_COMMANDER: usize = 0x1d;
const LEGACY_TEXT_SYSTEM: usize = 0x35;
const LEGACY_TEXT_STAR: usize = 0x46;
/// `DATA_009117` is a 20-byte commander/file-name buffer.
const LEGACY_NAME_BYTES: usize = 20;

/// The DOS-era player block uses the source `gmstruct` layout from
/// `jjffe-master/srctools/gmstruct.txt`; these are the offsets used by the
/// reference's F171 output. (`docs/notes/persistence.md` records the earlier
/// `+0x22E` observation as an artefact of the seven-dword scatter misparse.)
pub mod legacy_offsets {
    /// `DATA_008804`'s game clock: tics and days.
    pub const CLOCK_TICS: usize = 0x000;
    pub const CLOCK_DAYS: usize = 0x004;
    /// `DATA_008818`/`DATA_008820`, the shared random pair (the first is 24-bit).
    pub const RANDOM_A: usize = 0x024;
    pub const RANDOM_B: usize = 0x028;
    /// `DATA_008857`: the player's object index in the object array.
    pub const PLAYER_OBJECT: usize = 0x0c0;
    /// `DATA_008888`: credits.
    pub const CREDITS: usize = 0x114;
    /// `DATA_008889`/`DATA_008891`: hold capacity and used tonnes.
    pub const CARGO_CAPACITY: usize = 0x118;
    pub const CARGO_USED: usize = 0x120;
    /// `DATA_008901`/`DATA_008902`/`DATA_008903`: fuel (tenths of a light year).
    pub const HYDROGEN_FUEL: usize = 0x15a;
    pub const MILITARY_FUEL: usize = 0x15c;
    pub const RADIOACTIVES: usize = 0x16e;
    /// `+0x1BC`/`+0x1D2` (`DATA_008919`/`DATA_008920`): commander name copies.
    pub const COMMANDER_NAME: usize = 0x1bc;
    pub const COMMANDER_NAME_UPPER: usize = 0x1d2;
    /// The name field's maximum length (both copies are 22 bytes).
    pub const NAME_BYTES: usize = 22;
}

/// The commander fields recovered from the DOS-era player block, plus the raw
/// block so a caller can match any other field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyPlayer {
    pub name: String,
    pub credits: u32,
    pub cargo_capacity: u32,
    pub cargo_used: u32,
    /// `DATA_008901`: hydrogen fuel in tenths of a light year.
    pub hydrogen_fuel: u16,
    pub military_fuel: u16,
    pub radioactives: u16,
    /// `DATA_008804`: the tics within the day.
    pub clock_tics: u32,
    /// `DATA_008807`: the day counter.
    pub clock_days: u32,
    /// `DATA_008818`: 24-bit shared random part 1.
    pub random_a: u32,
    /// `DATA_008820`: shared random part 2.
    pub random_b: u32,
    /// `DATA_008857`: the player's object index.
    pub player_object: u32,
    /// The player object's model (`PhysObj +0x82`), the ship's table index.
    pub ship_model: u32,
    /// The player object's registration text (`PhysObj +0x124`).
    pub ship_name: String,
    /// The whole 0x5535-byte block exactly as the original un-mangles it.
    pub raw: Vec<u8>,
}

/// The fields recovered from an original commander file.
///
/// Only the `F170` body is decoded: its magic and trailing checksum are checked
/// and every byte must belong to one of the five sections, so a malformed or
/// truncated file is rejected rather than partially accepted. The F65 tables
/// and object payloads are not needed for (and are not claimed by) this import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyCommander {
    /// `DATA_008656`'s commander field (also copied in the player block).
    pub commander_name: String,
    /// The current system's name, when the string block carries one.
    pub system_name: Option<String>,
    /// The current star's description, when present.
    pub star_description: Option<String>,
    /// The save file's base name (the string the save dialog stored).
    pub save_name: Option<String>,
    /// The raw extension text (the dot is included, e.g. `._01`).
    pub save_extension: Option<String>,
    /// Whether the player block contains a second copy of the name.
    pub player_name_confirmed: bool,
    /// The commander fields and the raw player block (`DATA_008804`).
    pub player: LegacyPlayer,
    /// The 0x984a-byte object array exactly as the original un-mangles it.
    pub objects: Vec<u8>,
    /// Offset of the `F170` body's first byte in the file.
    pub body_offset: usize,
    /// Body length including the trailing checksum dword.
    pub body_bytes: usize,
    /// The checksum stored in the body's trailing dword.
    pub body_checksum: u32,
    /// The every-other-dword globals (`DATA_008622..8653`, 0x40 dwords).
    pub header_dwords: [u32; LEGACY_SCATTER_DWORDS],
    /// Decoded size of the object-array section.
    pub object_bytes: usize,
    /// Decoded size of the player-block section.
    pub player_bytes: usize,
    /// Decoded size of the final string section.
    pub string_bytes: usize,
}

struct LegacyBody {
    offset: usize,
    bytes: usize,
    checksum: u32,
    header_dwords: [u32; LEGACY_SCATTER_DWORDS],
    object_bytes: usize,
    player_bytes: usize,
    string_bytes: usize,
    objects: Vec<u8>,
    player: Vec<u8>,
    text: Vec<u8>,
}

/// Read an original commander file **without modifying it**.
///
/// The file is validated as an `F170` body: the magic, the trailing `F168`
/// checksum and an exact decompression of all five sections must agree, and the
/// body must end at the end of the file. Several matches are treated as an
/// error instead of guessing. No byte is ever written.
pub fn legacy_import(bytes: &[u8]) -> Result<LegacyCommander, String> {
    if bytes.len() < LEGACY_MIN_BYTES {
        return Err(format!(
            "Not an original commander file: {} bytes is below the {LEGACY_MIN_BYTES}-byte (F185) minimum.",
            bytes.len()
        ));
    }
    if bytes.len() > LEGACY_MAX_BYTES {
        return Err(format!(
            "Not an original commander file: {} bytes is above the {LEGACY_MAX_BYTES}-byte (F185) maximum.",
            bytes.len()
        ));
    }

    let mut bodies = Vec::new();
    let mut attempts = 0usize;
    let mut start = 0usize;
    while start + 8 <= bytes.len() {
        // Cheap filter: the first word only unscrambles to 0x0011 when the
        // candidate start is right. The full check below is authoritative.
        let first = u16::from_le_bytes([bytes[start], bytes[start + 1]]);
        if first ^ (LEGACY_SCRAMBLE_SEED as u16) == LEGACY_BODY_MAGIC as u16 {
            // A real file has about one candidate; a pathological one must not
            // make the scan quadratic.
            if attempts == 16 {
                return Err(
                    "Too many candidate F170 data blocks; the file is not a valid original commander save."
                        .to_owned(),
                );
            }
            attempts += 1;
            if let Some(body) = legacy_parse_body(bytes, start) {
                bodies.push(body);
                if bodies.len() > 4 {
                    return Err(
                        "More than one F170 data block matches; refusing to guess which one is the commander."
                            .to_owned(),
                    );
                }
            }
        }
        start += 1;
    }

    let body = match bodies.len() {
        0 => {
            return Err(
                "No valid F170 data block: the file is not an original commander save (or it is truncated/corrupt)."
                    .to_owned(),
            )
        }
        1 => bodies.pop().expect("one body"),
        _ => {
            return Err(
                "More than one F170 data block matches; refusing to guess which one is the commander."
                    .to_owned(),
            )
        }
    };

    let commander_name = legacy_text_field(&body.text, LEGACY_TEXT_COMMANDER, LEGACY_NAME_BYTES)
        .ok_or_else(|| "The F170 string block has no commander name.".to_owned())?;
    let system_name = legacy_text_field(
        &body.text,
        LEGACY_TEXT_SYSTEM,
        LEGACY_TEXT_STAR - LEGACY_TEXT_SYSTEM,
    );
    let star_description = legacy_text_field(
        &body.text,
        LEGACY_TEXT_STAR,
        body.text.len().saturating_sub(LEGACY_TEXT_STAR),
    );
    let save_name = legacy_text_field(
        &body.text,
        LEGACY_TEXT_SAVE_NAME,
        LEGACY_TEXT_SAVE_EXTENSION - LEGACY_TEXT_SAVE_NAME,
    );
    let save_extension = legacy_text_field(
        &body.text,
        LEGACY_TEXT_SAVE_EXTENSION,
        LEGACY_TEXT_COMMANDER - LEGACY_TEXT_SAVE_EXTENSION,
    );
    let player_name_confirmed = legacy_contains_name(&body.player, commander_name.as_bytes());
    let player = legacy_player(&body.player, &body.objects, &commander_name);

    Ok(LegacyCommander {
        commander_name,
        system_name,
        star_description,
        save_name,
        save_extension,
        player_name_confirmed,
        player,
        objects: body.objects,
        body_offset: body.offset,
        body_bytes: body.bytes,
        body_checksum: body.checksum,
        header_dwords: body.header_dwords,
        object_bytes: body.object_bytes,
        player_bytes: body.player_bytes,
        string_bytes: body.string_bytes,
    })
}

/// Read the commander fields from the DOS-era player block. The DOS block is
/// the source `gmstruct` layout shifted by [`LEGACY_PLAYER_SHIFT`], so every
/// field is read at `gmstruct offset + the shift`; `game/FIRSTENC._01`'s cash
/// (`10,000`), name, cargo and clock match those offsets. A block whose name
/// or object index is not present keeps the string block's commander name and
/// zero fields instead of failing the container.
fn legacy_player(player: &[u8], objects: &[u8], commander_name: &str) -> LegacyPlayer {
    use legacy_offsets as at;
    let u16_at = |offset: usize| -> u16 {
        player
            .get(offset..offset + 2)
            .and_then(|b| b.try_into().ok())
            .map(u16::from_le_bytes)
            .unwrap_or(0)
    };
    let u32_at = |offset: usize| -> u32 {
        player
            .get(offset..offset + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0)
    };
    let name = legacy_text_field(player, at::COMMANDER_NAME, at::NAME_BYTES)
        .or_else(|| legacy_text_field(player, at::COMMANDER_NAME_UPPER, at::NAME_BYTES))
        .unwrap_or_else(|| commander_name.to_owned());
    let player_object = u32_at(at::PLAYER_OBJECT);
    let object = usize::try_from(player_object)
        .ok()
        .and_then(|index| index.checked_mul(0x152))
        .and_then(|at| 0x74usize.checked_add(at))
        .and_then(|at| objects.get(at..at + 0x152));
    let ship_model = object
        .and_then(|o| o.get(0x82..0x86))
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0);
    let ship_name = object
        .and_then(|o| legacy_text_field(o, 0x124, 16))
        .unwrap_or_default();
    LegacyPlayer {
        name,
        credits: u32_at(at::CREDITS),
        cargo_capacity: u32_at(at::CARGO_CAPACITY),
        cargo_used: u32_at(at::CARGO_USED),
        hydrogen_fuel: u16_at(at::HYDROGEN_FUEL),
        military_fuel: u16_at(at::MILITARY_FUEL),
        radioactives: u16_at(at::RADIOACTIVES),
        clock_tics: u32_at(at::CLOCK_TICS),
        clock_days: u32_at(at::CLOCK_DAYS),
        random_a: u32_at(at::RANDOM_A) & 0x00ff_ffff,
        random_b: u32_at(at::RANDOM_B),
        player_object,
        ship_model,
        ship_name,
        raw: player.to_vec(),
    }
}

/// Validate one candidate `F170` body that runs from `offset` to the end of the
/// file (`offset` + body + 4-byte checksum).
fn legacy_parse_body(bytes: &[u8], offset: usize) -> Option<LegacyBody> {
    let rest = bytes.get(offset..)?;
    let body_len = rest.len().checked_sub(4)?;
    if body_len % 2 != 0 || body_len < LEGACY_STRING_MIN_BYTES {
        return None;
    }
    let (plain, checksum) = legacy_unscramble(&rest[..body_len]);
    let stored = u32::from_le_bytes([
        rest[body_len],
        rest[body_len + 1],
        rest[body_len + 2],
        rest[body_len + 3],
    ]);
    if checksum != stored {
        return None;
    }
    if u32::from_le_bytes([plain[0], plain[1], plain[2], plain[3]]) != LEGACY_BODY_MAGIC {
        return None;
    }

    let mut pos = 4usize;
    let objects = legacy_expand(&plain, &mut pos, LEGACY_OBJECT_BYTES)?;
    if objects.len() != LEGACY_OBJECT_BYTES {
        return None;
    }
    let mut header_dwords = [0u32; LEGACY_SCATTER_DWORDS];
    for slot in header_dwords.iter_mut() {
        let chunk = plain.get(pos..pos + 4)?;
        *slot = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        pos += 4;
    }
    pos = pos.checked_add(LEGACY_RAW_BYTES)?;
    if pos > plain.len() {
        return None;
    }
    let player = legacy_expand(&plain, &mut pos, LEGACY_PLAYER_BYTES)?;
    if player.len() != LEGACY_PLAYER_BYTES {
        return None;
    }
    // The last section has no established size: read it to the end of the body.
    let text = legacy_expand(&plain, &mut pos, LEGACY_STRING_MAX_BYTES)?;
    if pos != plain.len() || text.len() < LEGACY_STRING_MIN_BYTES {
        return None;
    }

    Some(LegacyBody {
        offset,
        bytes: body_len + 4,
        checksum,
        header_dwords,
        object_bytes: objects.len(),
        player_bytes: player.len(),
        string_bytes: text.len(),
        objects,
        player,
        text,
    })
}

/// `F169`: XOR each little-endian word with the accumulator's low word, then
/// fold the unscrambled word back in. Returns the data and the final checksum.
fn legacy_unscramble(body: &[u8]) -> (Vec<u8>, u32) {
    let mut plain = body.to_vec();
    let mut acc = LEGACY_SCRAMBLE_SEED;
    for word in 0..plain.len() / 2 {
        let at = word * 2;
        let scrambled = i16::from_le_bytes([plain[at], plain[at + 1]]);
        let key = (acc & 0xffff) as u16;
        let original = (scrambled as u16 ^ key).to_le_bytes();
        plain[at] = original[0];
        plain[at + 1] = original[1];
        acc = acc
            .wrapping_add(i16::from_le_bytes(original) as i32 as u32)
            .wrapping_mul(2);
    }
    (plain, acc)
}

/// `F166`: copy bytes, expanding each `0x00, n` into `n` further zeros, until
/// `limit` bytes have been produced or the input ends.
fn legacy_expand(src: &[u8], pos: &mut usize, limit: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(limit.min(src.len() * 2));
    while out.len() < limit {
        let Some(&byte) = src.get(*pos) else { break };
        *pos += 1;
        out.push(byte);
        if byte == 0 {
            // A trailing literal zero is possible at the very end of the last
            // section (F170 pads the plain body to an even length).
            let Some(&run) = src.get(*pos) else { break };
            *pos += 1;
            let run = run as usize;
            let take = run.min(limit - out.len());
            out.resize(out.len() + take, 0);
            if take < run {
                // `F166` stops at the destination limit.
                break;
            }
        }
    }
    Some(out)
}

/// Read a NUL-terminated printable ASCII field from the final string block.
fn legacy_text_field(block: &[u8], offset: usize, max: usize) -> Option<String> {
    let end = offset.checked_add(max)?.min(block.len());
    let slice = block.get(offset..end)?;
    let len = slice.iter().position(|byte| *byte == 0)?;
    let text = &slice[..len];
    if text.is_empty() || !text.iter().all(|byte| (0x20..0x7f).contains(byte)) {
        return None;
    }
    String::from_utf8(text.to_vec()).ok()
}

/// Look for `name` (any case) in the player block. The original keeps two
/// copies of the commander name; its layout is build-specific, so this is a
/// confirmation, not a source of truth.
fn legacy_contains_name(player: &[u8], name: &[u8]) -> bool {
    !name.is_empty()
        && player
            .windows(name.len())
            .any(|window| window.eq_ignore_ascii_case(name))
}

/// The original commander file is at least `0x258` bytes (`F185`). A native
/// save is a JSON object, so any other bounded file is reported as a legacy
/// commander file rather than as invalid JSON.
fn looks_like_legacy_binary(bytes: &[u8]) -> bool {
    if bytes.first() == Some(&b'{') {
        return false;
    }
    if !(LEGACY_MIN_BYTES..=LEGACY_MAX_BYTES).contains(&bytes.len()) {
        return false;
    }
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 1
        || legacy_import(bytes).is_ok()
}

fn legacy_binary_message(bytes: &[u8]) -> String {
    match legacy_import(bytes) {
        Ok(legacy) => format!(
            "Legacy commander file detected ({} bytes): commander {:?} in {}. The port \
             imports original files read-only; the file was not modified. Use a native \
             save (version {VERSION}).",
            bytes.len(),
            legacy.commander_name,
            legacy.system_name.as_deref().unwrap_or("an unknown system"),
        ),
        Err(_) => format!(
            "Legacy commander file detected ({} bytes) but its F170 body is not valid; \
             the file was not modified. Use a native save (version {VERSION}).",
            bytes.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::Mode;
    use std::io::Write;

    #[test]
    fn stale_loadout_cannot_restore_a_destroyed_or_degraded_drive() {
        for class in [0, 1, 2, 10, 14] {
            let mut state = Flight::default();
            state.loadout.drive = 14;
            state.combat.drive = class;
            migrate_drive(&mut state);
            assert_eq!(state.loadout.drive, class);
            assert_eq!(state.combat.drive, class);
        }
    }

    fn sample_navigation() -> NavigatorSave {
        NavigatorSave {
            system_code: 0x1234_5678,
            clock: 1234.5,
            start_clock: 100.0,
            world_epoch: Some(100.0),
            new_game: None,
            page: 2,
            view: crate::navigation::View::Places,
            selected: 3,
            target: Some(2),
            locked: Some(4),
            destination: Some(crate::navigation::Destination {
                code: 0x00ab_cdef,
                position: [1, -2, 3],
            }),
            altitude: 0x00b7_1b00,
            docked: Some(1),
            legal: ffe_core::npc::Legal {
                wanted: true,
                alert: true,
                chase: 1,
                illegal_fire: true,
                offences: 3,
                fines: [0, 6000, 0, 0, 0, 0],
                records: [0; 6],
                points: [0; 6],
            },
            offended: None,
            ship_lock: Some(7),
            missile_lock: Some(9),
            missile_view: Some(11),
            sequence: Some(crate::navigation::SequenceSave {
                station: 1,
                state: 0x1a,
                timeout: 0xffff,
                drawn: Some((0x1a, 0x1234)),
                launching: true,
            }),
            takeoff: Some(0x24),
            landed: Some(crate::navigation::LandedSave {
                state: 0x34,
                index: 2,
            }),
            final_leg: true,
            spawned: Some(5),
            ships: Vec::new(),
            traffic: Vec::new(),
            random: None,
            deep_space: false,
            markets: Vec::new(),
            script_canisters: Vec::new(),
            script_sites: Vec::new(),
            runtime: None,
        }
    }

    fn sample() -> SaveGame {
        let mut state = Flight::free_flight();
        state.credits = 12_345;
        state.hold = vec![0; ffe_core::market::NAMES];
        state.hold[0] = 3;
        state.hold[1] = 2;
        state.combat = crate::combat::Combat::new_game();
        state.loadout = ffe_core::equipment::Loadout::new_game();
        state.shields = [0x40, 0x40];
        state.hull = [4608, 4608];
        state.axes = Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        state.system_flight = true;
        SaveGame {
            format: FORMAT.to_owned(),
            version: VERSION,
            commander: Commander {
                name: "Jameson".to_owned(),
                ranks: [4, 7],
                missions: vec![Mission {
                    id: 12,
                    system: Some(0x00ab_cdef),
                    completed: false,
                }],
            },
            state,
            navigation: Some(sample_navigation()),
            original_flight: true,
            acceleration: 3,
        }
    }

    #[test]
    fn a_complete_save_round_trips_every_field() {
        let save = sample();
        let bytes = encode_game(&save).unwrap();
        let loaded = decode_game(&bytes).unwrap();
        assert_eq!(loaded, save);
        assert!(loaded.state.validate().is_ok());
        loaded.validate().unwrap();
    }

    /// Save migration (release-3): every field a later build added carries a
    /// default, so the oldest version-2 save, written before any of them,
    /// still loads. The test finds the optional fields (each loads when it
    /// alone is missing), removes them all at once and loads the result.
    #[test]
    fn the_oldest_version_two_saves_still_load() {
        let bytes = encode_game(&sample()).unwrap();
        let full: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let mut optional = Vec::new();
        for section in ["state", "navigation"] {
            let keys: Vec<String> = full[section].as_object().unwrap().keys().cloned().collect();
            for key in keys {
                let mut trial = full.clone();
                trial[section].as_object_mut().unwrap().remove(&key);
                if decode_game(&serde_json::to_vec(&trial).unwrap()).is_ok() {
                    optional.push((section, key));
                }
            }
        }
        // The fields added since the first version-2 saves are among them.
        for added in [
            "ships",
            "traffic",
            "random",
            "deep_space",
            "markets",
            "world_epoch",
        ] {
            assert!(
                optional
                    .iter()
                    .any(|(section, key)| *section == "navigation" && key == added),
                "navigation.{added} is required"
            );
        }
        for added in ["missions", "mining", "hull_heat", "loadout"] {
            assert!(
                optional
                    .iter()
                    .any(|(section, key)| *section == "state" && key == added),
                "state.{added} is required"
            );
        }
        let mut oldest = full.clone();
        for (section, key) in &optional {
            oldest[*section].as_object_mut().unwrap().remove(key);
        }
        let loaded = decode_game(&serde_json::to_vec(&oldest).unwrap())
            .expect("a save without every later field loads");
        loaded.validate().unwrap();
        assert_eq!(loaded.version, VERSION);
    }

    #[test]
    fn version_one_snapshots_migrate_and_default_the_new_fields() {
        let v1 = br#"{
            "format": "ffe-rs-experimental",
            "version": 1,
            "state": {
                "mode": "Flight",
                "position": [0.0, 0.0, -180.0],
                "velocity": [0.0, 0.0, 5.0],
                "yaw": 0.0,
                "pitch": 0.0,
                "credits": 1000,
                "cargo": 0,
                "stock": 50,
                "ticks": 0,
                "docking_requested": false
            }
        }"#;
        let loaded = decode_game(v1).unwrap();
        assert_eq!(loaded.version, VERSION);
        assert_eq!(loaded.state.mode, Mode::Flight);
        assert_eq!(loaded.state.credits, 1000);
        assert_eq!(loaded.commander, Commander::default());
        assert_eq!(loaded.navigation, None);
        assert!(!loaded.original_flight);
    }

    #[test]
    fn corrupt_newer_and_unknown_files_are_rejected_clearly() {
        let bytes = encode(&Flight::default()).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap().to_owned();
        let truncated = &bytes[..bytes.len() - 2];
        assert!(decode(truncated).is_err());
        assert!(decode(&[]).is_err());
        assert!(decode(&vec![b' '; MAX_SAVE_BYTES + 1]).is_err());
        for (from, to) in [
            ("\"version\": 2", "\"version\": 99"),
            ("\"version\": 2", "\"version\": 1"),
            ("\"format\": \"ffe-rs-save\"", "\"format\": \"other\""),
            ("\"credits\": 1000", "\"credits\": 1000000001"),
            ("\"cargo\": 0", "\"cargo\": 11"),
            ("\"stock\": 50", "\"stock\": 1001"),
            ("\"ticks\": 0", "\"ticks\": 0, \"surprise\": 0"),
        ] {
            let changed = text.replace(from, to);
            assert_ne!(changed, text);
            assert!(decode(changed.as_bytes()).is_err(), "{from} -> {to}");
        }
    }

    #[test]
    fn invalid_state_is_rejected_on_encode_and_decode() {
        let mut invalid = Flight::default();
        invalid.velocity[0] = 1.0;
        assert!(encode(&invalid).is_err());
        let mut invalid = Flight::free_flight();
        invalid.yaw = f64::INFINITY;
        assert!(encode(&invalid).is_err());
        let mut dead = Flight::free_flight();
        dead.death = Some(crate::death::Death::exploding());
        assert!(encode(&dead).is_err());
        let mut save = sample();
        save.acceleration = 5;
        assert!(encode_game(&save).is_err());
        save.acceleration = 0;
        save.commander.name = "x".repeat(MAX_COMMANDER_NAME + 1);
        assert!(encode_game(&save).is_err());
    }

    #[test]
    fn atomic_writes_replace_and_leave_a_good_file_on_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("save.json");
        let first = sample();
        save_atomic(&path, &first).unwrap();
        assert_eq!(load_game(&path).unwrap(), first);
        let mut second = first.clone();
        second.state.credits = 54321;
        save_atomic(&path, &second).unwrap();
        assert_eq!(load_game(&path).unwrap(), second);
        // A temporary file from an interrupted writer never replaces the save.
        let temp = directory.path().join(".save.json.999.tmp");
        let mut file = fs::File::create(&temp).unwrap();
        file.write_all(b"{ broken").unwrap();
        assert_eq!(load_game(&path).unwrap(), second);
        drop(file);
        fs::remove_file(temp).unwrap();
    }

    #[test]
    fn quick_and_numbered_paths_are_new_only_and_latest_prefers_quick() {
        let directory = tempfile::tempdir().unwrap();
        assert!(load_quick_or_latest(directory.path()).is_err());
        let save = sample();
        let quick = save_quick(directory.path(), &save).unwrap();
        assert_eq!(quick, quick_path(directory.path()));
        let numbered = save_numbered(directory.path(), &save).unwrap();
        assert!(numbered.ends_with("commander-00000001.json"));
        assert!(save_numbered(directory.path(), &save)
            .unwrap()
            .ends_with("commander-00000002.json"));
        let (path, loaded) = load_quick_or_latest(directory.path()).unwrap();
        assert_eq!(path, quick);
        assert_eq!(loaded, save);
        fs::remove_file(quick).unwrap();
        let (path, loaded) = load_quick_or_latest(directory.path()).unwrap();
        assert!(path.ends_with("commander-00000002.json"));
        assert_eq!(loaded, save);
    }

    #[test]
    fn save_names_are_plain_file_stems() {
        for good in ["commander-00000001", "Jameson", "my save 2", "a_b-c", "x"] {
            assert_eq!(save_name_error(good), None, "{good}");
        }
        let too_long = "x".repeat(MAX_SAVE_NAME + 1);
        for bad in [
            "",
            " lead",
            "trail ",
            "a/b",
            r"a\b",
            "..",
            "a.b",
            "con",
            "NUL",
            "com1",
            "Lpt9",
            "quick",
            "Quick",
            "autosave",
            "AutoSave",
            "caf\u{e9}",
            too_long.as_str(),
        ] {
            assert!(save_name_error(bad).is_some(), "{bad:?}");
        }
        assert_eq!(save_name_error(&"x".repeat(MAX_SAVE_NAME)), None);
    }

    #[test]
    fn a_named_save_is_new_only_listed_loadable_and_deletable() {
        let directory = tempfile::tempdir().unwrap();
        let save = sample();
        // The offered name is the next numbered one, and it is a valid name.
        let default = default_save_name(directory.path());
        assert_eq!(default, "commander-00000001");
        assert_eq!(save_name_error(&default), None);
        let named = save_named(directory.path(), "My Commander", &save).unwrap();
        assert!(named.ends_with("My Commander.json"));
        // A name is never overwritten.
        assert!(save_named(directory.path(), "My Commander", &save)
            .unwrap_err()
            .contains("already exists"));
        // Names that could leave the directory are refused before any write.
        assert!(save_named(directory.path(), "../escape", &save).is_err());
        // Numbering ignores the custom name; both appear in the lists, the quick
        // save first.
        save_quick(directory.path(), &save).unwrap();
        let numbered = save_numbered(directory.path(), &save).unwrap();
        assert!(numbered.ends_with("commander-00000001.json"));
        let listed = all_saves(directory.path()).unwrap();
        assert_eq!(listed[0], quick_path(directory.path()));
        assert!(listed.contains(&named) && listed.contains(&numbered));
        assert_eq!(listed.len(), 3);
        // The named save loads and can be deleted; other files cannot.
        load_game(&named).unwrap();
        fs::write(directory.path().join("config.cfg"), b"x").unwrap();
        assert!(delete_save(directory.path(), &directory.path().join("config.cfg")).is_err());
        delete_save(directory.path(), &named).unwrap();
        assert!(!named.exists());
    }

    /// enhancement-18: the automatic save replaces itself, is listed after the quick
    /// save, loads and can be deleted.
    #[test]
    fn the_automatic_save_has_its_own_slot_after_the_quick_save() {
        let directory = tempfile::tempdir().unwrap();
        let save = sample();
        let auto = save_auto(directory.path(), &save).unwrap();
        assert_eq!(auto, auto_path(directory.path()));
        save_auto(directory.path(), &save).unwrap();
        let named = save_named(directory.path(), "Jameson", &save).unwrap();
        let quick = save_quick(directory.path(), &save).unwrap();
        assert_eq!(
            all_saves(directory.path()).unwrap(),
            [quick, auto.clone(), named]
        );
        load_game(&auto).unwrap();
        delete_save(directory.path(), &auto).unwrap();
        assert!(!auto.exists());
    }

    #[test]
    fn delete_save_removes_only_a_quick_or_numbered_save_in_the_directory() {
        let directory = tempfile::tempdir().unwrap();
        let save = sample();
        let quick = save_quick(directory.path(), &save).unwrap();
        let numbered = save_numbered(directory.path(), &save).unwrap();
        delete_save(directory.path(), &quick).unwrap();
        assert!(!quick.exists());
        assert!(numbered.exists());
        let unrelated = directory.path().join("notes.txt");
        fs::write(&unrelated, "keep").unwrap();
        assert!(delete_save(directory.path(), &unrelated).is_err());
        assert!(unrelated.exists());
        delete_save(directory.path(), &numbered).unwrap();
        assert!(!numbered.exists());
    }

    #[test]
    fn a_legacy_binary_is_recognised_and_left_untouched() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("JAMESON.000");
        let mut bytes = vec![0x41u8; 0x300];
        bytes[0..4].copy_from_slice(&1u32.to_le_bytes());
        fs::write(&path, &bytes).unwrap();
        let error = load_game(&path).unwrap_err();
        assert!(error.contains("Legacy commander file detected"), "{error}");
        assert_eq!(
            fs::read(&path).unwrap(),
            bytes,
            "the original was not written"
        );
    }

    // -- original commander import ------------------------------------------

    /// `F167`: compress zero runs as `0x00, n` (n following zeros, at most 255).
    fn legacy_compress(src: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut index = 0;
        while index < src.len() {
            let byte = src[index];
            out.push(byte);
            index += 1;
            if byte == 0 {
                let mut run = 0;
                while index < src.len() && src[index] == 0 && run < 255 {
                    run += 1;
                    index += 1;
                }
                out.push(run as u8);
            }
        }
        out
    }

    /// `F168`: XOR every word and append the checksum dword.
    fn legacy_scramble(plain: &[u8]) -> Vec<u8> {
        assert!(plain.len() % 2 == 0);
        let mut encoded = plain.to_vec();
        let mut acc = LEGACY_SCRAMBLE_SEED;
        for word in 0..plain.len() / 2 {
            let at = word * 2;
            let original = i16::from_le_bytes([plain[at], plain[at + 1]]);
            let key = (acc & 0xffff) as u16;
            let scrambled = (original as u16 ^ key).to_le_bytes();
            encoded[at] = scrambled[0];
            encoded[at + 1] = scrambled[1];
            acc = acc.wrapping_add(original as i32 as u32).wrapping_mul(2);
        }
        let mut out = encoded;
        out.extend_from_slice(&acc.to_le_bytes());
        out
    }

    /// The final string block in the layout established from the specimen.
    fn legacy_text_block(name: &str, system: &str, star: &str) -> Vec<u8> {
        let mut block = vec![0u8; 135];
        block[LEGACY_TEXT_SAVE_NAME..LEGACY_TEXT_SAVE_NAME + 4].copy_from_slice(b"TEST");
        block[LEGACY_TEXT_SAVE_EXTENSION..LEGACY_TEXT_SAVE_EXTENSION + 4].copy_from_slice(b".123");
        block[LEGACY_TEXT_COMMANDER..LEGACY_TEXT_COMMANDER + name.len()]
            .copy_from_slice(name.as_bytes());
        block[LEGACY_TEXT_SYSTEM..LEGACY_TEXT_SYSTEM + system.len()]
            .copy_from_slice(system.as_bytes());
        block[LEGACY_TEXT_STAR..LEGACY_TEXT_STAR + star.len()].copy_from_slice(star.as_bytes());
        block
    }

    fn legacy_body(name: &str, system: &str, star: &str) -> Vec<u8> {
        let objects = vec![0u8; LEGACY_OBJECT_BYTES];
        let raw = [0u8; LEGACY_RAW_BYTES];
        let mut player = vec![0u8; LEGACY_PLAYER_BYTES];
        player[legacy_offsets::COMMANDER_NAME..legacy_offsets::COMMANDER_NAME + name.len()]
            .copy_from_slice(name.as_bytes());
        let upper = name.to_uppercase();
        player[legacy_offsets::COMMANDER_NAME_UPPER
            ..legacy_offsets::COMMANDER_NAME_UPPER + upper.len()]
            .copy_from_slice(upper.as_bytes());
        player[legacy_offsets::CREDITS..legacy_offsets::CREDITS + 4]
            .copy_from_slice(&4321u32.to_le_bytes());
        let text = legacy_text_block(name, system, star);

        let mut plain = Vec::new();
        plain.extend_from_slice(&LEGACY_BODY_MAGIC.to_le_bytes());
        plain.extend_from_slice(&legacy_compress(&objects));
        for value in [0u32, 0, 0x2f2, 0x7f67f1, 0xff0b00, 0xf2, 0]
            .into_iter()
            .chain(std::iter::repeat_n(0, LEGACY_SCATTER_DWORDS - 7))
        {
            plain.extend_from_slice(&value.to_le_bytes());
        }
        plain.extend_from_slice(&raw);
        plain.extend_from_slice(&legacy_compress(&player));
        plain.extend_from_slice(&legacy_compress(&text));
        if plain.len() % 2 == 1 {
            plain.push(0);
        }
        legacy_scramble(&plain)
    }

    /// A synthetic original file: an arbitrary F65 prefix followed by the body.
    fn legacy_file(name: &str, system: &str, star: &str) -> Vec<u8> {
        let mut file = vec![0u8; 0x40];
        file.extend_from_slice(b"Imperial Herald\0");
        file.extend_from_slice(&[0xa5u8; 0x30]);
        file.extend_from_slice(&legacy_body(name, system, star));
        file
    }

    #[test]
    fn an_original_commander_body_imports_read_only() {
        let file = legacy_file("Test Pilot", "Sol", "Yellow star");
        let original = file.clone();
        let legacy = legacy_import(&file).unwrap();
        assert_eq!(legacy.commander_name, "Test Pilot");
        assert_eq!(legacy.system_name.as_deref(), Some("Sol"));
        assert_eq!(legacy.star_description.as_deref(), Some("Yellow star"));
        assert_eq!(legacy.save_name.as_deref(), Some("TEST"));
        assert_eq!(legacy.save_extension.as_deref(), Some(".123"));
        assert!(legacy.player_name_confirmed);
        assert_eq!(legacy.body_offset, 0x40 + 0x10 + 0x30);
        assert_eq!(legacy.object_bytes, LEGACY_OBJECT_BYTES);
        assert_eq!(legacy.player_bytes, LEGACY_PLAYER_BYTES);
        assert_eq!(legacy.string_bytes, 135);
        assert_eq!(
            legacy.body_bytes,
            file.len() - legacy.body_offset,
            "the body runs to the end of the file"
        );
        assert_eq!(file, original, "the import never rewrites the input");
    }

    #[test]
    fn an_original_commander_body_needs_its_commander_name() {
        let file = legacy_file("", "Sol", "Yellow star");
        let error = legacy_import(&file).unwrap_err();
        assert!(error.contains("no commander name"), "{error}");
    }

    #[test]
    fn a_truncated_original_commander_file_is_rejected() {
        let file = legacy_file("Test Pilot", "Sol", "Yellow star");
        for cut in [1, 5, 40] {
            let error = legacy_import(&file[..file.len() - cut]).unwrap_err();
            assert!(error.contains("No valid F170 data block"), "{error}");
        }
        let short = vec![0u8; LEGACY_MIN_BYTES - 1];
        assert!(legacy_import(&short)
            .unwrap_err()
            .contains("below the 600-byte"));
    }

    #[test]
    fn a_corrupt_original_commander_body_is_rejected() {
        let mut file = legacy_file("Test Pilot", "Sol", "Yellow star");
        let body = legacy_import(&file).unwrap().body_offset;
        file[body + 20] ^= 0xff;
        assert!(legacy_import(&file)
            .unwrap_err()
            .contains("No valid F170 data block"));
    }

    #[test]
    fn a_native_save_is_not_an_original_commander_file() {
        let bytes = encode_game(&sample()).unwrap();
        assert!(legacy_import(&bytes)
            .unwrap_err()
            .contains("No valid F170 data block"));
    }

    /// Local-only: reads the supplied original `game/FIRSTENC._01` (an
    /// "Imperial Herald" newspaper-era FFE save made by a fresh Jameson in
    /// Gateway). Never writes and never copies the file.
    #[test]
    #[ignore = "Local-only: needs game/FIRSTENC._01"]
    fn the_supplied_original_commander_file_imports_read_only() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("game")
            .join("FIRSTENC._01");
        let Ok(bytes) = fs::read(&path) else {
            eprintln!("skipping: {} is absent", path.display());
            return;
        };
        let before = bytes.clone();
        let legacy = legacy_import(&bytes).unwrap();

        assert_eq!(legacy.commander_name, "Jameson");
        assert_eq!(legacy.system_name.as_deref(), Some("Gateway"));
        assert_eq!(legacy.star_description.as_deref(), Some("Red giant star"));
        assert_eq!(legacy.save_name.as_deref(), Some("FirstEnc"));
        assert_eq!(legacy.save_extension.as_deref(), Some("._01"));
        assert!(legacy.player_name_confirmed);
        assert_eq!(legacy.body_offset, 0x1c9e);
        assert_eq!(legacy.body_bytes, 0x891c + 4);
        assert_eq!(legacy.body_checksum, 0x8175_6aca);
        assert_eq!(legacy.object_bytes, LEGACY_OBJECT_BYTES);
        assert_eq!(legacy.player_bytes, LEGACY_PLAYER_BYTES);
        assert_eq!(legacy.string_bytes, 135);
        assert_eq!(
            &legacy.header_dwords[..7],
            &[0, 0, 0x2f2, 0x7f67f1, 0xff0b00, 0xf2, 0]
        );
        // The DOS-era player block: the source `gmstruct` offsets.
        assert_eq!(legacy.player.name, "Jameson");
        assert_eq!(legacy.player.credits, 10_000);
        assert_eq!(legacy.player.cargo_capacity, 7);
        assert_eq!(legacy.player.cargo_used, 1);
        assert_eq!(legacy.player.hydrogen_fuel, 1);
        assert_eq!(legacy.player.military_fuel, 0);
        assert_eq!(legacy.player.radioactives, 0);
        assert_eq!(legacy.player.clock_tics, 1_014_497_784);
        assert_eq!(legacy.player.clock_days, 1_187_064);
        assert_eq!(legacy.player.random_a, 0x38_acd1);
        assert_eq!(legacy.player.random_b, 0x0fc1_68d5);
        assert_eq!(legacy.player.player_object, 87);
        assert_eq!(legacy.player.ship_model, 31);
        assert_eq!(legacy.player.ship_name, "TY-198");
        assert_eq!(legacy.player.raw.len(), LEGACY_PLAYER_BYTES);
        assert_eq!(fs::read(&path).unwrap(), before, "the file was not written");

        // `decode_game` refuses it instead of treating it as JSON.
        let error = decode_game(&bytes).unwrap_err();
        assert!(error.contains("Legacy commander file detected"), "{error}");
    }
}
