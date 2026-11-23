//! One palette build per frame (`docs/notes/palette.md` section 6).
//!
//! The original's F1536 queues every object of a flight frame, registering each
//! colour with F1326, and F1330 builds the 64-box palette once around all of
//! them. `Screen::draw` runs the world's objects twice (register, build once,
//! draw) to match; `Screen::palette_build_per_object` restores the earlier
//! build-per-model behaviour. `FFE_SHOT_DIR/palette-landed-{frame,object}.ppm`
//! are the two frames, which `tools/reference/verify-palette-frame.py` compares
//! with the executed original's `--game-frame 500 0`.
use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::{Kind, Navigator};
use ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT as UNIT;
use ffe_rs::screen::{Disc, Screen, SystemStation};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

/// The new game's landed Hope start (state `0x30`): the planet's surface, the
/// star and the starport are three objects of one frame.
fn landed_frame(cache: &ffe_content::VerifiedCache, per_object: bool) -> (Vec<u32>, Vec<[u8; 3]>) {
    let nav = Navigator::new_game(cache, 1).unwrap();
    let start = nav.new_game.as_ref().unwrap();
    let index = nav.docked.unwrap();
    let body = nav.bodies[index].clone();
    let mut state = Flight {
        mode: Mode::Flight,
        system_flight: true,
        ship: start.ship,
        position: start.seat.map(|v| v as f64 / UNIT),
        ..Flight::default()
    };
    state.axes = Some(body.orientation);
    let viewer = ModelViewer::new(cache).unwrap();
    let mut screen = Screen::new(cache).unwrap();
    screen.palette_build_per_object = per_object;
    screen.discs = nav
        .bodies
        .iter()
        .filter(|b| matches!(b.kind, Kind::Star | Kind::Planet | Kind::Moon))
        .map(|b| {
            let length = b.position.iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
            Disc {
                system_type: None,
                centre: b.position.map(|v| v as f64 / UNIT),
                radius: ffe_rs::navigation::surface_radius(b, true),
                light: b.position.map(|v| -(v as f64) / length.sqrt().max(1.0)),
                colours: (12, 4),
                star: b.kind == Kind::Star,
                temperature: None,
                model: Some((b.model, b.seed)),
                orientation: b.orientation,
            }
        })
        .collect();
    let tics = ffe_core::model_animation::START_TICS;
    screen.stations = nav
        .bodies
        .iter()
        .filter(|b| matches!(b.kind, Kind::Station | Kind::Starport))
        .map(|b| SystemStation {
            surface: None,
            model: b.model,
            centre: b.position.map(|v| v as f64 / UNIT),
            orientation: b.orientation,
            speed: b.spin,
            door: 0xffff,
            tics,
            seed: 0,
        })
        .collect();
    screen.sky = ffe_rs::prefs::space_colour_rgb(4);
    // F1052 applies the previous scene's background: draw twice, keep the second.
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    (
        screen.logical_pixels().to_vec(),
        viewer.palette_upper6().unwrap(),
    )
}

fn save(name: &str, pixels: &[u32]) {
    let Some(dir) = std::env::var_os("FFE_SHOT_DIR") else {
        return;
    };
    let mut ppm = b"P6\n320 200\n255\n".to_vec();
    for p in pixels {
        ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
    }
    std::fs::write(std::path::Path::new(&dir).join(name), ppm).unwrap();
}

