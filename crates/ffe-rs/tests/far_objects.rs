//! Far bodies in the flight view: F1680 draws nothing past a model's far class and a dot from
//! its dot class (F1700 with the object flag: a plus-shaped glint for a lit planet or moon),
//! where the port used to draw the full model a hundredth of a pixel across, which is nothing.
//! The original shows Gateway's other planets this way from Hope.
//!
//! `FFE_WORLD_CACHE=<data.bin> cargo test -p ffe-rs --test far_objects -- --ignored --nocapture`
//! (`FFE_SHOT_DIR=<dir>` also writes each frame).

use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::{Kind, Navigator};
use ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT as UNIT;
use ffe_rs::screen::{Disc, Screen};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

/// The descriptor fields as the floppy 1.02 executable has them (read straight from the file):
/// planets 134/135/136 and moon-sized 121, and the unlit star models.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_descriptors_carry_the_executables_far_and_dot_classes() {
    let viewer = ModelViewer::new(&cache()).unwrap();
    let wanted = [
        // model, far, dot, colour, unlit
        (121, 39, 25, [6, 6, 6], false),
        (131, 40, 25, [6, 8, 6], false),
        (134, 45, 31, [8, 6, 4], false),
        (135, 41, 27, [6, 6, 8], false),
        (137, 50, 34, [0xf, 0, 0], true),
        (139, 69, 38, [0xf, 7, 4], true),
        (31, 19, 10, [6, 6, 6], false),
    ];
    for (model, far, dot, colour, unlit) in wanted {
        let found = viewer.object_model(model).expect("a descriptor");
        assert_eq!(
            (found.far, found.dot, found.colour, found.unlit),
            (far, dot, colour, unlit),
            "model {model}"
        );
    }
}

/// From Hope, looking at each of Gateway's other planets, the planet is a few pixels at the
/// view centre (a glint, orange for a lit body); Gateway 7 is past its far class and is not drawn.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn gateways_far_planets_are_glints_from_hope() {
    let cache = cache();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let hope = nav.bodies.iter().find(|b| b.name == "Hope").unwrap();
    let discs: Vec<Disc> = nav
        .bodies
        .iter()
        .filter(|b| matches!(b.kind, Kind::Star | Kind::Planet | Kind::Moon))
        .map(|b| {
            let length = b.position.iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
            Disc {
                system_type: Some(b.system_type),
                centre: b.position.map(|v| v as f64 / UNIT),
                radius: b.radius as f64 / UNIT,
                light: b.position.map(|v| (v as f64) / length.sqrt().max(1.0)),
                colours: match b.kind {
                    Kind::Star => (17, 17),
                    Kind::Planet => (12, 4),
                    _ => (10, 3),
                },
                star: b.kind == Kind::Star,
                temperature: None,
                model: Some((b.model, b.seed)),
                orientation: b.orientation,
            }
        })
        .collect();
    let render = |screen: &mut Screen, state: &Flight, discs: Vec<Disc>| {
        screen.discs = discs;
        screen.draw(state, ViewState::default(), Some(&viewer), false, "");
        screen.logical_pixels().to_vec()
    };
    for (name, drawn) in [
        ("Gateway 1", true),
        ("Gateway 2", true),
        ("Gateway 4", true),
        ("Gateway 5", true),
        ("Gateway 6", true),
        ("Gateway 7", false),
    ] {
        let target = nav.bodies.iter().find(|b| b.name == name).unwrap();
        let from = hope.position.map(|v| v as f64 / UNIT);
        let to = target.position.map(|v| v as f64 / UNIT);
        let mut z: [f64; 3] = std::array::from_fn(|k| to[k] - from[k]);
        let n = z.iter().map(|v| v * v).sum::<f64>().sqrt();
        z = z.map(|v| v / n);
        let up = if z[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let mut x = [
            up[1] * z[2] - up[2] * z[1],
            up[2] * z[0] - up[0] * z[2],
            up[0] * z[1] - up[1] * z[0],
        ];
        let xn = x.iter().map(|v| v * v).sum::<f64>().sqrt();
        x = x.map(|v| v / xn);
        let y = [
            z[1] * x[2] - z[2] * x[1],
            z[2] * x[0] - z[0] * x[2],
            z[0] * x[1] - z[1] * x[0],
        ];
        let mut state = Flight {
            mode: Mode::Flight,
            ..Flight::default()
        };
        state.axes = Some([x, y, z]);
        let radius = hope.radius as f64 / UNIT;
        state.position = std::array::from_fn(|k| from[k] + z[k] * 20.0 * radius);
        let with = render(&mut screen, &state, discs.clone());
        let without = render(
            &mut screen,
            &state,
            discs.iter().filter(|d| d.centre != to).cloned().collect(),
        );
        let changed: Vec<(usize, usize)> = (0..158usize)
            .flat_map(|y| (0..320usize).map(move |x| (x, y)))
            .filter(|(x, y)| with[y * 320 + x] != without[y * 320 + x])
            .collect();
        println!("{name}: {} pixels", changed.len());
        if drawn {
            // A few pixels (the glint's arms), all within a handful of pixels of the centre.
            assert!(
                (3..=40).contains(&changed.len()),
                "{name}: {} pixels",
                changed.len()
            );
            assert!(
                changed
                    .iter()
                    .all(|(x, y)| (x.abs_diff(160)) <= 8 && (y.abs_diff(79)) <= 8),
                "{name}: {changed:?}"
            );
        } else {
            assert!(
                changed.is_empty(),
                "{name} is past its far class: {changed:?}"
            );
        }
        if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
            let mut ppm = b"P6\n320 200\n255\n".to_vec();
            for p in &with {
                ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            let file = format!("far-{}.ppm", name.replace(' ', "-").to_lowercase());
            std::fs::write(std::path::Path::new(&dir).join(file), ppm).unwrap();
        }
    }
}

