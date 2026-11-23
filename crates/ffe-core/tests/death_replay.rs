//! Replays F942's destruction, F926's pods and F939's sparks (`ffe_core::effects`) against the
//! executed original, frame for frame (parity-08, ui-12). Local only (`#[ignore]`): it needs the
//! per-frame object rows of the reference harness in `FFE_DEATH_TRACE_FILE`:
//!
//! ```text
//! FFE_PAIR=12345678,9abcdef0 FFE_DEATH_OBJECTS=1 \
//!   .cache/jjffe-reference/jjffe-smoke.exe --death-probe 700 > death.jsonl
//! FFE_DEATH_TRACE_FILE=death.jsonl \
//!   cargo test -p ffe-core --test death_replay -- --ignored --nocapture
//! ```
//!
//! `FFE_START_KEY`, `FFE_DEATH_AT` and the hit's `FFE_PAIR` choose the start and the shared random
//! pair; `FFE_DEATH_OBJECTS=1` prints `{"death_hit":..}` (the destroyed ship and the pair) and one
//! `{"fx":..}` row per frame from the hit (`hit:1` right after F953 returned) with every effect
//! object: `[slot, model, control, AI, +0x7e, +0xa4, +0x9e, +0x14e, x, y, z, vx, vy, vz, rows]`.
//! The port's `Effects` is built with `explosion` from the hit row and swept once per later row
//! with that frame's tics and the game clock before it, and the two object lists are compared:
//! slot, model, AI state, distance word, timer, global word 1, the pod's spark count, position,
//! velocity words and the matrix rows. The shared random pair is carried by the port itself, so a
//! mismatch in the number of draws any routine makes shows as a different list from then on.

use ffe_core::effects::{
    derelict, explosion, EffectTable, Kind, DEBRIS_MODEL, EXPLOSION_MODEL, HIT_MODEL, OBJECT_SLOTS,
};
use ffe_core::npc::{Random, MM_PER_VELOCITY_UNIT};

struct Object {
    slot: usize,
    model: u32,
    ai: u8,
    shift: i32,
    timer: u32,
    g1: u16,
    countdown: i8,
    position: [i64; 3],
    velocity: [i32; 3],
    rows: [i32; 9],
}

struct Frame {
    fx: u64,
    frame: u64,
    tics: u32,
    objects: Vec<Object>,
}

fn num(v: &serde_json::Value) -> i64 {
    v.as_i64().unwrap_or_else(|| v.as_u64().unwrap() as i64)
}

fn trace() -> (serde_json::Value, Vec<Frame>) {
    let path = std::env::var("FFE_DEATH_TRACE_FILE").expect("FFE_DEATH_TRACE_FILE");
    let text = std::fs::read_to_string(path).unwrap();
    let mut hit = serde_json::Value::Null;
    let mut frames = Vec::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("death_hit").is_some() {
            hit = v;
        } else if let Some(fx) = v.get("fx") {
            let objects = v["objs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| {
                    let o = o.as_array().unwrap();
                    let mut rows = [0i32; 9];
                    for (k, r) in o[14].as_array().unwrap().iter().enumerate() {
                        rows[k] = num(r) as i32;
                    }
                    Object {
                        slot: num(&o[0]) as usize,
                        model: num(&o[1]) as u32,
                        ai: num(&o[3]) as u8,
                        shift: num(&o[4]) as i32,
                        timer: num(&o[5]) as u32,
                        g1: num(&o[6]) as u16,
                        countdown: num(&o[7]) as i8,
                        position: [num(&o[8]), num(&o[9]), num(&o[10])],
                        velocity: [num(&o[11]) as i32, num(&o[12]) as i32, num(&o[13]) as i32],
                        rows,
                    }
                })
                .collect();
            frames.push(Frame {
                fx: fx.as_u64().unwrap(),
                frame: v["frame"].as_u64().unwrap(),
                tics: v["tics"].as_u64().unwrap() as u32,
                objects,
            });
        }
    }
    assert!(
        !hit.is_null(),
        "no death_hit row: run with FFE_DEATH_OBJECTS=1"
    );
    (hit, frames)
}

