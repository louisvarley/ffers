//! Replays a pirate group leader (AI state 4, F750) from the executed original through the
//! port's guidance and object physics (PAR-15). Local only (`#[ignore]`): it needs a trace from
//! the reference harness, which puts the player 20 km from the leader (the NPC is not touched, so
//! the original's own AI runs) and writes the leader's, its target's and the player's whole
//! 0x152-byte object records after every frame:
//!
//! ```text
//! FFE_PTP_RAW=1 FFE_PLAYER_TO_PIRATE=500,20000000 //!   .cache/jjffe-reference/jjffe-smoke.exe --flight hover 1000 --start-key=0x39 > ptp.jsonl
//! FFE_NPC_PTP_TRACE=ptp.jsonl cargo test -p ffe-core --test npc_pirate_replay -- --ignored --nocapture
//! ```
//!
//! What it shows (2026-11-19, floppy 1.02, Hope default start, leader slot 70, model 36):
//! - F750 flies the ship to its target object (`+0xfe`, the player) through F596 with modifier 1,
//!   F593 adding the ship's waypoint words (`+0x102`, `+0x104`, `+0x106`, each shifted left 10)
//!   to the relative position; `autopilot::guide` gives the original's thrust words there.
//! - The AI step runs once per game second (0xc22e tics, 47.67 frames of 1,043 tics): the
//!   `+0x14d` timer, the thrust words, the speed caps and the turn rates change on those frames
//!   only, and F727's physics applies the stored words every frame in between.
//! - Reapplied with that cadence, the port's `Motion` reproduces the record's position, velocity,
//!   orientation, thrust, caps and rates exactly. The residual is a y velocity accumulator
//!   (`+0xee`) one count lower in the original every frame, and a one-unit velocity (a few mm
//!   of position) difference on some frames: both unexplained and far below anything visible.

use ffe_core::autopilot::guide;
use ffe_core::physics::Motion;
use ffe_core::thrust::Limits;

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn i16_at(o: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([o[at], o[at + 1]])
}
fn i32_at(o: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(o[at..at + 4].try_into().unwrap())
}
fn i64_at(o: &[u8], at: usize) -> i64 {
    i64::from_le_bytes(o[at..at + 8].try_into().unwrap())
}

struct Frame {
    ship: Vec<u8>,
    player: Vec<u8>,
}

fn frames() -> Vec<Frame> {
    let path = std::env::var("FFE_NPC_PTP_TRACE").expect("FFE_NPC_PTP_TRACE");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| l.contains("\"raw\""))
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            Frame {
                ship: hex(v["ship"].as_str().unwrap()),
                player: hex(v["player"].as_str().unwrap()),
            }
        })
        .collect()
}

fn motion(o: &[u8]) -> Motion {
    let mut m = Motion::default();
    for (r, row) in m.orientation.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            *cell = i32_at(o, 4 * (3 * r + c));
        }
    }
    for k in 0..3 {
        m.velocity[k] = i32_at(o, 0x8c + 4 * k);
        m.position[k] = i64_at(o, 0x3e + 8 * k);
    }
    m
}

fn limits(o: &[u8]) -> Limits {
    // +0xbc.. : x max, x min, y max, y min, z max, z min
    Limits {
        axes: std::array::from_fn(|k| [i16_at(o, 0xbc + 4 * k), i16_at(o, 0xbe + 4 * k)]),
    }
}

fn thrust(o: &[u8]) -> [i16; 3] {
    [i16_at(o, 0xb6), i16_at(o, 0xb8), i16_at(o, 0xba)]
}