/// F1680 runs for every object of the flight view (F1536/F1537), not only the
/// planets: a station is a model at docking range, an F1700 glint from its model's
/// dot class and absent past its far class, and a ship likewise. Station model 74's
/// classes are 16 and 24 (537 km and 137,439 km); ship model 31's are 10 and 19
/// (8.39 km and 4,295 km). The glint is a few pixels at the view centre; nothing is
/// drawn beyond the far class.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn ships_and_stations_transition_to_glints_at_their_dot_classes() {
    use ffe_rs::screen::{SystemShip, SystemStation};
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let mut state = Flight {
        mode: Mode::Flight,
        system_flight: true,
        ..Flight::default()
    };
    state.axes = Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    let render = |screen: &mut Screen| {
        screen.draw(&state, ViewState::default(), Some(&viewer), false, "");
        screen.logical_pixels().to_vec()
    };
    let changed = |before: &[u32], after: &[u32]| -> Vec<(usize, usize)> {
        (0..158usize)
            .flat_map(|y| (0..320usize).map(move |x| (x, y)))
            .filter(|(x, y)| before[y * 320 + x] != after[y * 320 + x])
            .collect()
    };
    // The space colour is index 0, so a non-black sky does not make a dark hull
    // visible: the model case is a station, which fills the view at 2 km.
    screen.sky = 0x101820;
    let empty = render(&mut screen);

    let station = SystemStation {
        surface: None,
        model: 74,
        centre: [0.0, 0.0, 2_000_000.0 / UNIT],
        orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        speed: 0,
        door: 0,
        tics: 0,
        seed: 0,
    };
    screen.stations = vec![station];
    let model = changed(&empty, &render(&mut screen));
    println!("station at 2 km: {} pixels", model.len());
    assert!(
        model.len() > 1000,
        "the station model did not draw: {}",
        model.len()
    );

    // 600 km: past dot class 16, so F1700's glint at the view centre.
    screen.stations[0].centre = [0.0, 0.0, 600_000_000.0 / UNIT];
    let glint = changed(&empty, &render(&mut screen));
    println!("station at 600 km: {} pixels", glint.len());
    assert!(
        (1..=40).contains(&glint.len()),
        "the station's glint: {} pixels",
        glint.len()
    );
    assert!(
        glint
            .iter()
            .all(|(x, y)| x.abs_diff(160) <= 8 && y.abs_diff(79) <= 8),
        "{glint:?}"
    );

    // 140,000 km: past far class 24, nothing.
    screen.stations[0].centre = [0.0, 0.0, 140_000_000_000.0 / UNIT];
    assert!(
        changed(&empty, &render(&mut screen)).is_empty(),
        "the station past its far class still drew"
    );

    // A ship at 9 km: past model 31's dot class 10 (8.39 km), so the glint.
    screen.stations.clear();
    screen.ships = vec![SystemShip {
        model: 31,
        centre: [0.0, 0.0, 9_000_000.0 / UNIT],
        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        flash: false,
        dead: false,
    }];
    let ship_glint = changed(&empty, &render(&mut screen));
    println!("ship at 9 km: {} pixels", ship_glint.len());
    assert!(
        (1..=40).contains(&ship_glint.len()),
        "the ship's glint: {} pixels",
        ship_glint.len()
    );
    assert!(
        ship_glint
            .iter()
            .all(|(x, y)| x.abs_diff(160) <= 8 && y.abs_diff(79) <= 8),
        "{ship_glint:?}"
    );

    // 5,000 km: past far class 19, nothing.
    screen.ships[0].centre = [0.0, 0.0, 5_000_000_000.0 / UNIT];
    assert!(
        changed(&empty, &render(&mut screen)).is_empty(),
        "the ship past its far class still drew"
    );
}

