//! Existing backlog PAR-23/PAR-24: source entry guards and fresh-commander replay.
use ffe_rs::{
    boot,
    navigation::Navigator,
    saves::{Commander, SaveGame},
};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn a_normal_load_rejects_missing_world_and_prototype_flight() {
    let cache = cache();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let (state, _) = boot::new_commander(&cache, &nav).unwrap();
    let mut game = SaveGame::new(Commander::default(), state, None);
    assert!(boot::validate_save(&cache, &game)
        .unwrap_err()
        .contains("navigation"));
    game.navigation = Some(nav.save_state());
    boot::validate_save(&cache, &game).unwrap();
    game.state.system_flight = false;
    assert!(boot::validate_save(&cache, &game)
        .unwrap_err()
        .contains("prototype"));
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn missing_essential_records_fail_without_constructing_a_fallback() {
    let cache = cache();
    for id in [
        "base:model/records",
        "base:ship/stats",
        "base:gameplay/tables",
        "base:gameplay/pads",
        "base:terrain/tables",
        "base:terrain/maps",
        "base:world/system-tables",
        "base:world/galaxy-tables",
    ] {
        let assets = cache
            .manifest()
            .files
            .iter()
            .filter(|record| record.id != id)
            .map(|record| ffe_content::Asset {
                id: record.id.clone(),
                path: record.path.clone(),
                kind: record.asset.clone(),
                bytes: cache.asset(&record.id).unwrap().to_vec(),
            })
            .collect();
        let bytes = ffe_content::build(cache.manifest().clone(), assets).unwrap();
        let broken = ffe_content::read(&bytes).unwrap();
        assert!(boot::validate_assets(&broken).unwrap_err().contains(id));
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn invalid_essential_records_fail_at_cache_validation_or_source_boot() {
    let cache = cache();
    // The cache reader validates table layouts before a VerifiedCache exists.
    // A truncated essential block must therefore fail import, not reach startup.
    for id in [
        "base:model/records",
        "base:ship/stats",
        "base:gameplay/tables",
        "base:gameplay/pads",
        "base:terrain/tables",
        "base:terrain/maps",
        "base:world/system-tables",
        "base:world/galaxy-tables",
    ] {
        let assets = cache
            .manifest()
            .files
            .iter()
            .map(|record| {
                let mut bytes = cache.asset(&record.id).unwrap().to_vec();
                if record.id == id {
                    bytes.truncate(1);
                }
                ffe_content::Asset {
                    id: record.id.clone(),
                    path: record.path.clone(),
                    kind: record.asset.clone(),
                    bytes,
                }
            })
            .collect();
        let error = ffe_content::build(cache.manifest().clone(), assets).unwrap_err();
        assert!(!error.is_empty(), "truncated {id} was accepted");
    }

    // A correctly sized but empty ship slot passes the raw table format check;
    // source boot must reject the selected ship's unusable statistics.
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let ship = nav.new_game.unwrap().ship as usize;
    let stats_id = "base:ship/stats";
    let assets = cache
        .manifest()
        .files
        .iter()
        .map(|record| {
            let mut bytes = cache.asset(&record.id).unwrap().to_vec();
            if record.id == stats_id {
                bytes[ship * 48..(ship + 1) * 48].fill(0);
            }
            ffe_content::Asset {
                id: record.id.clone(),
                path: record.path.clone(),
                kind: record.asset.clone(),
                bytes,
            }
        })
        .collect();
    let invalid =
        ffe_content::read(&ffe_content::build(cache.manifest().clone(), assets).unwrap()).unwrap();
    let error = boot::new_commander(&invalid, &nav).expect_err("invalid ship data must fail");
    assert!(
        error.contains("flight statistics"),
        "unexpected boot error: {error}"
    );
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn fresh_boot_and_post_death_boot_have_identical_source_state() {
    let cache = cache();
    for start in [0, 1, 2, 3, 5] {
        let nav = Navigator::new_game(&cache, start).unwrap();
        let (fresh, fresh_model) = boot::new_commander(&cache, &nav).unwrap();
        let (mut destroyed, _) = boot::new_commander(&cache, &nav).unwrap();
        destroyed.take_damage(u32::MAX, 28);
        assert!(destroyed.destroyed());
        let restarted_nav = Navigator::new_game(&cache, start).unwrap();
        let (restarted, restarted_model) = boot::new_commander(&cache, &restarted_nav).unwrap();
        assert_eq!(fresh, restarted);
        assert_eq!(fresh_model.player(), restarted_model.player());
        assert_eq!(nav.save_state(), restarted_nav.save_state());
        assert_eq!(fresh_model.mode(), nav.new_game.unwrap().player_state);
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_START_RECORDS"]
fn all_delivered_start_records_match_the_executed_original() {
    let cache = cache();
    let rows: Vec<serde_json::Value> =
        std::fs::read_to_string(std::env::var_os("FFE_START_RECORDS").unwrap())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(rows.len(), 5);
    for row in rows {
        let start = row["start_type"].as_u64().unwrap() as u8;
        let nav = Navigator::new_game(&cache, start).unwrap();
        let (mut state, model) = boot::new_commander(&cache, &nav).unwrap();
        let stats = boot::ship_stats(&cache, state.ship).unwrap();
        state.ensure_damage(stats.mass * 4);
        assert_eq!(u64::from(state.ship), row["ship"].as_u64().unwrap());
        assert_eq!(u64::from(state.credits), row["cash"].as_u64().unwrap());
        assert_eq!(u64::from(model.mode()), row["mode"].as_u64().unwrap());
        assert_eq!(
            u64::from(nav.new_game.unwrap().player_index),
            row["player"].as_u64().unwrap()
        );
        assert_eq!(
            nav.hold(&cache, state.ship) as u64,
            row["hold"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(state.loadout.general),
            row["general"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(state.combat.drive),
            row["drive"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(state.combat.energy),
            row["energy"].as_u64().unwrap()
        );
        assert_eq!(u64::from(state.hull[0]), row["hull"].as_u64().unwrap());
        assert_eq!(
            u64::from(state.shields[1]),
            row["shields"].as_u64().unwrap()
        );
        assert_eq!(
            serde_json::to_value(state.combat.lasers).unwrap(),
            row["lasers"]
        );
        assert_eq!(
            serde_json::to_value(&state.combat.missiles[..4]).unwrap(),
            row["missiles"]
        );
        assert_eq!(
            u64::from(state.hold[ffe_core::hyperspace::HYDROGEN]),
            row["fuel"].as_u64().unwrap()
        );
        assert_eq!(
            state.hold.iter().map(|v| u64::from(*v)).sum::<u64>(),
            row["used"].as_u64().unwrap()
        );
        assert_eq!(
            ffe_rs::trade::capacity(&nav, &cache, &state),
            nav.hold(&cache, state.ship)
        );
        if start == 3 {
            assert_eq!(nav.legal.fines[1..3], [700000, 700000]);
        }
        state.validate().unwrap();
    }
}

/// PAR-24: the RIP restart (`main.rs`'s `IntroExit::Start` arm) takes the same
/// two calls as the first boot, `Navigator::new_game` + `boot::new_commander`,
/// and resets the commander record to `Commander::default()`. This pins that
/// sequence to the initial boot: the dead commander's name, ranks, missions,
/// cash, ship, hold/fuel, damage, service days and legal state, and the dead
/// model's view, all come back as the first boot's values. HUD inputs are the
/// state fields the console reads (`mode`/`view`/`position`); the per-game UI
/// locals live in `main.rs` and are reset by the caller, not by this layer.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn a_post_death_restart_builds_the_first_boots_commander_and_state() {
    let cache = cache();
    for start in [0, 1] {
        // The initial Enter boot.
        let first_nav = Navigator::new_game(&cache, start).unwrap();
        let (first_state, first_model) = boot::new_commander(&cache, &first_nav).unwrap();
        let first_commander = Commander::default();

        // The commander that died: every field the restart must replace.
        let mut dead_nav = Navigator::new_game(&cache, start).unwrap();
        let (mut dead_state, mut dead_model) = boot::new_commander(&cache, &dead_nav).unwrap();
        let dead_commander = Commander {
            name: "Dead".to_owned(),
            ranks: [3, 5],
            missions: vec![ffe_rs::saves::Mission {
                id: 3,
                system: Some(1),
                completed: true,
            }],
        };
        let mass = boot::ship_stats(&cache, dead_state.ship).unwrap().mass;
        dead_state.take_damage(u32::MAX, mass);
        dead_state.credits = 7;
        dead_state.hold = vec![0; ffe_core::market::NAMES];
        dead_state.hold[0] = 3;
        dead_state.damage.engine_failed = true;
        dead_state.damage.equipment = 0xffff;
        dead_state.service_days = [40, 41];
        dead_state.last_service_day = Some(30);
        dead_state.loadout.general = 0;
        dead_state.combat.lasers = [0; 4];
        dead_state.position = [1.0, 2.0, 3.0];
        dead_model.set_view(2, [12, 34]);
        dead_nav.clock += 9_000.0;
        dead_nav.legal.fines[1] = 5_000;
        dead_nav.legal.wanted = true;
        assert!(dead_state.destroyed());
        assert_ne!(dead_commander, Commander::default());
        assert_ne!(dead_state, first_state);

        // The restart after GAME OVER.
        let restart_nav = Navigator::new_game(&cache, start).unwrap();
        let (restart_state, restart_model) = boot::new_commander(&cache, &restart_nav).unwrap();
        let restart_commander = Commander::default();

        // The same source world, frame model, seat, rows and view as the first boot.
        assert_eq!(restart_state, first_state);
        assert_eq!(restart_model, first_model);
        assert_eq!(restart_nav.save_state(), first_nav.save_state());
        assert_eq!(restart_model.player(), first_model.player());
        assert_eq!(restart_model.mode(), first_model.mode());
        assert_eq!(restart_model.view(), first_model.view());
        assert_eq!(
            restart_model.absolute_position(),
            first_model.absolute_position()
        );
        assert_eq!(restart_model.set_speed_units(), 0);
        // Nothing carries over from the dead commander.
        assert_eq!(restart_commander, first_commander);
        assert_ne!(restart_commander, dead_commander);
        assert!(!restart_state.destroyed());
        assert_eq!(restart_state.credits, first_state.credits);
        assert_eq!(restart_state.ship, first_state.ship);
        assert_eq!(restart_state.hold, first_state.hold);
        assert_eq!(
            restart_state.hold[ffe_core::hyperspace::HYDROGEN],
            first_state.hold[ffe_core::hyperspace::HYDROGEN]
        );
        assert_eq!(restart_state.damage, first_state.damage);
        assert_eq!(restart_state.service_days, first_state.service_days);
        assert_eq!(restart_state.last_service_day, first_state.last_service_day);
        assert_eq!(restart_state.loadout, first_state.loadout);
        assert_eq!(restart_state.combat, first_state.combat);
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn a_saved_pad_resumes_its_supporting_planet_and_parked_handler() {
    let cache = cache();
    for start in [0, 1, 2] {
        let nav = Navigator::new_game(&cache, start).unwrap();
        let (state, _) = boot::new_commander(&cache, &nav).unwrap();
        let game = SaveGame::new(Commander::default(), state.clone(), Some(nav.save_state()));
        let loaded =
            ffe_rs::saves::decode_game(&ffe_rs::saves::encode_game(&game).unwrap()).unwrap();
        let nav = Navigator::from_save(&cache, loaded.navigation.as_ref().unwrap()).unwrap();
        let mut model = boot::resume_model(&cache, &loaded.state, &nav, 0).unwrap();
        assert!(model.on_pad());
        assert_eq!(model.mode(), 0x30);
        let before = model.absolute_position();
        for _ in 0..600 {
            model.step_landed(0.02, &Default::default(), 0x30);
        }
        assert_eq!(model.absolute_position(), before);
        assert!(model.on_pad());
    }
}