#[test]
#[ignore = "needs the reference trace in FFE_NPC_PTP_TRACE"]
fn leader_thrust_matches_the_original() {
    let frames = frames();
    assert!(frames.len() > 100);
    // (use the previous frame's player, modifier) variants: which state the AI read.
    let mut report = Vec::new();
    for (previous_player, modifier) in [(true, 0), (true, 1), (false, 0), (false, 1)] {
        let (mut equal, mut total, mut first_bad) = (0, 0, None);
        for n in 1..frames.len() {
            let before = &frames[n - 1].ship;
            let player = if previous_player {
                &frames[n - 1].player
            } else {
                &frames[n].player
            };
            let m = motion(before);
            let target: [i64; 3] = std::array::from_fn(|k| {
                i64_at(player, 0x3e + 8 * k)
                    .wrapping_add(i64::from(i16_at(before, 0x102 + 2 * k)) << 10)
            });
            let target_velocity: [i32; 3] = std::array::from_fn(|k| i32_at(player, 0x8c + 4 * k));
            let g = guide(
                &m,
                m.position,
                target,
                target_velocity,
                &limits(before),
                modifier,
            );
            let want = thrust(&frames[n].ship);
            total += 1;
            if g.thrust == want {
                equal += 1;
            } else {
                if first_bad.is_none() {
                    first_bad = Some((n, g.thrust, want));
                }
                if modifier == 0 && previous_player && std::env::var("FFE_VERBOSE").is_ok() {
                    eprintln!("frame {n}: port {:?} original {:?}", g.thrust, want);
                }
            }
        }
        report.push(format!(
            "previous_player={previous_player} modifier={modifier}: {equal}/{total} frames equal, first difference {first_bad:?}"
        ));
    }
    eprintln!("{}", report.join("\n"));
}

#[test]
#[ignore = "needs the reference trace in FFE_NPC_PTP_TRACE"]
fn which_frame_state_gives_each_thrust_change() {
    let frames = frames();
    for (previous_player, modifier) in [(true, 0), (true, 1)] {
        for want_at in [532usize, 579] {
            let want = thrust(&frames[want_at - 500].ship);
            let mut hits = Vec::new();
            for n in 1..frames.len() {
                let before = &frames[n - 1].ship;
                let player = if previous_player {
                    &frames[n - 1].player
                } else {
                    &frames[n].player
                };
                let m = motion(before);
                let target: [i64; 3] = std::array::from_fn(|k| {
                    i64_at(player, 0x3e + 8 * k)
                        .wrapping_add(i64::from(i16_at(before, 0x102 + 2 * k)) << 10)
                });
                let tv: [i32; 3] = std::array::from_fn(|k| i32_at(player, 0x8c + 4 * k));
                let g = guide(&m, m.position, target, tv, &limits(before), modifier);
                if g.thrust == want {
                    hits.push(n + 500);
                }
            }
            eprintln!(
                "previous_player={previous_player} modifier={modifier}: thrust {want:?} (first seen at frame {want_at}) is the port's guidance at frames {:?}",
                hits.iter().take(6).collect::<Vec<_>>()
            );
            if modifier == 1 {
                assert!(
                    !hits.is_empty(),
                    "no frame's state gives {want:?} with modifier 1"
                );
            }
        }
    }
}

/// Every field of the object record that the object physics (F727: F601's thrust, F585's turn,
/// F1512's move) reads or writes.
fn full_motion(o: &[u8]) -> Motion {
    let mut m = motion(o);
    for k in 0..3 {
        m.thrust[k] = i16_at(o, 0xb6 + 2 * k);
        m.fraction[k] = u16::from_le_bytes([o[0xec + 2 * k], o[0xed + 2 * k]]);
        m.rate[k] = i16_at(o, 0xb0 + 2 * k);
        m.autopilot[k] = i16_at(o, 0x110 + 2 * k);
    }
    m.cap = [i32_at(o, 0xf2), i32_at(o, 0xf6), i32_at(o, 0xfa)];
    m.drift = o[0x24] as i8;
    m.model = u32::from_le_bytes(o[0x82..0x86].try_into().unwrap());
    m.craft = o[0x86];
    m.parent = o[0x56];
    m.gun_mounts = o[0xd1] as i8;
    m.rotating = o[0x57];
    m.switch_delay = i32_at(o, 0x148);
    m.updated = o[0x58];
    m
}

fn differences(a: &Motion, b: &Motion) -> Vec<&'static str> {
    let mut out = Vec::new();
    if a.position != b.position {
        out.push("position");
    }
    if a.velocity != b.velocity {
        out.push("velocity");
    }
    if a.orientation != b.orientation {
        out.push("orientation");
    }
    if a.fraction != b.fraction {
        out.push("fraction");
    }
    if a.thrust != b.thrust {
        out.push("thrust");
    }
    if a.cap != b.cap {
        out.push("cap");
    }
    if a.rate != b.rate {
        out.push("rate");
    }
    if a.autopilot != b.autopilot {
        out.push("autopilot");
    }
    out
}

