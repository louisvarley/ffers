//! Local-only replay of the executed original's launch from an orbital station
//! (`jjffe-smoke --start-key=0x04 --flight launch 1200`, the Lave Station start,
//! F381's Launch Request row at frame 170) against the port's `Navigator`
//! (PAR-10).
//!
//! The original's modes: 0x2a docked, then the lift stages 0x1a, 0x1c, 0x1e and
//! 0x20, the door's 0x22, F1088's 0x36 (the ship parked at (0, 0, 0x10000) in the
//! station's frame) and F1089's 0x26 once the door word is 0. Every frame is 21 ms
//! (1,043 tics at 1x).
//!
//! Run with `FFE_WORLD_CACHE=<data.bin> FFE_LAUNCH_TRACE=<launch.jsonl>`.
use ffe_rs::navigation::Navigator;
use serde::Deserialize;

const LAUNCH_FRAME: u64 = 170;
const FRAME_SECONDS: f64 = 0.021;
const FRAME_TICS: u32 = 1043;

#[derive(Deserialize)]
struct Row {
    frame: u64,
    mode: u8,
    position: [i64; 3],
}

fn reference() -> Vec<Row> {
    let path = std::env::var_os("FFE_LAUNCH_TRACE").expect("FFE_LAUNCH_TRACE");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"scenario\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_LAUNCH_TRACE"]
fn the_launch_stages_follow_the_original_frame_by_frame() {
    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let reference = reference();
    let mut nav = Navigator::new_game(&cache, 3).unwrap();
    nav.begin_launch().unwrap();
    // The original's mode at each frame from the launch frame on.
    let theirs: Vec<(u64, u8)> = reference
        .iter()
        .filter(|row| row.frame >= LAUNCH_FRAME)
        .map(|row| (row.frame, row.mode))
        .collect();
    let mut changes_theirs = Vec::new();
    let mut previous = None;
    for (frame, mode) in &theirs {
        if previous != Some(*mode) {
            changes_theirs.push((*mode, *frame));
        }
        previous = Some(*mode);
    }
    // The port's stage state after each frame's F1088-style tick; the frame of
    // the launch request itself is LAUNCH_FRAME (the row there already shows 0x1a).
    let mut changes_ours = vec![(
        nav.sequence.expect("launch begun").stages.state,
        LAUNCH_FRAME,
    )];
    let mut released = None;
    for frame in LAUNCH_FRAME + 1..=theirs.last().unwrap().0 {
        let events = nav.tick_traffic(FRAME_TICS, FRAME_SECONDS);
        if let Some(sequence) = nav.sequence {
            let state = sequence.stages.state;
            if changes_ours.last().map(|c| c.0) != Some(state) {
                changes_ours.push((state, frame));
            }
        }
        if released.is_none()
            && events
                .iter()
                .any(|e| matches!(e, ffe_rs::navigation::Traffic::LaunchDone(_)))
        {
            released = Some(frame);
        }
    }
    // Every stage change lands on the frame the original's does, and the door's
    // release (F1089's 0x26) is the frame the original's mode becomes 0x26.
    let stage_changes: Vec<(u8, u64)> = changes_theirs
        .iter()
        .copied()
        .filter(|c| (0x1a..=0x36).contains(&c.0) && c.0 != 0x26)
        .collect();
    assert_eq!(stage_changes, changes_ours, "stage changes");
    assert_eq!(
        released,
        changes_theirs.iter().find(|c| c.0 == 0x26).map(|c| c.1)
    );
    eprintln!("original: {changes_theirs:x?}");
    eprintln!("port:     {changes_ours:x?}, released at {released:?}");
    let seat = reference.iter().find(|row| row.mode == 0x36).unwrap();
    eprintln!("original's seat at 0x36: {:?}", seat.position);
    // The stage that matters for the comparison is each change's frame.
    for ((mode, theirs_frame), (state, ours_frame)) in changes_theirs
        .iter()
        .filter(|c| (0x1a..=0x36).contains(&c.0))
        .zip(&changes_ours)
    {
        eprintln!(
            "mode {mode:#x}: original frame {theirs_frame}, port state {state:#x} frame {ours_frame}"
        );
    }
}

