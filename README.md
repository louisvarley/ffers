# ffe-rs

A native Rust reimplementation of **Frontier: First Encounters** (Elite 3) that runs on modern
Windows, Linux and macOS. It looks and plays like the original (320x200, the original palette,
font and cockpit), with a small launcher, modern controls and a few optional improvements.

You need your own installed copy of the game. Nothing from it is in this repository or in any
download.

## Download

Get the latest build from the [Releases page](https://github.com/louisvarley/ffers/releases/latest).

| Platform | File | Notes |
|---|---|---|
| Windows 10/11 x64 | `ffers-<tag>-Windows-x64.zip` | Unzip anywhere. Updates itself. |
| Linux x86_64 | `ffers-<tag>-x86_64.AppImage` | `chmod +x` it and run. Sound uses ALSA. |
| macOS | `ffers-<tag>-macOS.zip` | Unsigned: right-click the app, choose Open. Sound uses CoreAudio. |

`SHA256SUMS.txt` on each release lists the checksum of every file.

## What you need

Your installed game folder, the one holding `FIRSTENC.EXE`. Supported releases:

- the floppy **English 1.02** (the version most players have),
- the CD **English 1.01** (its `DATA` folder also holds the optional game videos),
- the Elite Club shareware 1.06 data.

[docs/GAME_FILES.md](docs/GAME_FILES.md) lists the recognised files and checksums. A raw BIN/CUE
image or a patch installer will not work: install the game first. The game folder is only ever
read, never changed.

## Run it

- **Windows:** unzip, run `ffe-launcher.exe`, and browse to your game folder (or put it in a folder
  called `game` beside the launcher).
- **Linux:** `chmod +x ffers-<tag>-x86_64.AppImage && ./ffers-<tag>-x86_64.AppImage`, then browse to
  your game folder.
- **macOS:** open `ffers.app` (first time: right-click, Open, Open) and browse to your game folder.
  If macOS says the app is damaged: `xattr -dr com.apple.quarantine ffers.app`.

The first launch checks your game files and renders the music (`music.bin`, about 100 MB, made
from the game's own music with a bundled sound bank). This happens once.

## The launcher

The launcher checks your game folder, remembers your choices and starts the game.

- **Play:** shows the release it found and whether every file checks out. Pick a starting
  location (Gateway/Hope, Ross 154, Sol/Mars or Lave Station), optionally skip the intro, and press
  **Launch**. If you have the CD, it also finds the optional game videos.
- **Settings:** window size, full screen, video filter (Nearest, Linear, xBRZ 2x, Scale2x), pixel
  aspect, an optional CRT scanline effect, the control scheme and the optional enhancements below.
- **About:** the verification report, links to your saves folder and the last game log, and update
  settings.

More detail: [docs/notes/launcher.md](docs/notes/launcher.md).

### Updates

On Windows the launcher checks GitHub for a newer release when it opens (switch this off in
About). When one exists, an **Update to vX.Y.Z - click to install** button appears. It downloads the
release, verifies every file against its checksum list, replaces the programs and restarts. Your
game folder, saves, recordings and music pack are never touched, and a failed update is undone. On
Linux and macOS the button opens the release page instead. The check sends nothing but the request
itself: no identifiers and no game information.

## Controls

Two presets, chosen in the launcher or on the in-game Advanced Options page:

- **Modern** (default): W/S and the arrow keys.
- **Classic**: the original's keys. Hold the right mouse button to steer; Enter and right Shift set
  the speed.

The full key tables are in [docs/CONTROLS.md](docs/CONTROLS.md).

## Enhancements

Every enhancement is off by default, so the game plays like the original until you turn one on
under **Settings > Enhancements**. They never change your saves.

| Group | Enhancement | What it does |
|---|---|---|
| Combat & missiles | Turrets turn while time is paused | Turrets keep tracking their target while time is paused. |
| | Missiles ignore their launcher on launch | A missile no longer hits the ship that fired it. |
| | Missiles use swept collision | A fast missile cannot step past a thin target. |
| | ECM can disable NN500 missiles | The ECM stops NN500-series missiles as well as the standard models. |
| Autopilot | Autopilot offsets its waypoint | Adds a waypoint when the direct path would hit a nearby body. |
| | Autopilot braking uses retro thrust and gravity | Approaches no longer crash. |
| | Autopilot avoids every nearby body | Collisions are tested against every body in range. |
| Missions | Mission fixes | Assassination kill credit, hitch-hiker completion, early medals, the hidden Alioth deadline, blocked docking bays, and dialogues under accelerated time. |
| Combat display | NPC relative speed | The selected ship's readout also shows its speed relative to you. |
| | Combat Targeting Computer | A dashboard button listing every ship within 50 km; click one to target it. |
| | Edge target indicator | An arrow at the screen edge points to an off-screen target. |

The full definitions are in [docs/notes/enhancements.md](docs/notes/enhancements.md). Several
missions the original shipped broken are always fixed; see
[docs/notes/broken-missions.md](docs/notes/broken-missions.md) and
[docs/DEVIATIONS.md](docs/DEVIATIONS.md).

## Where your files go

| What | Where |
|---|---|
| Saves, launcher and game settings, the last game log | `saves/` |
| Recorded sessions | `recordings/` |
| The rendered music pack | `music.bin` |

These sit beside the program on Windows, beside the `.AppImage` on Linux, and in
`~/Library/Application Support/ffe-rs` on macOS. Delete `music.bin` to have it rendered again.

## Building from source

You need the [Rust toolchain](https://rustup.rs) (1.85 or later) and your installed game in
`game\`. On Linux also install `libasound2-dev` and `pkg-config`.

| How | What it does |
|---|---|
| `launcher.bat` | Builds and opens the launcher (Windows). |
| `play.bat` | Builds the game and starts it on `.\game` (Windows). Errors stay on screen. |
| `cargo build --release --locked -p ffe-rs -p ffe-launcher` | Builds both programs into `target/release/` on any platform. Run `ffe-launcher`. |
| `target\release\ffe-rs.exe --root .` | Runs the game directly. `--game <folder>` picks another game folder. |

Command-line options:

```
ffe-rs [--game DIR] [--root DIR] [--start 0..5|sol] [--skip-intro] [--load SAVE.json]
       [--record FILE] [--replay FILE [--replay-exit]] [--prepare-music]
ffe-rs --help | --version
```

`--dev` (or `FFE_DEV=1`) enables developer keys: Ctrl+F11 screenshot, Ctrl+F12 model viewer,
Ctrl+Shift+F12 station model lab. Every run can record input to `recordings/`, and
`replay.bat recordings\FILE` plays one back exactly.

`ffe-tools` inspects and converts a game installation without changing it:

```
ffe-tools inspect-game   [--game DIR]          recognise the release and check every file
ffe-tools export-images  --game DIR --out NEW  every bitmap as PNG
ffe-tools export-media   --game DIR --out DIR  the CD videos and stills into FMVs\
ffe-tools build-data     --game DIR --out NEW  the content cache the local tests read
ffe-tools inspect-data   --data FILE           verify a content cache
ffe-tools model-census   [--game DIR]          which model programs the port executes
```

### Tests

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Tests that need the original content are `#[ignore]`d and read a local cache: build one with
`ffe-tools build-data --game game --out .cache\w.bin`, then run
`FFE_WORLD_CACHE=.cache\w.bin cargo test -p ffe-rs -- --ignored`.

### Packages and releases

| Script | Produces |
|---|---|
| `tools\package.ps1 [-Name N]` | The Windows zip: both programs, documents, licence texts and a checksum list. |
| `tools/package-appimage.sh <name>` | The Linux AppImage. |
| `tools/package-macos.sh <name>` | The macOS app, zipped. |

Releases are automatic. Every push to `main` that changes the program runs the **Release**
workflow ([.github/workflows/release.yml](.github/workflows/release.yml)): it picks the next
version, builds every package on its own platform, smoke-tests the game and publishes a GitHub
release with all downloads and a checksum list.

- **Version rule** (`tools/next-version.py`): the first release uses the Cargo version as it is;
  after that the latest tag gets a patch bump. Put `#minor` or `#major` in the commit message for a
  larger bump.
- Pushes that only change documents, the dashboard or licence files do not release.
- **Run workflow** (Actions tab) builds all packages as downloadable artifacts without releasing;
  tick *publish* to release from there.
- Releasing commits to `main`, so if `main` is protected let `github-actions` push to it.

The **CI** workflow checks formatting, clippy and the tests on Windows, Linux and macOS.

### Repository layout

```
crates/ffe-core/      deterministic simulation: RNG, physics, models, galaxy, traffic, markets, missions
crates/ffe-rs/        the game: software renderer, window, input, sound, saves
crates/ffe-content/   the verified content cache read from the game files
crates/ffe-convert/   bounds-checked decoders for the executable and the asset files
crates/ffe-audio/     HMP music and effect playback
crates/ffe-launcher/  the launcher, with its update check
crates/ffe-tools/     the command-line tools above
tools/                packaging scripts and reference oracles
docs/                 status, decisions, formats and per-subsystem notes (see docs/README.md)
.github/workflows/    CI and the release packaging
```

[docs/STATUS.md](docs/STATUS.md) is the current state and [docs/BURNDOWN.md](docs/BURNDOWN.md) is
what is left.

## Legal

An unofficial preservation and reimplementation project, not endorsed by the original or
community authors. Players supply their own game; no original or derived game data ships with it.
The Rust code is MIT-licensed (`LICENSE`), covering only the new code, not the game or third-party
assets. The bundled sound bank and every linked crate keep their own licences
(`THIRD_PARTY_LICENSES.txt` in each package). The synthesiser crates are LGPL-2.1 and linked
statically; [docs/LGPL.md](docs/LGPL.md) explains how to relink with your own copy. See
[docs/LEGAL.md](docs/LEGAL.md) and [docs/TAKEDOWN.md](docs/TAKEDOWN.md).