/// The object physics from the recorded frame 531 onward, with the guidance (thrust words, caps,
/// turn rates) recomputed only on the frames where the original's `+0x14d` timer changes.
#[test]
#[ignore = "needs the reference trace in FFE_NPC_PTP_TRACE"]
fn leader_physics_with_throttled_guidance_matches_the_original() {
    use ffe_core::physics::Scanner;
    let frames = frames();
    let start = 531 - 500;
    for (guidance_before_physics, orient_first) in
        [(false, false), (false, true), (true, false), (true, true)]
    {
        let mut m = full_motion(&frames[start].ship);
        let (mut exact, mut total) = (0usize, 0usize);
        let mut first: Option<(usize, Vec<&'static str>)> = None;
        for n in start + 1..frames.len() {
            let timer_changed = frames[n].ship[0x14d] != frames[n - 1].ship[0x14d];
            let player = &frames[n].player;
            let regenerate = |m: &mut Motion| {
                let before = &frames[n].ship;
                let target: [i64; 3] = std::array::from_fn(|k| {
                    i64_at(player, 0x3e + 8 * k)
                        .wrapping_add(i64::from(i16_at(before, 0x102 + 2 * k)) << 10)
                });
                let tv: [i32; 3] = std::array::from_fn(|k| i32_at(player, 0x8c + 4 * k));
                let g = guide(m, m.position, target, tv, &limits(before), 1);
                m.thrust = g.thrust;
                m.cap = g.cap;
                m.autopilot = g.turns;
            };
            if timer_changed && guidance_before_physics {
                regenerate(&mut m);
            }
            let tics = 1043u32;
            let mut scanner = Scanner::default();
            if orient_first {
                m.orient(tics as i32, true, tics as i32, &mut scanner);
                m.accelerate(tics);
            } else {
                m.accelerate(tics);
                m.orient(tics as i32, true, tics as i32, &mut scanner);
            }
            m.advance(i64::from(tics));
            if timer_changed && !guidance_before_physics {
                regenerate(&mut m);
            }
            let want = full_motion(&frames[n].ship);
            total += 1;
            let diff = differences(&m, &want);
            if !guidance_before_physics
                && std::env::var("FFE_VERBOSE").is_ok()
                && (n + 500 <= 536 || n + 500 == 579 || n + 500 == 580)
            {
                eprintln!(
                    "{}: diff {diff:?}
  port rate {:?} autopilot {:?} fraction {:?} thrust {:?} cap {:?}
  orig rate {:?} autopilot {:?} fraction {:?} thrust {:?} cap {:?}
  port rows {:?}
  orig rows {:?}",
                    n + 500,
                    m.rate,
                    m.autopilot,
                    m.fraction,
                    m.thrust,
                    m.cap,
                    want.rate,
                    want.autopilot,
                    want.fraction,
                    want.thrust,
                    want.cap,
                    m.orientation,
                    want.orientation
                );
            }
            if !guidance_before_physics && !orient_first && std::env::var("FFE_DELTAS").is_ok() {
                let d: Vec<i32> = (0..3)
                    .map(|k| i32::from(want.fraction[k]) - i32::from(m.fraction[k]))
                    .collect();
                eprintln!(
                    "delta {} fraction(orig-port) {:?} diff {:?} thrust {:?}",
                    n + 500,
                    d,
                    diff,
                    want.thrust
                );
            }
            if !guidance_before_physics {
                // Exact except the y accumulator's constant one-count offset and a one-unit velocity
                // difference now and then (a few mm in the position).
                assert!(
                    diff.iter()
                        .all(|field| matches!(*field, "fraction" | "velocity" | "position")),
                    "frame {}: the port's physics differs from the original in {diff:?}",
                    n + 500
                );
                for k in 0..3 {
                    assert!(
                        (m.velocity[k] - want.velocity[k]).abs() <= 1,
                        "frame {}: velocity {:?} vs {:?}",
                        n + 500,
                        m.velocity,
                        want.velocity
                    );
                }
                for k in 0..3 {
                    assert!(
                        (m.position[k] - want.position[k]).abs() <= 16,
                        "frame {}: position {k} {} vs {}",
                        n + 500,
                        m.position[k],
                        want.position[k]
                    );
                }
            }
            if diff.is_empty() {
                exact += 1;
            } else if first.is_none() {
                first = Some((n + 500, diff));
            }
            // Re-seed from the record so one difference does not hide later ones.
            m = want;
        }
        eprintln!(
            "guidance_before_physics={guidance_before_physics} orient_first={orient_first}: {exact}/{total} frames reproduce the record exactly; first difference {first:?}"
        );
    }
}
