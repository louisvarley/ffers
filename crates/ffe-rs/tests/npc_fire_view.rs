//! An NPC's laser shot and missile are seen: F1776 draws model 0x9c at the gun of any object
//! whose gun fired (F931 sets the flag hit or miss), and F941's missile object is drawn by the
//! ship renderer.
//!
//! `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test npc_fire_view --
//! --ignored --nocapture`
use ffe_core::laser;
use ffe_core::missile::PLAYER_MISSILE;
use ffe_core::npc::{Role, ShotMark};
use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::missile::MissileField;
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::{Navigator, NEW_GAME_SHIP};
use ffe_rs::screen::{Screen, ShipBeam, SystemMissile, SystemShip};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

fn save(screen: &Screen, name: &str) {
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for pixel in screen.logical_pixels() {
            ppm.extend([(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join(name), ppm).unwrap();
    }
}

fn changed(before: &[u32], after: &[u32]) -> usize {
    before.iter().zip(after).filter(|(a, b)| a != b).count()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(v: [f64; 3]) -> [f64; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    v.map(|c| c / length)
}

fn look_at(from: [f64; 3], to: [f64; 3]) -> [[f64; 3]; 3] {
    let forward = normalise(std::array::from_fn(|i| to[i] - from[i]));
    let right = normalise(cross([0.0, 1.0, 0.0], forward));
    let up = cross(forward, right);
    [right, up, forward]
}

/// What `main` hands the screen for the ships, their shots and the missiles.
fn draw(screen: &mut Screen, nav: &Navigator, state: &Flight, viewer: &ModelViewer, beams: bool) {
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    screen.ships = nav
        .ships
        .iter()
        .map(|ship| SystemShip {
            model: ship.model,
            centre: ship.position.map(|v| v as f64 / unit),
            axes: ship.axes(),
            flash: ship.flash > 0.0,
            dead: !ship.alive(),
        })
        .collect();
    screen.ship_beams = if beams {
        nav.ships
            .iter()
            .filter(|ship| ship.visible())
            .filter_map(|ship| {
                let shot = ship.shot?;
                Some(ShipBeam {
                    model: ship.model,
                    centre: ship.position.map(|v| v as f64 / unit),
                    axes: ship.axes(),
                    code: shot.code,
                    length: shot.length,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    screen.missiles = nav
        .missiles
        .missiles
        .iter()
        .filter(|missile| missile.alive())
        .map(|missile| SystemMissile {
            model: missile.spec.model,
            centre: missile.position.map(|v| v as f64 / unit),
            axes: missile.axes,
        })
        .collect();
    screen.draw(state, ViewState::default(), Some(viewer), true, "");
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn an_npcs_laser_and_missile_are_drawn() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.respawn_traffic(&cache, at, forward);
    let index = nav
        .ships
        .iter()
        .position(|ship| ship.role == Role::Pirate)
        .expect("the capture's pirate");
    // The player 400 m from the pirate, looking at it; the pirate turned to face the player
    // (so its gun points down the view).
    let target = nav.ships[index].position;
    let from_mm = [target[0], target[1], target[2] - 400_000];
    let from = from_mm.map(|v| v as f64 / unit);
    let mut state = Flight {
        mode: Mode::Flight,
        position: from,
        system_flight: true,
        ship: NEW_GAME_SHIP,
        ..Flight::default()
    };
    state.axes = Some(look_at(from, target.map(|v| v as f64 / unit)));
    nav.ships[index].forward = [0.0, 0.0, -1.0];
    nav.ships[index].up = [0.0, 1.0, 0.0];

    // Without a shot: the ship alone.
    draw(&mut screen, &nav, &state, &viewer, true);
    let quiet = screen.logical_pixels().to_vec();
    save(&screen, "npc-quiet.ppm");

    // A pulse laser that missed (F931's `+0xe6` is 0xffff) and a beam laser that hit 400 m away.
    for (code, length, name) in [
        (laser::PULSE_1MW, ShotMark::MISS, "npc-laser-miss.ppm"),
        (
            laser::BEAM_20MW,
            ffe_rs::combat::hit_length(400_000.0),
            "npc-laser-hit.ppm",
        ),
    ] {
        nav.ships[index].shot = Some(ShotMark {
            code,
            length,
            seconds: ffe_core::npc::BEAM_SECONDS,
        });
        draw(&mut screen, &nav, &state, &viewer, true);
        save(&screen, name);
        let shot = screen.logical_pixels().to_vec();
        assert!(
            changed(&quiet, &shot) > 20,
            "the NPC's laser (code {code:#x}) did not draw"
        );
        // And it is the beam: the same frame with the shot hidden is the quiet one.
        draw(&mut screen, &nav, &state, &viewer, false);
        assert_eq!(changed(&quiet, screen.logical_pixels()), 0);
    }
    nav.ships[index].shot = None;

    // A missile an NPC launched at the player: the same renderer draws it.
    let records = cache.asset("base:model/records").unwrap();
    let stats = cache.asset("base:ship/stats").unwrap();
    let model = ffe_core::missile::model(PLAYER_MISSILE).unwrap();
    let missile_radius =
        ffe_core::objects::ModelInfo::read(records, stats, model).map_or(0, |info| info.radii().1);
    let spec =
        MissileField::spec(PLAYER_MISSILE, viewer.ship_stats(model), missile_radius).unwrap();
    let owner = nav.ships[index].id;
    nav.launch_npc_missile(
        spec,
        PLAYER_MISSILE,
        owner,
        0,
        // 25 m ahead of the eye and a little to one side, flying at the player's ship.
        [from_mm[0] + 4_000, from_mm[1] + 2_000, from_mm[2] + 25_000],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
        0,
        0,
    );
    assert_eq!(nav.missiles.missiles.len(), 1);
    draw(&mut screen, &nav, &state, &viewer, false);
    save(&screen, "npc-missile.ppm");
    let with_missile = screen.logical_pixels().to_vec();
    assert!(
        changed(&quiet, &with_missile) > 8,
        "the NPC's missile did not draw"
    );
}
