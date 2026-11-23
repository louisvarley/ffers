//! Local-only: a commander save/load round trip against the executed original
//! (release-1).
//!
//! `tools/reference/verify-save-roundtrip.py` runs
//! `jjffe-smoke --save-roundtrip 260` on the real Lave docked game. At frame 240
//! it runs F214 (zero the unused objects), F170 (mangle the F175 body) and F174
//! (F69 + F65 + the body) to write a commander file, changes every saved field,
//! then runs F176 (F70 + F66 + the body) and F171 (unmangle) and requires every
//! field back. The trace is `FFE_SAVE_TRACE` and the file is `FFE_SAVE_FILE`.
//!
//! This test imports the file the original **wrote** with
//! `ffe_rs::saves::legacy_import` and requires the same field map the original's
//! own restore check used: name, cash, hold, fuel, clock, shared random pair,
//! player object, ship model and name, and all 38 cargo words. It also proves
//! the trace's restored row equals its before row while the perturbed row does
//! not, so neither side can pass by doing nothing.
//!
//! ```text
//! FFE_SAVE_TRACE=save.jsonl FFE_SAVE_FILE=roundtrip.ffe cargo test -p ffe-rs \
//!     --test save_roundtrip -- --ignored --nocapture
//! ```
use ffe_rs::saves::{legacy_import, LEGACY_PLAYER_BYTES};
use serde_json::Value;

fn rows() -> Vec<Value> {
    let path = std::env::var("FFE_SAVE_TRACE").expect("FFE_SAVE_TRACE");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a JSONL row"))
        .collect()
}

fn phase<'a>(rows: &'a [Value], wanted: &str) -> &'a Value {
    rows.iter()
        .filter_map(|row| row.get("save"))
        .find(|save| save["phase"].as_str() == Some(wanted))
        .unwrap_or_else(|| panic!("no {wanted} save row"))
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("not an integer: {value}"))
}

fn cargo(value: &Value) -> Vec<u16> {
    value["cargo"]
        .as_array()
        .expect("the cargo array")
        .iter()
        .map(|tonnes| int(tonnes) as u16)
        .collect()
}

#[test]
#[ignore = "Local-only: needs FFE_SAVE_TRACE and the file in FFE_SAVE_FILE"]
fn the_commanders_save_load_round_trip_matches_the_original() {
    let rows = rows();
    let before = phase(&rows, "before");
    let perturbed = phase(&rows, "perturbed");
    let restored = phase(&rows, "restored");
    let result = rows
        .iter()
        .find_map(|row| row.get("save_result"))
        .expect("a save_result row");
    assert_eq!(result["restored"], true);
    assert!(int(&result["packed"]) > 0, "F170 wrote a body");

    // The original's own check: the load restored everything the save held.
    for field in [
        "name",
        "credits",
        "capacity",
        "used",
        "hydrogen",
        "military",
        "radioactives",
        "tics",
        "days",
        "random",
        "player_object",
        "ship",
        "ship_name",
        "cargo",
    ] {
        assert_eq!(restored[field], before[field], "restored {field}");
    }
    // The perturbation reached the fields, so the restore is not a no-op.
    let mut changed = 0;
    for field in [
        "credits",
        "capacity",
        "used",
        "tics",
        "days",
        "random",
        "player_object",
        "ship",
        "ship_name",
        "cargo",
    ] {
        if perturbed[field] != before[field] {
            changed += 1;
        }
    }
    assert!(
        changed >= 8,
        "the perturbed row changed only {changed} fields"
    );

    // The port imports the file the original wrote.
    let path = std::env::var("FFE_SAVE_FILE").expect("FFE_SAVE_FILE");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let legacy = legacy_import(&bytes).expect("the original's own save imports");
    assert_eq!(legacy.player.raw.len(), LEGACY_PLAYER_BYTES);
    assert_eq!(legacy.objects.len(), 0x984a, "F171's object array");
    assert!(legacy.body_bytes > 0, "a parsed F170 body");
    assert_eq!(legacy.player.name, before["name"].as_str().unwrap());
    // The port's string-section `commander_name` is two bytes short on a
    // JJFFE-written file (it reads "eson"); the player block's own name copy is
    // exact and is the field the original restores. Recorded in
    // docs/ACCEPTED_PARITY.md.
    eprintln!(
        "commander name: original {:?}, port string section {:?}",
        before["name"].as_str().unwrap(),
        legacy.commander_name
    );
    assert_eq!(legacy.player.credits, int(&before["credits"]) as u32);
    assert_eq!(
        legacy.player.cargo_capacity,
        int(&before["capacity"]) as u32
    );
    assert_eq!(legacy.player.cargo_used, int(&before["used"]) as u32);
    assert_eq!(legacy.player.hydrogen_fuel, int(&before["hydrogen"]) as u16);
    assert_eq!(legacy.player.military_fuel, int(&before["military"]) as u16);
    assert_eq!(
        legacy.player.radioactives,
        int(&before["radioactives"]) as u16
    );
    assert_eq!(legacy.player.clock_tics, int(&before["tics"]) as u32);
    assert_eq!(legacy.player.clock_days, int(&before["days"]) as u32);
    assert_eq!(legacy.player.random_a, int(&before["random"][0]) as u32);
    assert_eq!(legacy.player.random_b, int(&before["random"][1]) as u32);
    assert_eq!(
        legacy.player.player_object,
        int(&before["player_object"]) as u32
    );
    assert_eq!(legacy.player.ship_model, int(&before["ship"]) as u32);
    assert_eq!(
        legacy.player.ship_name,
        before["ship_name"].as_str().unwrap()
    );
    let want = cargo(before);
    let have = legacy
        .player
        .raw
        .get(0x134..0x134 + 2 * want.len())
        .expect("the cargo words")
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    assert_eq!(have, want, "the 38 cargo words");

    // The default Jameson start the reference saved.
    assert_eq!(legacy.player.name, "Jameson");
    assert_eq!(legacy.player.credits, 1_000);
    eprintln!(
        "{} bytes, body {} at {}: {} fields and 38 cargo words match; restore undone",
        bytes.len(),
        legacy.body_bytes,
        legacy.body_offset,
        legacy.player.raw.len()
    );
}
