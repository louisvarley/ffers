//! The executed original's first day boundary, draw for draw (PAR-20).
//!
//! `FFE_DAILY_TRACE` is the stdout of
//! `jjffe-smoke.exe --daily 2000 --start-key=0x02`: the pad at 10000x and every
//! F922/F1530 draw. The first day boundary is the frame with F633's 125 draws.
//! The test checks the ledger's shape, replays the pair through every recorded
//! step, then runs the port's `Navigator::tick_days` from the same pair and
//! compares the final pair, the board/job state and the `DayTick` counts.
//!
//! ```text
//! FFE_WORLD_CACHE=data.bin FFE_DAILY_TRACE=daily.jsonl cargo test -p ffe-rs \
//!   --test daily_reference -- --ignored --nocapture
//! ```
//!
//! The per-station markets keep their own streams (EXP-006), so the 99 market
//! steps are consumed on the shared pair but not read from it; that is the
//! accepted residual.

use ffe_core::npc::Random;
use serde_json::Value;

type Draw = (String, u32, u32, [u32; 2], [u32; 2]);

fn draws() -> (u64, Vec<Draw>) {
    let path = std::env::var("FFE_DAILY_TRACE").expect("FFE_DAILY_TRACE");
    let text = std::fs::read_to_string(path).unwrap();
    let mut boundary = 0;
    for line in text.lines() {
        if line.starts_with("{\"day_boundary\"") {
            boundary = serde_json::from_str::<Value>(line).unwrap()["day_boundary"]
                .as_u64()
                .unwrap();
        }
    }
    assert_ne!(boundary, 0, "the trace has no day_boundary row");
    let mut out = Vec::new();
    for line in text.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(draw) = row.get("draw") else {
            continue;
        };
        if draw["frame"].as_u64() != Some(boundary) {
            continue;
        }
        let pair = |value: &Value| -> [u32; 2] {
            [
                value[0].as_u64().unwrap() as u32,
                value[1].as_u64().unwrap() as u32,
            ]
        };
        out.push((
            draw["kind"].as_str().unwrap().to_owned(),
            draw["limit"].as_u64().unwrap() as u32,
            draw["result"].as_u64().unwrap() as u32,
            pair(&draw["before"]),
            pair(&draw["after"]),
        ));
    }
    (boundary, out)
}

#[test]
#[ignore = "needs the reference trace in FFE_DAILY_TRACE and the content cache"]
fn the_original_day_boundary_is_the_ports_day_boundary() {
    std::env::set_var("FFE_REFERENCE_GALAXY", "1");
    let (boundary, draws) = draws();
    assert_eq!(
        draws.len(),
        125,
        "frame {boundary} did not draw F633's 125 steps"
    );
    let limits: Vec<u32> = draws.iter().map(|(_, limit, _, _, _)| *limit).collect();
    assert_eq!(
        &limits[..12],
        &[25, 0, 25, 0, 8, 3, 12, 12, 1, 0, 14, 25],
        "F633's job/news draws changed"
    );
    assert_eq!(
        draws
            .iter()
            .filter(|(kind, _, _, _, _)| kind == "advance")
            .count(),
        105,
        "the 99 market steps plus the six F1530 news/ship-queue draws"
    );
    // The original's own ledger: every recorded pair is one F922 or F1530 step.
    let start = draws[0].3;
    let mut replay = Random::new(start[0], start[1]);
    for (kind, limit, _result, before, after) in &draws {
        assert_eq!(
            (replay.a, replay.b),
            (before[0], before[1]),
            "the pair chain broke before {kind}"
        );
        if kind == "advance" {
            replay.advance();
        } else {
            replay.bounded(*limit);
        }
        assert_eq!(
            (replay.a, replay.b),
            (after[0], after[1]),
            "the pair chain broke after {kind}"
        );
    }
    let end = draws.last().unwrap().4;
    eprintln!("day {boundary}: 125 draws, 105 F1530, pair {start:?} -> {end:?}");

    // The port's side: same start pair, same clock boundary, one new day.
    let bytes = std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap();
    let cache = ffe_content::read(&bytes).unwrap();
    let mut nav = ffe_rs::navigation::Navigator::new_game(&cache, 1).unwrap();
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.spawn_traffic(&cache, at, forward);
    nav.random = Random::new(start[0], start[1]);
    let before_boards: Vec<_> = nav
        .pair_boards()
        .map(|(key, board)| (*key, format!("{board:?}")))
        .collect();
    let before_jobs = format!("{:?}", nav.bbs_jobs);
    nav.clock = nav.start_clock + 43_300.0;
    let ticks = nav.tick_days(&cache);
    assert_eq!(
        (nav.random.a, nav.random.b),
        (end[0], end[1]),
        "the port's day ended on another pair"
    );
    assert!(
        !before_boards.is_empty(),
        "no board was generated for the day"
    );
    let after_boards: Vec<_> = nav
        .pair_boards()
        .map(|(key, board)| (*key, format!("{board:?}")))
        .collect();
    let tick = ticks.last().expect("a day tick");
    assert_eq!(tick.day, 1);
    // F633's visible half: a replaced advertisement, a changed ship queue or a
    // changed dated job list. (Visited station markets are EXP-006's stream.)
    assert!(
        after_boards != before_boards || format!("{:?}", nav.bbs_jobs) != before_jobs,
        "F633 replaced nothing on the board, ship queue or job list"
    );
    eprintln!(
        "port day {}: markets {} boards {} hulls {}, board/jobs changed",
        tick.day, tick.markets, tick.boards, tick.hulls
    );
}
