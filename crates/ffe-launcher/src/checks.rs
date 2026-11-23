//! What the launcher verifies: the original game files (through
//! `ffe_tools::inspect_game`, the fingerprint check the game's own loading
//! runs), with optional CD video presence reported separately.

use std::path::Path;

/// How a check came out, for the badge colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Good,
    Warning,
    Bad,
}

/// One line of a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub level: Level,
    pub text: String,
}

impl Line {
    fn good(text: impl Into<String>) -> Self {
        Self {
            level: Level::Good,
            text: text.into(),
        }
    }
    fn warning(text: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            text: text.into(),
        }
    }
    fn bad(text: impl Into<String>) -> Self {
        Self {
            level: Level::Bad,
            text: text.into(),
        }
    }
}

/// Which kind of release the files are, for the launcher's icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    CdRom,
    Floppy,
    Shareware,
}

/// The recognised release: its kind and a line naming it and its version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub kind: Kind,
    pub text: String,
}

/// The release a recognised executable belongs to (`None` for a profile this
/// launcher does not describe).
pub fn release_of(profile: &ffe_tools::SourceProfile) -> Option<Release> {
    let (kind, text) = match profile.id {
        "ffe-cd-1.01-en" => (Kind::CdRom, "CD-ROM release 1.01 (English, 13/04/95)"),
        "ffe-floppy-1.02-en" => (Kind::Floppy, "Floppy release 1.02 (English, 21/04/95)"),
        "ffe-shareware-1.06-en" => (Kind::Shareware, "Shareware 1.06 (Elite Club, 2001)"),
        _ => return None,
    };
    Some(Release {
        kind,
        text: text.to_owned(),
    })
}

/// A finished check: the headline and the details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub level: Level,
    pub summary: String,
    pub lines: Vec<Line>,
    /// The release found, when the executable was recognised.
    pub release: Option<Release>,
}

impl Report {
    fn from_lines(summary_good: &str, summary_bad: &str, lines: Vec<Line>) -> Self {
        let level = if lines.iter().any(|line| line.level == Level::Bad) {
            Level::Bad
        } else if lines.iter().any(|line| line.level == Level::Warning) {
            Level::Warning
        } else {
            Level::Good
        };
        let summary = if level == Level::Bad {
            summary_bad
        } else {
            summary_good
        };
        Self {
            level,
            summary: summary.to_owned(),
            lines,
            release: None,
        }
    }
}

/// The original game files: `FIRSTENC.EXE` must be one of the recognised
/// English releases and the other files must match it.
pub fn verify_game(folder: &Path) -> Report {
    let inspection = match ffe_tools::inspect_game(folder) {
        Ok(inspection) => inspection,
        Err(error) => {
            return Report {
                level: Level::Bad,
                summary: "Game files not found".to_owned(),
                lines: vec![Line::bad(error)],
                release: None,
            }
        }
    };
    let mut lines = Vec::new();
    let shareware = inspection
        .source_profile
        .is_some_and(|p| p.id == "ffe-shareware-1.06-en");
    if shareware {
        lines.push(Line::warning("Shareware identified; experimental launch is enabled. Its embedded data layout is not supported by the game importer yet; startup errors appear in the game log."));
    }
    match inspection.source_profile {
        Some(profile) => lines.push(Line::good(format!(
            "Game executable: {} ({})",
            profile.release, profile.id
        ))),
        None => lines.push(Line::bad("FIRSTENC.EXE is not a recognised release")),
    }
    lines.push(Line::good(format!(
        "{} files fingerprinted",
        inspection.files.len()
    )));
    for diagnostic in &inspection.diagnostics {
        match diagnostic.code {
            // The tool's own disclaimer, not a finding about these files.
            "inspection_only" => continue,
            // Floppy installations have no CD videos; that is normal.
            "no_video_media" => {
                lines.push(Line::good("No CD videos (normal for the floppy release)"));
                continue;
            }
            _ => {}
        }
        let text = if diagnostic.path.is_empty() {
            diagnostic.message.clone()
        } else {
            format!("{}: {}", diagnostic.path, diagnostic.message)
        };
        lines.push(match diagnostic.severity {
            ffe_tools::Severity::Error => Line::bad(text),
            ffe_tools::Severity::Warning => Line::warning(text),
        });
    }
    let release = inspection.source_profile.as_ref().and_then(release_of);
    let mut report = if inspection.inspection_passed && !lines.iter().any(|l| l.level == Level::Bad)
    {
        Report::from_lines(
            if shareware {
                "Shareware identified (experimental launch)"
            } else {
                "Game files verified"
            },
            "Game files have problems",
            lines,
        )
    } else {
        let mut report =
            Report::from_lines("Game files verified", "Game files have problems", lines);
        report.level = Level::Bad;
        report.summary = "Game files have problems".to_owned();
        report
    };
    report.release = release;
    report
}

/// Verify the selected game and create its packed music only on a missing-file
/// first run. Existing bundles are left untouched for the game to reuse.
pub fn verify_game_and_prepare_music(folder: &Path, music_bin: &Path) -> Report {
    let mut report = verify_game(folder);
    if report.level == Level::Bad {
        return report;
    }
    let line = if music_bin.exists() {
        Line::good(format!("Using existing {}", music_bin.display()))
    } else {
        let library = ffe_audio::MusicLibrary::from_game_dir(folder);
        match ffe_audio::RenderedMusic::ensure_music_bin(&library, music_bin) {
            Ok(true) => Line::good(format!(
                "Generated {} with all 17 rendered music tracks",
                music_bin.display()
            )),
            Ok(false) => Line::good(format!("Using existing {}", music_bin.display())),
            Err(error) => Line::warning(format!("Music bundle generation failed: {error}")),
        }
    };
    if line.level == Level::Warning && report.level == Level::Good {
        report.level = Level::Warning;
        report.summary = "Game files verified; music needs attention".to_owned();
    }
    report.lines.push(line);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_release_is_named_with_its_kind_and_version() {
        for (id, kind, text) in [
            ("ffe-cd-1.01-en", Kind::CdRom, "CD-ROM release 1.01"),
            ("ffe-floppy-1.02-en", Kind::Floppy, "Floppy release 1.02"),
            ("ffe-shareware-1.06-en", Kind::Shareware, "Shareware 1.06"),
        ] {
            let profile = ffe_tools::SourceProfile { id, release: "x" };
            let release = release_of(&profile).unwrap();
            assert_eq!(release.kind, kind);
            assert!(release.text.starts_with(text), "{}", release.text);
        }
        let unknown = ffe_tools::SourceProfile {
            id: "other",
            release: "x",
        };
        assert_eq!(release_of(&unknown), None);
    }
}