/// F1700's only unported far path is a *lit* body nine pixels or more across at its
/// dot class (a shaded sphere drawn by polygons). It is unreachable for system
/// bodies: their dot class is where the body is about a pixel across, so the
/// boundary case is the largest dot. This walks every body of every start system at
/// its own dot-class distance and checks the size F1680 would hand F1700.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn no_system_body_is_a_nine_pixel_sphere_at_its_dot_class() {
    use ffe_core::model_animation::{setup_object, Camera as ObjectCamera, Draw, ObjectState};
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut checked = 0;
    let mut biggest = 0;
    for start in 0..4 {
        let Ok(nav) = Navigator::new_game(&cache, start) else {
            continue;
        };
        for body in &nav.bodies {
            if !matches!(body.kind, Kind::Star | Kind::Planet | Kind::Moon) {
                continue;
            }
            let Some(descriptor) = viewer.object_descriptor(body.model) else {
                continue;
            };
            // The dot class is the distance class `msb - 0xd`, so its first distance
            // is 2^(class + 13) mm; placing the camera there puts the body at the
            // boundary where the dot is largest.
            let boundary_mm = 2f64.powi(descriptor.dot + 13);
            let camera = ObjectCamera {
                matrix: [
                    2_147_483_647,
                    0,
                    0, //
                    0,
                    2_147_483_647,
                    0, //
                    0,
                    0,
                    2_147_483_647,
                ],
                position: [0, 0, -(boundary_mm as i64)],
            };
            let object = ObjectState::default();
            match setup_object(&object, &camera, &descriptor, [0; 3], &[[0; 3]; 8]) {
                Draw::Dot { radius, depth, .. } => {
                    let size = ffe_core::model_thruster::special_divide(radius, depth).unwrap();
                    biggest = biggest.max(size);
                    assert!(
                        size < 9,
                        "{} (model {}) is {size} px at its dot class {}",
                        body.name,
                        body.model,
                        descriptor.dot
                    );
                    checked += 1;
                }
                other => panic!("{} at its dot class: {other:?}", body.name),
            }
        }
    }
    println!("{checked} bodies checked, largest dot {biggest} px");
    assert!(checked > 0, "no body reached a dot");
}