/// The ship's position and velocity in the station's rotating frame while it
/// leaves (modes 0x26, frames 861 to 1089): F1088 parks it at (0, 0, 0x10000), the
/// set speed is 0xb5d and F1037/F1041/F601 fly it out of the slot.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_LAUNCH_TRACE"]
fn the_exit_flight_follows_the_original_frame_by_frame() {
    use ffe_rs::navigation::{landing_body, LAUNCH_SEAT};
    use ffe_rs::original_flight::{Keys, OriginalFlight};
    #[derive(Deserialize)]
    struct Pose {
        rows: u64,
        mode: u8,
        pos: [i64; 3],
        v: [i32; 3],
    }
    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let path = std::env::var_os("FFE_LAUNCH_POSES").expect("FFE_LAUNCH_POSES");
    let poses: Vec<Pose> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"rows\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let release = poses.iter().find(|p| p.mode == 0x26).unwrap().rows;
    let nav = Navigator::new_game(&cache, 3).unwrap();
    let index = nav.docked.unwrap();
    let station = nav.bodies[index].clone();
    let rows = nav.spun_rows(&cache, index).unwrap_or(station.orientation);
    let (state, _) = ffe_rs::boot::new_commander(&cache, &nav).unwrap();
    let stats = ffe_rs::boot::ship_stats(&cache, state.ship).unwrap();
    let records = cache.asset("base:model/records").unwrap();
    let raw = cache.asset("base:ship/stats").unwrap();
    let info = ffe_core::objects::ModelInfo::read(records, raw, state.ship).unwrap();
    let station_body = landing_body(
        &station,
        &ffe_core::objects::ModelInfo::read(records, raw, station.model).unwrap(),
    );
    let library = ffe_rs::navigation::Library(&cache);
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
    let mut model = OriginalFlight::from_flight(&state, Some(&stats));
    model.set_collision_radius(info.radii().1);
    model.teleport(nav.launch_seat(index).unwrap());
    model.align_to(rows);
    model.docked_rows();
    model.set_set_speed_units(0xb5d);
    let local = |v: [f64; 3]| -> [f64; 3] {
        std::array::from_fn(|k| {
            (0..3)
                .map(|j| v[j] * f64::from(station.rows[3 * k + j]) / 2_147_483_648.0)
                .sum::<f64>()
        })
    };
    let mut worst = (0.0f64, 0.0f64);
    for pose in poses.iter().filter(|p| p.rows > release && p.mode == 0x26) {
        let result = model.step_station(
            FRAME_SECONDS,
            &Keys::default(),
            &station_body,
            station.position,
            &info,
            tools.as_ref(),
        );
        assert!(result.is_ok(), "frame {}: {result:?}", pose.rows);
        let at = model.absolute_position();
        let relative: [f64; 3] = std::array::from_fn(|k| (at[k] - station.position[k]) as f64);
        let ours = local(relative);
        let velocity = local(model.player().motion.velocity.map(f64::from));
        if std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some() {
            eprintln!(
                "frame {}: original z {} v {:?}; port z {:.0} v {:.0?}; thrust {:?} limits {:?}",
                pose.rows,
                pose.pos[2],
                pose.v,
                ours[2],
                velocity,
                model.thrust_words(),
                model.player().limits
            );
        }
        let along =
            (ours[2] - pose.pos[2] as f64).abs() / (pose.pos[2] as f64 - 65_536.0).max(1_000.0);
        let speed = (velocity[2] - f64::from(pose.v[2])).abs() / f64::from(pose.v[2]).max(100.0);
        worst = (worst.0.max(along), worst.1.max(speed));
    }
    assert!(worst.0 < 0.02 && worst.1 < 0.02, "exit flight differs");
    eprintln!(
        "exit flight: worst distance error {:.2}% of the way out, worst speed error {:.2}%, seat {LAUNCH_SEAT:#x}",
        worst.0 * 100.0,
        worst.1 * 100.0
    );
}

