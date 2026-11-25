//! Update checking and installing from the project's GitHub releases.
//!
//! The launcher asks GitHub for the latest release of the project, compares its tag with the
//! running version and, when the player clicks the update button, downloads the release's
//! package for this platform and replaces the program in place. Nothing is sent to GitHub but
//! the request itself: no identifiers, no game information.
//!
//! * **Windows**: the `ffers-<tag>-Windows-x64.zip` is unpacked, every file in it is checked
//!   against the `SHA256SUMS.txt` it carries and the program files are swapped in place. Only
//!   program files are ever replaced (the executables, the licence texts and the documents).
//!   `game`, `saves`, `recordings`, `FMVs`, the music pack and the player's settings are never
//!   touched; a zip that lists one is refused. A running program cannot be deleted on Windows
//!   but can be renamed, so each existing executable is renamed to `<name>.old` (removed the
//!   next time the launcher starts) before the new one is written, and put back if anything
//!   fails half way.
//! * **Linux**: the `ffers-<tag>-x86_64.AppImage` replaces the running AppImage file
//!   (`$APPIMAGE`). It is downloaded beside it, checked against the release's `SHA256SUMS.txt`
//!   and renamed over it; the running copy keeps working until the launcher restarts.
//! * **macOS**: the `ffers-<tag>-macOS.zip` is checked against the release's `SHA256SUMS.txt`,
//!   unpacked beside `ffers.app` and swapped for it (the old bundle is kept as `ffers.app.old`
//!   until the next start, and put back if the swap fails).
//!
//! Network access uses `curl` (it ships with Windows 10 and later and macOS, and is common on
//! Linux) and unpacking uses the `zip` crate, so the launcher needs no HTTP or TLS crate of its
//! own.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// The GitHub repository releases are read from.
pub const REPO: &str = "louisvarley/ffers";

/// The release asset prefix of the project's own packages. The release also carries the same
/// packages under another publisher's name (`MyRecomp-<tag>-...`); the project's own is preferred.
const ASSET_PREFIX: &str = "ffers-";

/// A platform the project publishes a package for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// `ffers-<tag>-Windows-x64.zip`, written by `tools/package.ps1`.
    Windows,
    /// `ffers-<tag>-x86_64.AppImage`, written by `tools/package-appimage.sh`.
    Linux,
    /// `ffers-<tag>-macOS.zip`, written by `tools/package-macos.sh`.
    MacOs,
}

impl Platform {
    /// The platform this launcher was built for, when the project publishes one for it.
    pub fn current() -> Option<Self> {
        if cfg!(windows) {
            Some(Self::Windows)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(Self::Linux)
        } else if cfg!(target_os = "macos") {
            Some(Self::MacOs)
        } else {
            None
        }
    }

    /// The end of the package's file name, lower case.
    fn asset_suffix(self) -> &'static str {
        match self {
            Self::Windows => "-windows-x64.zip",
            Self::Linux => "-x86_64.appimage",
            Self::MacOs => "-macos.zip",
        }
    }
}

/// Top-level names an update never writes, even if a zip lists them.
const PROTECTED: [&str; 4] = ["game", "saves", "recordings", "fmvs"];
/// File names an update never writes (the player's generated or chosen files).
const PROTECTED_FILES: [&str; 5] = [
    "music.bin",
    "data.bin",
    "assets.bin",
    "launcher.cfg",
    "config.cfg",
];

/// A release found on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The tag, e.g. `v0.9.1`.
    pub tag: String,
    pub version: [u64; 3],
    /// The release's web page (notes and downloads).
    pub page: String,
    /// This platform's package download address, when the release has one.
    pub asset_url: Option<String>,
    /// The release's `SHA256SUMS.txt` (the checksum of every download), when it has one.
    pub sums_url: Option<String>,
    pub notes: String,
}

/// `0.9.1`, `v0.9.1` or `v0.9.1-beta.2` as numbers; `None` for anything else.
pub fn parse_version(text: &str) -> Option<[u64; 3]> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let core = text.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let mut numbers = [0u64; 3];
    for slot in &mut numbers {
        *slot = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(numbers)
}

/// Whether `latest` is a newer version than `current`.
pub fn is_newer(current: [u64; 3], latest: [u64; 3]) -> bool {
    latest > current
}

/// The running version.
pub fn current_version() -> [u64; 3] {
    parse_version(env!("CARGO_PKG_VERSION")).unwrap_or([0, 0, 0])
}

/// Read a release from the GitHub API's `releases/latest` answer.
pub fn parse_release(json: &str) -> Result<Release, String> {
    parse_release_for(json, Platform::current())
}