/// The shared pair before the first draw of F939/F942/F926 in each frame, from the harness's
/// `FFE_RNG_TRACE` file (`FFE_DEATH_RNG_FILE`, default `rng.txt` beside the trace). Other
/// consumers of the pair (the starport's F922(12) draws, say) run between the frames' effect
/// draws; the replay resumes from the trace's pair at the first effect draw of each frame and
/// counts the frames where another consumer sits between two effect draws.
fn pairs() -> (
    std::collections::HashMap<u64, (u32, u32)>,
    std::collections::HashSet<u64>,
) {
    let trace = std::env::var("FFE_DEATH_TRACE_FILE").unwrap();
    let path = std::env::var("FFE_DEATH_RNG_FILE").unwrap_or_else(|_| {
        std::path::Path::new(&trace)
            .with_file_name("rng.txt")
            .to_string_lossy()
            .into_owned()
    });
    let mut first = std::collections::HashMap::new();
    let mut interleaved = std::collections::HashSet::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return (first, interleaved);
    };
    // The return addresses' low 16 bits of F939's gate and two angle draws, F942's two angle
    // draws and F922(6), and F926's drift draws.
    const EFFECT: [&str; 8] = [
        "c350", "c3ad", "c3b8", "c896", "c8a1", "c96c", "b629", "b63a",
    ];
    let mut last_effect: std::collections::HashMap<u64, bool> = Default::default();
    for line in text.lines() {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() != 7 {
            continue;
        }
        let frame: u64 = c[0].parse().unwrap();
        let caller = c[6].to_lowercase();
        let effect = EFFECT.iter().any(|tail| caller.ends_with(tail));
        if effect {
            first.entry(frame).or_insert((
                u32::from_str_radix(c[2], 16).unwrap(),
                u32::from_str_radix(c[3], 16).unwrap(),
            ));
            if last_effect.get(&frame) == Some(&false) {
                interleaved.insert(frame);
            }
        }
        if first.contains_key(&frame) {
            last_effect.insert(frame, effect);
        }
    }
    (first, interleaved)
}

fn kind_model(kind: Kind) -> (u32, u8) {
    match kind {
        Kind::Explosion => (EXPLOSION_MODEL, 0x1c),
        Kind::Debris => (DEBRIS_MODEL, 0x15),
        Kind::Hit => (HIT_MODEL, 0x1a),
        _ => (0, 0),
    }
}

/// The port's list in the trace's terms; the matrix rows are compared as `f64` of `rows / 2^31`.
/// One port object: slot, model, AI state, distance word, timer, global word 1, pod count,
/// position, velocity words and matrix rows.
type Row = (
    usize,
    u32,
    u8,
    i32,
    u32,
    u16,
    i8,
    [i64; 3],
    [i32; 3],
    [f64; 9],
);

fn describe(effects: &EffectTable) -> Vec<Row> {
    effects
        .iter()
        .filter(|(_, e)| e.tag.is_none())
        .map(|(slot, e)| {
            let (model, ai) = kind_model(e.kind);
            let axes = e.axes();
            let rows = [
                axes[0][0], axes[0][1], axes[0][2], axes[1][0], axes[1][1], axes[1][2], axes[2][0],
                axes[2][1], axes[2][2],
            ];
            (
                slot,
                model,
                ai,
                i32::from(e.shift),
                u32::from(e.timer),
                e.size,
                e.countdown,
                e.position,
                e.velocity_words(),
                rows,
            )
        })
        .collect()
}

fn same(frame: &Frame, effects: &EffectTable) -> Result<(), String> {
    let port = describe(effects);
    let mut report = Vec::new();
    let original: std::collections::BTreeMap<usize, &Object> =
        frame.objects.iter().map(|o| (o.slot, o)).collect();
    let ported: std::collections::BTreeMap<usize, _> = port.iter().map(|p| (p.0, p)).collect();
    for slot in original
        .keys()
        .chain(ported.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        match (original.get(slot), ported.get(slot)) {
            (Some(o), None) => report.push(format!(
                "slot {slot}: only in the original (model {})",
                o.model
            )),
            (None, Some(p)) => {
                report.push(format!("slot {slot}: only in the port (model {})", p.1))
            }
            (Some(o), Some(p)) => {
                let mut diff = Vec::new();
                if o.model != p.1 || o.ai != p.2 {
                    diff.push(format!("kind {}/{} vs {}/{}", o.model, o.ai, p.1, p.2));
                }
                if o.shift != p.3 {
                    diff.push(format!("shift {} vs {}", o.shift, p.3));
                }
                // The timer is a dword in the original; the port keeps its low word.
                if (o.timer & 0xffff) != p.4 && o.model != DEBRIS_MODEL {
                    diff.push(format!("timer {} vs {}", o.timer & 0xffff, p.4));
                }
                if o.g1 != p.5 {
                    diff.push(format!("g1 {} vs {}", o.g1, p.5));
                }
                if o.model == DEBRIS_MODEL && o.countdown != p.6 {
                    diff.push(format!("countdown {} vs {}", o.countdown, p.6));
                }
                if o.position != p.7 {
                    diff.push(format!("position {:?} vs {:?}", o.position, p.7));
                }
                if o.velocity != p.8 {
                    diff.push(format!("velocity {:?} vs {:?}", o.velocity, p.8));
                }
                // A station's rows are 0x7f000000, a little under unit length; the port
                // keeps unit axes, so the original's rows are compared by direction.
                let unit = |row: usize| {
                    let v: [f64; 3] = std::array::from_fn(|k| f64::from(o.rows[3 * row + k]));
                    let length = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1.0);
                    v.map(|x| x / length)
                };
                let worst = (0..9)
                    .map(|k| (unit(k / 3)[k % 3] - p.9[k]).abs())
                    .fold(0.0, f64::max);
                // The original's Sin32 table is accurate to about 1e-4 of full scale.
                if worst > 2e-4 {
                    diff.push(format!("rows differ by {worst:.3e}"));
                }
                if !diff.is_empty() {
                    report.push(format!(
                        "slot {slot} (model {}): {}",
                        o.model,
                        diff.join("; ")
                    ));
                }
            }
            (None, None) => {}
        }
    }
    if report.is_empty() {
        Ok(())
    } else {
        Err(report.join("\n"))
    }
}

