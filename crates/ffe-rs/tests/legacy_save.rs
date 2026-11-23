//! Local-only: the supplied DOS-era commander file replayed against the
//! original.
//!
//! `tools/reference/verify-legacy-save.py` runs `jjffe-smoke --legacy-save
//! game/FIRSTENC._01` (the real F70/F66/F171) and writes its one JSONL row to
//! `FFE_LEGACY_SAVE_TRACE`. This test imports the same file through
//! `ffe_rs::saves::legacy_import` and requires every field and both raw blocks
//! (the object array and the player block) to match the reference exactly.
//!
//! The reference's raw player block is the authority for the DOS-era field map:
//! it is the source `gmstruct` layout (`jjffe-master/srctools/gmstruct.txt`), not
//! the shifted block the earlier seven-dword scatter misparse produced
//! (`docs/notes/persistence.md`).
use ffe_rs::saves::{legacy_import, LEGACY_PLAYER_BYTES};
use serde_json::Value;

fn read(name: &str) -> String {
    let path = std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn int(value: &Value) -> u64 {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("not an integer: {value}"))
}

#[test]
#[ignore = "Local-only: needs FFE_LEGACY_SAVE_TRACE and game/FIRSTENC._01"]
fn the_supplied_legacy_save_matches_the_reference_player_block() {
    let trace: Value = serde_json::from_str(read("FFE_LEGACY_SAVE_TRACE").trim()).unwrap();
    assert_eq!(trace["scenario"], "jjffe-legacy-save-v1");
    let path = std::env::var("FFE_LEGACY_SAVE")
        .unwrap_or_else(|_| format!("{}/../../game/FIRSTENC._01", env!("CARGO_MANIFEST_DIR")));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let legacy = legacy_import(&bytes).expect("the supplied original imports read-only");

    // Container facts.
    assert_eq!(legacy.body_offset, int(&trace["body_offset"]) as usize);
    assert_eq!(legacy.body_bytes, int(&trace["body_bytes"]) as usize);
    assert_eq!(legacy.player_bytes, int(&trace["player_bytes"]) as usize);
    assert_eq!(legacy.player_bytes, LEGACY_PLAYER_BYTES);
    assert_eq!(LEGACY_PLAYER_BYTES, 0x5535, "the DOS block size");

    // The player block and the object array, byte for byte.
    let raw_player = trace["raw_player"].as_str().unwrap();
    let raw_objects = trace["raw_objects"].as_str().unwrap();
    assert_eq!(legacy.player.raw.len(), LEGACY_PLAYER_BYTES);
    assert_eq!(
        legacy.player.raw,
        hex(raw_player),
        "the player block differs from the original's"
    );
    assert_eq!(
        legacy.objects,
        hex(raw_objects),
        "the object array differs from the original's"
    );

    // Every recovered commander field, field for field.
    assert_eq!(legacy.player.name, trace["name"].as_str().unwrap());
    assert_eq!(legacy.player.credits, int(&trace["credits"]) as u32);
    assert_eq!(
        legacy.player.cargo_capacity,
        int(&trace["cargo_capacity"]) as u32
    );
    assert_eq!(legacy.player.cargo_used, int(&trace["cargo_used"]) as u32);
    assert_eq!(
        legacy.player.hydrogen_fuel,
        int(&trace["hydrogen_fuel"]) as u16
    );
    assert_eq!(
        legacy.player.military_fuel,
        int(&trace["military_fuel"]) as u16
    );
    assert_eq!(
        legacy.player.radioactives,
        int(&trace["radioactives"]) as u16
    );
    assert_eq!(legacy.player.clock_tics, int(&trace["clock_tics"]) as u32);
    assert_eq!(legacy.player.clock_days, int(&trace["clock_days"]) as u32);
    assert_eq!(legacy.player.random_a, int(&trace["random_a"]) as u32);
    assert_eq!(legacy.player.random_b, int(&trace["random_b"]) as u32);
    assert_eq!(
        legacy.player.player_object,
        int(&trace["ship_object"]) as u32
    );
    assert_eq!(legacy.player.ship_model, int(&trace["ship_model"]) as u32);
    assert_eq!(
        legacy.player.ship_name,
        trace["ship_name"].as_str().unwrap()
    );

    // The values the reference establishes, so a trace edited together with
    // the importer still fails: the default Jameson at Gateway.
    assert_eq!(legacy.player.name, "Jameson");
    assert_eq!(legacy.player.credits, 10_000);
    assert_eq!(legacy.player.ship_model, 31, "the Saker Mk III");
    assert_eq!(legacy.player.ship_name, "TY-198");
    assert_eq!(legacy.player.player_object, 87);
    assert_eq!(legacy.player.clock_tics, 1_014_497_784);
    assert_eq!(legacy.player.clock_days, 1_187_064);
    assert_eq!(legacy.player.random_a, 0x38_acd1);
    assert_eq!(legacy.player.random_b, 0x0fc1_68d5);
}

fn hex(text: &str) -> Vec<u8> {
    assert!(text.len() % 2 == 0, "odd hex length");
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