/// [`parse_release`] for an explicit platform (`None`: no package is wanted).
pub fn parse_release_for(json: &str, platform: Option<Platform>) -> Result<Release, String> {
    let value: Value = serde_json::from_str(json).map_err(|e| format!("Bad release data: {e}"))?;
    let tag = value["tag_name"]
        .as_str()
        .ok_or("The release has no tag.")?
        .to_owned();
    let version =
        parse_version(&tag).ok_or_else(|| format!("Unrecognised release tag {tag:?}."))?;
    let prefix = format!("https://github.com/{REPO}/releases/download/");
    let assets: Vec<(String, String)> = value["assets"]
        .as_array()
        .map(|assets| {
            assets
                .iter()
                .filter_map(|asset| {
                    let name = asset["name"].as_str()?.to_ascii_lowercase();
                    let url = asset["browser_download_url"].as_str()?;
                    url.starts_with(&prefix).then(|| (name, url.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    let packages: Vec<&(String, String)> = platform
        .map(|platform| {
            assets
                .iter()
                .filter(|(name, _)| name.ends_with(platform.asset_suffix()))
                .collect()
        })
        .unwrap_or_default();
    let sums_url = assets
        .iter()
        .find(|(name, _)| name == "sha256sums.txt")
        .map(|(_, url)| url.clone());
    let asset_url = packages
        .iter()
        .find(|(name, _)| name.starts_with(ASSET_PREFIX))
        .or_else(|| packages.first())
        .map(|(_, url)| url.clone());
    Ok(Release {
        page: value["html_url"]
            .as_str()
            .filter(|url| url.starts_with(&format!("https://github.com/{REPO}/")))
            .map_or_else(
                || format!("https://github.com/{REPO}/releases"),
                str::to_owned,
            ),
        notes: value["body"].as_str().unwrap_or_default().to_owned(),
        tag,
        version,
        asset_url,
        sums_url,
    })
}

/// A `curl` command that shows no console window on Windows.
fn curl() -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("curl.exe");
        command.creation_flags(0x0800_0000);
        command
    }
    #[cfg(not(windows))]
    Command::new("curl")
}

/// Ask GitHub for the latest release.
pub fn fetch_latest() -> Result<Release, String> {
    let output = curl()
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--fail",
            "--max-time",
            "20",
            "--proto",
            "=https",
            "--header",
            "Accept: application/vnd.github+json",
            "--header",
            "User-Agent: ffe-rs-launcher",
        ])
        .arg(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .output()
        .map_err(|e| format!("Could not run curl: {e}"))?;
    if !output.status.success() {
        let why = String::from_utf8_lossy(&output.stderr);
        // A project with no release yet answers 404.
        return Err(if why.contains("404") {
            "No release has been published yet.".to_owned()
        } else {
            format!("Could not reach GitHub: {}", why.trim())
        });
    }
    parse_release(&String::from_utf8_lossy(&output.stdout))
}

/// One line of `SHA256SUMS.txt`: the hash and the relative path.
fn parse_sums(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (hash, path) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("Bad checksum line: {line}"))?;
        let path = path.trim().trim_start_matches('*');
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Bad checksum line: {line}"));
        }
        entries.push((hash.to_ascii_lowercase(), path.to_owned()));
    }
    if entries.is_empty() {
        return Err("The package's checksum list is empty.".to_owned());
    }
    Ok(entries)
}

/// Whether `path` (as `SHA256SUMS.txt` writes it, with `/`) may be installed: a plain
/// relative path that stays inside the install folder and is not the player's data.
fn installable(path: &str) -> bool {
    if path.is_empty() || path.contains('\\') || path.contains(':') {
        return false;
    }
    let relative = Path::new(path);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return false;
    }
    let first = path
        .split('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    !PROTECTED.contains(&first.as_str()) && !PROTECTED_FILES.contains(&name.as_str())
}

