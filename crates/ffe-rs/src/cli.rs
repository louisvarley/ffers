//! Command-line text that needs no game data: `--help` and `--version`.
//!
//! The owner playtest contract (CLAUDE.MD section 8) asks every build to carry
//! a version/build ID. These two flags are answered before any window, cache or
//! game folder is touched, so they work on a machine with no game files.

/// The usage line, shared by `--help` and the unknown-argument error.
pub const USAGE: &str = "Usage: ffe-rs [--game <game files folder>] [--root <data folder>] \
[--videos <AVI folder>] [--start 0..5|sol] [--skip-intro] [--dev] [--load SAVE.json] [--smoke-frames 1..300] \
[--capture DIR] [--capture-planet DIR] [--capture-traffic DIR] [--capture-start DIR] \
[--capture-prefs DIR] [--capture-intro DIR] [--capture-display DIR] \
[--capture-gameover DIR] [--capture-save DIR] [--record FILE] \
[--replay FILE [--replay-exit]] [--prepare-music] | --help | --version";

/// The one-line version string, `ffe-rs 0.9.0`.
pub fn version_line() -> String {
    format!("ffe-rs {}", env!("CARGO_PKG_VERSION"))
}

/// The full `--help` text.
pub fn help_text() -> String {
    format!(
        "{}\n\n\
         First Encounters, reimplemented in Rust. Supply an installed original game \
         folder (see README.md); no game data is bundled.\n\n\
         {USAGE}\n\n\
         Options:\n  \
         --game DIR         the installed original game (default: the `game` folder in the root)\n  \
         --root DIR         where the content cache, saves and settings live\n  \
         --videos DIR       the folder of the CD videos (default: the game folder's DATA folder)\n  \
         --start N|sol     the start type (0..5); `sol` is the developer sandbox and needs --dev\n  \
         --skip-intro       go straight to the cockpit\n  \
         --prepare-music    render the music pack (music.bin) in the root and exit\n  \
         --dev              developer inspection mode (also FFE_DEV=1)\n  \
         --load FILE        resume a native save (JSON)\n  \
         --record/--replay  write or play back a per-frame input recording\n  \
         --capture-*        write evidence images for the named screen and exit\n  \
         --help, --version  print this text or the version and exit\n\n\
         Controls are in docs/CONTROLS.md.",
        version_line()
    )
}

/// The text to print for `--help`/`--version` if one of them is the only
/// argument asked for, else `None`. `-h`, `-V` and `/?` are accepted too.
pub fn early_answer<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut answer = None;
    for arg in args {
        match arg.as_ref().to_str() {
            Some("--help" | "-h" | "/?") => return Some(help_text()),
            Some("--version" | "-V") => answer = Some(version_line()),
            _ => {}
        }
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_names_the_crate_version() {
        let line = version_line();
        assert!(line.starts_with("ffe-rs "));
        assert!(line.ends_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn help_wins_over_version_and_carries_the_usage() {
        let text = early_answer(["--version", "--help"]).expect("help");
        assert!(text.contains(USAGE));
        assert!(text.contains(&version_line()));
        assert!(text.contains("--skip-intro"));
    }

    #[test]
    fn only_the_two_flags_answer_early() {
        assert_eq!(early_answer(["--version"]), Some(version_line()));
        assert_eq!(early_answer(["-V"]), Some(version_line()));
        assert!(early_answer(["-h"]).is_some());
        assert!(early_answer(["--dev", "--skip-intro"]).is_none());
        assert!(early_answer(Vec::<&str>::new()).is_none());
    }

    #[test]
    fn every_option_the_parser_reads_is_in_the_usage() {
        // The parser lives in main.rs; keep its flag names and the usage in step.
        for flag in [
            "--game",
            "--root",
            "--videos",
            "--start",
            "--skip-intro",
            "--dev",
            "--load",
            "--smoke-frames",
            "--capture-save",
            "--record",
            "--replay",
            "--replay-exit",
        ] {
            assert!(USAGE.contains(flag), "{flag} missing from the usage text");
        }
    }
}