/// The docking stages after the autopilot's touch (`jjffe-smoke --start-key=0x04
/// --flight dock 12000`: the launch, then F1020's autopilot back to Lave Station):
/// the original's modes 0xa, 0xc, 0xe, 0x10, 0x12, 0x14, 0x16, 0x18 and then 0x28/0x2a
/// begin on the frames `ffe_core::traffic::Stages` reaches them from the touch
/// (PAR-09). Run with `FFE_DOCK_TRACE`.
#[test]
#[ignore = "Local-only: needs FFE_DOCK_TRACE"]
fn the_docking_stages_follow_the_original_frame_by_frame() {
    use ffe_core::traffic::Stages;
    let path = std::env::var_os("FFE_DOCK_TRACE").expect("FFE_DOCK_TRACE");
    let rows: Vec<Row> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"scenario\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let touch = rows
        .iter()
        .find(|row| row.mode == 0xa)
        .expect("the original docks")
        .frame;
    let mut theirs: Vec<(u8, u64)> = Vec::new();
    for row in rows.iter().filter(|row| row.frame >= touch) {
        if theirs.last().map(|c| c.0) != Some(row.mode) {
            theirs.push((row.mode, row.frame));
        }
    }
    let mut stages = Stages::touched(false);
    let mut ours = vec![(stages.state, touch)];
    for frame in touch + 1..=rows.last().unwrap().frame {
        if stages.step(FRAME_TICS as i32, false, 0xffff).is_some()
            && ours.last().map(|c| c.0) != Some(stages.state)
        {
            ours.push((stages.state, frame));
        }
    }
    eprintln!("original: {theirs:x?}");
    eprintln!("port:     {ours:x?}");
    // Every stage from the touch to the dock changes on the original's frame.
    assert_eq!(theirs.len(), ours.len());
    for ((mode, theirs_frame), (state, ours_frame)) in theirs.iter().zip(&ours) {
        assert_eq!(
            (mode, theirs_frame),
            (state, ours_frame),
            "stage {mode:#x}: original frame {theirs_frame}, port {state:#x} frame {ours_frame}"
        );
    }
}

