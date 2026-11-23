//! The second ten bounded native-resume tasks (persist-16 through persist-25).
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

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn mid_jump_transit_resumes_to_the_same_arrival_time() {
    let cache = cache();
    let mut nav = Navigator::new(&cache).unwrap();
    let centauri = (1..15).find(|i| nav.range_to(*i) == Some(435)).unwrap();
    nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 3,
    });
    nav.hyperspace(&cache, centauri).unwrap();
    assert!(!nav.transit_step(&cache, 0.01));
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.transit, nav.transit);
    assert!(loaded.transit.unwrap().animation() > 0);
    for _ in 0..1000 {
        let done = nav.transit_step(&cache, 0.1);
        assert_eq!(loaded.transit_step(&cache, 0.1), done);
        assert_eq!(loaded.transit, nav.transit);
        assert_eq!(loaded.clock, nav.clock);
        if done {
            break;
        }
    }
    assert_eq!(loaded.transit.unwrap().remaining, 0.0);
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn jump_pursuers_resume_and_spawn_with_the_same_equipment() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let player = [1000, 2000, 3000];
    nav.player_at = player;
    let ship = nav.ships.iter_mut().find(|ship| ship.alive()).unwrap();
    ship.hostile = true;
    ship.position = [2000, 3000, 4000];
    nav.persisted = npc::persist(&nav.ships, player);
    assert!(!nav.persisted.is_empty());
    nav.ships.clear();
    nav.spawned = None;
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.persisted, nav.persisted);
    assert_eq!(loaded.player_at, player);
    assert_eq!(
        loaded.spawn_traffic(&cache, player, [0.0, 0.0, 1.0]),
        nav.spawn_traffic(&cache, player, [0.0, 0.0, 1.0])
    );
    assert_eq!(loaded.ships, nav.ships);
    assert!(loaded.persisted.is_empty());
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn purchased_shipyard_stock_and_its_daily_rng_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    let offers = nav.station_offers(&cache, station).unwrap();
    assert!(!offers.hulls.is_empty());
    offers.buy(0, NEW_GAME_SHIP, 1_000_000_000).unwrap();
    let mut loaded = resume(&cache, &nav);
    assert_eq!(
        loaded.station_offers(&cache, station).unwrap(),
        nav.station_offers(&cache, station).unwrap()
    );
    for _ in 0..3 {
        assert_eq!(
            loaded
                .station_offers(&cache, station)
                .unwrap()
                .daily(&cache, 1),
            nav.station_offers(&cache, station)
                .unwrap()
                .daily(&cache, 1)
        );
        assert_eq!(loaded.offers, nav.offers);
    }
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn removed_bulletin_posts_do_not_reappear_on_load() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    let offers = nav.station_offers(&cache, station).unwrap();
    assert!(!offers.posts.is_empty());
    offers.posts.remove(0);
    let mut loaded = resume(&cache, &nav);
    assert_eq!(
        loaded.station_offers(&cache, station).unwrap().posts,
        nav.station_offers(&cache, station).unwrap().posts
    );
    nav.advance(&cache, 86_400.0);
    loaded.advance(&cache, 86_400.0);
    assert_eq!(loaded.offers, nav.offers);
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn unvisited_boards_resume_before_the_first_station_visit() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    assert!(!nav.boards.is_empty());
    assert!(nav.offers.is_empty());
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.boards, nav.boards);
    assert_eq!(loaded.boards_entered, nav.boards_entered);
    assert_eq!(
        loaded.station_offers(&cache, station).unwrap(),
        nav.station_offers(&cache, station).unwrap()
    );
    assert_eq!(loaded.boards, nav.boards);
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn bbs_jobs_and_destination_memory_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.bbs_remembered = ffe_core::bbs::Remembered([11, 22, 33, 44]);
    nav.bbs_jobs.push(ffe_core::bbs::Job {
        code: 0x98ff,
        system: nav.entries[nav.current].params.code,
        date: nav.day(),
        money: 100,
        text: 10,
        word: 2,
    });
    let loaded = resume(&cache, &nav);
    assert_eq!(loaded.bbs_jobs, nav.bbs_jobs);
    assert_eq!(loaded.bbs_remembered, nav.bbs_remembered);
    assert_eq!(loaded.random, nav.random);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn calendar_and_undelivered_day_events_resume_exactly_once() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.advance(&cache, 86_400.0);
    // The next day's clock is present, but its daily handler has not run yet.
    nav.clock += 86_400.0;
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.last_day_tick(), nav.last_day_tick());
    assert_eq!(loaded.take_crossed_days(), nav.take_crossed_days());
    assert_eq!(loaded.tick_days(&cache), nav.tick_days(&cache));
    assert_eq!(loaded.take_crossed_days(), nav.take_crossed_days());
    assert!(loaded.tick_days(&cache).is_empty());
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn pending_and_queued_mission_deadlines_resume_once() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.schedule_deadline(10, 1);
    nav.schedule_deadline(20, 3);
    nav.advance(&cache, 2.0 * 86_400.0);
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.deadlines(), nav.deadlines());
    assert_eq!(loaded.take_expired(), nav.take_expired());
    assert!(loaded.take_expired().is_empty());
    nav.advance(&cache, 86_400.0);
    loaded.advance(&cache, 86_400.0);
    assert_eq!(loaded.take_expired(), nav.take_expired());
    assert!(loaded.deadlines().is_empty());
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn autopilot_history_and_clearance_retry_time_resume() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    nav.docked = None;
    nav.landed = None;
    nav.takeoff = None;
    nav.sequence = None;
    nav.target = Some(station);
    nav.starport(station).release(PLAYER);
    assert!(nav.starport(station).request(42));
    let offset = nav.long_leg_offset(&nav.bodies[station]);
    let aim = std::array::from_fn(|k| nav.bodies[station].position[k] + offset[k].round() as i64);
    let start = crate::flight::Flight {
        mode: crate::flight::Mode::Flight,
        ..crate::flight::Flight::default()
    };
    let mut flight = OriginalFlight::from_flight(&start, None);
    flight.teleport(aim);
    nav.memory.angle = Some(0.13);
    nav.clearance_at = Some(nav.clock - 0.7);
    nav.skips = 3;
    nav.drift = [11, 22, 33];
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.memory, nav.memory);
    assert_eq!(loaded.clearance_at, nav.clearance_at);
    assert_eq!(loaded.skips, nav.skips);
    assert_eq!(loaded.follow([0; 3]), nav.follow([0; 3]));
    assert_eq!(loaded.drift, [0; 3]);
    let previous = nav.clearance_at;
    assert_eq!(
        loaded
            .prototype_command(&cache, &flight)
            .map(|(command, _)| command),
        nav.prototype_command(&cache, &flight)
            .map(|(command, _)| command)
    );
    assert_eq!(
        loaded.clearance_at, previous,
        "the refusal must not be retried early"
    );
    nav.clock += 1.4;
    loaded.clock += 1.4;
    assert_eq!(
        loaded
            .prototype_command(&cache, &flight)
            .map(|(command, _)| command),
        nav.prototype_command(&cache, &flight)
            .map(|(command, _)| command)
    );
    assert_eq!(loaded.clearance_at, Some(loaded.clock));
    assert_eq!(loaded.notice, nav.notice);
    assert_eq!(loaded.traffic, nav.traffic);
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn navigation_frame_resumes_before_the_next_orbital_refresh() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let index = nav.docked.unwrap();
    nav.frame = Some(index);
    let at = nav.bodies[index].position;
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.frame(), Some(index));
    nav.advance(&cache, 2.0);
    loaded.advance(&cache, 2.0);
    assert_eq!(loaded.drift, nav.drift);
    assert_eq!(loaded.follow(at), nav.follow(at));
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn earlier_runtime_saves_seed_the_calendar_without_replaying_old_days() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.advance(&cache, 3.0 * 86_400.0);
    nav.career.award_medal(3);
    let game = SaveGame::new(
        Commander::default(),
        crate::flight::Flight::default(),
        Some(nav.save_state()),
    );
    let mut json: serde_json::Value =
        serde_json::from_slice(&saves::encode_game(&game).unwrap()).unwrap();
    json["navigation"]["runtime"]
        .as_object_mut()
        .unwrap()
        .remove("continuation");
    let game = saves::decode_game(&serde_json::to_vec(&json).unwrap()).unwrap();
    let mut loaded = Navigator::from_save(&cache, game.navigation.as_ref().unwrap()).unwrap();
    assert_eq!(loaded.career, nav.career);
    assert_eq!(loaded.last_day_tick(), loaded.day());
    assert!(loaded.tick_days(&cache).is_empty());
    assert!(loaded.take_crossed_days().is_empty());
    assert!(loaded.transit.is_none());
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn malformed_continuation_records_are_rejected() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let station = nav.docked.unwrap();
    nav.station_offers(&cache, station).unwrap();
    nav.transit = Some(Transit {
        remaining: 5.0,
        total: 10.0,
    });
    nav.schedule_deadline(1, 3);
    nav.crossed = vec![1, 2];
    nav.calendar = Calendar::at(2);
    // Keep a second, unvisited board for the malformed-entry check.
    if nav.boards.is_empty() {
        let code = nav.entries[(nav.current + 1) % nav.entries.len()]
            .params
            .code;
        nav.boards
            .insert((code, 0), ffe_core::bbs::Board::default());
    }
    let save = nav.save_state();
    let invalid: [fn(&mut NavigatorContinuationSave); 14] = [
        |s| s.transit.as_mut().unwrap().remaining = f64::NAN,
        |s| s.transit.as_mut().unwrap().remaining = 11.0,
        |s| s.offers.push(s.offers[0].clone()),
        |s| s.offers[0].2.day = u32::MAX,
        |s| s.offers[0].2.hulls.push((u32::MAX, 0)),
        |s| {
            s.boards[0].2.entries.resize(
                19,
                ffe_core::bbs::Raw {
                    code: 0,
                    args: [0; 5],
                },
            )
        },
        |s| {
            s.bbs_jobs.resize(
                26,
                ffe_core::bbs::Job {
                    code: 0,
                    system: 0,
                    date: 0,
                    money: 0,
                    text: 0,
                    word: 0,
                },
            )
        },
        |s| s.memory.angle = Some(f64::NAN),
        |s| s.memory.angle = Some(-1.0),
        |s| s.clearance_at = Some(f64::INFINITY),
        |s| s.deadlines.push(s.deadlines[0]),
        |s| s.crossed = vec![2, 1],
        |s| s.crossed = vec![3],
        |s| s.calendar_day = u32::MAX,
    ];
    for mutate in invalid {
        let mut bad = save.clone();
        mutate(bad.runtime.as_mut().unwrap().continuation.as_mut().unwrap());
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
    bad.runtime
        .as_mut()
        .unwrap()
        .continuation
        .as_mut()
        .unwrap()
        .frame = Some(nav.bodies.len());
    assert!(Navigator::from_save(&cache, &bad).is_err());
    let mut bad = save;
    let board = &mut bad
        .runtime
        .as_mut()
        .unwrap()
        .continuation
        .as_mut()
        .unwrap()
        .boards[0];
    board.0 = nav.entries[nav.current].params.code;
    board.1 = nav.bodies.len();
    assert!(Navigator::from_save(&cache, &bad).is_err());
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn the_daily_draws_resume_to_the_same_pair_boards_and_jobs() {
    // PAR-17/18: F633's draws depend on every board's entry and ship counts, so
    // a save taken before the first new day has to carry them: the resumed
    // navigator and the uninterrupted one then draw the same days.
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let at = nav.station().map(|(_, at)| at).unwrap();
    let forward = nav.bodies[nav.docked.unwrap()].orientation[2];
    nav.spawn_traffic(&cache, at, forward);
    assert!(!nav.pair_boards.is_empty());
    let mut loaded = resume(&cache, &nav);
    assert_eq!(loaded.pair_boards, nav.pair_boards);
    for day in 1..=5 {
        // Midnight is twelve hours into the new game, then a day apart.
        let seconds = 43_200.0 + 86_400.0 * f64::from(day - 1) + 60.0;
        for navigator in [&mut nav, &mut loaded] {
            navigator.clock = navigator.start_clock + seconds;
            navigator.tick_days(&cache);
        }
        assert_eq!(loaded.random, nav.random, "day {day}");
        assert_eq!(loaded.pair_boards, nav.pair_boards, "day {day}");
        assert_eq!(loaded.bbs_jobs, nav.bbs_jobs, "day {day}");
        assert_eq!(loaded.bbs_remembered, nav.bbs_remembered, "day {day}");
    }
    // The days really drew: the pair is not where F688 left it.
    assert_ne!(nav.random, Navigator::new_game(&cache, 1).unwrap().random);
}
