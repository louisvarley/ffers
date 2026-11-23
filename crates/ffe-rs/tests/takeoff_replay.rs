//! Local-only, frame-by-frame replay of the executed original's open-air takeoff
//! (`jjffe-smoke --flight takeoff 900`, start record 1, Ross 154) through the
//! port's `Navigator`/`OriginalFlight` (PAR-04).
//!
//! The original's script: the Tab launch action at frame 170 (mode `0x24` from
//! frame 171), then the faster key held from frame 260 (the set speed is 16 at
//! frame 261 and F1013 hands off to `0x26` there). Every frame is 21 ms (1,043
//! tics at 1x). Each port frame is compared with the trace row of the same
//! number: mode, F1033's altitude, F1121's set speed, the player's position in
//! its parent's frame and its velocity.
//!
//! Run with `FFE_WORLD_CACHE=<data.bin> FFE_TAKEOFF_TRACE=<takeoff.jsonl>`.
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::navigation::{landing_body, Library, Navigator};
use ffe_rs::original_flight::{Keys as OriginalKeys, OriginalFlight, UNITS_PER_FLIGHT_UNIT};
use serde::Deserialize;

const LAUNCH_FRAME: u64 = 170;
const FASTER_FROM: u64 = 260;
const FRAME_SECONDS: f64 = 0.021;

thread_local! {
    /// The frame period the replay steps with (the original's 21 ms by default).
    static PERIOD: std::cell::Cell<f64> = const { std::cell::Cell::new(FRAME_SECONDS) };
}

thread_local! {
    /// The original's tics for each frame, when a trace with uneven frames is
    /// replayed; the frame being stepped.
    static TICS: std::cell::RefCell<std::collections::HashMap<u64, u32>> = Default::default();
    static NOW: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// The player's laser words after each replayed frame: refire, heat, energy.
    static LASER: std::cell::RefCell<Vec<(u64, u16, u16, u32)>> = Default::default();
    /// The ship's position relative to the planet's centre in the planet's rotating
    /// frame after each replayed frame (the frame `FFE_ROWS_DUMP`'s `pos` is in).
    static LOCAL: std::cell::RefCell<Vec<(u64, [i64; 3])>> = Default::default();
}

/// This frame's length in milliseconds: the traced one when there is a trace
/// (`ms * 0x3e2 / 20` tics, so 994 is 20 ms and 1,043 is 21 ms), else the period.
fn frame_ms() -> u32 {
    let traced = TICS.with(|t| t.borrow().get(&NOW.with(std::cell::Cell::get)).copied());
    match traced {
        Some(tics) => (f64::from(tics) * 20.0 / f64::from(0x3e2_u32)).round() as u32,
        None => (PERIOD.with(std::cell::Cell::get) * 1000.0).round() as u32,
    }
}

fn period() -> f64 {
    f64::from(frame_ms()) / 1000.0
}

/// F1418's tics for the current frame: whole milliseconds times 0x3e2, divided
/// by 20 (1,043 for 21 ms).
fn frame_tics() -> u32 {
    frame_ms() * 0x3e2 / 20
}

