//! A launch from Lave Station must leave the slot level with it, as the game loop
//! flies it: the ship is placed at F1088's seat on the station's current rows, then
//! turns with the spinning station (F572) while the scan tests the slot.
//!
//! Local-only: needs the original game files (`FFE_GAME`, default the checkout's
//! `game` folder). `cargo test -p ffe-rs --test lave_launch_alignment -- --ignored --nocapture`

use ffe_core::objects::ModelInfo;
use ffe_rs::navigation::{landing_body, Library, Navigator, Traffic};
use ffe_rs::original_flight::{Keys, OriginalFlight};

const FRAME_SECONDS: f64 = 0.021;
const FRAME_TICS: u32 = 1043;

fn cache() -> ffe_content::VerifiedCache {
    let game = std::env::var_os("FFE_GAME").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game"),
        std::path::PathBuf::from,
    );
    ffe_content::read(&ffe_tools::cache::build_bytes(&game).unwrap()).unwrap()
}

type Rows = [[f64; 3]; 3];

/// The angle in degrees between the ship's rows and the station's rows.
fn misalignment(ship: [[i32; 3]; 3], station: Rows) -> f64 {
    // The docking stages leave the rows 0.992 of unit length: compare directions.
    let ship: Rows = ship.map(|row| {
        let row = row.map(|v| f64::from(v) / 2_147_483_648.0);
        let length = row.iter().map(|v| v * v).sum::<f64>().sqrt();
        row.map(|v| v / length)
    });
    // The trace of ship * station^T is 1 + 2 cos(angle) for a rotation.
    let trace: f64 = (0..3)
        .map(|i| (0..3).map(|k| ship[i][k] * station[i][k]).sum::<f64>())
        .sum();
    ((trace - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees()
}

#[test]
#[ignore = "Local-only: needs the original game files (FFE_GAME)"]
fn a_lave_launch_leaves_the_slot_level_with_the_station() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 3).unwrap();
    let index = nav.docked.unwrap();
    nav.begin_launch().unwrap();
    let mut done = false;
    for _ in 0..5_000 {
        nav.advance(&cache, FRAME_SECONDS);
        if nav
            .tick_traffic(FRAME_TICS, FRAME_SECONDS)
            .iter()
            .any(|e| matches!(e, Traffic::LaunchDone(_)))
        {
            done = true;
            break;
        }
    }
    assert!(done, "the launch never finished");
    let station = nav.bodies[index].clone();
    let records = cache.asset("base:model/records").unwrap();
    let raw = cache.asset("base:ship/stats").unwrap();
    let (state, _) = ffe_rs::boot::new_commander(&cache, &nav).unwrap();
    let stats = ffe_rs::boot::ship_stats(&cache, state.ship).unwrap();
    let info = ModelInfo::read(records, raw, state.ship).unwrap();
    let station_info = ModelInfo::read(records, raw, station.model).unwrap();
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
    let mut model = OriginalFlight::from_flight(&state, Some(&stats));
    model.set_collision_radius(info.radii().1);
    model.teleport(nav.launch_seat(index).unwrap());
    nav.clear_drift();
    let rows = nav.spun_rows(&cache, index).unwrap();
    model.align_to(rows);
    model.docked_rows();
    model.set_set_speed_units(0xb5d);
    let viewer = ffe_rs::model_viewer::ModelViewer::new(&cache).unwrap();
    let mut screen = ffe_rs::screen::Screen::new(&cache).ok();
    if let Some(screen) = screen.as_mut() {
        screen.system = nav.system_name().to_owned();
    }
    let mut previous: Option<Rows> = None;
    let mut worst = 0.0f64;
    let mut crashed = None;
    for frame in 0..1_500 {
        nav.advance(&cache, FRAME_SECONDS);
        let station = nav.bodies[index].clone();
        let at = station.position;
        let rows_now = nav.spun_rows(&cache, index).unwrap();
        if let Some(previous) = previous {
            let turn: Rows = std::array::from_fn(|i| {
                std::array::from_fn(|j| (0..3).map(|k| rows_now[k][i] * previous[k][j]).sum())
            });
            model.co_rotate(at, turn);
        }
        previous = Some(rows_now);
        let mut body = landing_body(&station, &station_info);
        body.orientation =
            std::array::from_fn(|k| (rows_now[k / 3][k % 3] * 2_147_483_647.0) as i32);
        let contact = model
            .step_station(
                FRAME_SECONDS,
                &Keys::default(),
                &body,
                at,
                &info,
                tools.as_ref(),
            )
            .unwrap();
        let angle = misalignment(model.player().motion.orientation, rows_now);
        worst = worst.max(angle);
        if let (Some(screen), true) = (screen.as_mut(), [0, 20, 40].contains(&frame)) {
            let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
            let tics = ffe_core::model_animation::START_TICS
                .wrapping_add((nav.clock * ffe_rs::original_flight::TICS_PER_SECOND) as u64 as u32);
            screen.stations = nav
                .bodies
                .iter()
                .enumerate()
                .filter(|(_, b)| matches!(b.kind, ffe_rs::navigation::Kind::Station))
                .map(|(i, b)| ffe_rs::screen::SystemStation {
                    surface: None,
                    model: b.model,
                    centre: b.position.map(|v| v as f64 / unit),
                    orientation: b.orientation,
                    speed: b.spin,
                    door: nav.door(i),
                    tics,
                    seed: b.seed,
                })
                .collect();
            let mut shown = state.clone();
            model.apply(&mut shown);
            shown.system_flight = true;
            shown.mode = ffe_rs::flight::Mode::Flight;
            screen.draw(
                &shown,
                ffe_rs::camera::ViewState::default(),
                Some(&viewer),
                true,
                "",
            );
            if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
                let mut ppm = b"P6\n320 200\n255\n".to_vec();
                for p in screen.logical_pixels() {
                    ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
                }
                std::fs::write(
                    std::path::Path::new(&dir).join(format!("lave-exit-{frame:03}.ppm")),
                    ppm,
                )
                .unwrap();
            }
        }
        if frame % 25 == 0 {
            eprintln!(
                "frame {frame}: misaligned {angle:.2} deg, flags {:#x}",
                contact.flags
            );
        }
        if contact.flags & 0x80 != 0 && crashed.is_none() {
            crashed = Some((frame, angle));
        }
        if contact.flags & 0x20 == 0 {
            break;
        }
    }
    eprintln!("worst misalignment {worst:.2} deg, crash {crashed:?}");
    assert!(crashed.is_none(), "hull strike {crashed:?}");
    assert!(worst < 1.0, "the ship drifts off the station's rows");
}