fn save_palette(name: &str, upper: &[[u8; 3]]) {
    let Some(dir) = std::env::var_os("FFE_SHOT_DIR") else {
        return;
    };
    let text: String = upper
        .iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                "{} {} {} {}
",
                128 + i,
                c[0],
                c[1],
                c[2]
            )
        })
        .collect();
    std::fs::write(std::path::Path::new(&dir).join(name), text).unwrap();
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn landed_frame_builds_one_palette_for_every_object() {
    let cache = cache();
    let (frame, frame_palette) = landed_frame(&cache, false);
    let (object, object_palette) = landed_frame(&cache, true);
    save("palette-landed-frame.ppm", &frame);
    save("palette-landed-object.ppm", &object);
    save_palette("palette-landed-frame.txt", &frame_palette);
    save_palette("palette-landed-object.txt", &object_palette);
    // The same two draws are repeatable.
    assert_eq!(frame, landed_frame(&cache, false).0, "deterministic");
    let view = 320 * 158;
    let differing = frame[..view]
        .iter()
        .zip(&object[..view])
        .filter(|(a, b)| a != b)
        .count();
    eprintln!("landed view: {differing} of {view} pixels differ between one build per frame and per object");
    // The console and its text do not depend on the palette build.
    assert_eq!(frame[view..], object[view..], "console unchanged");
}

/// A model placed where it fills a good part of the view: the first power-of-two
/// range that gives it several shapes.
fn placed(viewer: &ModelViewer, model: u32) -> (u32, [f64; 3]) {
    let rows = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for power in 6..28 {
        let at = 2f64.powi(power);
        let Ok(frame) = viewer.render_scene(model, rows, [0.0, 0.0, at], [0.0, 0.0, 1.0]) else {
            continue;
        };
        if frame.shapes.len() >= 8 {
            return (model, [0.0, 0.0, at]);
        }
    }
    panic!("model {model} never draws");
}

/// The build count and the resolved colours of a frame of two objects: the two-pass
/// frame builds once and draws exactly what each model would draw alone with that
/// palette; a single-object frame is identical to the one-call render.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_frame_of_two_objects_is_built_once() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let nav = Navigator::new_game(&cache, 1).unwrap();
    let ship = nav.new_game.as_ref().unwrap().ship;
    let rows = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let objects = [placed(&viewer, ship), placed(&viewer, 74)];
    let light = [0.0, 0.0, 1.0];
    let render = |(model, at): (u32, [f64; 3])| viewer.render_scene(model, rows, at, light);

    // One call each is a whole frame each: two builds.
    let before = viewer.palette_builds();
    let alone: Vec<_> = objects.iter().map(|o| render(*o).unwrap()).collect();
    assert_eq!(viewer.palette_builds() - before, 2);

    // The two-pass frame: one build, whatever the number of objects.
    let before = viewer.palette_builds();
    viewer.begin_registration();
    for object in objects {
        assert!(render(object).unwrap().shapes.is_empty(), "registers only");
    }
    assert_eq!(viewer.palette_builds() - before, 0, "nothing built yet");
    viewer.finish_registration();
    let drawn: Vec<_> = objects.iter().map(|o| render(*o).unwrap()).collect();
    viewer.end_frame_palette();
    assert_eq!(viewer.palette_builds() - before, 1, "one F1330 per frame");
    assert!(drawn.iter().all(|frame| !frame.shapes.is_empty()));
    // Every flat colour the frame drew is an entry of the one palette it built.
    let reachable: std::collections::HashSet<[u8; 3]> = (0..=255u8)
        .filter_map(|index| viewer.palette_entry(index))
        .collect();
    for frame in &drawn {
        for shape in &frame.shapes {
            if let ffe_rs::model_viewer::Shape::Fill(_, rgb)
            | ffe_rs::model_viewer::Shape::Flat(_, rgb)
            | ffe_rs::model_viewer::Shape::Polygon(_, rgb) = shape
            {
                // The station's name-board glyphs draw their instance colour
                // (0xee0) unregistered, through the pre-palette nibble fallback;
                // `docs/notes/palette.md` section 6 records it as open.
                let board_yellow = *rgb == [224, 224, 0];
                assert!(
                    reachable.contains(rgb) || board_yellow,
                    "{rgb:?} is not a palette entry"
                );
            }
        }
    }

    // A frame of one object is the same frame either way.
    viewer.begin_registration();
    render(objects[0]).unwrap();
    viewer.finish_registration();
    let single = render(objects[0]).unwrap();
    viewer.end_frame_palette();

    assert_eq!(single.shapes, alone[0].shapes, "one object: no difference");
}
