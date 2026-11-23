//! Replays F238 and F240 (`ffe_core::police`) against the executed original (PAR-15). Local only
//! (`#[ignore]`): it needs a trace from the reference harness in `FFE_POLICE_TRACE_FILE`:
//!
//! ```text
//! FFE_POLICE_TRACE=1 FFE_FIRE_FROM=200 FFE_FIRE_TO=200 FFE_RNG_TRACE=rng.txt \
//!   .cache/jjffe-reference/jjffe-smoke.exe --flight idle 5000 --start-key=0x04 > police.jsonl
//! FFE_POLICE_TRACE_FILE=police.jsonl FFE_POLICE_RNG_FILE=rng.txt \
//!   cargo test -p ffe-core --test police_replay -- --ignored --nocapture
//! ```
//!
//! `--start-key=0x04` is the orbital-station start (an offended station without the open-air bit):
//! the first game day after the offence launches two police in state 0x22 and sets the record's
//! mode to 0xff; `--start-key=0x02` is Hope, an open-air starport, which starts a chase and then
//! rolls once a day. The trace's per-frame record (`mode`, `left`, `clock`, `chase`) is compared
//! with the port's after each frame, with the things outside F238/F240 injected where the trace
//! shows them: the door finishing (mode 0xff to 0xfe) and a ship's docking request taking the
//! record back to mode 0 (F655), and a removed police ship clearing the chase (F783). The shared random pair is set from the trace before each roll.

use ffe_core::npc::Random;
use ffe_core::police;
use ffe_core::traffic::Starport;

const PLAYER: u8 = 1;
const FRAME_TICS: u32 = 1043;
const DAY: u32 = 0xc22e;

struct Row {
    frame: u64,
    chase: i8,
    wanted: bool,
    mode: u8,
    current: u8,
    left: i16,
    clock: u16,
    flags: u8,
    face: u16,
}

fn rows() -> Vec<Row> {
    let path = std::env::var("FFE_POLICE_TRACE_FILE").expect("FFE_POLICE_TRACE_FILE");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"police\""))
        .filter_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            v.get("offended")?;
            Some(Row {
                frame: v["police"].as_u64()?,
                chase: v["chase"].as_u64()? as u8 as i8,
                wanted: v["wanted"].as_u64()? & 1 != 0,
                mode: v["mode"].as_u64()? as u8,
                current: v["current"].as_u64()? as u8,
                left: v["left"].as_i64()? as i16,
                clock: v["clock"].as_u64()? as u16,
                flags: v["flags"].as_u64()? as u8,
                face: v["face"].as_u64()? as u16,
            })
        })
        .collect()
}

/// The shared pair before each F238 roll (`caller` is F238's F1530 call site), by frame.
fn draws() -> std::collections::HashMap<u64, (u32, u32)> {
    let Ok(path) = std::env::var("FFE_POLICE_RNG_FILE") else {
        return Default::default();
    };
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let c: Vec<&str> = line.split_whitespace().collect();
            // frame seq before_a before_b limit result caller; limit 0 is F1530.
            if c.len() == 7 && c[4] == "0" && c[6].ends_with("AB1C9") {
                Some((
                    c[0].parse().ok()?,
                    (
                        u32::from_str_radix(c[2], 16).ok()?,
                        u32::from_str_radix(c[3], 16).ok()?,
                    ),
                ))
            } else {
                None
            }
        })
        .collect()
}

/// Mode, police left, clock and chase flag.
type Record = (u8, i16, u16, i8);

struct Outcome {
    equal: usize,
    total: usize,
    launches: usize,
    first_bad: Option<(u64, Record, Record)>,
}

/// Replay with F152's day boundary at `phase` game tics into the first frame's clock.
fn replay(rows: &[Row], rolls: &std::collections::HashMap<u64, (u32, u32)>, phase: u64) -> Outcome {
    let first = &rows[0];
    let law = first.flags & 0x40 != 0;
    let mut port = Starport::new(first.face);
    port.mode = first.mode;
    port.current = first.current;
    port.response = Some(first.left);
    port.heat = first.clock;
    let (mut chase, mut wanted) = (first.chase, first.wanted);
    let mut out = Outcome {
        equal: 0,
        total: 0,
        launches: 0,
        first_bad: None,
    };
    for pair in rows.windows(2) {
        let (before, now) = (&pair[0], &pair[1]);
        // Things outside F238/F240: the door finishing and a docking request.
        port.current = before.current;
        if before.mode == 0xff && now.mode == 0xfe {
            port.mode = 0xfe;
        }
        if before.mode == 1 && now.mode == 0 {
            port.mode = 0;
        }
        police::frame(&mut port, &mut chase, FRAME_TICS);
        let day = |f: u64| (f * u64::from(FRAME_TICS) + phase) / u64::from(DAY);
        if day(now.frame) > day(before.frame) {
            let mut rng = rolls
                .get(&now.frame)
                .map_or(Random::new(1, 2), |(a, b)| Random::new(*a, *b));
            out.launches +=
                police::day(&mut port, law, &mut chase, &mut wanted, PLAYER, &mut rng).len();
        }
        // F783: a police ship was freed this frame, which clears the chase and the wanted bit.
        if now.chase == 0 && chase != 0 {
            chase = 0;
        }
        out.total += 1;
        let same = port.mode == now.mode
            && port.police_left() == now.left
            && port.heat == now.clock
            && chase == now.chase;
        if same {
            out.equal += 1;
        } else if out.first_bad.is_none() {
            out.first_bad = Some((
                now.frame,
                (port.mode, port.police_left(), port.heat, chase),
                (now.mode, now.left, now.clock, now.chase),
            ));
        }
        // Re-seed from the record so one difference does not hide the next.
        port.mode = now.mode;
        port.current = now.current;
        port.response = Some(now.left);
        port.heat = now.clock;
        chase = now.chase;
        wanted = now.wanted;
    }
    out
}

#[test]
#[ignore = "needs the reference trace in FFE_POLICE_TRACE_FILE"]
fn f238_and_f240_follow_the_executed_original() {
    let rows = rows();
    assert!(rows.len() > 500, "{} rows", rows.len());
    let rolls = draws();
    // The day boundary falls somewhere inside a frame: try every phase in steps of 7 tics and
    // keep the best (the original's marker advances by 0xc22e a day from its own start).
    let best = (0..u64::from(DAY))
        .step_by(7)
        .map(|phase| (phase, replay(&rows, &rolls, phase)))
        .max_by_key(|(_, outcome)| outcome.equal)
        .unwrap();
    let (phase, outcome) = best;
    eprintln!(
        "phase {phase}: {}/{} frames equal, {} launches; first difference {:?}",
        outcome.equal, outcome.total, outcome.launches, outcome.first_bad
    );
    assert!(
        outcome.equal == outcome.total,
        "the port differs from the original on {} of {} frames: {:?}",
        outcome.total - outcome.equal,
        outcome.total,
        outcome.first_bad
    );
}
