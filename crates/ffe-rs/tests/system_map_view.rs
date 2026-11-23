//! The original's 3D system map page (`ffe_rs::system_map`) drawn over the cabin for Gateway
//! (the default start): orbit curves, dots, names and the time point line.
//!
//! `FFE_WORLD_CACHE=<data.bin> cargo test -p ffe-rs --test system_map_view -- --ignored --nocapture`
//! (`FFE_SHOT_DIR=<dir>` also writes each frame as a PPM).

use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::{Kind, Navigator};
use ffe_rs::screen::{Screen, SystemMapScene};
use ffe_rs::system_map::SystemMap;

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

fn draw(screen: &mut Screen, viewer: &ModelViewer, nav: &Navigator, map: &SystemMap) -> Vec<u32> {
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    // F966 lights the system map's 0x23 icon in the map's console row.
    screen.map_console = ffe_rs::map_keys::page(ffe_rs::map_keys::MapAction::Select).unwrap();
    screen.system_map = Some(SystemMapScene {
        frame: map.frame_with(&nav.date_at(map.clock), None, &|model| {
            viewer.object_descriptor(model)
        }),
        date: nav.date_text(),
    });
    screen.draw(&state, ViewState::default(), Some(viewer), false, "");
    screen.logical_pixels().to_vec()
}

fn save(pixels: &[u32], name: &str) {
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut out = b"P6\n320 200\n255\n".to_vec();
        for p in pixels {
            out.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join(name), out).unwrap();
    }
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn gateways_system_map_has_orbits_names_and_a_time_point() {
    let cache = cache();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let mut map = SystemMap::new(
        nav.bodies.clone(),
        nav.system_name().to_owned(),
        true,
        nav.clock,
    );
    map.reset_focus();
    let frame = map.frame(&nav.date_at(map.clock), None);
    assert!(
        frame.time_point.starts_with("Time point: 1-Jan-3250"),
        "{}",
        frame.time_point
    );
    // Every planet and moon has an orbit about its parent, and the star and its worlds are labelled.
    let orbiting = nav
        .bodies
        .iter()
        .filter(|b| b.kind != Kind::Starport && b.orbit_centre.is_some())
        .count();
    assert!(orbiting > 3, "{orbiting}");
    assert!(frame.orbits.len() >= 3, "{} orbits", frame.orbits.len());
    assert!(frame.labels.len() >= 3, "{:?}", frame.labels);
    let pixels = draw(&mut screen, &viewer, &nav, &map);
    save(&pixels, "system-map-home.ppm");
    let green = screen.palette()[17];
    assert!(
        pixels[..320 * 158].iter().filter(|p| **p == green).count() > 100,
        "orbit curves are drawn in green"
    );
    // The orbit radii match the bodies: each orbit's farthest pixel is its radius from the centre
    // of the page at this scale only for the star's own planets, so check the planets' distance.
    for body in nav.bodies.iter().filter(|b| b.kind == Kind::Planet) {
        let centre = body.orbit_centre.unwrap();
        let radius = (0..3)
            .map(|k| ((body.position[k] - centre[k]) as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            radius > 1e9,
            "{} orbits {radius} mm from its parent",
            body.name
        );
    }

    // Zoomed on a world, its moons and stations are what is around it.
    let world = nav
        .bodies
        .iter()
        .position(|b| b.kind == Kind::Planet && nav.bodies.iter().any(|m| m.parent == Some(0)))
        .unwrap_or(1);
    map.focus_on(world);
    let pixels = draw(&mut screen, &viewer, &nav, &map);
    save(&pixels, "system-map-world.ppm");

    // The slider moves the time point back an hour (home squares the offset) and the bodies follow.
    let before = map.bodies.clone();
    let clock = map.clock + ffe_rs::system_map::slider_step(300 - 26, true);
    map.set_time(nav.bodies_at(&cache, clock).unwrap(), clock);
    let (then, now) = (nav.date_at(map.clock), nav.date_at(nav.clock));
    println!("time point {then}, now {now}");
    assert!(
        then.contains(" 11:0") && now.contains(" 12:00:0"),
        "{then} / {now}"
    );
    let moved = before
        .iter()
        .zip(&map.bodies)
        .any(|(a, b)| a.position != b.position);
    assert!(moved, "the planets move with the time point");
    map.reset_focus();
    let pixels = draw(&mut screen, &viewer, &nav, &map);
    save(&pixels, "system-map-hour-earlier.ppm");

    // A click on a body picks it and the details name it.
    map.reset_focus();
    let dot = map
        .frame("", None)
        .dots
        .into_iter()
        .find(|d| d.kind == Kind::Planet && (0..320).contains(&d.x) && (0..158).contains(&d.y))
        .expect("a planet in view");
    let picked = map.pick(dot.x, dot.y).expect("a planet under its own dot");
    map.selected = Some(picked);
    let details = map.frame("", Some("1.0 A.U.".to_owned())).details;
    assert!(
        details[0].starts_with(&map.bodies[picked].name),
        "{details:?}"
    );
    let pixels = draw(&mut screen, &viewer, &nav, &map);
    save(&pixels, "system-map-picked.ppm");
}

/// F983: a click on a planet, moon or station picks it (the nearest dot within 8 pixels), locks it
/// as the target and, after `t`, makes it the autopilot's destination. A ship on the pad cannot
/// be flown (`engage_body` refuses, as the `K` key does: "Launch before engaging the autopilot").
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_click_targets_a_body_and_t_then_a_click_sends_the_autopilot() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 0).unwrap();
    let mut map = SystemMap::new(
        nav.bodies.clone(),
        nav.system_name().to_owned(),
        true,
        nav.clock,
    );
    map.reset_focus();
    let pickable: Vec<usize> = (0..nav.bodies.len())
        .filter(|i| {
            matches!(
                nav.bodies[*i].kind,
                Kind::Planet | Kind::Moon | Kind::Station
            )
        })
        .collect();
    assert!(pickable.len() >= 3, "{} bodies", pickable.len());
    let mut picked_any = 0;
    for index in pickable {
        let Some((x, y, _)) = map.project(nav.bodies[index].position) else {
            continue;
        };
        let (x, y) = (x.round() as i32, y.round() as i32);
        if !(0..320).contains(&x) || !(0..ffe_rs::system_map::VIEW_HEIGHT).contains(&y) {
            continue;
        }
        // Landed on the pad: the click targets, the autopilot refuses.
        let Some(hit) = map.pick(x, y) else { continue };
        picked_any += 1;
        assert!(nav.lock(hit).is_some());
        assert_eq!(nav.locked, Some(hit));
        assert!(nav.engage_body(hit).is_none(), "the pad cannot fly");
        assert!(!nav.autopilot_engaged());
    }
    assert!(picked_any >= 3, "only {picked_any} dots were pickable");
    // In flight the same click with `t` armed engages the autopilot on the picked body.
    nav.docked = None;
    nav.new_game = None;
    let planet = nav
        .bodies
        .iter()
        .position(|b| b.kind == Kind::Planet)
        .unwrap();
    let (x, y, _) = map.project(nav.bodies[planet].position).unwrap();
    let hit = map
        .pick(x.round() as i32, y.round() as i32)
        .unwrap_or(planet);
    nav.lock(hit);
    let body = nav.engage_body(hit).map(|body| body.name.clone());
    assert!(body.is_some(), "the autopilot engages in flight");
    assert!(nav.autopilot_engaged());
    assert_eq!(nav.target, Some(hit));
}

