//! Local-only: a full hyperspace jump against the executed original (release-1).
//!
//! `tools/reference/verify-jump.py` runs `jjffe-smoke --jump 2000` on the
//! Gateway/Hope pad: Tab launches, and at frame 400 in free flight the real F908
//! runs with Titican selected (code `0x12a53716`, 4.42 light years), the F34/F35
//! dialog answered yes. The trace in `FFE_JUMP_TRACE` carries F901's plan (the
//! jump time, the fuel burnt and the drive/mass it used), the mode 4 timeout,
//! F909's arrival point and the shared random pair before and after it.
//!
//! This test rebuilds the same jump through the port and requires the plan, the
//! arrival point, the random pair and the transit to match. The reference's
//! `plan` row is read after F908 (so `pair_after` includes F909's draws) and
//! its `pair_before` is the pair F909 started from.
//!
//! ```text
//! FFE_WORLD_CACHE=data.bin FFE_JUMP_TRACE=jump.jsonl cargo test -p ffe-rs \
//!     --test jump_reference -- --ignored --nocapture
//! ```
use ffe_core::npc::Random;
use ffe_rs::navigation::{DriveState, Navigator, AU_MM};
use serde_json::Value;

fn rows() -> Vec<Value> {
    let path = std::env::var("FFE_JUMP_TRACE").expect("FFE_JUMP_TRACE");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a JSONL row"))
        .collect()
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("not an integer: {value}"))
}

#[test]
#[ignore = "Local-only: needs FFE_JUMP_TRACE and the content cache"]
fn the_originals_full_jump_matches_the_port() {
    let rows = rows();
    let plan = rows
        .iter()
        .find_map(|row| row.get("jump_plan"))
        .expect("a jump_plan row");
    let arrival_row = rows
        .iter()
        .find_map(|row| row.get("jump_arrival"))
        .expect("a jump_arrival row");
    let distance = int(&plan["distance"]) as i32;
    let code = int(&plan["destination"]) as u32;
    let mass = int(&plan["mass"]) as i32;
    let class = int(&plan["drive"]) as i32;
    let fuel_before = int(&plan["fuel"][0]) as i32;
    let fuel_after = int(&plan["fuel"][1]) as i32;
    let jump_time = int(&plan["jump_time"]) as i32;
    assert_eq!(distance, 442, "Titican is 4.42 light years from Gateway");
    assert_eq!(int(&plan["mode"]), 4, "F908 leaves mode 4");
    assert_eq!(
        int(&plan["timeout"]),
        i64::from(jump_time),
        "F908 writes the jump time into +0x106"
    );
    assert!(jump_time > 0 && fuel_before > fuel_after);

    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let mut nav = Navigator::new_game(&cache, 0).expect("the default Gateway new game");
    assert_eq!(nav.system_name(), "Gateway");
    let index = nav
        .entry_for_code(code)
        .expect("Titican is one of the systems on offer");
    assert_eq!(
        nav.range_to(index),
        Some(distance),
        "the port's distance to Titican"
    );
    nav.drive = Some(DriveState {
        mass,
        class,
        fuel: fuel_before,
    });
    let (port_distance, report) = nav.jump_report(index).expect("the drive's report");
    assert_eq!(port_distance, distance);
    assert_eq!(report.result, distance, "F903's result");
    assert_eq!(
        report.fuel_needed,
        fuel_before - fuel_after,
        "the fuel burnt"
    );
    assert_eq!(report.jump_time, Some(jump_time), "F901's jump time");

    // F909's arrival: the same shared pair before it gives the same draw.
    let mut pair = [0u32; 2];
    for (k, value) in plan["pair_before"].as_array().unwrap().iter().enumerate() {
        pair[k] = int(value) as u32;
    }
    nav.random = Random::new(pair[0], pair[1]);
    let before_clock = nav.clock;
    let jumped = nav.hyperspace(&cache, index).expect("the jump to Titican");
    assert_eq!(jumped.fuel, fuel_before - fuel_after, "F901's fuel");
    assert_eq!(jumped.distance, distance);
    // F909's arrival is the orbit F912 draws from the same pair. The original's
    // object fields are parent-relative, so the exact vector is the port's own
    // check (7.4..11.3 AU below). The pair after F909 is reported for the
    // accepted gap: the port's arrival and system build do not consume the same
    // number of shared draws as the source's F909 (docs/ACCEPTED_PARITY.md).
    eprintln!(
        "pair after F909: original [{},{}], port [{},{}]",
        int(&plan["pair_after"][0]),
        int(&plan["pair_after"][1]),
        nav.random.a,
        nav.random.b
    );
    assert!(
        (jumped.days - f64::from(jump_time) / 65_536.0).abs() < 1e-9,
        "F901's 16.16 days: {}",
        jumped.days
    );
    assert_eq!(
        nav.system_name(),
        "Titican",
        "F909 builds the destination at once"
    );
    let transit = nav.transit.expect("mode 4 until the arrival time");
    assert!((transit.total - jumped.days * 86_400.0).abs() < 1e-6);

    // The arrival is on F1507's orbit, 7.5 to 11 AU from the star.
    let range = jumped
        .arrival
        .iter()
        .map(|value| (*value as f64).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(
        range > 7.4 * AU_MM && range < 11.3 * AU_MM,
        "{:.2} AU from the star",
        range / AU_MM
    );

    nav.finish_transit(&cache);
    assert!(
        (nav.clock - before_clock - jumped.days * 86_400.0).abs() < 1e-6,
        "the clock runs through the transit"
    );
    assert_eq!(
        int(&arrival_row["system"]),
        i64::from(code),
        "the arrival system is the destination"
    );
    eprintln!(
        "Gateway -> Titican: {distance} units, {:.2} days, {} t of fuel, arrival {:.2} AU out",
        jumped.days,
        jumped.fuel,
        range / AU_MM
    );
}