/// The approach leg (PAR-11): the original's autopilot, engaged on frame 1090 with the
/// ship 1,039,351 mm out along the station's axis, flies out, turns round and comes in
/// to the clearance point (modes 2, frames 1090 to 7660). Start the port's
/// `Navigator::source_command` from the original's pose and compare every frame.
///
/// Run with `FFE_WORLD_CACHE=<data.bin> FFE_DOCK_POSES=<dock-poses.jsonl>`.
#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE and FFE_DOCK_POSES"]
fn the_autopilot_approach_follows_the_original_frame_by_frame() {
    use ffe_rs::navigation::{landing_body, PLAYER};
    use ffe_rs::original_flight::{Keys, OriginalFlight};
    #[derive(Deserialize)]
    struct Pose {
        rows: u64,
        mode: u8,
        pos: [i64; 3],
        v: [i32; 3],
        r: [i32; 9],
        thrust: [i16; 3],
    }
    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let path = std::env::var_os("FFE_DOCK_POSES").expect("FFE_DOCK_POSES");
    let poses: Vec<Pose> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"rows\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let start = poses.iter().find(|p| p.mode == 2).unwrap();
    let mut nav = Navigator::new_game(&cache, 3).unwrap();
    let index = nav.docked.unwrap();
    // The first advance repositions the bodies for the world clock; do it before the ship is placed.
    nav.advance(&cache, 1.0);
    let records = cache.asset("base:model/records").unwrap();
    let raw = cache.asset("base:ship/stats").unwrap();
    let (state, _) = ffe_rs::boot::new_commander(&cache, &nav).unwrap();
    let stats = ffe_rs::boot::ship_stats(&cache, state.ship).unwrap();
    let info = ffe_core::objects::ModelInfo::read(records, raw, state.ship).unwrap();
    let station = nav.bodies[index].clone();
    let station_body = landing_body(
        &station,
        &ffe_core::objects::ModelInfo::read(records, raw, station.model).unwrap(),
    );
    let library = ffe_rs::navigation::Library(&cache);
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
    let mut model = OriginalFlight::from_flight(&state, Some(&stats));
    model.set_collision_radius(info.radii().1);
    let axis = station.orientation[2];
    let at = |z: f64| -> [i64; 3] {
        std::array::from_fn(|k| station.position[k] + (axis[k] * z).round() as i64)
    };
    model.place(at(start.pos[2] as f64), start.r);
    model.leave_hyperspace(start.v);
    nav.starport(index).release(PLAYER);
    nav.docked = None;
    nav.lock(index);
    nav.engage_locked().unwrap();
    let local = |nav: &Navigator, p: [i64; 3]| -> f64 {
        let now = &nav.bodies[index];
        (0..3)
            .map(|k| (p[k] - now.position[k]) as f64 * now.orientation[2][k])
            .sum()
    };
    let verbose = std::env::var_os("FFE_TAKEOFF_VERBOSE").is_some();
    // F152's whole-second events: the new game's first dispatch is frame 162, then
    // every 1,043-tic frame adds to the day fraction. F1007 runs the AI states (F741..F746)
    // from the event, after the frame's flight step: the thrust it sets first acts on the
    // next frame.
    let mut events = ffe_core::second_events::SecondEvents::new_game();
    let mut event_frames = std::collections::BTreeSet::new();
    events.advance(FRAME_TICS);
    for frame in 163..=poses.last().unwrap().rows {
        if events.advance(FRAME_TICS) != 0 {
            event_frames.insert(frame);
        }
    }
    // F1020 only sets the state: the thrust words the launch flight left stay until the first event.
    model.apply_guidance(ffe_core::autopilot::Guidance {
        thrust: start.thrust,
        cap: [0; 3],
        turns: [0; 3],
        distance_shift: 0,
    });
    let mut worst = 0.0f64;
    let mut ended = None;
    for pose in poses.iter().filter(|p| p.rows >= start.rows && p.mode == 2) {
        let ours = local(&nav, model.absolute_position());
        // The error against the farthest point of the leg (4.1 km out).
        let error = (ours - pose.pos[2] as f64).abs() / 4_100_000.0;
        worst = worst.max(error);
        if verbose && (pose.rows % 100 == 0 || error > 0.005) {
            eprintln!(
                "frame {}: original z {} v {} thrust {:?}; port z {:.0} v {} thrust {:?}; error {:.2}%",
                pose.rows, pose.pos[2], pose.v[2], pose.thrust, ours, model.player().motion.velocity[2],
                model.thrust_words(), error * 100.0
            );
        }
        nav.advance(&cache, FRAME_SECONDS);
        let moved = nav.follow(model.absolute_position());
        if let Some(moved) = moved {
            model.shift(moved);
        }
        if event_frames.contains(&pose.rows) {
            let Some((guidance, _)) = nav.source_command(&cache, &mut model) else {
                ended = Some(pose.rows);
                break;
            };
            if verbose && pose.rows < 1700 {
                eprintln!("event {}: guidance {guidance:?}", pose.rows);
            }
            model.apply_guidance(guidance);
        }
        let result = model.step_station(
            FRAME_SECONDS,
            &Keys::default(),
            &station_body,
            station.position,
            &info,
            tools.as_ref(),
        );
        assert!(result.is_ok(), "frame {}: {result:?}", pose.rows);
    }
    eprintln!(
        "approach: worst error {:.3}% of the 4.1 km leg over {} frames",
        worst * 100.0,
        poses.len()
    );
    assert!(ended.is_none(), "the port's autopilot ended at {ended:?}");
    assert!(worst < 0.005, "the approach differs from the original");
}