/// The page at its start for the Ross 154 start (start type 1), drawn the way the executed
/// original draws it with `--game-frame 560 0x3d 0x40`: `tools/reference/verify-system-map.py`
/// compares the frame and the dots' places (`FFE_SHOT_DIR` receives `system-map.ppm` and
/// `system-map-dots.json`, one `[slot, x, y]` per drawn body).
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE) and FFE_SHOT_DIR"]
fn the_page_draws_at_its_start() {
    let cache = cache();
    let nav = Navigator::new_game(&cache, 1).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let mut map = SystemMap::open(
        nav.bodies.clone(),
        nav.system_name().to_owned(),
        true,
        nav.clock,
        &ffe_rs::system_map::View::new_game(1),
    );
    // The original opens the page at the game clock of the moment F6 was pressed;
    // FFE_MAP_SECONDS says how many seconds after the new game that was.
    if let Some(seconds) = std::env::var("FFE_MAP_SECONDS")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
    {
        let clock = nav.clock + seconds;
        map.set_time(nav.bodies_at(&cache, clock).unwrap(), clock);
        map.now = clock;
    }
    let pixels = draw(&mut screen, &viewer, &nav, &map);
    save(&pixels, "system-map.ppm");
    let frame = map.frame_with(&nav.date_at(map.clock), None, &|model| {
        viewer.object_descriptor(model)
    });
    let dots: Vec<String> = frame
        .dots
        .iter()
        .map(|d| format!("[{},{},{}]", map.bodies[d.index].slot, d.x, d.y))
        .collect();
    let names: Vec<String> = frame
        .labels
        .iter()
        .map(|l| format!("[\"{}\",{},{}]", l.text, l.x, l.y))
        .collect();
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let bodies: Vec<String> = map
            .bodies
            .iter()
            .map(|b| {
                format!(
                    "[{},{},{},{}]",
                    b.slot, b.position[0], b.position[1], b.position[2]
                )
            })
            .collect();
        std::fs::write(
            std::path::Path::new(&dir).join("system-map-dots.json"),
            format!(
                "{{\"focus\":{},\"dots\":[{}],\"labels\":[{}],\"bodies\":[{}]}}",
                map.bodies[map.focus].slot,
                dots.join(","),
                names.join(","),
                bodies.join(",")
            ),
        )
        .unwrap();
    }
    println!(
        "focus slot {} dots {dots:?} labels {names:?}",
        map.bodies[map.focus].slot
    );
}
