# ffe-rs

A native Rust reimplementation of **Frontier: First Encounters** (Elite 3). The gameplay
reference is **JJFFE 2.8a7**; the aim is a like-for-like **Classic** game, drawn the original's
way (320x200, the original palette, font, cockpit and software rasterisers) and run by a small
launcher. You supply your own installed copy of the game; nothing from it is in this repository
or in any download.

**Status:** playable and close to the original, still experimental.
[docs/STATUS.md](docs/STATUS.md) is the current state, [docs/BURNDOWN.md](docs/BURNDOWN.md) is
what is left, and [CLAUDE.MD](CLAUDE.MD) records the owner's decisions and the working rules.
Several missions the original shipped broken (an unreachable story step, wrong prices, a dead
dialogue option, a lost branch) are fixed as always-on corrections; see
[docs/notes/broken-missions.md](docs/notes/broken-missions.md).

## Download

Get the latest build from the [Releases page](https://github.com/louisvarley/ffers/releases/latest).
Every platform has a ready-to-run package; none contains game data.

| Platform | File | Notes |
|---|---|---|
| Windows 10/11 x64 | `ffers-<tag>-Windows-x64.zip` | Unzip anywhere. Updates itself. |
| Linux x86_64 | `ffers-<tag>-x86_64.AppImage` | `chmod +x` it and run. Sound uses ALSA. |
| macOS | `ffers-<tag>-macOS.zip` | Unsigned: right-click the app, choose Open. Sound uses CoreAudio. |

The `MyRecomp-<tag>-...` files on the release are the same builds under the MyRecomp name.
`SHA256SUMS.txt` lists the checksum of every file.

### You need the game

The port reads your installed game; it never changes it. Supported releases:

- the floppy **English 1.02** (the default most players have),
- the CD **English 1.01** (the CD's `DATA` folder also holds the optional game videos),
- the Elite Club shareware 1.06 data also loads.

[docs/GAME_FILES.md](docs/GAME_FILES.md) lists the recognised files and checksums. Put the
installed folder (the one holding `FIRSTENC.EXE`) wherever you like and point the launcher at it, or
name it `game` beside the program. A raw BIN/CUE image or a patch installer will not work: install
the game first.

### Run it

- **Windows:** unzip, then run `ffe-launcher.exe`. Put your game in a folder called `game` beside
  it (or browse to it in the launcher).
- **Linux:** `chmod +x ffers-<tag>-x86_64.AppImage && ./ffers-<tag>-x86_64.AppImage`. Keep the
  `game` folder (or browse to it) and the launcher's `saves`, `recordings` and `music.bin` beside
  the `.AppImage` file.
- **macOS:** open `ffers.app` (first time: right-click, Open, Open). In the launcher, browse to your
  game folder. Saves and the music pack go to `~/Library/Application Support/ffe-rs`, or beside
  the app if that folder already has a `game` or `saves` folder. If macOS reports the app as
  damaged: `xattr -dr com.apple.quarantine ffers.app`.

The first launch checks the game files and renders the music pack (`music.bin`, about 100 MB,
made from the game's own music with a bundled sound bank). It is made once.

## The launcher

`ffe-launcher` is the front door. It checks your game folder, remembers your choices in
`saves/launcher.cfg`, edits the game's own settings in `saves/config.cfg`, and starts the game.

- **Play.** The *game files* row shows the release it found (CD 1.01, floppy 1.02 or shareware)
  and whether every file checks out; *Browse* picks another folder. The *game videos* row finds
  the CD's `DATA` folder of AVI clips (optional) or a folder you choose. Then pick a **starting
  location** (Gateway/Hope, Ross 154, Sol/Mars or Lave Station), tick **Skip intro** if you
  like, and press **Launch**. Launch stays disabled until the files are verified.
- **Settings.** *Display*: window size, full screen, the video filter (Nearest, Linear, xBRZ 2x,
  Scale2x), scaling, pixel aspect and an optional CRT scanline effect. *Game options*: the control
  scheme, other options and the galaxy map. *Enhancements*: the optional extras below. Settings are
  locked while the game runs and reload when it closes.
- **About.** The verification report, links to the saves folder and the last game log, the
  **Updates** section, and the project's GitHub link.
- **Recording.** Every run can record your keyboard and mouse to
  `recordings/<date-time>.ffe-input`, so a problem can be replayed exactly (see Developer options).

More detail: [docs/notes/launcher.md](docs/notes/launcher.md).

### Updates

On the Windows package the launcher checks GitHub for a newer release when it opens (switch this
off in About). When one exists, an **Update to vX.Y.Z - click to install** button appears in the
footer. It downloads the Windows zip, checks every file against the checksum list inside it,
replaces the programs and documents, and restarts the launcher. Your `game`, `saves`,
`recordings`, `FMVs` folders and the music pack are never touched, and a failed update is undone.
On Linux and macOS the same button opens the release page. The check sends only the request
itself to GitHub: no identifiers and no game information. It uses `curl` (built into Windows 10
and later).

## Enhancements

Every enhancement is off by default, so the game plays like the original until you opt in under **Settings > Enhancements** (stored as `enhancement.<name>=on`
lines in `saves/config.cfg`). They never change your saves, and the 320x200 picture changes only
where an indicator is added.

| Group | Enhancement | What it does |
|---|---|---|
| Combat & missiles | Turrets turn while time is paused | Turrets keep tracking their target while time is paused. |
| | Missiles ignore their launcher on launch | A missile's collision ray ignores the ship that fired it. |
| | Missiles use swept collision | A missile's whole path is tested each frame, so a fast missile cannot step past a thin target. |
| | ECM can disable NN500 missiles | The ECM stops NN500-series missiles as well as the standard models. |
| Autopilot | Autopilot offsets its waypoint | Adds a waypoint when the direct path would hit a nearby body. |
| | Autopilot braking uses retro thrust and gravity | Braking is calculated from the weaker retro thrust less gravity, so approaches do not crash. |
| | Autopilot avoids every nearby body | Collisions are tested against every body in range. |
| Mission fixes | Mission fixes | Assassination kill credit, hitch-hiker completion, early medals, the hidden Alioth deadline, blocked docking bays, and dialogues under accelerated time. |
| Combat display | NPC relative speed | The selected ship's hull and shield readout also shows its speed relative to you. |
| | Combat Targeting Computer | A dashboard button (in the escape capsule's slot) lists every ship within 50 km in the navigation computer's style, coloured red for hostile, blue for police and green for the rest. Click one to target it; it gets a red target square. |
| | Edge target indicator | An arrow at the screen edge points to the selected target when it is out of view. |

The full definitions are in [docs/notes/enhancements.md](docs/notes/enhancements.md); the other
deliberate differences from the original are recorded in [docs/DEVIATIONS.md](docs/DEVIATIONS.md).

## Controls

Two control presets, chosen in the launcher or on the Escape pages' Advanced Options page:
**Modern** (W/S and the arrows, the default) and the original's **Classic** keys (hold the right
mouse button to steer; Enter and right Shift set the speed). The full tables are in
[docs/CONTROLS.md](docs/CONTROLS.md). The window size, fit, filter and pixel aspect never change
the 320x200 frame ([docs/notes/display.md](docs/notes/display.md)).

## Where your files go

| What | Where |
|---|---|
| Saves, launcher and game settings, the last game log | `saves/` |
| Recorded sessions | `recordings/` |
| The rendered music pack | `music.bin` |

`saves/`, `recordings/` and `music.bin` sit beside the program (Windows), beside the `.AppImage`
(Linux) or in `~/Library/Application Support/ffe-rs` (macOS). Delete `music.bin` to have it
rendered again. Original saves can be imported read-only; the game writes its own versioned saves.

## Building from source

You need the [Rust toolchain](https://rustup.rs) (1.85 or later) and your installed game in
`game\`. On Linux also install `libasound2-dev` and `pkg-config`.

| How | What it does |
|---|---|
| `play.bat` | Builds the release game and starts it on `.\game` (Windows). Errors stay on screen. |
| `launcher.bat` | Builds and opens the launcher (Windows). |
| `cargo build --release --locked -p ffe-rs -p ffe-launcher` | Builds both programs into `target/release/` on any platform. Run `ffe-launcher`. |
| `target\release\ffe-rs.exe --root .` | The game directly. `--game <folder>` picks another game folder. |

### Packages and releases

| Script | Produces |
|---|---|
| `tools\package.ps1 [-Name N]` | The Windows zip: both programs, documents, licence texts and a checksum list. |
| `tools/package-appimage.sh <name>` | The Linux AppImage. |
| `tools/package-macos.sh <name>` | The macOS app, zipped. |

Releases are automatic. Every push to `main` that changes the program runs the **Release**
workflow ([.github/workflows/release.yml](.github/workflows/release.yml)): it picks the next
version, commits it (`Release vX.Y.Z`), builds every package on its own platform, smoke-tests the
game and publishes the GitHub release with all downloads and a checksum list.

- **Version rule** (`tools/next-version.py`): the first release is the Cargo version as it is
  (`0.9.0`); after that the latest tag gets a patch bump (`0.9.0` to `0.9.1`). Put `#minor` or
  `#major` in the commit message for a larger bump, or raise `version` in both
  `crates/ffe-rs/Cargo.toml` and `crates/ffe-launcher/Cargo.toml` by hand to release exactly that.
- Pushes that only change documents, the dashboard or licence files do not release.
- **Run workflow** (Actions tab) builds all packages as downloadable run artifacts without
  releasing; tick *publish* to release from there, or choose the bump. A `v*` tag pushed by hand is
  released as it is.
- Releasing commits to `main`, so if `main` is protected let `github-actions` push to it.

The **CI** workflow checks formatting, clippy, the tests on Windows, Linux and macOS, the version
rules, `recomplauncher.json` against its schema, and that no forbidden file is tracked.

## RecompLauncher

[recomplauncher.json](recomplauncher.json) describes ffe-rs to
[RecompLauncher](https://recomplauncher.com): it asks the player for their installed game folder
(checked for `FIRSTENC.EXE`), renders the music pack once (`ffe-rs --prepare-music`), and starts
`ffe-rs` with `--game <that folder> --root <the launcher's data folder>`. Starting location, skip
intro, controls, display and every enhancement are settings it can change. It uses the
`MyRecomp-<tag>-...` downloads, whose zips keep their files at the top level (`ffe-rs.exe`,
`ffers.app`) and whose AppImage runs the game itself when given `--` arguments.

## Developer options

`--dev` (or `FFE_DEV=1`) enables Ctrl+F11 (a screenshot of the presented frame), Ctrl+F12 (the
model viewer), Ctrl+Shift+F12 (the station model lab) and O (the synthetic prototype flight
model). `replay.bat recordings\FILE` (or `ffe-rs --replay FILE [--replay-exit]`) plays a recording
back exactly; `FFE_TRACE=1` prints the flight state while it runs.

```
ffe-rs [--game DIR] [--root DIR] [--start 0..5|sol] [--skip-intro] [--load SAVE.json]
       [--smoke-frames N] [--capture* DIR] [--record FILE] [--replay FILE [--replay-exit]]
       [--prepare-music]                   (render music.bin in the root and exit)
ffe-rs --help | --version            (answered without game files; --version is the build ID)
```

`ffe-tools` inspects and converts a game installation without changing it:

```
ffe-tools inspect-game   [--game DIR]          recognise the release and check every file
ffe-tools export-images  --game DIR --out NEW  every bitmap as PNG
ffe-tools export-media   --game DIR --out DIR  the CD videos and stills into FMVs\
ffe-tools build-data     --game DIR --out NEW  the content cache the local tests read
ffe-tools inspect-data   --data FILE           verify a content cache
ffe-tools model-census   [--game DIR]          which model programs the port executes
```

`tools\clean-machine-check.ps1` checks the first-run import from a fresh folder;
`python tools/third-party.py` lists the linked crates' licences ([docs/THIRD_PARTY.md](docs/THIRD_PARTY.md)).

## Tests

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Tests that need the original content are `#[ignore]`d and read a local cache: build one with
`ffe-tools build-data --game game --out .cache\w.bin`, then run them with
`FFE_WORLD_CACHE=.cache\w.bin cargo test -p ffe-rs -- --ignored`. The differential checks against
the executed original JJFFE are in [tools/reference/README.md](tools/reference/README.md), and
[docs/REFERENCE_HARNESS.md](docs/REFERENCE_HARNESS.md) is the recipe for adding one.

## Repository layout

```
crates/ffe-core/      deterministic simulation: RNG, physics, models, galaxy, traffic, markets, missions
crates/ffe-rs/        the game: software renderer, window, input, sound, saves
crates/ffe-content/   the verified content cache read from the game files
crates/ffe-convert/   bounds-checked decoders for the executable and the asset files
crates/ffe-audio/     HMP music and effect playback
crates/ffe-launcher/  the launcher, with its update check
crates/ffe-tools/     the command-line tools above
tools/                packaging scripts and local oracles against the executed JJFFE reference
dashboard/            the burndown page (open index.html); data in burndown.js
docs/                 status, decisions, formats and per-subsystem notes (see docs/README.md)
.github/workflows/    CI and the release packaging
```

## Never committed

`game\`, `jjffe-master\`, `ffed3d-master\`, the supplied `Frontier-First-Encounters*` release and
patch folders, generated caches, extracted art, captures, screenshots, recordings and saves stay
local. `ci/check_forbidden.py` fails CI if a forbidden path is tracked. Never use an
indiscriminate `git add .`.

## Legal

An unofficial preservation and reimplementation project, not endorsed by the original or
community authors. Players supply their own game; no original or derived game data ships with it.
The Rust code is MIT-licensed (`LICENSE`), covering only the new code, not the game or third-party
assets. The bundled sound bank and every linked crate keep their own licences
(`THIRD_PARTY_LICENSES.txt` in each package). The synthesiser crates are LGPL-2.1 and linked
statically; [docs/LGPL.md](docs/LGPL.md) explains how to relink with your own copy. See
[docs/LEGAL.md](docs/LEGAL.md) and [docs/TAKEDOWN.md](docs/TAKEDOWN.md).
