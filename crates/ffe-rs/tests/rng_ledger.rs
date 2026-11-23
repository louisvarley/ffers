//! Local-only draw ledger for the shared random pair (PAR-17).
//!
//! `FFE_RESERVE_DUMP=1 jjffe-smoke --flight idle 2000` with `FFE_ACCEL_AT=200:4`
//! parks the new game on its pad and runs the clock at 10000x, so the original
//! crosses a game-day boundary with nothing else touching the pair. Its dump's
//! `rng` field is `DATA_008818`/`DATA_008820` after every frame.
//!
//! The pair is moved only by F922 (`Random::bounded`) and F1530
//! (`Random::advance`), so a change between two frames is some run of those two
//! steps. This reads each change and finds the shortest run that explains it
//! (a draws of one kind then b of the other, in either order), which is the
//! original's draw count for that frame. The port, given the same events, has to
//! make the same draws.
//!
//! Run with `FFE_LEDGER_TRACE=<stderr of that run>`.

use ffe_core::npc::Random;
use serde::Deserialize;

#[derive(Deserialize)]
struct Row {
    reserve: u64,
    day: u32,
    rng: [u32; 2],
}

/// The shortest (F922 draws, F1530 draws, F922 first) taking `from` to `to`.
fn explain(from: [u32; 2], to: [u32; 2]) -> Option<(u32, u32, bool)> {
    let mut best: Option<(u32, u32, bool)> = None;
    for first_922 in [true, false] {
        let mut state = Random::new(from[0], from[1]);
        // F633 draws a few F922 and then F1530 runs of 66 per station.
        let (first_limit, second_limit) = if first_922 {
            (200, 20_000)
        } else {
            (20_000, 200)
        };
        for n in 0..first_limit {
            let mut tail = state;
            for m in 0..=second_limit {
                if [tail.a, tail.b] == to && (n + m > 0) {
                    let cost = n + m;
                    if best.is_none_or(|(a, b, _)| cost < a + b) {
                        best = Some(if first_922 {
                            (n, m, true)
                        } else {
                            (m, n, false)
                        });
                    }
                }
                if first_922 {
                    tail.advance();
                } else {
                    tail.bounded(1);
                }
            }
            if first_922 {
                state.bounded(1);
            } else {
                state.advance();
            }
        }
    }
    best
}

#[test]
#[ignore = "Local-only: needs FFE_LEDGER_TRACE"]
fn the_original_draws_which_steps_between_frames() {
    let rows: Vec<Row> = std::fs::read_to_string(std::env::var_os("FFE_LEDGER_TRACE").unwrap())
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"reserve\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for pair in rows.windows(2) {
        if pair[0].rng != pair[1].rng {
            eprintln!(
                "frame {} (day {}): {:?} -> {:?}: {:?}",
                pair[1].reserve,
                pair[1].day,
                pair[0].rng,
                pair[1].rng,
                explain(pair[0].rng, pair[1].rng)
            );
        }
    }
}

/// One row of `FFE_RNG_TRACE`: `frame seq before_a before_b limit result caller`.
/// F922 rows carry their limit; F1530 rows (`--flight` runs) carry limit 0.
fn draws(path: &std::ffi::OsStr) -> Vec<(u64, u32)> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| {
            let mut words = line.split_whitespace();
            let frame = words.next().unwrap().parse().unwrap();
            let limit = words.nth(3).unwrap().parse().unwrap();
            (frame, limit)
        })
        .collect()
}

/// `FFE_RNG_TRACE=<file> FFE_RESERVE_DUMP=1 FFE_ACCEL_AT=200:4 jjffe-smoke
/// --flight idle 2000`, then `FFE_RNG_LOG=<file>`: the original's draw ledger for
/// the boot, the quiet pad and the day boundary. The port's day tick makes none
/// of the boundary's 125 draws on the shared pair (its markets and boards keep
/// per-station streams, EXP-006), which is the open part of PAR-17.
#[test]
#[ignore = "Local-only: needs FFE_RNG_LOG"]
fn the_original_draws_at_boot_and_at_the_day_boundary() {
    let rows = draws(&std::env::var_os("FFE_RNG_LOG").unwrap());
    let in_frame = |frame: u64| -> Vec<u32> {
        rows.iter()
            .filter(|(f, _)| *f == frame)
            .map(|(_, limit)| *limit)
            .collect()
    };
    // The boot's F688 and what follows it: nothing else until the day changes.
    assert_eq!(in_frame(161).len(), 291);
    assert_eq!(in_frame(162), [0, 0]);
    let frames: std::collections::BTreeSet<u64> = rows.iter().map(|(f, _)| *f).collect();
    assert_eq!(frames.into_iter().collect::<Vec<_>>(), [161, 162, 406]);
    // F633 on the new day: F922(25) with F623's draws, then the places' market
    // drift on F1530 (three words a commodity) and the bulletin boards.
    let day = in_frame(406);
    eprintln!("day boundary: {} draws: {day:?}", day.len());
    assert_eq!(day.len(), 125);
    assert_eq!(day[..12], [25, 0, 25, 0, 8, 3, 12, 12, 1, 0, 14, 25]);
    assert_eq!(day.iter().filter(|limit| **limit == 0).count(), 105);
}

/// The port's side of the same ledger: the new game from the start the
/// reference harness runs, its pair after boot plus the two F1530 steps the
/// original makes on frame 162, and then the first new day through
/// `Navigator::tick_days`. The original's pair at frame 405 was
/// `[1273017021, 228009932]` and at 406 `[1772616542, 3616182582]`.
///
/// `FFE_WORLD_CACHE=<data.bin> cargo test -p ffe-rs --test rng_ledger --
/// --ignored --nocapture the_port_makes`
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_port_makes_the_originals_day_boundary_draws() {
    std::env::set_var("FFE_REFERENCE_GALAXY", "1");
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    // The reference harness's start is Ross 154 (start type 1). Its boot's own
    // draws are not the port's (the harness's flight run leaves the pair at
    // `a3ce90ba 5ad9a9b2`), so the pair is put where the original's was before
    // the day: what has to match is what the day then draws.
    let mut nav = ffe_rs::navigation::Navigator::new_game(&cache, 1).unwrap();
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.spawn_traffic(&cache, at, forward);
    // Ross 154's one board has the ten advertisements the original's day
    // drew its victim from (F922(10)).
    assert!(nav
        .pair_boards()
        .all(|(_, board)| board.entries.len() == 10));
    nav.random = ffe_core::npc::Random::new(1_273_017_021, 228_009_932);
    nav.clock = nav.start_clock + 43_300.0;
    nav.tick_days(&cache);
    // F633 on the new day, the original's 125 draws: three news chances (the
    // third a no-op round), the place's 99 market steps, the board entry
    // replaced, a ship leaving and the ship queue. The pair ends where the
    // original's did.
    assert_eq!((nav.random.a, nav.random.b), (1_772_616_542, 3_616_182_582));
    // With the original's log, also that it is the same 125 steps: the pair
    // after each of them, in order, is the original's.
    if let Some(log) = std::env::var_os("FFE_RNG_LOG") {
        let limits: Vec<u32> = draws(&log)
            .into_iter()
            .filter(|(frame, _)| *frame == 406)
            .map(|(_, limit)| limit)
            .collect();
        assert_eq!(limits.len(), 125);
        let mut replay = ffe_core::npc::Random::new(1_273_017_021, 228_009_932);
        for limit in limits {
            if limit == 0 {
                replay.advance();
            } else {
                replay.bounded(limit);
            }
        }
        assert_eq!((replay.a, replay.b), (nav.random.a, nav.random.b));
    }
}
