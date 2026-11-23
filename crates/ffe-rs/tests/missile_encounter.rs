//! Missile, ECM, mine and escape outcomes from the executed original (PAR-22).
//!
//! `FFE_MISSILE_TRACE` is a directory of the five
//! `jjffe-smoke.exe --missile-encounter <scenario> 900 --start-key=0x39`
//! stdout captures (`hit.jsonl`, `ecm.jsonl`, `naval.jsonl`, `mine.jsonl`,
//! `escape.jsonl`). Each row has the player, the hostile group, the missile
//! objects and F719/F942's effect objects. The test checks:
//!
//! * `hit`: a model-6 player missile reaches slot 70, damages it, and leaves
//!   F719's model-0x9b explosion;
//! * `ecm`: standard E.C.M. (F935 bit 0x4) destroys model 6;
//! * `naval`: model 7 survives the standard burst and is destroyed by naval
//!   E.C.M. (bit 0x8);
//! * `mine`: 0xc1 launches model 5, AI state 0x14, with the launch impulse and
//!   no expiry;
//! * `escape`: F1098 leaves model 16 (Stowmaster), the abandoned hull in state
//!   0x16 and the new craft at 0xff.
//!
//! ```text
//! FFE_MISSILE_TRACE=.cache/replay cargo test -p ffe-rs --test missile_encounter -- --ignored --nocapture
//! ```
use serde_json::Value;

