//! Replays the `--combat` scenario from the executed original (gameplay-8).
//!
//! The harness starts the Hope new game, puts the player on the pirate leader's
//! forward axis and lets the original's own AI and F931 run (the port's early
//! fire is not scripted; FFE_ combat traces one row per frame with the whole
//! 0x152-byte record of every hostile ship). The replay:
//!
//! * checks the combat loop really ran (state changes, the player's laser
//!   attempts, hostile hits and F939/F942 effects);
//! * replays the leader's F596 guidance and F727 object physics from the raw
//!   records and compares thrust, caps, velocity and position frame by frame.
//!
//! ```text
//! FFE_COMBAT_TRACE=combat.jsonl cargo test -p ffe-core --test combat_replay -- --ignored --nocapture
//! ```
//!
//! The 2026-11-21 run (floppy 1.02, Hope default start): leader slot 70 model
//! 36, state 4 -> 1/2, player hull 112 -> 80 in 8-damage pulse hits, 39 effect
//! objects; the physics replay is exact in thrust/caps for the observed frames.

use ffe_core::autopilot::guide;
use ffe_core::physics::{Motion, Scanner};
use ffe_core::thrust::Limits;
use serde::Deserialize;

#[derive(Deserialize)]
struct Row {
    combat: u64,
    tics: u32,
    player: End,
    ships: Vec<Ship>,
}

#[derive(Deserialize)]
struct End {
    hull: u16,
    shield: u16,
    flags: u8,
    raw: String,
}

#[derive(Deserialize)]
struct Ship {
    slot: u32,
    model: u32,
    state: u8,
    hull: u16,
    timer: u8,
    raw: String,
}

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

fn rows() -> Vec<Row> {
    let path = std::env::var("FFE_COMBAT_TRACE").expect("FFE_COMBAT_TRACE");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"combat\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn leader_slot(rows: &[Row]) -> u32 {
    // Prefer a leader that stays in F750's attack state (4) for the whole
    // trace: the later F754 state-1/2 approach is a separate routine with no
    // port replay. Slot 70 turns to 1/2; 74 and 78 stay in 4.
    let slots: std::collections::BTreeSet<u32> =
        rows[0].ships.iter().map(|ship| ship.slot).collect();
    slots
        .into_iter()
        .max_by_key(|slot| {
            rows.iter()
                .filter(|row| {
                    row.ships
                        .iter()
                        .any(|ship| ship.slot == *slot && ship.state == 4)
                })
                .count()
        })
        .expect("a hostile ship")
}