#[derive(Deserialize)]
struct Row {
    frame: u64,
    mode: u8,
    altitude: i64,
    set_speed: i64,
    position: [i64; 3],
    velocity: [i64; 3],
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct State {
    mode: u8,
    altitude: i64,
    set_speed: i64,
    position: [i64; 3],
    velocity: [i64; 3],
}

fn reference() -> Vec<Row> {
    trace("FFE_TAKEOFF_TRACE")
}

fn trace(variable: &str) -> Vec<Row> {
    let path = std::env::var_os(variable).unwrap_or_else(|| panic!("{variable}"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"scenario\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// The `keyboard` scenario's per-frame input clock and `DATA_007690`.
#[derive(Deserialize)]
struct KeyboardRow {
    keyboard: u64,
    timer_last: u32,
    timer_next: u32,
}

/// The keys the takeoff scenario holds for the port's frame `frame`: the
/// original's frame reads them at the end of the frame before (Return from
/// the read at frame 260 on). Its reference never reports a last key, so no
/// scancodes are held.
fn takeoff_keys(frame: u64) -> OriginalKeys {
    OriginalKeys {
        faster: frame > FASTER_FROM,
        ..OriginalKeys::default()
    }
}

/// The `paddock` scenario's keys: Return and keypad 8 for the reads at frames 260 to
/// 299 only, then none (the autopilot flies the rest).
fn pad_burst_keys(frame: u64) -> OriginalKeys {
    OriginalKeys {
        faster: (260..300).contains(&(frame - 1)),
        ..OriginalKeys::default()
    }
}

/// `keyboard_down()`'s schedule in `tools/reference/headless.c`, read at the
/// end of the previous frame: Return (0x1c), right Shift (0x36) and A (0x1e,
/// the Classic pitch-up key), with their scancodes for the last-key counter.
fn keyboard_keys(frame: u64) -> OriginalKeys {
    let f = frame - 1;
    let ret = (260..400).contains(&f) || (700..800).contains(&f);
    let shift = (450..600).contains(&f) || (620..690).contains(&f);
    let a = (740..746).contains(&f);
    let scancodes = [(ret, 0x1c), (shift, 0x36), (a, 0x1e)]
        .into_iter()
        .filter(|(down, _)| *down)
        .map(|(_, code)| code)
        .collect();
    OriginalKeys {
        faster: ret,
        slower: shift,
        pitch_up: a,
        scancodes,
        ..OriginalKeys::default()
    }
}

/// The port's frames from the launch action on, keyed by the trace's frame number.
/// The start record the trace was captured from: `FFE_START_TYPE`, 1 (Ross 154, the
/// harness's default) or 0 (`--start-key=0x39`, Hope).
fn start_type() -> u8 {
    std::env::var("FFE_START_TYPE").map_or(1, |v| v.parse().expect("FFE_START_TYPE"))
}

fn replay(last: u64) -> Vec<(u64, State)> {
    replay_with(last, takeoff_keys, None).0
}

/// The same with `keys_for(frame)` held and, when given, the input clock set
/// to the original's `(tmLastTime, tmNext50Hz)` before the first stepped frame.
#[allow(clippy::type_complexity)]
fn replay_with(
    last: u64,
    keys_for: fn(u64) -> OriginalKeys,
    clock: Option<(u32, u32)>,
) -> (Vec<(u64, State)>, Vec<(u64, u32)>) {
    use ffe_core::traffic::{state as traffic_state, TakeoffStep};
    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let mut nav = Navigator::new_game(&cache, start_type()).unwrap();
    let index = nav.docked.expect("landed at the start starport");
    // The executed original's RNG pair changes once in 900 frames of this takeoff
    // (the boot's F688 at frame 162) and never during the climb or the free flight
    // (`FFE_RESERVE_DUMP`'s `rng`), so nothing here may draw from the shared pair.
    let rng_at_start = (nav.random.a, nav.random.b);
    let start = nav.new_game.unwrap();
    let body = nav.bodies[index].clone();
    let planet = nav.bodies[body.parent.unwrap()].clone();
    let records = cache.asset("base:model/records").unwrap();
    if std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some() {
        eprintln!(
            "port: planet {:?} station {:?} seat {:?} station rows {:?} planet rows {:?}",
            planet.position, body.position, start.seat, body.rows, planet.rows
        );
    }
    let stats = cache.asset("base:ship/stats").unwrap();
    let info = ffe_core::objects::ModelInfo::read(records, stats, start.ship).unwrap();
    let library = Library(&cache);
    let tools = cache
        .asset("base:terrain/tables")
        .zip(cache.asset("base:terrain/maps"))
        .map(
            |(terrain_table, terrain_maps)| ffe_core::surface::SurfaceTools {
                shapes: &library,
                terrain_table,
                terrain_maps,
                detail: 0,
            },
        );
    let planet_body = landing_body(
        &planet,
        &ffe_core::objects::ModelInfo::read(records, stats, planet.model).unwrap(),
    );
    let station_body = landing_body(
        &body,
        &ffe_core::objects::ModelInfo::read(records, stats, body.model).unwrap(),
    );
    let mut flight = Flight {
        mode: Mode::Flight,
        system_flight: true,
        ship: start.ship,
        ..Flight::default()
    };
    flight.position = start.seat.map(|v| v as f64 / UNITS_PER_FLIGHT_UNIT);
    // F998's type-0 arm calls F997 after F994: the ship's first and third rows are
    // reversed, so it faces out of the port (`new_commander` does the same).
    let mut axes = body.orientation;
    if start_type() == 0 {
        for row in [0, 2] {
            axes[row] = axes[row].map(|v| -v);
        }
    }
    flight.axes = Some(axes);
    // The ship's own statistics (F600's limits), as the game passes them.
    let ship_stats = stats
        .chunks_exact(48)
        .nth(start.ship as usize)
        .and_then(ffe_rs::ships::ShipStats::from_block);
    assert!(
        ship_stats.is_some(),
        "no statistics for ship {}",
        start.ship
    );
    let mut model = OriginalFlight::from_flight(&flight, ship_stats.as_ref());
    // The reserve F938/F929 spend (`+0x11e`): the new game's first F152 dispatches
    // 43,200 events at frame 162; after that every 1,043-tic frame adds to the
    // day fraction and a whole event is due each 0xc22e units. The events run on
    // the thrust words the frame left (F1007 runs after the flight step).
    let mut reserve_state = ffe_rs::boot::new_commander(&cache, &nav).unwrap().0;
    let mut events = ffe_core::second_events::SecondEvents::new_game();
    let mut reserve: Vec<(u64, u32)> = Vec::new();
    let ship = ship_stats.expect("ship statistics");
    NOW.with(|n| n.set(162));
    reserve_state.player_events(events.advance(frame_tics()), 0x30, &ship, 0);
    for frame in 163..=LAUNCH_FRAME + 1 {
        NOW.with(|n| n.set(frame));
        assert_eq!(events.advance(frame_tics()), 0);
    }
    // Parked on the pad until the launch action.
    model.begin_pad_takeoff(
        &planet_body,
        planet.position,
        &station_body,
        body.position,
        &info,
    );
    // The original keeps the parked and climbing ship in the planet's rotating
    // frame (`+0x57`); the port's pad world keeps it in the planet's linear frame
    // with the planet's rows fixed. Turn the port's vectors into the rotating
    // frame (`local = rows * linear`) to compare them.
    let rows = planet.rows;
    let to_planet = move |v: [i64; 3]| -> [i64; 3] {
        std::array::from_fn(|k| {
            (0..3)
                .map(|axis| v[axis] as f64 * f64::from(rows[3 * k + axis]) / 2_147_483_648.0)
                .sum::<f64>()
                .round() as i64
        })
    };
    let state = |model: &OriginalFlight, mode: u8, altitude: i64| State {
        mode,
        altitude,
        set_speed: i64::from(model.set_speed_units()),
        position: to_planet(model.player().motion.position),
        velocity: to_planet(model.player().motion.velocity.map(i64::from)),
    };
    let mut rows = vec![(LAUNCH_FRAME, state(&model, 0x30, 0))];
    // The frame the launch action is processed (171 in the trace): as in the
    // game loop's docked-to-flight hand-off, the model is set up after that
    // frame's flight step, so the ship does not move until the next frame.
    nav.begin_launch().unwrap();
    model.begin_takeoff();
    rows.push((LAUNCH_FRAME + 1, state(&model, model.mode(), 0)));
    if let Some((now, next_tick)) = clock {
        model.set_input_clock(now, next_tick);
    }
    let mut mode;
    let mut altitude;
    let mut autopilot_ended: Option<u64> = None;
    LOCAL.with(|l| l.borrow_mut().clear());
    for frame in LAUNCH_FRAME + 2..=last {
        if autopilot_ended.is_some() {
            break;
        }
        NOW.with(|n| n.set(frame));
        // The harness's test cases, applied at the end of the frame before: the engines
        // off (`FFE_ENGINES_OFF_AT`, F1056's action 9: `+0xe9` bit 0) and a damaged
        // thruster (`FFE_LIMITS_CUT_AT`: the six thrust limits halved).
        let at = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok());
        if at("FFE_ENGINES_OFF_AT") == Some(frame - 1) {
            model.set_engines_on(false);
        }
        if at("FFE_LIMITS_CUT_AT") == Some(frame - 1) {
            let mut limits = model.player().limits;
            limits.axes = limits.axes.map(|axis| axis.map(|v| v / 2));
            model.set_limits(limits);
        }
        // The harness's `paddock` scenario (`FFE_AUTOPILOT_AT=<frame>`): at the end of
        // that frame the starport the ship left is the navigation target and F1020
        // engages the autopilot (docking computer fitted). F1020 only sets the state; the
        // thrust the launch flight left holds until the first whole-second event.
        if at("FFE_AUTOPILOT_AT") == Some(frame - 1) {
            nav.lock(index);
            nav.engage_locked().expect("the autopilot engages");
            model.apply_guidance(model.held_guidance());
        }
        // Once the autopilot flies the game loop advances the world clock and carries
        // the ship with the frame body, as `Navigator::advance`/`follow` do.
        if nav.autopilot_engaged() {
            nav.advance(&cache, period());
            if let Some(moved) = nav.follow(model.absolute_position()) {
                model.shift(moved);
            }
        }
        let keys = if nav.autopilot_engaged() {
            OriginalKeys::default()
        } else {
            keys_for(frame)
        };
        let contact = if model.on_pad() {
            model.step_pad(period(), &keys, tools.as_ref())
        } else {
            // As the game loop chooses: the body under the ship, and for a
            // starport its planet with the port as F994's sibling.
            let at = model.absolute_position();
            let parent_index = nav.landing_parent(at, info.radii().0).unwrap_or(index);
            let parent = nav.bodies[parent_index].clone();
            let read =
                |model: u32| ffe_core::objects::ModelInfo::read(records, stats, model).unwrap();
            if parent.kind == ffe_rs::navigation::Kind::Starport {
                let ground = nav.bodies[parent.parent.unwrap()].clone();
                model.step_starport_landing(
                    period(),
                    &keys,
                    &landing_body(&ground, &read(ground.model)),
                    ground.position,
                    &landing_body(&parent, &read(parent.model)),
                    parent.position,
                    &info,
                    tools.as_ref(),
                )
            } else {
                model.step_landing(
                    period(),
                    &keys,
                    &landing_body(&parent, &read(parent.model)),
                    parent.position,
                    &info,
                    tools.as_ref(),
                )
            }
        };
        let contact = match contact {
            Ok(contact) => contact,
            Err(error) => {
                eprintln!("frame {frame}: {error}");
                break;
            }
        };
        // The game loop's hand-off (main.rs): F1013 runs inside the model, which
        // changes its own mode; the navigator follows once it is 0x26, and F1031
        // runs on the following frames with F1033's altitude.
        if nav.takeoff.is_some() {
            let step = if nav.taking_off() {
                if model.mode() == traffic_state::AFTER_TAKEOFF {
                    nav.takeoff_step(i32::MAX, model.set_speed_units(), contact.flags)
                } else {
                    None
                }
            } else {
                nav.takeoff_step(contact.altitude(), model.set_speed_units(), contact.flags)
            };
            match step {
                Some(TakeoffStep::HandOff) => model.set_mode(nav.takeoff_state().unwrap()),
                Some(TakeoffStep::Cleared) => {
                    model.set_mode(0);
                    if model.on_pad() {
                        model.end_pad_takeoff();
                    }
                }
                Some(TakeoffStep::Retakeoff) => model.begin_takeoff(),
                None => {}
            }
        }
        mode = model.mode();
        altitude = i64::from(model.player().altitude);
        rows.push((frame, state(&model, mode, altitude)));
        // F730 counts the refire word down in the object update, before F1007's
        // events; the harness's `FFE_FIRE_FROM` fires the front mount at the end of
        // the frame (F931 refuses while the word counts, the laser is hot or the
        // reserve is empty).
        reserve_state.combat.tick(frame_tics(), 0);
        let due = events.advance(frame_tics());
        if due != 0 {
            let thrust = reserve_state.thrust_sum(model.thrust_words());
            reserve_state.player_events(due, model.mode(), &ship, thrust);
            // F1007 runs the AI states from the event, after the frame's flight step.
            if nav.autopilot_engaged() && nav.sequence.is_none() {
                match nav.source_command(&cache, &mut model) {
                    Some((guidance, _)) => model.apply_guidance(guidance),
                    None => {
                        eprintln!("frame {frame}: the autopilot ended");
                        autopilot_ended = Some(frame);
                    }
                }
            }
        }
        if at("FFE_FIRE_FROM").is_some_and(|from| frame >= from) {
            let _ = reserve_state.combat.fire(0, frame_tics());
        }
        reserve.push((frame, reserve_state.combat.energy));
        {
            // The planet's place and rows now (the navigator moves them once the autopilot flies).
            let now = &nav.bodies[body.parent.unwrap()];
            let at = model.absolute_position();
            let relative: [i64; 3] = std::array::from_fn(|k| at[k] - now.position[k]);
            let turned: [i64; 3] = std::array::from_fn(|k| {
                (0..3)
                    .map(|axis| {
                        relative[axis] as f64 * f64::from(now.rows[3 * k + axis]) / 2_147_483_648.0
                    })
                    .sum::<f64>()
                    .round() as i64
            });
            LOCAL.with(|l| l.borrow_mut().push((frame, turned)));
        }
        LASER.with(|l| {
            let c = &reserve_state.combat;
            l.borrow_mut().push((frame, c.refire, c.heat, c.energy));
        });
    }
    assert_eq!(
        (nav.random.a, nav.random.b),
        rng_at_start,
        "the takeoff drew from the shared RNG pair"
    );
    (rows, reserve)
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_TAKEOFF_TRACE"]
fn port_takeoff_replays_the_original_frame_by_frame() {
    let reference = reference();
    let last = reference.last().unwrap().frame;
    let port = replay(last);
    let verbose = std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some();
    let length = |v: [i64; 3]| v.iter().map(|c| (*c as f64).powi(2)).sum::<f64>().sqrt();
    let mut changes = (Vec::new(), Vec::new());
    let mut previous = (None, None);
    let mut worst_speed = 0.0_f64;
    for (frame, ours) in &port {
        let Some(row) = reference.iter().find(|row| row.frame == *frame) else {
            continue;
        };
        let theirs = State {
            mode: row.mode,
            altitude: row.altitude,
            set_speed: row.set_speed,
            position: row.position,
            velocity: row.velocity,
        };
        if verbose {
            eprintln!(
                "frame {frame}
  original {theirs:?}
  port     {ours:?}"
            );
        }
        // F1121's set speed, from the same keys, on every frame.
        assert_eq!(
            ours.set_speed, theirs.set_speed,
            "set speed at frame {frame}"
        );
        if previous.0 != Some(theirs.mode) {
            changes.0.push((theirs.mode, *frame));
        }
        if previous.1 != Some(ours.mode) {
            changes.1.push((ours.mode, *frame));
        }
        previous = (Some(theirs.mode), Some(ours.mode));
        // The climb integrates in the planet's rotating frame as the original
        // does (F601's per-axis floor rounding depends on the axes' signs), so
        // its velocity is the original's to within the frame turn's rounding.
        if theirs.mode != 0 && theirs.mode != 0x2c && *frame > LAUNCH_FRAME {
            for axis in 0..3 {
                assert!(
                    (ours.velocity[axis] - theirs.velocity[axis]).abs() <= 3,
                    "frame {frame} axis {axis}: original {:?}, port {:?}",
                    theirs.velocity,
                    ours.velocity
                );
            }
        }
        let speed = length(theirs.velocity);
        if speed > 500.0 {
            worst_speed = worst_speed.max((length(ours.velocity) - speed).abs() / speed);
        }
    }
    eprintln!("modes: original {:?}, port {:?}", changes.0, changes.1);
    eprintln!(
        "largest speed difference above 500 units: {:.2}%",
        worst_speed * 100.0
    );
    // 0x30 -> 0x24 (the launch) -> 0x26 (F1013's set-speed hand-off) -> 0x00.
    assert_eq!(
        changes.0.iter().map(|c| c.0).collect::<Vec<_>>(),
        changes.1.iter().map(|c| c.0).collect::<Vec<_>>()
    );
    for (theirs, ours) in changes.0.iter().zip(&changes.1) {
        // The launch and F1013's hand-off are on the same frame. F1031's clear
        // is one frame early (429 against 430): the altitude at the clear is
        // within 0.1% (PAR-05, flight-1).
        let slack = if theirs.0 == 0 && theirs.1 > LAUNCH_FRAME {
            1
        } else {
            0
        };
        assert!(
            theirs.1 - ours.1.min(theirs.1) <= slack && ours.1 <= theirs.1,
            "mode {:#x}: original frame {}, port frame {}",
            theirs.0,
            theirs.1,
            ours.1
        );
    }
    // The climb and the free flight after it (F1014's drag included; without it
    // the port was 23% fast by frame 899) keep the speed within 0.5%. A per-frame
    // F1520 on the ship's rows once cut the thrust 1.5% from the clear on and
    // left the port 1.25% slow.
    assert!(worst_speed < 0.005, "speed {:.2}% off", worst_speed * 100.0);
}

/// The long climb and fall (PAR-05): `jjffe-smoke --flight takeoff 4000` holds the
/// faster key for the whole flight. The original never leaves the planet's frame
/// (its parent stays the planet for all 3,119 frames), the ship climbs to about 6,700
/// km, turns back under the planet's pull and strikes the ground at frame 3120
/// (mode 0x2c). The replay must keep the modes and the set speed exactly, and the
/// speed within 1% on every frame to 2800. Past that the port's fall is steeper
/// (docs/ACCEPTED_PARITY.md): it reaches the ground about 25 frames early.
/// Run with `FFE_WORLD_CACHE` and `FFE_LONG_TRACE`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_LONG_TRACE"]
fn port_long_flight_follows_the_original_to_the_descent() {
    const LAST: u64 = 2800;
    let reference = trace("FFE_LONG_TRACE");
    assert!(
        reference
            .iter()
            .any(|row| row.frame == 3120 && row.mode == 0x2c),
        "the original strikes the ground at frame 3120"
    );
    // The player's parent body (`planet.index`) is the same on every row after the
    // launch: the original's frame selector never switches in this flight.
    let parents: std::collections::BTreeSet<u64> =
        std::fs::read_to_string(std::env::var_os("FFE_LONG_TRACE").unwrap())
            .unwrap()
            .lines()
            .filter(|line| line.starts_with("{\"scenario\""))
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|row| row["frame"].as_u64().unwrap() > LAUNCH_FRAME)
            .map(|row| row["planet"]["index"].as_u64().unwrap())
            .collect();
    assert_eq!(
        parents.len(),
        1,
        "the original left the planet's frame: {parents:?}"
    );
    let port = replay(LAST);
    let length = |v: [i64; 3]| v.iter().map(|c| (*c as f64).powi(2)).sum::<f64>().sqrt();
    let (mut worst, mut worst_frame, mut compared) = (0.0f64, 0, 0);
    for (frame, ours) in &port {
        let Some(row) = reference.iter().find(|row| row.frame == *frame) else {
            continue;
        };
        assert_eq!(ours.set_speed, row.set_speed, "set speed at frame {frame}");
        compared += 1;
        let speed = length(row.velocity);
        if speed > 500.0 {
            let error = (length(ours.velocity) - speed).abs() / speed;
            if error > worst {
                (worst, worst_frame) = (error, *frame);
            }
        }
    }
    eprintln!(
        "{compared} frames compared; worst speed difference {:.2}% (frame {worst_frame})",
        worst * 100.0
    );
    assert!(compared > 2600);
    assert!(
        worst < 0.01,
        "speed {:.2}% off at frame {worst_frame}",
        worst * 100.0
    );
}

/// PAR-11: the autopilot's whole descent to a planet's starport. `jjffe-smoke
/// --start-key=0x02 --flight paddock 20000` (with `FFE_ROWS_DUMP=1`, stderr) launches
/// from Ross 154's pad, holds Return for frames 260 to 299, and at the end of frame 770
/// targets the starport and engages F1020; the original flies mode 2 to its pad
/// (frame 15,704). The replay does the same from the same keys, with
/// `FFE_AUTOPILOT_AT=770`, and runs F741..F746 on F152's events, then compares the
/// ship's place relative to the planet, in the planet's rotating frame, on every frame.
/// Run with `FFE_WORLD_CACHE`, `FFE_PAD_POSES` (that stderr) and `FFE_AUTOPILOT_AT`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE, FFE_PAD_POSES and FFE_AUTOPILOT_AT"]
fn port_autopilot_flies_the_descent_to_a_pad() {
    #[derive(Deserialize)]
    struct Pose {
        rows: u64,
        mode: u8,
        pos: [i64; 3],
    }
    let poses: Vec<Pose> = std::fs::read_to_string(std::env::var_os("FFE_PAD_POSES").unwrap())
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"rows\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let engaged = poses
        .iter()
        .find(|p| p.mode == 2)
        .expect("the original engages the autopilot");
    let landed = poses
        .iter()
        .find(|p| p.rows > engaged.rows && p.mode == 0x30)
        .expect("the original lands");
    LOCAL.with(|l| l.borrow_mut().clear());
    replay_with(landed.rows, pad_burst_keys, None);
    let ours = LOCAL.with(|l| l.borrow().clone());
    let verbose = std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some();
    let length = |v: [i64; 3]| v.iter().map(|c| (*c as f64).powi(2)).sum::<f64>().sqrt();
    // The descent is about 8 km (the launch climbs to 257,000 altitude units).
    const LEG: f64 = 8_000_000.0;
    let (mut worst, mut worst_late, mut worst_frame, mut compared) = (0.0f64, 0.0f64, 0, 0);
    for (frame, at) in &ours {
        let Some(pose) = poses.iter().find(|p| p.rows == *frame && p.mode == 2) else {
            continue;
        };
        let error = length(std::array::from_fn(|k| at[k] - pose.pos[k])) / LEG;
        compared += 1;
        if verbose && frame % 250 == 0 {
            eprintln!(
                "frame {frame}: original {:?}, port {at:?}, error {:.2}%",
                pose.pos,
                error * 100.0
            );
        }
        if error > worst {
            (worst, worst_frame) = (error, *frame);
        }
        if *frame > engaged.rows + 800 {
            worst_late = worst_late.max(error);
        }
    }
    eprintln!(
        "{compared} frames compared, the port flew to frame {}; worst error {:.2}% of the 8 km descent (frame {worst_frame}), {:.2}% after the first 800 frames",
        ours.last().map_or(0, |o| o.0),
        worst * 100.0,
        worst_late * 100.0
    );
    assert!(compared > 14_000, "the port's autopilot ended early");
    // The first seconds after the engage are 5% off (frame 914, the guidance's first
    // events); after the first 800 frames the port stays within 1.2% of it (50 to 65 m) to the pad.
    assert!(
        worst < 0.06,
        "the descent differs from the original: {:.2}%",
        worst * 100.0
    );
    assert!(
        worst_late < 0.02,
        "the late descent differs from the original: {:.2}%",
        worst_late * 100.0
    );
}

/// F1037/F1121 against the executed original's `--flight keyboard 900`: the
/// takeoff, then Return held (F1121's step grows with `DATA_007690` once that
/// passes 0x10), right Shift held down through zero (flag 0x80 stops it there),
/// released and held again (the speed falls below zero by the input time over
/// 256), Return again (a negative speed snaps to zero first), and A pressed and
/// released inside that hold. The last-key counter follows the platform layer's
/// initial presses and repeats. Run with `FFE_WORLD_CACHE` and
/// `FFE_KEYBOARD_TRACE`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_KEYBOARD_TRACE"]
fn port_set_speed_follows_the_original_keyboard() {
    let reference = trace("FFE_KEYBOARD_TRACE");
    let path = std::env::var_os("FFE_KEYBOARD_TRACE").unwrap();
    let clocks: Vec<KeyboardRow> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"keyboard\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    // The port's first stepped frame is LAUNCH_FRAME + 2; it reads the keys and
    // advances from the clock the original's previous row shows.
    let start = clocks
        .iter()
        .find(|row| row.keyboard == LAUNCH_FRAME + 1)
        .expect("keyboard row for the launch frame");
    let last = reference.last().unwrap().frame;
    // The modes are the takeoff replay's to check (F1031's clear may come up to
    // two frames early here too, PAR-05); F1037 runs the same in 0x26 and 0.
    let (port, _) = replay_with(
        last,
        keyboard_keys,
        Some((start.timer_last, start.timer_next)),
    );
    let mut compared = 0;
    let mut lowest = i64::MAX;
    let mut highest = i64::MIN;
    for (frame, ours) in &port {
        let Some(row) = reference.iter().find(|row| row.frame == *frame) else {
            continue;
        };
        let theirs = row.set_speed as u32 as i32 as i64;
        assert_eq!(
            ours.set_speed as u32 as i32 as i64, theirs,
            "set speed at frame {frame}"
        );
        lowest = lowest.min(theirs);
        highest = highest.max(theirs);
        compared += 1;
    }
    eprintln!("{compared} frames; set speed {lowest}..={highest}");
    assert!(compared > 700 && lowest < 0 && highest > 150_000);
}

/// The default start's seat (type 0, Hope) in Hope's frame against the executed
/// original's `--geometry` (ship minus port: (128961, 203981, 157118)).
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn hope_seat_matches_the_original() {
    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let index = nav.docked.unwrap();
    let start = nav.new_game.unwrap();
    let body = &nav.bodies[index];
    let (_, planet_rows) = body.parent_frame.expect("Hope's port is in Hope's frame");
    let in_planet = |seat: [i64; 3]| -> [i64; 3] {
        std::array::from_fn(|k| {
            (0..3)
                .map(|axis| {
                    (seat[axis] - body.position[axis]) as f64 * f64::from(planet_rows[3 * k + axis])
                        / 2_147_483_648.0
                })
                .sum::<f64>()
                .round() as i64
        })
    };
    // The new game's seat and the one the playable parks and launches from.
    let parked = nav.parked_pad_seat(index, 4000).unwrap();
    for (name, seat) in [("new game", start.seat), ("parked", parked)] {
        let local = in_planet(seat);
        eprintln!(
            "{name} seat - port in Hope's frame {local:?}, bay {}",
            start.bay
        );
        for (got, want) in local.iter().zip([128_961i64, 203_981, 157_118]) {
            assert!((got - want).abs() <= 1, "{name}: {local:?}");
        }
    }
}

/// The launch with no key held (`--flight hover 900`; `FFE_HOVER_TRACE`): the set speed
/// stays 0, so F1013 hands the climb to 0x26 on altitude alone (frame 292 on the Ross
/// start, 315 on Hope's) and the ship climbs on F1013's fixed up-thrust (PAR-04).
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_HOVER_TRACE"]
fn port_hover_follows_the_original_without_keys() {
    let reference = trace("FFE_HOVER_TRACE");
    let last = reference.last().unwrap().frame;
    let (port, _) = replay_with(last, |_| OriginalKeys::default(), None);
    let mut changes = (Vec::new(), Vec::new());
    let mut previous = (None, None);
    let mut worst = 0i64;
    for (frame, ours) in &port {
        let Some(row) = reference.iter().find(|row| row.frame == *frame) else {
            continue;
        };
        assert_eq!(ours.set_speed, row.set_speed, "set speed at frame {frame}");
        if previous.0 != Some(row.mode) {
            changes.0.push((row.mode, *frame));
        }
        if previous.1 != Some(ours.mode) {
            changes.1.push((ours.mode, *frame));
        }
        previous = (Some(row.mode), Some(ours.mode));
        if row.mode != 0 && *frame > LAUNCH_FRAME {
            for axis in 0..3 {
                worst = worst.max((ours.velocity[axis] - row.velocity[axis]).abs());
            }
        }
    }
    eprintln!(
        "hover modes: original {:?}, port {:?}; worst velocity difference {worst}",
        changes.0, changes.1
    );
    assert_eq!(
        changes.0, changes.1,
        "the hand-off to 0x26 is on the same frame"
    );
    assert!(worst <= 6, "velocity differs by {worst}");
}

/// The original's reserve (`+0x11e`, `jjffe-smoke --flight keyboard 900` with
/// `FFE_RESERVE_DUMP=1`, stderr) against `Flight::player_events` driven by the same
/// frame clock, thrust words and mode (PAR-01). Run with `FFE_WORLD_CACHE`,
/// `FFE_KEYBOARD_TRACE` and `FFE_RESERVE_TRACE`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE, FFE_KEYBOARD_TRACE and FFE_RESERVE_TRACE"]
fn port_reserve_follows_the_original() {
    #[derive(Deserialize)]
    struct Reserve {
        reserve: u64,
        energy: u32,
    }
    let theirs: Vec<Reserve> =
        std::fs::read_to_string(std::env::var_os("FFE_RESERVE_TRACE").unwrap())
            .unwrap()
            .lines()
            .filter(|line| line.starts_with("{\"reserve\""))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    let last = trace("FFE_KEYBOARD_TRACE").last().unwrap().frame;
    let clock: Vec<KeyboardRow> =
        std::fs::read_to_string(std::env::var_os("FFE_KEYBOARD_TRACE").unwrap())
            .unwrap()
            .lines()
            .filter(|line| line.starts_with("{\"keyboard\""))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    let first = clock
        .iter()
        .find(|row| row.keyboard == LAUNCH_FRAME + 1)
        .expect("keyboard row for the launch frame");
    let (_, ours) = replay_with(
        last,
        keyboard_keys,
        Some((first.timer_last, first.timer_next)),
    );
    let mut worst = 0i64;
    for (frame, energy) in &ours {
        let Some(row) = theirs.iter().find(|row| row.reserve == *frame) else {
            continue;
        };
        let difference = i64::from(*energy) - i64::from(row.energy);
        if std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some() && difference != 0 {
            eprintln!(
                "frame {frame}: original {} port {energy} ({difference})",
                row.energy
            );
        }
        worst = worst.max(difference.abs());
    }
    eprintln!("largest reserve difference: {worst}");
    // Exact to frame 484 (every event, the 43,200-event first dispatch included);
    // after that the thrust words differ by a few units (the velocity's rounding),
    // which moves the spend by at most 6 (Hope's start: 12) of about 5,600 units.
    assert!(worst <= 12, "reserve differs by {worst}");
}

/// Firing under the reserve's events (PAR-01): `FFE_FIRE_FROM=280 FFE_RESERVE_DUMP=1
/// jjffe-smoke --flight keyboard 900` calls the real F931 on the front mount at the
/// end of every frame from 280 on; the replay fires the same way. The laser's refire
/// countdown and heat (cooled by F938's events) must match every frame, and the
/// reserve (the shots' energy and the thrust spend) as in the unfired replay.
/// Run with `FFE_WORLD_CACHE`, `FFE_FIRE_TRACE` (stderr), `FFE_KEYBOARD_TRACE`
/// (stdout) and `FFE_FIRE_FROM`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE, FFE_FIRE_TRACE, FFE_KEYBOARD_TRACE and FFE_FIRE_FROM"]
fn port_firing_follows_the_original() {
    #[derive(Deserialize)]
    struct Laser {
        reserve: u64,
        refire: u16,
        laser_heat: u16,
        energy: u32,
    }
    let theirs: Vec<Laser> = std::fs::read_to_string(std::env::var_os("FFE_FIRE_TRACE").unwrap())
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"reserve\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let last = trace("FFE_KEYBOARD_TRACE").last().unwrap().frame;
    let clock: Vec<KeyboardRow> =
        std::fs::read_to_string(std::env::var_os("FFE_KEYBOARD_TRACE").unwrap())
            .unwrap()
            .lines()
            .filter(|line| line.starts_with("{\"keyboard\""))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    let first = clock
        .iter()
        .find(|row| row.keyboard == LAUNCH_FRAME + 1)
        .expect("keyboard row for the launch frame");
    LASER.with(|l| l.borrow_mut().clear());
    replay_with(
        last,
        keyboard_keys,
        Some((first.timer_last, first.timer_next)),
    );
    let ours = LASER.with(|l| l.borrow().clone());
    let (mut worst_energy, mut shots, mut compared) = (0i64, 0, 0);
    let mut previous = u16::MAX;
    for (frame, refire, heat, energy) in &ours {
        let Some(row) = theirs.iter().find(|row| row.reserve == *frame) else {
            continue;
        };
        compared += 1;
        assert_eq!(*refire, row.refire, "refire at frame {frame}");
        assert_eq!(*heat, row.laser_heat, "laser heat at frame {frame}");
        let difference = (i64::from(*energy) - i64::from(row.energy)).abs();
        if std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some() && difference > 6 {
            eprintln!(
                "frame {frame}: original {} port {energy} ({difference})",
                row.energy
            );
        }
        worst_energy = worst_energy.max(difference);
        if *refire > previous {
            shots += 1;
        }
        previous = *refire;
    }
    eprintln!(
        "{compared} frames compared, {shots} shots, largest reserve difference {worst_energy}"
    );
    assert!(shots >= 15, "the scenario must fire repeatedly");
    // Ross: within 6. Hope: one thrust event (frame 818, 51 of the 14,000 units spent) is
    // zero in the fired original and 51 in the unfired one (the thrust word there sits
    // on a velocity-rounding edge), and the port spends 57: 0.4% of the spend.
    assert!(worst_energy <= 70, "reserve differs by {worst_energy}");
}

/// The original at an uneven frame rate (PAR-12): `FFE_CLOCK_SKIP=21 jjffe-smoke
/// --flight takeoff 900` makes every other frame 20 ms (994 tics) instead of 21
/// ms (1,043), and the replay steps each frame with the tics its row shows.
/// Run with `FFE_WORLD_CACHE` and `FFE_TAKEOFF_SKEW_TRACE`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_TAKEOFF_SKEW_TRACE"]
fn port_takeoff_replays_an_uneven_frame_rate() {
    #[derive(Deserialize)]
    struct Timed {
        frame: u64,
        tics: u32,
    }
    let path = std::env::var_os("FFE_TAKEOFF_SKEW_TRACE").expect("FFE_TAKEOFF_SKEW_TRACE");
    let text = std::fs::read_to_string(&path).unwrap();
    let timed: Vec<Timed> = text
        .lines()
        .filter(|line| line.starts_with("{\"scenario\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    TICS.with(|t| *t.borrow_mut() = timed.iter().map(|row| (row.frame, row.tics)).collect());
    let reference = trace("FFE_TAKEOFF_SKEW_TRACE");
    let last = reference.last().unwrap().frame;
    let port = replay(last);
    TICS.with(|t| t.borrow_mut().clear());
    let length = |v: [i64; 3]| v.iter().map(|c| (*c as f64).powi(2)).sum::<f64>().sqrt();
    let (mut worst_altitude, mut worst_speed, mut worst_frame) = (0.0f64, 0.0f64, 0u64);
    let mut changes = (Vec::new(), Vec::new());
    let mut previous = (None, None);
    for (frame, ours) in &port {
        let Some(row) = reference.iter().find(|row| row.frame == *frame) else {
            continue;
        };
        if previous.0 != Some(row.mode) {
            changes.0.push((row.mode, *frame));
        }
        if previous.1 != Some(ours.mode) {
            changes.1.push((ours.mode, *frame));
        }
        previous = (Some(row.mode), Some(ours.mode));
        assert_eq!(ours.set_speed, row.set_speed, "set speed at frame {frame}");
        if row.altitude > 20_000 {
            let error = (ours.altitude - row.altitude).abs() as f64 / row.altitude as f64;
            if error > worst_altitude {
                worst_frame = *frame;
            }
            worst_altitude = worst_altitude.max(error);
        }
        let speed = length(row.velocity);
        if speed > 500.0 {
            worst_speed = worst_speed.max((length(ours.velocity) - speed).abs() / speed);
        }
    }
    eprintln!("modes: original {:?}, port {:?}", changes.0, changes.1);
    eprintln!(
        "uneven frames: worst altitude difference {:.2}% (frame {}), worst speed difference {:.2}%",
        worst_altitude * 100.0,
        worst_frame,
        worst_speed * 100.0
    );
    // The altitude is F1033's, in 32-unit steps that jump at F1013's hand-off (the
    // worst frame, 263, is two frames after it); the velocity is the flight model's.
    assert!(worst_altitude < 0.025 && worst_speed < 0.005);
    for (theirs, ours) in changes.0.iter().zip(&changes.1) {
        assert!(
            theirs.0 == ours.0 && theirs.1 - ours.1.min(theirs.1) <= 1 && ours.1 <= theirs.1,
            "mode {:#x}: original frame {}, port frame {}",
            theirs.0,
            theirs.1,
            ours.1
        );
    }
}