fn rows(scenario: &str) -> Vec<Value> {
    let dir = std::env::var("FFE_MISSILE_TRACE").expect("FFE_MISSILE_TRACE");
    let path = std::path::Path::new(&dir).join(format!("{scenario}.jsonl"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"enc\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn frame(row: &Value) -> u64 {
    row["enc"].as_u64().unwrap()
}

fn missiles(row: &Value) -> &Vec<Value> {
    row["missiles"].as_array().unwrap()
}

fn explosion(row: &Value) -> bool {
    row["effects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|effect| effect["model"].as_u64() == Some(155))
}

fn explosion_after(rows: &[Value], last_missile: u64) -> Option<u64> {
    rows.iter()
        .filter(|row| frame(row) >= last_missile && frame(row) <= last_missile + 5)
        .find(|row| explosion(row))
        .map(frame)
}

fn missile_frames(rows: &[Value], model: u64) -> Vec<u64> {
    rows.iter()
        .filter(|row| {
            missiles(row)
                .iter()
                .any(|missile| missile["model"].as_u64() == Some(model))
        })
        .map(frame)
        .collect()
}

#[test]
#[ignore = "needs the reference traces in FFE_MISSILE_TRACE"]
fn a_player_missile_hits_and_leaves_f719s_explosion() {
    let rows = rows("hit");
    assert!(rows.len() > 100);
    let first = &rows[0];
    let player_slot = first["player"]["slot"].as_u64().unwrap();
    let missile = missiles(first)
        .iter()
        .find(|missile| missile["model"].as_u64() == Some(6))
        .expect("a model-6 player missile");
    assert_eq!(missile["ai"].as_u64(), Some(19), "F941's state 0x13");
    assert_eq!(missile["owner"].as_u64(), Some(player_slot));
    assert_eq!(missile["target"].as_u64(), Some(70));
    let present = missile_frames(&rows, 6);
    let last = *present.last().unwrap();
    assert!(last < 900, "the missile never ended: {present:?}");
    let explosion_at = explosion_after(&rows, last).expect("F719 after the hit");
    let hull_first = rows[0]["ships"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ship| ship["slot"].as_u64() == Some(70))
        .unwrap()["hull"]
        .as_u64()
        .unwrap();
    let hull_last = rows.last().unwrap()["ships"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ship| ship["slot"].as_u64() == Some(70))
        .unwrap()["hull"]
        .as_u64()
        .unwrap();
    assert!(
        hull_last < hull_first,
        "the missile did not damage the target"
    );
    eprintln!(
        "hit: missile {present:?} ... {last}, F719 at {explosion_at}, target hull {hull_first} -> {hull_last}"
    );
}

#[test]
#[ignore = "needs the reference traces in FFE_MISSILE_TRACE"]
fn standard_and_naval_ecm_choose_their_models() {
    // Standard E.C.M. destroys model 6 on the burst frame's next update.
    let standard = rows("ecm");
    let present = missile_frames(&standard, 6);
    let last = *present.last().unwrap();
    assert!(
        last <= 510,
        "standard ECM did not destroy model 6: {present:?}"
    );
    let explosion_at = explosion_after(&standard, last).expect("F719 after ECM");
    eprintln!("ecm: model 6 ends at {last}, F719 at {explosion_at}");

    // Model 7's F718 branch is naval-only: it survives the frame-502 standard
    // burst and ends after the harness's frame-530 naval burst.
    let naval = rows("naval");
    let present = missile_frames(&naval, 7);
    let last = *present.last().unwrap();
    assert!(
        present.iter().any(|frame| *frame >= 520) && last >= 530,
        "model 7 did not survive standard ECM: {present:?}"
    );
    assert!(last < 900);
    let explosion_at = explosion_after(&naval, last).expect("F719 after naval ECM");
    eprintln!("naval: model 7 ends at {last}, F719 at {explosion_at}");
}

#[test]
#[ignore = "needs the reference traces in FFE_MISSILE_TRACE"]
fn a_mine_keeps_f941s_state_and_never_expires() {
    let rows = rows("mine");
    let first = &rows[0];
    let player_slot = first["player"]["slot"].as_u64().unwrap();
    let missile = missiles(first)
        .iter()
        .find(|missile| missile["model"].as_u64() == Some(5))
        .expect("a model-5 mine");
    assert_eq!(
        missile["ai"].as_u64(),
        Some(0x14),
        "F941's non-model-4 mine state"
    );
    assert_eq!(missile["owner"].as_u64(), Some(player_slot));
    let present = missile_frames(&rows, 5);
    assert_eq!(present.len(), rows.len(), "the mine expired");
    let start = first["missiles"][0]["position"].as_array().unwrap();
    let end = rows.last().unwrap()["missiles"][0]["position"]
        .as_array()
        .unwrap();
    let moved: i64 = (0..3)
        .map(|k| (start[k].as_i64().unwrap() - end[k].as_i64().unwrap()).abs())
        .sum();
    assert!(
        moved > 1_000_000,
        "the mine did not keep its launch impulse"
    );
    eprintln!(
        "mine: {} frames, state 0x14, movement {moved}",
        present.len()
    );
}

#[test]
#[ignore = "needs the reference traces in FFE_MISSILE_TRACE"]
fn f1098_leaves_the_stowmaster_and_the_abandoned_hull() {
    let rows = rows("escape");
    let before = rows
        .iter()
        .find(|row| frame(row) == 500)
        .expect("the launch frame");
    let old_slot = before["player"]["slot"].as_u64().unwrap();
    let old_model = before["player"]["model"].as_u64().unwrap();
    let after = rows
        .iter()
        .find(|row| row["player"]["model"].as_u64() == Some(16))
        .expect("F1098's Stowmaster");
    assert_eq!(frame(after), 502, "F1098 did not run at the scripted frame");
    assert_eq!(
        after["player"]["type"].as_u64(),
        Some(0xff),
        "F1098's new craft type"
    );
    assert_ne!(after["player"]["slot"].as_u64(), Some(old_slot));
    let abandoned = after["ships"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ship| ship["slot"].as_u64() == Some(old_slot))
        .expect("the abandoned hull");
    assert_eq!(abandoned["model"].as_u64(), Some(old_model));
    assert_eq!(
        abandoned["state"].as_u64(),
        Some(0x16),
        "F1098's drifting state"
    );
    assert_eq!(
        after["player"]["model"].as_u64(),
        Some(ffe_rs::death::EscapeCraft::Stowmaster.model() as u64),
        "the port's F1098 craft model"
    );
    eprintln!("escape: slot {old_slot} model {old_model} -> model 16, abandoned state 0x16");
}

/// The port's side of the F719 gap: a normal hit and a timeout both leave the
/// model-0x9b explosion, as F758/F719 do. Cache-backed.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_port_spawns_f719_on_a_missile_hit_and_on_a_timeout() {
    let bytes = std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap();
    let cache = ffe_content::read(&bytes).unwrap();
    let mut nav = ffe_rs::navigation::Navigator::new_game(&cache, 1).unwrap();
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.spawn_traffic(&cache, at, forward);
    nav.docked = None;
    let pirate = nav
        .ships
        .iter()
        .find(|ship| ship.role == ffe_core::npc::Role::Pirate)
        .or_else(|| nav.ships.first())
        .expect("a ship");
    let target_id = pirate.id;
    let target = pirate.position;
    let from = [target[0], target[1], target[2] - 120_000];
    let records = cache.asset("base:model/records").unwrap();
    let stats = cache.asset("base:ship/stats").unwrap();
    let launcher_radius =
        ffe_core::objects::ModelInfo::read(records, stats, ffe_rs::navigation::NEW_GAME_SHIP)
            .map_or(0, |info| info.radii().1);
    let spec = ffe_rs::missile::MissileField::spec(ffe_core::missile::PLAYER_MISSILE, None, 2100)
        .expect("the player missile spec");
    nav.launch_missile(
        spec,
        ffe_core::missile::PLAYER_MISSILE,
        0,
        target_id,
        from,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        1,
        launcher_radius,
    );
    // Put the missile on the target so the fuse fires on the next tick.
    nav.missiles.missiles[0].position = [target[0], target[1], target[2] - 100_000];
    nav.tick_missiles_at(0.4, from, [0.0; 3], 0);
    assert!(
        nav.effects
            .iter()
            .any(|effect| effect.model == ffe_core::effects::EXPLOSION_MODEL),
        "a missile hit left no F719 explosion"
    );

    // A timeout: the same missile fired at nothing still leaves F719 at the end.
    let spec =
        ffe_rs::missile::MissileField::spec(ffe_core::missile::PLAYER_MISSILE, None, 2100).unwrap();
    nav.missiles.missiles.clear();
    nav.effects.clear();
    nav.launch_missile(
        spec,
        ffe_core::missile::PLAYER_MISSILE,
        0,
        target_id,
        from,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        1,
        launcher_radius,
    );
    nav.missiles.missiles[0].position = [target[0] + 100_000_000, target[1], target[2]];
    nav.missiles.missiles[0].updates = ffe_core::missile::LIFETIME_UPDATES;
    nav.tick_missiles_at(0.4, from, [0.0; 3], 0);
    assert!(
        nav.effects
            .iter()
            .any(|effect| effect.model == ffe_core::effects::EXPLOSION_MODEL),
        "a missile timeout left no F719 explosion"
    );
}