#[test]
#[ignore = "needs FFE_DEATH_TRACE_FILE from the reference harness (see the module doc)"]
fn the_destruction_matches_the_executed_original_frame_for_frame() {
    let (hit, frames) = trace();
    let pair = hit["pair"].as_array().unwrap();
    let mut rng = Random::new(num(&pair[0]) as u32, num(&pair[1]) as u32);
    let m = hit["m"].as_array().unwrap();
    let rows: Vec<f64> = m.iter().map(|v| num(v) as f64 / 2_147_483_648.0).collect();
    // The port keeps unit axes (a station's rows are 0x7f000000, a little under unit length).
    let row = |k: usize| {
        let v = [rows[3 * k], rows[3 * k + 1], rows[3 * k + 2]];
        let length = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-9);
        v.map(|x| x / length)
    };
    let axes = [row(0), row(1), row(2)];
    let position = [
        num(&hit["pos"][0]),
        num(&hit["pos"][1]),
        num(&hit["pos"][2]),
    ];
    let words = [num(&hit["v"][0]), num(&hit["v"][1]), num(&hit["v"][2])];
    let velocity = words.map(|w| w as f64 * MM_PER_VELOCITY_UNIT);
    let cargo = num(&hit["cargo"]) as u16;
    // DATA_008867, the player's own speed, in the port's mm/s: the ship's velocity words (the
    // sparks keep their thrower's velocity once it passes F939's 0x9c40 cutoff).
    let speed = words
        .iter()
        .map(|w| (*w as f64).powi(2))
        .sum::<f64>()
        .sqrt()
        * MM_PER_VELOCITY_UNIT;
    // The slots the other objects (planet, starport, ships, the player) hold: placeholders the
    // sweep leaves alone (a tag keeps F767 from removing them), the player's own freed by F924
    // at the end of F942.
    let occupied: Vec<usize> = hit["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| num(v) as usize)
        .collect();
    let player_slot = num(&hit["player_slot"]) as usize;
    let mut effects = EffectTable::new();
    for _ in 1..OBJECT_SLOTS {
        let mut filler = derelict(
            0,
            [0; 3],
            [0.0; 3],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );
        filler.tag = Some(0);
        effects.spawn(filler);
    }
    for slot in 1..OBJECT_SLOTS {
        if !occupied.contains(&slot) {
            effects.free(slot);
        }
    }
    let mut burst = explosion(position, velocity, cargo, axes, &mut rng).into_iter();
    effects.spawn(burst.next().unwrap());
    let fraction = [0, 1, 2].map(|k| hit["fraction"].get(k).map_or(0, |v| num(v) as u16));
    let parent = ffe_core::effects::Parent {
        centre: [0; 3],
        mass: num(&hit["parent_mass"]) as u32,
    };
    for mut pod in burst {
        // F926 clones the ship, so a pod is in the ship's parent frame.
        pod.parent = Some(parent);
        // ... and carries the ship's F603 fraction words (+0xec..+0xf0).
        pod.fraction = fraction;
        effects.spawn_pod(pod);
    }
    effects.free(player_slot);

    let (resume, interleaved) = pairs();
    let mut failures = 0;
    let mut previous: Option<u32> = None;
    let mut compared = 0;
    for frame in &frames {
        if let Some(before) = previous {
            if frame.fx > 0 {
                if let Some(&(a, b)) = resume.get(&frame.frame) {
                    rng = Random::new(a, b);
                }
                let tics = frame.tics.wrapping_sub(before);
                effects.sweep(tics, Some(before), speed, position, &mut rng);
            }
        }
        previous = Some(frame.tics);
        compared += 1;
        if let Err(report) = same(frame, &effects) {
            failures += 1;
            if failures <= 6 {
                println!("frame {} differs:\n{report}", frame.fx);
            }
        }
    }
    println!(
        "{compared} frames compared, {failures} differ ({} frames with another consumer between effect draws); {} objects at the end",
        interleaved.len(),
        effects.iter().filter(|(_, e)| e.tag.is_none()).count()
    );
    assert_eq!(failures, 0, "{failures} of {compared} frames differ");
}