fn sha256_of(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Check the unpacked package in `staged` and copy its files into `install`.
///
/// Every file the checksum list names must exist and match, every path must be installable, and
/// no other file may be in the package. Existing executables are renamed to `.old` first;
/// on any error the renamed files are put back and the files already written are removed.
/// Returns the paths installed.
pub fn install_staged(staged: &Path, install: &Path) -> Result<Vec<String>, String> {
    let sums_text = std::fs::read_to_string(staged.join("SHA256SUMS.txt"))
        .map_err(|_| "The package has no SHA256SUMS.txt.".to_owned())?;
    let entries = parse_sums(&sums_text)?;
    // Refuse anything unexpected before touching the install folder.
    for (hash, path) in &entries {
        if !installable(path) {
            return Err(format!(
                "The package lists a file that is not installed: {path}"
            ));
        }
        let source = staged.join(path);
        if !source.is_file() {
            return Err(format!("The package is missing {path}."));
        }
        if sha256_of(&source)? != *hash {
            return Err(format!(
                "{path} does not match its checksum; the download is damaged."
            ));
        }
    }
    let listed: Vec<&str> = entries.iter().map(|(_, path)| path.as_str()).collect();
    let mut stray = Vec::new();
    collect_files(staged, staged, &mut stray);
    if let Some(extra) = stray
        .iter()
        .find(|path| path.as_str() != "SHA256SUMS.txt" && !listed.contains(&path.as_str()))
    {
        return Err(format!(
            "The package holds a file its checksum list omits: {extra}"
        ));
    }

    let mut renamed: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut written: Vec<PathBuf> = Vec::new();
    let mut result = Ok(());
    for (_, path) in &entries {
        let destination = install.join(path);
        let step = (|| -> Result<(), String> {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{path}: {e}"))?;
            }
            if destination.is_file() && is_program(&destination) {
                let old = old_name(&destination);
                let _ = std::fs::remove_file(&old);
                std::fs::rename(&destination, &old).map_err(|e| format!("{path}: {e}"))?;
                renamed.push((old, destination.clone()));
            }
            std::fs::copy(staged.join(path), &destination).map_err(|e| format!("{path}: {e}"))?;
            written.push(destination.clone());
            Ok(())
        })();
        if let Err(error) = step {
            result = Err(error);
            break;
        }
    }
    if let Err(error) = result {
        for file in &written {
            let _ = std::fs::remove_file(file);
        }
        for (old, destination) in renamed.iter().rev() {
            let _ = std::fs::rename(old, destination);
        }
        return Err(format!(
            "The update could not be installed and was undone: {error}"
        ));
    }
    Ok(entries.into_iter().map(|(_, path)| path).collect())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// An executable or library: the files a running process keeps locked.
fn is_program(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "exe" | "dll"))
}

fn old_name(path: &Path) -> PathBuf {
    old_name_with(path, ".old")
}

fn old_name_with(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Remove what a previous update left behind: the `.old` executables beside the launcher
/// (Windows), the `ffers.app.old` bundle (macOS) and a half-downloaded `.update` AppImage (Linux).
pub fn remove_leftovers(install: &Path) {
    if let Some(bundle) = running_bundle() {
        let _ = std::fs::remove_dir_all(old_name(&bundle));
    }
    if let Some(appimage) = appimage_path() {
        let _ = std::fs::remove_file(old_name_with(&appimage, ".update"));
    }
    if let Ok(read) = std::fs::read_dir(install) {
        for entry in read.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".exe.old") || name.ends_with(".dll.old") {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

/// The running AppImage file (`$APPIMAGE`), when this launcher was started from one.
fn appimage_path() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("APPIMAGE")?);
    path.is_file().then_some(path)
}

/// The `.app` bundle that holds `exe` (`<name>.app/Contents/MacOS/<exe>`), if there is one.
fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let is_bundle = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle
            .extension()
            .is_some_and(|extension| extension == "app");
    is_bundle.then(|| bundle.to_path_buf())
}

fn running_bundle() -> Option<PathBuf> {
    bundle_of(&std::env::current_exe().ok()?)
}

/// Whether this launcher is a packaged release that can replace itself: not a checkout
/// built with cargo (whose folder holds the workspace `Cargo.toml`), and on Linux and macOS
/// running from the AppImage or the `.app` the release ships.
pub fn can_install(install: &Path) -> bool {
    match Platform::current() {
        Some(Platform::Windows) => !install
            .ancestors()
            .take(5)
            .any(|dir| dir.join("Cargo.toml").is_file()),
        Some(Platform::Linux) => appimage_path().is_some(),
        Some(Platform::MacOs) => running_bundle().is_some(),
        None => false,
    }
}

/// The program to start again after an update. On Linux the running executable lives inside
/// the AppImage's temporary mount, so the AppImage file itself is started.
pub fn relaunch_path(install: &Path) -> Option<PathBuf> {
    if Platform::current() == Some(Platform::Linux) {
        if let Some(appimage) = appimage_path() {
            return Some(appimage);
        }
    }
    let exe = std::env::current_exe().ok()?;
    Some(install.join(exe.file_name()?))
}

/// Download `release`'s package for this platform, check it and install it.
/// Returns the number of files written.
pub fn download_and_install(release: &Release, install: &Path) -> Result<usize, String> {
    let url = release
        .asset_url
        .as_deref()
        .ok_or("This release has no package for this system.")?;
    match Platform::current() {
        Some(Platform::Windows) => install_windows(url, install),
        Some(Platform::Linux) => {
            let target = appimage_path().ok_or("This launcher is not running from an AppImage.")?;
            install_appimage(release, url, &target).map(|()| 1)
        }
        Some(Platform::MacOs) => {
            let bundle = running_bundle().ok_or("This launcher is not running from ffers.app.")?;
            install_bundle(release, url, &bundle).map(|()| 1)
        }
        None => Err("Updates are not available on this system.".to_owned()),
    }
}

