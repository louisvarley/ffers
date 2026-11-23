//! Deterministic resume regressions for the ten PAR-18 follow-up tasks.
use super::*;
use crate::saves::{self, Commander, SaveGame};

fn cache() -> VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

fn resume(cache: &VerifiedCache, nav: &Navigator) -> Navigator {
    let game = SaveGame::new(
        Commander::default(),
        crate::flight::Flight::default(),
        Some(nav.save_state()),
    );
    let loaded = saves::decode_game(&saves::encode_game(&game).unwrap()).unwrap();
    Navigator::from_save(cache, loaded.navigation.as_ref().unwrap()).unwrap()
}

fn launch(nav: &mut Navigator) {
    nav.missiles.launch(
        ffe_core::missile::Spec::from_words(6, 2000, -1000, 100),
        0x82,
        ffe_core::missile::PLAYER_TARGET,
        None,
        [0; 3],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        1,
        100,
    );
    // F941/F928: the missile takes its object slot (the live launch path does
    // this in `Navigator::launch_missile`).
    nav.sync_object_slots();
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn the_mission_fix_ship_serial_resumes_and_off_saves_keep_the_original_words() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    // Off: the serial is never used and never written.
    assert!(serde_json::to_string(&nav.save_state())
        .unwrap()
        .find("script_ship_serial")
        .is_none());
    // On: an id above the old system's continues after a jump and a load, so
    // a mission's stored target id cannot name a new system's ship.
    nav.mission_fixes = true;
    assert_eq!(nav.script_ship_id(40), 40);
    let mut loaded = resume(&cache, &nav);
    loaded.mission_fixes = true;
    assert_eq!(loaded.script_ship_id(1), 41);
    assert_eq!(loaded.script_ship_id(1), 42);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn career_points_and_medals_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.career.award_federal(250);
    nav.career.award_imperial(400);
    nav.career.award_kill(200);
    nav.career.award_medal(7);
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.career, nav.career);
    assert_eq!(
        loaded.career.award_federal(100),
        nav.career.award_federal(100)
    );
    assert!(!loaded.career.award_medal(7));
    assert_eq!(loaded.career, nav.career);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn live_missiles_resume_guidance_and_allocate_new_ids() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    launch(&mut nav);
    let decoy = nav.deploy_chaff([100_000_000, 50_000_000, 100_000_000], 100);
    nav.missiles.missiles[0].target = Some(decoy);
    nav.missile_view = Some(0);
    nav.tick_missiles(0.23, [0; 3]);
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.viewed_missile(), nav.viewed_missile());
    assert!(loaded.viewed_missile().is_some());
    for _ in 0..30 {
        assert_eq!(
            loaded.tick_missiles(0.02, [0; 3]),
            nav.tick_missiles(0.02, [0; 3])
        );
        assert_eq!(loaded.missiles, nav.missiles);
    }
    launch(&mut nav);
    launch(&mut loaded);
    assert_eq!(loaded.missiles, nav.missiles);
    assert_ne!(
        loaded.missiles.missiles[0].id,
        loaded.missiles.missiles[1].id
    );
    assert_eq!(loaded.ecm(false), nav.ecm(false));
    assert_eq!(loaded.effects, nav.effects);
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn chaff_and_next_object_id_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    for i in 0..20 {
        nav.deploy_chaff([i, 100, 200], 50);
    }
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.decoys, nav.decoys);
    assert_eq!(
        loaded.deploy_chaff([1, 2, 3], 70),
        nav.deploy_chaff([1, 2, 3], 70)
    );
    assert_eq!(loaded.decoys, nav.decoys);
    assert_eq!(loaded.decoys.len(), 16);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn effects_resume_without_duplicate_script_canisters_or_rng_draws() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.effects.clear();
    nav.effects
        .push(effects::departure_cloud([1, 2, 3], 100000));
    nav.effects.push(effects::exit_cloud([4, 5, 6], 100000));
    let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    nav.effects.push_burst(effects::explosion(
        [100, 0, 0],
        [0.0; 3],
        3,
        axes,
        &mut nav.random,
    ));
    let mut canister =
        effects::canister([1000, 0, 0], [1.0; 3], axes, [0.0; 3], 15, &mut nav.random);
    canister.tag = Some(123);
    nav.effects.push(canister);
    nav.tick_effects(0.17, 8000);
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.effects, nav.effects);
    assert_eq!(loaded.random, nav.random);
    for _ in 0..20 {
        nav.tick_effects(0.02, 994);
        loaded.tick_effects(0.02, 994);
        assert_eq!(loaded.effects, nav.effects);
        assert_eq!(loaded.random, nav.random);
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn live_ship_slots_and_object_records_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    assert!(!nav.ships.is_empty(), "the new game spawns traffic");
    // F927: every ship has a slot from 0x72 down, none below 0x14, no clash.
    let mut assigned: Vec<u8> = nav.ships.iter().map(|ship| ship.slot).collect();
    assigned.sort_unstable();
    assigned.dedup();
    assert_eq!(assigned.len(), nav.ships.len());
    assert!(assigned.iter().all(|slot| *slot >= 0x14));
    // DATA_008857: the player object is reserved above the traffic.
    assert_eq!(nav.player_object_slot(), Some(0x72));
    assert!(assigned.iter().all(|slot| *slot < 0x72));
    for ship in &nav.ships {
        assert_eq!(nav.ship_slot(ship.id), Some(ship.slot));
    }
    // A hostile with no ship target reports the player object's index.
    nav.ships[0].hostile = true;
    nav.ships[0].npc_target = None;
    nav.ships[0].state = npc::State::Attack;
    let record = nav.ship_object_record(nav.ships[0].id).unwrap();
    assert_eq!(record.agent.target, 0x72);
    // F941: the player's missile takes its own slot and its record names the
    // player object as the launcher (`+0x100`).
    launch(&mut nav);
    let records = nav.missile_object_records();
    let (missile_slot, record) = &records[0];
    assert_eq!(record.object, *missile_slot);
    assert!(*missile_slot < 0x72);
    assert_eq!(record.agent.nav_target, 0x72);
    assert_eq!(record.agent.kind, ffe_core::object_record::KIND_MISSILE);
    // The record carries the stored source fields and its own slot, and it
    // round-trips through the 0x152-byte slot layout.
    for (slot, record) in nav.object_records() {
        let ship = nav.ships.iter().find(|ship| ship.slot == slot).unwrap();
        assert_eq!(record.object, slot);
        assert_eq!(record.model, ship.model);
        assert_eq!(record.agent.flags, ship.class_flags);
        assert_eq!(record.drive, ship.drive);
        assert_eq!(record.agent.collision_radius, ship.collision_radius());
        // `+0x100` is the destination body's object slot, not its list index.
        if let Some(body) = ship.station.and_then(|index| nav.bodies.get(index)) {
            assert_eq!(usize::from(record.agent.nav_target), body.slot);
        }
        let mut list = PhysList::default();
        list.set_record(usize::from(slot), &record);
        assert_eq!(list.record(usize::from(slot)), record);
    }
    // The slots are saved with the ships and rebuilt on load.
    let loaded = resume(&cache, &nav);
    assert_eq!(loaded.ships.len(), nav.ships.len());
    for ship in &nav.ships {
        assert_eq!(loaded.ship_slot(ship.id), Some(ship.slot));
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn ship_autopilot_target_resumes() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.docked = None;
    nav.landed = None;
    nav.takeoff = None;
    nav.sequence = None;
    let id = nav.ships.iter().find(|s| s.visible()).unwrap().id;
    assert!(nav.engage_ship(id).is_some());
    let loaded = resume(&cache, &nav);
    assert_eq!(loaded.ship_target, Some(id));
    // Saving between target removal and the next autopilot update stays valid.
    nav.ships.retain(|ship| ship.id != id);
    assert_eq!(resume(&cache, &nav).ship_target, None);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn launch_clearance_resumes() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    assert!(nav.launch_needs_permission());
    nav.request_launch().unwrap();
    let loaded = resume(&cache, &nav);
    assert!(loaded.launch_cleared);
    assert!(!loaded.launch_needs_permission());
    assert_eq!(loaded.docked, nav.docked);
    assert_eq!(loaded.save_state().traffic, nav.save_state().traffic);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn landing_warning_is_not_repeated_after_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    nav.sequence = None;
    nav.target = None;
    nav.traffic.clear();
    nav.warned_at = Some(station);
    let mut loaded = resume(&cache, &nav);
    // from_save makes a traffic record for the docked station, so retain the
    // same empty, uncleared record in both paths before comparing the touch.
    nav.traffic = loaded.traffic.clone();
    assert_eq!(loaded.warned_at, nav.warned_at);
    assert_eq!(
        loaded.region_touch(station, 0),
        nav.region_touch(station, 0)
    );
    assert_eq!(loaded.warned_at, Some(station));
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn fractional_clearance_time_resumes_on_the_same_boundary() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    nav.sequence = None;
    let port = nav.starport(station);
    assert!(port.request(PLAYER));
    port.timeout = 1;
    nav.tick_traffic(0, 0.75);
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.carry, 0.75);
    assert_eq!(loaded.tick_traffic(0, 0.25), nav.tick_traffic(0, 0.25));
    assert_eq!(loaded.traffic, nav.traffic);
    assert_eq!(loaded.traffic[&station].timeout, 0);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn mission_targets_do_not_respawn_after_reload() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.spawned = Some(nav.current);
    assert!(!nav.place_contract_targets(&cache, &[123]).is_empty());
    nav.ships.retain(|ship| ship.contract != Some(123));
    let mut loaded = resume(&cache, &nav);
    let rng = loaded.random;
    let count = loaded.ships.len();
    assert!(loaded.place_contract_targets(&cache, &[123]).is_empty());
    assert_eq!(loaded.ships.len(), count);
    assert_eq!(loaded.random, rng);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn empty_npc_population_stays_empty_after_reload() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.spawned = Some(nav.current);
    nav.ships.clear();
    let mut loaded = resume(&cache, &nav);
    let rng = loaded.random;
    assert!(!loaded.spawn_traffic(&cache, [0; 3], [0.0, 0.0, 1.0]));
    assert!(loaded.ships.is_empty());
    assert_eq!(loaded.random, rng);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn old_version_two_saves_retain_the_canister_migration() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let mut effect = effects::canister(
        [100, 200, 300],
        [0.0; 3],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [0.0; 3],
        15,
        &mut nav.random,
    );
    effect.tag = Some(77);
    nav.effects.push(effect);
    let game = SaveGame::new(
        Commander::default(),
        crate::flight::Flight::default(),
        Some(nav.save_state()),
    );
    let mut json: serde_json::Value =
        serde_json::from_slice(&saves::encode_game(&game).unwrap()).unwrap();
    json["navigation"]
        .as_object_mut()
        .unwrap()
        .remove("runtime");
    let loaded = saves::decode_game(&serde_json::to_vec(&json).unwrap()).unwrap();
    let loaded = Navigator::from_save(&cache, loaded.navigation.as_ref().unwrap()).unwrap();
    assert_eq!(loaded.career, npc::Career::default());
    assert!(loaded.missiles.missiles.is_empty());
    assert_eq!(
        loaded.effects.iter().filter(|e| e.tag == Some(77)).count(),
        1
    );
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn malformed_runtime_records_are_rejected_before_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    launch(&mut nav);
    nav.deploy_chaff([0; 3], 10);
    nav.effects.push(effects::departure_cloud([0; 3], 1000));
    let save = nav.save_state();
    let invalid: [fn(&mut NavigatorRuntimeSave); 14] = [
        |r| r.traffic_carry = -0.1,
        |r| r.traffic_carry = 1.0,
        |r| r.traffic_carry = f64::NAN,
        |r| r.missiles.missiles[0].velocity[0] = f64::INFINITY,
        |r| r.missiles.missiles[0].spec.model = 466,
        |r| r.missiles.missiles[0].age = -1.0,
        |r| r.missiles.missiles.push(r.missiles.missiles[0]),
        |r| r.decoys[0].radius = -1,
        |r| r.decoys[0].id = 1,
        |r| r.next_decoy = 0,
        |r| r.decoys.push(r.decoys[0]),
        |r| r.effects.last_mut().unwrap().velocity[0] = f64::NAN,
        |r| r.effects.last_mut().unwrap().cargo = Some(u16::MAX),
        |r| r.effects.last_mut().unwrap().model = u32::MAX,
    ];
    for mutate in invalid {
        let mut bad = save.clone();
        mutate(bad.runtime.as_mut().unwrap());
        assert!(bad.validate().is_err());
        assert!(Navigator::from_save(&cache, &bad).is_err());
        assert!(saves::encode_game(&SaveGame::new(
            Commander::default(),
            crate::flight::Flight::default(),
            Some(bad)
        ))
        .is_err());
    }
    let mut bad = save.clone();
    bad.runtime.as_mut().unwrap().warned_at = Some(nav.bodies.len());
    assert!(Navigator::from_save(&cache, &bad).is_err());
    let mut bad = save;
    bad.runtime.as_mut().unwrap().ship_target = Some(123456);
    assert!(Navigator::from_save(&cache, &bad).is_err());
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn the_players_object_record_names_its_navigation_target_and_lock() {
    use ffe_core::object_record::{CONTROL_PLAYER, KIND_PLAYER};
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.sync_object_slots();
    assert_eq!(nav.player_object_record().control, CONTROL_PLAYER);
    assert_eq!(nav.player_object_record().agent.kind, KIND_PLAYER);
    // F1074's `+0xfe`/`+0x100`: the navigation computer's body target and the
    // locked ship. No target: both stay zero.
    assert_eq!(nav.player_object_record().agent.target, 0);
    assert_eq!(nav.player_object_record().agent.nav_target, 0);
    // `+0xff`: the landed state (cplayer's 0x20 case) on the new-game pad, and
    // no `+0x10e` distance while not flying.
    assert_eq!(nav.player_object_record().agent.ai_mode, 0x20);
    assert_eq!(nav.player_object_record().agent.target_distance, 0);
    let place = nav
        .bodies
        .iter()
        .position(|body| body.kind.is_place())
        .expect("a place to fly to");
    let slot = u8::try_from(nav.bodies[place].slot).unwrap();
    nav.target = Some(place);
    let record = nav.player_object_record();
    assert_eq!(record.agent.nav_target, slot);
    assert_eq!(record.agent.target, slot);
    // F1007/F610: the autopilot's basic-navigation mode and the exponential
    // distance to its target.
    assert_eq!(record.agent.ai_mode, 8);
    assert_eq!(
        record.agent.target_distance,
        u16::from(distance_byte(distance(
            nav.player_at,
            nav.bodies[place].position
        )))
    );
    // A ship lock wins for `+0xfe`; `+0x100` stays the route's destination.
    if let Some(id) = nav
        .ships
        .iter()
        .find(|ship| ship.alive())
        .map(|ship| ship.id)
    {
        nav.lock_ship(id);
        let ship_slot = nav.ship_slot(id).unwrap();
        let record = nav.player_object_record();
        assert_eq!(record.agent.target, ship_slot);
        assert_eq!(record.agent.nav_target, slot);
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn a_system_bodys_record_bridges_the_orbital_and_starport_halves() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.refresh(&cache);
    let place = nav
        .bodies
        .iter()
        .position(|body| body.kind.is_place())
        .expect("a place");
    let record = nav.body_object_record(place).expect("a body record");
    assert_eq!(usize::from(record.object), nav.bodies[place].slot);
    let overlay = record.orbit();
    // F880's orbital half (`+0xba` phase, `+0xc6` longitude): a place on a
    // planet has no radius dword of its own, but its phase/longitude are live.
    assert!(
        overlay.phase != 0 || overlay.longitude != 0,
        "overlay {overlay:?} slot {} kind {:?}",
        record.object,
        nav.bodies[place].kind
    );
    // F649's traffic half is written into the same record from the live
    // Starport, including the `+0x9e` door word outside the overlay.
    let port = *nav.starport(place);
    assert_eq!(overlay.bays, port.bays);
    assert_eq!(overlay.mode, port.mode);
    assert_eq!(record.timer, port.door);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn chaff_gets_an_object_slot_and_a_missile_points_at_it() {
    use ffe_core::object_record::KIND_CARGO;
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    launch(&mut nav);
    let decoy = nav.deploy_chaff([100_000_000, 50_000_000, 100_000_000], 100);
    nav.sync_object_slots();
    let record = nav.decoy_object_record(decoy).expect("a chaff record");
    let slot = record.object;
    assert_eq!(record.model, ffe_core::missile::DECOY_MODEL);
    assert_eq!(record.object, slot);
    assert_eq!(record.agent.kind, KIND_CARGO);
    assert_eq!(record.position, [100_000_000, 50_000_000, 100_000_000]);
    // F1057: the missile's `+0xfe` is the chaff object's slot, not zero.
    nav.missiles.missiles[0].target = Some(decoy);
    let missile = nav.missile_object_record(0).unwrap();
    assert_eq!(missile.agent.target, slot);
    assert_eq!(nav.decoy_object_records(), vec![(slot, record)]);
    // The chaff gone: F927 gets the slot back.
    nav.decoys.clear();
    nav.sync_object_slots();
    assert_eq!(nav.decoy_object_record(decoy), None);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn f233s_arrival_event_clears_the_wanted_and_alert_bits() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.legal.wanted = true;
    nav.legal.set_chase(ffe_core::police::CHASE);
    assert!(nav.legal.alert);
    let next = (nav.current_system() + 1) % nav.system_count();
    nav.jump(&cache, next).unwrap();
    assert!(!nav.legal.wanted);
    assert!(!nav.legal.alert);
    assert_eq!(nav.legal.chase, 0);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn an_attacker_that_hits_a_world_is_destroyed_as_state_1c() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.respawn_traffic(&cache, at, forward);
    let Some(planet) = nav.bodies.iter().position(|body| body.kind == Kind::Planet) else {
        panic!("no planet");
    };
    let centre = nav.bodies[planet].position;
    let id = nav
        .ships
        .iter()
        .find(|ship| ship.hostile && ship.alive())
        .map(|ship| ship.id)
        .expect("a hostile pirate");
    let ship = nav.ships.iter_mut().find(|ship| ship.id == id).unwrap();
    ship.position = centre;
    ship.state = ffe_core::npc::State::Attack;
    nav.tick_ships(0.02, 994, at, 17_600);
    let crashed = nav.ships.iter().find(|ship| ship.id == id).unwrap();
    assert_eq!(crashed.state, ffe_core::npc::State::Dead);
}