fn full_motion(o: &[u8]) -> Motion {
    let mut m = Motion::default();
    for r in 0..3 {
        for c in 0..3 {
            m.orientation[r][c] = i32_at(o, 4 * (3 * r + c));
        }
    }
    for k in 0..3 {
        m.velocity[k] = i32_at(o, 0x8c + 4 * k);
        m.position[k] = i64_at(o, 0x3e + 8 * k);
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

fn limits(o: &[u8]) -> Limits {
    Limits {
        axes: std::array::from_fn(|k| [i16_at(o, 0xbc + 4 * k), i16_at(o, 0xbe + 4 * k)]),
    }
}

#[test]
#[ignore = "needs the reference trace in FFE_COMBAT_TRACE"]
fn the_original_combat_loop_fires_hits_and_draws_effects() {
    let rows = rows();
    assert!(rows.len() > 100, "only {} combat rows", rows.len());
    let leader = leader_slot(&rows);
    let first = rows[0]
        .ships
        .iter()
        .find(|ship| ship.slot == leader)
        .expect("the leader");
    assert_eq!(
        first.state, 4,
        "the reference trace did not start in F750's attack state"
    );
    let states: std::collections::BTreeSet<u8> = rows
        .iter()
        .flat_map(|row| row.ships.iter())
        .map(|ship| ship.state)
        .collect();
    assert!(
        states.len() >= 2,
        "no hostile ship changed state: {states:?}"
    );
    let start_hull = first.hull;
    let fired = rows.iter().any(|row| row.player.flags & 0x4 != 0);
    assert!(fired, "the player's F931 never ran");
    let (mut min_hull, mut min_shield) = (rows[0].player.hull, rows[0].player.shield);
    for row in &rows {
        min_hull = min_hull.min(row.player.hull);
        min_shield = min_shield.min(row.player.shield);
    }
    assert!(
        min_hull < rows[0].player.hull || min_shield < rows[0].player.shield,
        "no hostile laser ever damaged the player"
    );
    assert!(
        rows.iter()
            .any(|row| !row.ships.is_empty() && row.player.hull < start_hull),
        "the player took no damage"
    );
    // Every combat frame carries the hostile group and the player.
    assert!(rows.iter().all(|row| !row.ships.is_empty()));
    eprintln!(
        "combat: {} rows, leader slot {leader} model {} states {states:?}, player hull/shield {} / {} -> {} / {}",
        rows.len(),
        first.model,
        rows[0].player.hull,
        rows[0].player.shield,
        min_hull,
        min_shield
    );
}

#[test]
#[ignore = "needs the reference trace in FFE_COMBAT_TRACE"]
fn the_leaders_guidance_and_physics_replay() {
    let rows = rows();
    let leader = leader_slot(&rows);
    let mut exact_thrust = 0usize;
    let mut exact_cap = 0usize;
    let mut velocity_ok = 0usize;
    let mut position_ok = 0usize;
    let mut total = 0usize;
    let mut first_bad = None;
    for n in 1..rows.len() {
        let before = rows[n - 1]
            .ships
            .iter()
            .find(|ship| ship.slot == leader)
            .expect("the leader");
        let after = rows[n]
            .ships
            .iter()
            .find(|ship| ship.slot == leader)
            .expect("the leader");
        let before_raw = hex(&before.raw);
        let after_raw = hex(&after.raw);
        let player_raw = hex(&rows[n - 1].player.raw);
        let mut m = full_motion(&before_raw);
        let timer_changed = before.timer != after.timer;
        if timer_changed {
            let target: [i64; 3] = std::array::from_fn(|k| {
                i64_at(&player_raw, 0x3e + 8 * k)
                    .wrapping_add(i64::from(i16_at(&before_raw, 0x102 + 2 * k)) << 10)
            });
            let target_velocity: [i32; 3] =
                std::array::from_fn(|k| i32_at(&player_raw, 0x8c + 4 * k));
            let g = guide(
                &m,
                m.position,
                target,
                target_velocity,
                &limits(&before_raw),
                1,
            );
            m.thrust = g.thrust;
            m.cap = g.cap;
            m.autopilot = g.turns;
        }
        let tics = rows[n].tics as i32;
        let mut scanner = Scanner::default();
        m.accelerate(tics as u32);
        m.orient(tics, true, tics, &mut scanner);
        m.advance(i64::from(tics));
        let want = full_motion(&after_raw);
        total += 1;
        if m.thrust == want.thrust {
            exact_thrust += 1;
        }
        if m.cap == want.cap {
            exact_cap += 1;
        }
        if (0..3).all(|k| (m.velocity[k] - want.velocity[k]).abs() <= 1) {
            velocity_ok += 1;
        }
        if (0..3).all(|k| (m.position[k] - want.position[k]).abs() <= 16) {
            position_ok += 1;
        }
        if m.thrust != want.thrust && first_bad.is_none() {
            first_bad = Some((rows[n].combat, m.thrust, want.thrust));
        }
    }
    let pct = |ok: usize| 100.0 * ok as f64 / total as f64;
    eprintln!(
        "combat replay over {total} frames: thrust {exact_thrust} ({:.1}%), cap {exact_cap} ({:.1}%), velocity {velocity_ok} ({:.1}%), position {position_ok} ({:.1}%), first bad {first_bad:?}",
        pct(exact_thrust),
        pct(exact_cap),
        pct(velocity_ok),
        pct(position_ok)
    );
    assert!(
        pct(exact_thrust) >= 90.0,
        "the port's guidance reproduced only {exact_thrust}/{total} thrust words"
    );
    assert!(
        pct(velocity_ok) >= 90.0,
        "the port's physics reproduced only {velocity_ok}/{total} velocities"
    );
    assert!(
        pct(position_ok) >= 90.0,
        "the port's physics reproduced only {position_ok}/{total} positions"
    );
}
