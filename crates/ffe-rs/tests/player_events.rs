//! Source F1007/F934 events must not depend on which UI module is visible.
use ffe_rs::{
    combat::Combat,
    flight::{Flight, Mode},
    ships::ShipStats,
};
fn stats() -> ShipStats {
    let mut bytes = [0; 48];
    bytes[0..2].copy_from_slice(&1000i16.to_le_bytes());
    bytes[2..4].copy_from_slice(&(-500i16).to_le_bytes());
    bytes[6..8].copy_from_slice(&28u16.to_le_bytes());
    ShipStats::from_block(&bytes).unwrap()
}
fn flight() -> Flight {
    let mut state = Flight {
        mode: Mode::Flight,
        system_flight: true,
        combat: Combat::new_game(),
        loadout: ffe_core::equipment::Loadout::new_game(),
        hull_display: 30,
        hull_heat: 4096,
        shields: [0, 64],
        hull: [112, 112],
        ..Flight::default()
    };
    state.damage.service_timer = 100000;
    state
}
#[test]
fn every_player_mode_cools_and_counts_the_hull_line_down() {
    for mode in [0, 2, 4, 0x2a, 0x30, 0x34] {
        for page_mode in [Mode::Flight, Mode::Docked] {
            let mut state = flight();
            state.mode = page_mode;
            let rng = state.rng;
            state.player_events(2, mode, &stats(), 0);
            assert_eq!(state.hull_display, 28);
            assert_eq!(state.hull_heat, 3072);
            assert_eq!(state.shields[0], if mode == 2 { 0 } else { 2 });
            assert_eq!(state.damage.service_timer, 99998);
            assert_eq!(state.rng, rng);
        }
    }
}
#[test]
fn a_stopped_clock_delivers_no_player_events_or_rng_draws() {
    let mut state = flight();
    let before = state.clone();
    assert_eq!(
        state.player_events(0, 0, &stats(), 32767),
        (Vec::new(), false)
    );
    assert_eq!(state, before);
}
#[test]
fn service_timer_and_counter_wrapping_survive_resume() {
    let mut state = flight();
    state.damage.service_timer = u32::MAX;
    state.damage.service_counters = [500; 2];
    state.service_events(43200);
    assert_eq!(state.damage.service_counters, [499; 2]);
    assert_eq!(state.damage.service_timer, 64799);
    let mut resumed: Flight =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    // Flight borrows the authoritative pair from Navigator before event 6;
    // it is intentionally not a second serialized RNG owner.
    resumed.rng = state.rng;
    for seconds in [43200, 43200, 0, 108000] {
        assert_eq!(
            resumed.service_events(seconds),
            state.service_events(seconds)
        );
        assert_eq!(resumed.damage, state.damage);
        assert_eq!(resumed.rng, state.rng);
    }
    let mut state = flight();
    state.damage.service_timer = 0;
    state.damage.service_counters = [0; 2];
    state.service_events(1);
    assert_eq!(state.damage.service_counters, [u32::MAX; 2]);
}
#[test]
#[ignore = "Local-only: needs FFE_SERVICE_RECORDS from the executed original"]
fn servicing_matches_original_f934_for_96_dispatches() {
    let rows: Vec<serde_json::Value> =
        std::fs::read_to_string(std::env::var_os("FFE_SERVICE_RECORDS").unwrap())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|row| row.get("case").is_some())
            .collect();
    assert_eq!(rows.len(), 96);
    for row in rows {
        let number = |name: &str| row[name].as_u64().unwrap() as u32;
        let mut state = flight();
        state.damage.service_timer = number("left_in");
        state.damage.service_counters = [number("counter_in"); 2];
        state.loadout.general = number("general_in");
        state.rng = ffe_core::npc::Random::new(0x12345678 + number("case"), 0x456789ab);
        state.hold = vec![0; ffe_core::market::NAMES];
        state.hold[ffe_core::hyperspace::HYDROGEN] = 1;
        state.service_events(number("seconds"));
        assert_eq!(
            state.damage.service_timer,
            number("left"),
            "case {}",
            number("case")
        );
        assert_eq!(
            serde_json::to_value(state.damage.service_counters).unwrap(),
            row["counters"]
        );
        assert_eq!(state.loadout.general, number("general"));
        assert_eq!(state.damage.equipment, number("damaged"));
        assert_eq!(u32::from(state.combat.drive), number("drive"));
        assert_eq!(state.combat.energy, number("energy"));
        assert_eq!(
            serde_json::to_value([state.rng.a, state.rng.b]).unwrap(),
            row["rng"],
            "case {}",
            number("case")
        );
        assert_eq!(
            u32::from(state.hold[ffe_core::hyperspace::RUBBISH]),
            number("rubbish")
        );
    }
}