/// Download `url` into the file `to` (HTTPS only).
fn download(url: &str, to: &Path) -> Result<(), String> {
    let status = curl()
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--fail",
            "--proto",
            "=https",
            "--max-time",
            "900",
            "--header",
            "User-Agent: ffe-rs-launcher",
            "--output",
        ])
        .arg(to)
        .arg(url)
        .output()
        .map_err(|e| format!("Could not run curl: {e}"))?;
    if status.status.success() {
        Ok(())
    } else {
        Err(format!(
            "The download failed: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ))
    }
}

/// The checksum the release lists for the package at `url`, read from its `SHA256SUMS.txt`.
fn listed_hash(release: &Release, url: &str, work: &Path) -> Result<String, String> {
    let sums_url = release
        .sums_url
        .as_deref()
        .ok_or("The release has no SHA256SUMS.txt, so the download cannot be checked.")?;
    let file = work.join("SHA256SUMS.txt");
    download(sums_url, &file)?;
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    hash_for(&text, url.rsplit('/').next().unwrap_or_default())
}

/// The hash `SHA256SUMS.txt` lists for the file called `name`.
fn hash_for(sums_text: &str, name: &str) -> Result<String, String> {
    parse_sums(sums_text)?
        .into_iter()
        .find(|(_, path)| path == name)
        .map(|(hash, _)| hash)
        .ok_or_else(|| format!("SHA256SUMS.txt does not list {name}."))
}

fn install_windows(url: &str, install: &Path) -> Result<usize, String> {
    let work = std::env::temp_dir().join(format!("ffe-rs-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let outcome = (|| -> Result<usize, String> {
        let zip = work.join("update.zip");
        download(url, &zip)?;
        let unpacked = work.join("unpacked");
        unpack(&zip, &unpacked)?;
        let package = package_folder(&unpacked)?;
        install_staged(&package, install).map(|files| files.len())
    })();
    let _ = std::fs::remove_dir_all(&work);
    outcome
}

/// Replace the running AppImage `target` with the release's.
fn install_appimage(release: &Release, url: &str, target: &Path) -> Result<(), String> {
    // Beside the target, so the final rename never crosses a file system.
    let staged = old_name_with(target, ".update");
    let work = std::env::temp_dir().join(format!("ffe-rs-update-{}", std::process::id()));
    let _ = std::fs::remove_file(&staged);
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let outcome = (|| -> Result<(), String> {
        let hash = listed_hash(release, url, &work)?;
        download(url, &staged)?;
        replace_program(&staged, target, &hash)
    })();
    let _ = std::fs::remove_file(&staged);
    let _ = std::fs::remove_dir_all(&work);
    outcome
}

/// Check `new` against `hash`, make it executable and move it over `target`. On any error the
/// target is untouched. A running program can be replaced this way on Linux and macOS: the
/// running copy keeps its old file.
fn replace_program(new: &Path, target: &Path, hash: &str) -> Result<(), String> {
    if sha256_of(new)? != hash.to_ascii_lowercase() {
        return Err("The download does not match its checksum; it is damaged.".to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(new, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Could not make the update executable: {e}"))?;
    }
    std::fs::rename(new, target).map_err(|e| {
        format!(
            "Could not replace {} (is the folder writable?): {e}",
            target.display()
        )
    })
}

/// Replace the app bundle `bundle` with the one in the release's zip.
fn install_bundle(release: &Release, url: &str, bundle: &Path) -> Result<(), String> {
    let parent = bundle.parent().ok_or("The app has no folder.")?;
    // Beside the bundle, so the swap is a rename on one file system.
    let work = parent.join(format!(".ffers-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| {
        format!(
            "Could not write to {} (move ffers.app somewhere you can write to): {e}",
            parent.display()
        )
    })?;
    let outcome = (|| -> Result<(), String> {
        let hash = listed_hash(release, url, &work)?;
        let zip = work.join("update.zip");
        download(url, &zip)?;
        if sha256_of(&zip)? != hash.to_ascii_lowercase() {
            return Err("The download does not match its checksum; it is damaged.".to_owned());
        }
        let unpacked = work.join("unpacked");
        unpack(&zip, &unpacked)?;
        let new_bundle = find_bundle(&unpacked)?;
        swap_bundle(&new_bundle, bundle)
    })();
    let _ = std::fs::remove_dir_all(&work);
    outcome
}

/// The `.app` folder inside an unpacked macOS package (at its top or one folder down).
fn find_bundle(unpacked: &Path) -> Result<PathBuf, String> {
    let is_app = |path: &Path| path.is_dir() && path.extension().is_some_and(|e| e == "app");
    let children = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .map(|read| read.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default()
    };
    let top = children(unpacked);
    top.iter()
        .find(|path| is_app(path))
        .cloned()
        .or_else(|| {
            top.iter()
                .filter(|path| path.is_dir())
                .flat_map(|dir| children(dir))
                .find(|path| is_app(path))
        })
        .ok_or_else(|| "The package holds no app.".to_owned())
}

/// Put `new` where `current` is. The old bundle is kept as `<name>.app.old` (removed at the next
/// start) and put back if the new one cannot be moved in.
fn swap_bundle(new: &Path, current: &Path) -> Result<(), String> {
    let old = old_name(current);
    let _ = std::fs::remove_dir_all(&old);
    std::fs::rename(current, &old).map_err(|e| {
        format!(
            "Could not replace {} (is the folder writable?): {e}",
            current.display()
        )
    })?;
    if let Err(error) = std::fs::rename(new, current) {
        let _ = std::fs::rename(&old, current);
        return Err(format!(
            "The update could not be installed and was undone: {error}"
        ));
    }
    Ok(())
}

/// The folder inside `unpacked` that holds `SHA256SUMS.txt` (the zip's one top folder).
fn package_folder(unpacked: &Path) -> Result<PathBuf, String> {
    if unpacked.join("SHA256SUMS.txt").is_file() {
        return Ok(unpacked.to_path_buf());
    }
    std::fs::read_dir(unpacked)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.join("SHA256SUMS.txt").is_file())
        .ok_or_else(|| "The package has no SHA256SUMS.txt.".to_owned())
}

/// Unpack `zip` into `into`. In-process: PowerShell's `Expand-Archive` is not reliable across
/// Windows PowerShell builds (it failed on a CI runner with a missing `ExtractToDirectory`
/// overload), and the zip crate refuses entries that escape the folder and keeps the Unix
/// permissions a macOS package stores.
fn unpack(zip: &Path, into: &Path) -> Result<(), String> {
    let fail = |e: &dyn std::fmt::Display| format!("The package could not be unpacked: {e}");
    let file = std::fs::File::open(zip).map_err(|e| fail(&e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| fail(&e))?;
    std::fs::create_dir_all(into).map_err(|e| fail(&e))?;
    archive.extract(into).map_err(|e| fail(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sums(files: &[(&str, &[u8])]) -> String {
        files
            .iter()
            .map(|(path, bytes)| {
                let hash: String = Sha256::digest(bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                format!("{hash}  {path}\n")
            })
            .collect()
    }

    fn package(dir: &Path, files: &[(&str, &[u8])]) {
        for (path, bytes) in files {
            let target = dir.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, bytes).unwrap();
        }
        std::fs::write(dir.join("SHA256SUMS.txt"), sums(files)).unwrap();
    }

    /// The release zips are made by `Compress-Archive` (tools/package.ps1), whose entry
    /// names may use backslashes on Windows PowerShell 5.1; they must still unpack.
    #[cfg(windows)]
    #[test]
    fn a_compress_archive_zip_unpacks() {
        let work = tempfile::tempdir().unwrap();
        let source = work.path().join("some package");
        package(
            &source,
            &[("ffe-rs.exe", b"program"), ("docs/a.md", b"doc")],
        );
        let zip = work.path().join("made by powershell.zip");
        let made = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg("Compress-Archive -LiteralPath $env:SRC -DestinationPath $env:ZIP -Force")
            .env("SRC", &source)
            .env("ZIP", &zip)
            .output()
            .unwrap();
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );
        let into = work.path().join("unpacked");
        unpack(&zip, &into).unwrap();
        assert_eq!(
            std::fs::read(into.join("some package").join("docs").join("a.md")).unwrap(),
            b"doc"
        );
        // The rest of the updater's path: find the package folder, verify and install.
        let staged = package_folder(&into).unwrap();
        let install = work.path().join("install");
        std::fs::create_dir_all(install.join("saves")).unwrap();
        std::fs::write(install.join("ffe-rs.exe"), b"old").unwrap();
        std::fs::write(install.join("saves").join("config.cfg"), b"mine").unwrap();
        let files = install_staged(&staged, &install).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(
            std::fs::read(install.join("ffe-rs.exe")).unwrap(),
            b"program"
        );
        assert_eq!(
            std::fs::read(install.join("docs").join("a.md")).unwrap(),
            b"doc"
        );
        assert_eq!(
            std::fs::read(install.join("saves").join("config.cfg")).unwrap(),
            b"mine"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_package_unpacks_from_a_path_with_spaces() {
        // The update failed with "LiteralPath ... null or empty" because the paths
        // never reached PowerShell. Build a real zip and unpack it from a spaced path.
        let work = tempfile::tempdir().unwrap();
        let zip = work.path().join("update file.zip");
        {
            use std::io::Write;
            let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, bytes) in [
                ("some package/ffe-rs.exe", &b"program"[..]),
                ("some package/docs/a.md", b"doc"),
            ] {
                writer.start_file(name, options).unwrap();
                writer.write_all(bytes).unwrap();
            }
            writer.finish().unwrap();
        }
        let into = work.path().join("unpacked here");
        unpack(&zip, &into).unwrap();
        assert_eq!(
            std::fs::read(into.join("some package").join("ffe-rs.exe")).unwrap(),
            b"program"
        );
        // A missing zip is an error, not a silent success.
        assert!(unpack(&work.path().join("missing.zip"), &into).is_err());
    }

    #[test]
    fn versions_parse_and_compare() {
        assert_eq!(parse_version("v0.9.1"), Some([0, 9, 1]));
        assert_eq!(parse_version("0.10.0"), Some([0, 10, 0]));
        assert_eq!(parse_version("v1.2.3-beta.1"), Some([1, 2, 3]));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
        assert!(is_newer([0, 9, 0], [0, 9, 1]));
        assert!(is_newer([0, 9, 9], [0, 10, 0]));
        assert!(!is_newer([0, 9, 1], [0, 9, 1]));
        assert!(!is_newer([1, 0, 0], [0, 99, 0]));
        assert!(current_version() > [0, 0, 0]);
    }

    #[test]
    fn a_release_is_read_from_the_github_answer() {
        let json = format!(
            r#"{{"tag_name":"v0.9.1","html_url":"https://github.com/{REPO}/releases/tag/v0.9.1","body":"Notes",
            "assets":[
              {{"name":"MyRecomp-v0.9.1-Windows-x64.zip","browser_download_url":"https://github.com/{REPO}/releases/download/v0.9.1/MyRecomp-v0.9.1-Windows-x64.zip"}},
              {{"name":"ffers-v0.9.1-x86_64.AppImage","browser_download_url":"https://github.com/{REPO}/releases/download/v0.9.1/ffers-v0.9.1-x86_64.AppImage"}},
              {{"name":"ffers-v0.9.1-Windows-x64.zip","browser_download_url":"https://github.com/{REPO}/releases/download/v0.9.1/ffers-v0.9.1-Windows-x64.zip"}},
              {{"name":"other.txt","browser_download_url":"https://github.com/{REPO}/releases/download/v0.9.1/other.txt"}}]}}"#
        );
        let release = parse_release_for(&json, Some(Platform::Windows)).unwrap();
        assert_eq!(release.version, [0, 9, 1]);
        assert_eq!(release.notes, "Notes");
        // The project's own package wins over the other publisher's, and the AppImage is no
        // Windows package.
        assert!(release
            .asset_url
            .unwrap()
            .ends_with("/ffers-v0.9.1-Windows-x64.zip"));
        // A download address that is not the project's is ignored, as is a bad tag.
        let foreign = r#"{"tag_name":"v1.0.0","html_url":"https://evil.example/x","assets":[
          {"name":"x-win-x64.zip","browser_download_url":"https://evil.example/x-win-x64.zip"}]}"#;
        let release = parse_release_for(foreign, Some(Platform::Windows)).unwrap();
        assert!(release.asset_url.is_none());
        assert_eq!(release.page, format!("https://github.com/{REPO}/releases"));
        assert!(parse_release(r#"{"tag_name":"nightly"}"#).is_err());
        assert!(parse_release("not json").is_err());
    }

    #[test]
    fn only_plain_program_paths_are_installable() {
        assert!(installable("ffe-rs.exe"));
        assert!(installable("docs/CONTROLS.md"));
        assert!(!installable("../evil.exe"));
        assert!(!installable("/etc/passwd"));
        assert!(!installable("C:/x.exe"));
        assert!(!installable("docs\\x.md"));
        assert!(!installable("game/FIRSTENC.EXE"));
        assert!(!installable("Saves/config.cfg"));
        assert!(!installable("docs/music.bin"));
        assert!(!installable(""));
    }

    #[test]
    fn a_good_package_replaces_programs_and_keeps_the_players_files() {
        let work = tempfile::tempdir().unwrap();
        let (staged, install) = (work.path().join("staged"), work.path().join("install"));
        package(
            &staged,
            &[
                ("ffe-rs.exe", b"new game"),
                ("ffe-launcher.exe", b"new launcher"),
                ("docs/CONTROLS.md", b"new controls"),
            ],
        );
        std::fs::create_dir_all(install.join("saves")).unwrap();
        std::fs::create_dir_all(install.join("game")).unwrap();
        std::fs::write(install.join("ffe-rs.exe"), b"old game").unwrap();
        std::fs::write(install.join("ffe-launcher.exe"), b"old launcher").unwrap();
        std::fs::write(install.join("saves/config.cfg"), b"mine").unwrap();
        std::fs::write(install.join("game/FIRSTENC.EXE"), b"original").unwrap();
        std::fs::write(install.join("music.bin"), b"music").unwrap();

        let files = install_staged(&staged, &install).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(
            std::fs::read(install.join("ffe-rs.exe")).unwrap(),
            b"new game"
        );
        assert_eq!(
            std::fs::read(install.join("ffe-rs.exe.old")).unwrap(),
            b"old game"
        );
        assert_eq!(
            std::fs::read(install.join("docs/CONTROLS.md")).unwrap(),
            b"new controls"
        );
        assert_eq!(
            std::fs::read(install.join("saves/config.cfg")).unwrap(),
            b"mine"
        );
        assert_eq!(
            std::fs::read(install.join("game/FIRSTENC.EXE")).unwrap(),
            b"original"
        );
        assert_eq!(std::fs::read(install.join("music.bin")).unwrap(), b"music");
        remove_leftovers(&install);
        assert!(!install.join("ffe-rs.exe.old").exists());
        assert!(!install.join("ffe-launcher.exe.old").exists());
    }

    #[test]
    fn a_damaged_or_unsafe_package_changes_nothing() {
        let work = tempfile::tempdir().unwrap();
        let install = work.path().join("install");
        std::fs::create_dir_all(&install).unwrap();
        std::fs::write(install.join("ffe-rs.exe"), b"old game").unwrap();

        // A file whose bytes do not match its checksum.
        let damaged = work.path().join("damaged");
        package(&damaged, &[("ffe-rs.exe", b"new game")]);
        std::fs::write(damaged.join("ffe-rs.exe"), b"tampered").unwrap();
        let error = install_staged(&damaged, &install).unwrap_err();
        assert!(error.contains("checksum"), "{error}");

        // A package that lists a player's folder.
        let unsafe_package = work.path().join("unsafe");
        package(&unsafe_package, &[("game/FIRSTENC.EXE", b"x")]);
        assert!(install_staged(&unsafe_package, &install).is_err());

        // A package with a file its checksum list leaves out.
        let stray = work.path().join("stray");
        package(&stray, &[("ffe-rs.exe", b"new game")]);
        std::fs::write(stray.join("extra.exe"), b"x").unwrap();
        assert!(install_staged(&stray, &install).is_err());

        // No checksum list at all.
        let bare = work.path().join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        std::fs::write(bare.join("ffe-rs.exe"), b"x").unwrap();
        assert!(install_staged(&bare, &install).is_err());

        assert_eq!(
            std::fs::read(install.join("ffe-rs.exe")).unwrap(),
            b"old game"
        );
        assert!(!install.join("ffe-rs.exe.old").exists());
    }

    #[test]
    fn a_failure_part_way_puts_the_old_programs_back() {
        let work = tempfile::tempdir().unwrap();
        let (staged, install) = (work.path().join("staged"), work.path().join("install"));
        // `docs` is a file in the install folder, so writing docs/x.md fails after
        // ffe-rs.exe has been replaced.
        package(&staged, &[("ffe-rs.exe", b"new"), ("docs/x.md", b"doc")]);
        std::fs::create_dir_all(&install).unwrap();
        std::fs::write(install.join("ffe-rs.exe"), b"old").unwrap();
        std::fs::write(install.join("docs"), b"not a folder").unwrap();
        let error = install_staged(&staged, &install).unwrap_err();
        assert!(error.contains("undone"), "{error}");
        assert_eq!(std::fs::read(install.join("ffe-rs.exe")).unwrap(), b"old");
        assert!(!install.join("ffe-rs.exe.old").exists());
    }

    #[test]
    fn each_platform_picks_its_own_package_and_the_checksum_list() {
        let base = format!("https://github.com/{REPO}/releases/download/v0.9.1");
        let json = format!(
            r#"{{"tag_name":"v0.9.1","assets":[
              {{"name":"MyRecomp-v0.9.1-x86_64.AppImage","browser_download_url":"{base}/MyRecomp-v0.9.1-x86_64.AppImage"}},
              {{"name":"ffers-v0.9.1-x86_64.AppImage","browser_download_url":"{base}/ffers-v0.9.1-x86_64.AppImage"}},
              {{"name":"ffers-v0.9.1-macOS.zip","browser_download_url":"{base}/ffers-v0.9.1-macOS.zip"}},
              {{"name":"ffers-v0.9.1-Windows-x64.zip","browser_download_url":"{base}/ffers-v0.9.1-Windows-x64.zip"}},
              {{"name":"SHA256SUMS.txt","browser_download_url":"{base}/SHA256SUMS.txt"}}]}}"#
        );
        let pick = |platform| {
            parse_release_for(&json, platform)
                .unwrap()
                .asset_url
                .unwrap_or_default()
        };
        assert!(pick(Some(Platform::Windows)).ends_with("/ffers-v0.9.1-Windows-x64.zip"));
        assert!(pick(Some(Platform::Linux)).ends_with("/ffers-v0.9.1-x86_64.AppImage"));
        assert!(pick(Some(Platform::MacOs)).ends_with("/ffers-v0.9.1-macOS.zip"));
        assert_eq!(pick(None), "");
        let release = parse_release_for(&json, None).unwrap();
        assert_eq!(release.sums_url.unwrap(), format!("{base}/SHA256SUMS.txt"));

        let hash = "a".repeat(64);
        let other = "b".repeat(64);
        let sums =
            format!("{hash}  ffers-v0.9.1-macOS.zip\n{other}  ffers-v0.9.1-x86_64.AppImage\n");
        assert_eq!(hash_for(&sums, "ffers-v0.9.1-macOS.zip").unwrap(), hash);
        assert!(hash_for(&sums, "ffers-v0.9.1-Windows-x64.zip").is_err());
    }

    #[test]
    fn the_app_bundle_is_found_from_the_running_program() {
        let exe = Path::new("/Applications/ffers.app/Contents/MacOS/ffe-launcher");
        assert_eq!(
            bundle_of(exe).unwrap(),
            Path::new("/Applications/ffers.app")
        );
        assert!(bundle_of(Path::new("/home/me/ffers/ffe-launcher")).is_none());
        assert!(bundle_of(Path::new("/x/ffers.app/Contents/Other/ffe-launcher")).is_none());
        assert!(bundle_of(Path::new("/x/folder/Contents/MacOS/ffe-launcher")).is_none());
    }

    #[test]
    fn an_appimage_is_replaced_only_by_a_download_that_matches() {
        let work = tempfile::tempdir().unwrap();
        let target = work.path().join("ffers.AppImage");
        let new = work.path().join("ffers.AppImage.update");
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(&new, b"new image").unwrap();
        let good: String = Sha256::digest(b"new image")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        // A damaged download leaves the running AppImage alone.
        assert!(replace_program(&new, &target, &"0".repeat(64)).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        replace_program(&new, &target, &good.to_ascii_uppercase()).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new image");
        assert!(!new.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "the AppImage must be executable");
        }
    }

    #[test]
    fn a_bundle_swap_keeps_the_old_one_and_undoes_a_failure() {
        let work = tempfile::tempdir().unwrap();
        let current = work.path().join("ffers.app");
        let new = work.path().join("unpacked").join("pkg").join("ffers.app");
        for (dir, text) in [(&current, "old"), (&new, "new")] {
            std::fs::create_dir_all(dir.join("Contents").join("MacOS")).unwrap();
            std::fs::write(
                dir.join("Contents").join("MacOS").join("ffe-launcher"),
                text,
            )
            .unwrap();
        }
        assert_eq!(find_bundle(&work.path().join("unpacked")).unwrap(), new);
        // A new bundle that cannot be moved in puts the old one back.
        assert!(swap_bundle(&work.path().join("missing.app"), &current).is_err());
        assert_eq!(
            std::fs::read(current.join("Contents/MacOS/ffe-launcher")).unwrap(),
            b"old"
        );
        swap_bundle(&new, &current).unwrap();
        assert_eq!(
            std::fs::read(current.join("Contents/MacOS/ffe-launcher")).unwrap(),
            b"new"
        );
        assert_eq!(
            std::fs::read(old_name(&current).join("Contents/MacOS/ffe-launcher")).unwrap(),
            b"old"
        );
    }

    /// A macOS zip stores the program's executable bit; unpacking must keep it.
    #[cfg(unix)]
    #[test]
    fn an_unpacked_app_keeps_its_executable_bit() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let work = tempfile::tempdir().unwrap();
        let zip = work.path().join("mac.zip");
        {
            let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);
            writer
                .start_file("pkg/ffers.app/Contents/MacOS/ffe-launcher", options)
                .unwrap();
            writer.write_all(b"program").unwrap();
            writer.finish().unwrap();
        }
        let into = work.path().join("unpacked");
        unpack(&zip, &into).unwrap();
        let exe = find_bundle(&into)
            .unwrap()
            .join("Contents/MacOS/ffe-launcher");
        assert_eq!(
            std::fs::metadata(exe).unwrap().permissions().mode() & 0o111,
            0o111
        );
    }
}
