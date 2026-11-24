//! Local-only frame checks for the enhanced (flat) galaxy map: the stars, the line to the
//! player's own system, the route (legs that cannot be flown in red), what hovering a star
//! says and the console panel, drawn through the same `enhanced_map::scene` the game uses.
//!
//! `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test
//! enhanced_map_view -- --ignored --nocapture`

use ffe_rs::camera::ViewState;
use ffe_rs::enhanced_map::{self, EnhancedMap, SceneContext, Toggled};
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::{DriveState, Navigator};
use ffe_rs::screen::Screen;

struct World {
    cache: ffe_content::VerifiedCache,
    viewer: ModelViewer,
    nav: Navigator,
}

fn world() -> World {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 12,
    });
    World { cache, viewer, nav }
}

fn save(screen: &Screen, name: &str) {
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for p in screen.logical_pixels() {
            ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join(name), ppm).unwrap();
    }
}

/// The map as the game builds and draws it for one frame.
fn draw(world: &World, map: &mut EnhancedMap, mouse: Option<(i32, i32)>, name: &str) -> Screen {
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let systems =
        ffe_core::galaxy::Tables::new(world.cache.asset("base:world/system-tables").unwrap())
            .unwrap();
    let mut note = |_: &enhanced_map::Plot| Some("Has a station or port".to_owned());
    let mut context = SceneContext {
        sectors: &sectors,
        systems: Some(&systems),
        name_of: &|code| {
            world
                .viewer
                .system_name(code)
                .map(|name| name.iter().map(|b| char::from(*b)).collect())
        },
        string_of: &|selector| world.viewer.system_string(selector),
        report_for: &|distance| world.nav.report_for(distance),
        drive_fuel: world.nav.drive.map_or(0, |drive| drive.fuel),
        date: world.nav.date_text(),
        mouse,
        station_note: &mut note,
    };
    let scene = enhanced_map::scene(map, &mut context);
    let mut screen = Screen::new(&world.cache).unwrap();
    screen.enhanced_galaxy = Some(scene);
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    screen.draw(&state, ViewState::default(), Some(&world.viewer), true, "");
    save(&screen, name);
    screen
}

fn open(world: &World) -> EnhancedMap {
    EnhancedMap::new(
        world.nav.current_position().unwrap(),
        world.nav.reach().unwrap_or(0),
    )
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_enhanced_map_draws_its_states() {
    let world = world();
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let mut map = open(&world);
    // Opened on the player's system: the faction view, the line home is a stub.
    eprintln!(
        "reach {:?} radius {}",
        world.nav.reach(),
        map.range_radius()
    );
    draw(&world, &mut map, None, "em-open.ppm");
    assert!(map.selected.is_some());

    // Click three stars into a route; hover the middle one.
    let plots = map.plots(&sectors);
    let mut added = Vec::new();
    for plot in plots.iter().filter(|p| p.x > 190 || p.y < 50).take(3) {
        assert_eq!(
            map.toggle_waypoint(plot.code, plot.position),
            Toggled::Added
        );
        added.push(*plot);
    }
    assert_eq!(map.route.len(), added.len());
    let hover = (added[0].x, added[0].y);
    draw(&world, &mut map, Some(hover), "em-route.ppm");

    // With little fuel the legs the hold cannot pay for are drawn red.
    let mut poor = self::world();
    poor.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 1,
    });
    let screen = draw(&poor, &mut map, None, "em-route-poor.ppm");
    drop(screen);

    // Hovering a star says whether it is inhabited, how dangerous it is and whether
    // it has a station or port.
    let sectors_again =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    for plot in map.plots(&sectors_again).iter().take(40) {
        let screen_scene = {
            let systems = ffe_core::galaxy::Tables::new(
                world.cache.asset("base:world/system-tables").unwrap(),
            )
            .unwrap();
            let mut note = |_: &enhanced_map::Plot| Some("Has a station or port".to_owned());
            let mut context = SceneContext {
                sectors: &sectors_again,
                systems: Some(&systems),
                name_of: &|code| {
                    world
                        .viewer
                        .system_name(code)
                        .map(|name| name.iter().map(|b| char::from(*b)).collect())
                },
                string_of: &|selector| world.viewer.system_string(selector),
                report_for: &|distance| world.nav.report_for(distance),
                drive_fuel: 12,
                date: String::new(),
                mouse: Some((plot.x, plot.y)),
                station_note: &mut note,
            };
            enhanced_map::scene(&mut map, &mut context)
        };
        let lines: Vec<&str> = screen_scene.hover.iter().map(|(t, _)| t.as_str()).collect();
        assert!(!lines.is_empty(), "hovering a plotted star says something");
        assert_eq!(lines.last().copied(), Some("Has a station or port"));
        if lines[0].contains("uninhabited") {
            assert_eq!(lines.len(), 2, "{lines:?}");
        } else {
            assert!(lines[0].contains("inhabited ("), "{lines:?}");
            assert!(lines[1].starts_with("Danger: "), "{lines:?}");
            assert_eq!(lines.len(), 3, "{lines:?}");
        }
    }
    draw(&world, &mut map, Some(hover), "em-hover.ppm");

    // Panned a long way: the line to the player's own system leaves the view.
    map.pan_pixels(420.0, 150.0);
    draw(&world, &mut map, None, "em-panned.ppm");
    let pointer = map.home_pointer();
    assert!(!pointer.on_view);
    assert!(pointer.distance > 0);
}

/// The low palette as a 16x16 grid of swatches (row = index >> 4), for choosing map colours.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_palette_swatches() {
    let world = world();
    let screen = Screen::new(&world.cache).unwrap();
    let palette = screen.palette();
    let mut ppm = b"P6
320 320
255
"
    .to_vec();
    for y in 0..320usize {
        for x in 0..320usize {
            let index = ((y / 20) * 16 + x / 20).min(palette.len() - 1);
            let p = palette[index];
            ppm.extend([(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        std::fs::write(std::path::Path::new(&dir).join("palette.ppm"), ppm).unwrap();
    }
}

/// Where Sol is: the star of its sector the name table calls "Sol".
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn find_sol() {
    let world = world();
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let mut found = false;
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            let (x, y) = ((0x1718 + dx) as u16, (0x1524 + dy) as u16);
            for (i, star) in sectors.stars(x, y).iter().enumerate() {
                let code = ffe_rs::galaxy_map::sector_code(x, y) | (i as u32) << 26;
                let name = world
                    .viewer
                    .system_name(code)
                    .map(|n| n.iter().map(|b| char::from(*b)).collect::<String>())
                    .unwrap_or_default();
                if name == "Sol" {
                    // The map's Sol pointer aims at the star, not the sector's corner.
                    assert_eq!(
                        ffe_core::sector::position(x, y, &star.vector),
                        ffe_rs::enhanced_map::SOL_POSITION
                    );
                    assert_eq!((x, y), (0x1718, 0x1524));
                    found = true;
                }
            }
        }
    }
    assert!(found, "Sol is not in its sector");
}

/// The stars around the player the drive can reach in a chain of short legs: (code, position).
fn chain(world: &World, legs: usize) -> Vec<(u32, [i32; 3])> {
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let map = open(world);
    let home = map.home;
    let mut plots = map.plots(&sectors);
    plots.sort_by_key(|plot| ffe_core::sector::distance(home, plot.position));
    let mut from = home;
    let mut route = Vec::new();
    for plot in plots {
        let leg = ffe_core::sector::distance(from, plot.position);
        if leg > 0 && leg < 700 && route.iter().all(|(code, _)| *code != plot.code) {
            route.push((plot.code, plot.position));
            from = plot.position;
        }
        if route.len() == legs {
            break;
        }
    }
    assert_eq!(route.len(), legs, "stars to chain");
    route
}

/// A route is one jump: its legs' jump times run together as a single transit, the fuel is
/// every leg's, the ship ends at the last star, and the clock moves by the whole distance.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_route_is_flown_as_one_long_transit() {
    let mut world = world();
    world.nav = Navigator::new(&world.cache).unwrap();
    world.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 12,
    });
    let route = chain(&world, 3);
    // What each leg alone would take and burn.
    let mut from = world.nav.current_position().unwrap();
    let (mut seconds, mut fuel, mut distance) = (0.0, 0, 0);
    let mut first_seconds = 0.0;
    let mut left = 12;
    for (index, (_, position)) in route.iter().enumerate() {
        let leg = ffe_core::sector::distance(from, *position);
        let report = world.nav.report_for_fuel(leg, left).expect("a plan");
        assert!(report.result >= 0, "leg {leg}: {report:?}");
        let leg_seconds = f64::from(report.jump_time.unwrap_or(0)) / 65_536.0 * 86_400.0;
        if index == 0 {
            first_seconds = leg_seconds;
        }
        seconds += leg_seconds;
        fuel += report.fuel_needed;
        left -= report.fuel_needed;
        distance += leg;
        from = *position;
    }
    let before = world.nav.clock;
    let jumped = world
        .nav
        .hyperspace_route(&world.cache, &route, false)
        .expect("the route");
    if jumped.misjump.is_some() {
        // One jump in 128 mis-jumps; the route then ends where it went wrong.
        return;
    }
    assert_eq!(jumped.fuel, fuel);
    assert_eq!(jumped.distance, distance);
    let transit = world.nav.transit.expect("one transit");
    assert!(
        (transit.total - seconds).abs() < 1.0,
        "{} vs {seconds}",
        transit.total
    );
    assert_eq!(transit.remaining, transit.total);
    // Three legs make a longer tunnel than the first alone.
    assert!(
        transit.total > first_seconds,
        "{} vs {first_seconds}",
        transit.total
    );
    eprintln!(
        "route: {} legs, {:.1} s of game time, {:.2} s of tunnel",
        route.len(),
        transit.total,
        transit.total / ffe_rs::navigation::HYPERSPACE_RATE
    );
    // The enhanced map stretches the tunnel to the distance flown, over the same game time.
    world.nav.stretch_transit(jumped.distance, route.len());
    let stretched = world.nav.transit.expect("still in transit");
    let want = ffe_rs::navigation::enhanced_transit_seconds(jumped.distance, route.len());
    assert!(
        (stretched.real_seconds_left() - want).abs() < 0.01,
        "{} vs {want}",
        stretched.real_seconds_left()
    );
    assert!(want > 3.0 * (transit.total / ffe_rs::navigation::HYPERSPACE_RATE));
    // One real second of the stretched transit covers 1/want of its game time.
    world.nav.transit_step(&world.cache, 1.0);
    let after = world.nav.transit.expect("still in transit");
    let covered = (stretched.remaining - after.remaining) / stretched.total;
    assert!(
        (covered - 1.0 / want).abs() < 0.01,
        "{covered} vs {}",
        1.0 / want
    );
    // The ship is at the last star, and the clock runs the whole distance.
    assert_eq!(world.nav.current_system_code(), route[2].0);
    world.nav.finish_transit(&world.cache);
    assert!(world.nav.clock - before >= seconds - 1.0);
}

/// A route no leg of which can be skipped is refused whole, before anything moves.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_route_with_an_unflyable_leg_is_refused_before_it_starts() {
    let mut world = world();
    world.nav = Navigator::new(&world.cache).unwrap();
    world.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 12,
    });
    let mut route = chain(&world, 2);
    // A third star far beyond the drive's range.
    route.push((
        route[1].0 ^ 1,
        [route[1].1[0] + 0x40_0000, route[1].1[1], route[1].1[2]],
    ));
    let code = world.nav.current_system_code();
    let clock = world.nav.clock;
    let error = world
        .nav
        .hyperspace_route(&world.cache, &route, false)
        .unwrap_err();
    assert!(error.starts_with("Leg 3 of 3"), "{error}");
    assert!(world.nav.transit.is_none());
    assert_eq!(world.nav.current_system_code(), code);
    assert_eq!(world.nav.clock, clock);
    // Not enough fuel for all legs together is refused too.
    world.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 1,
    });
    let route = chain(&world, 3);
    let error = world
        .nav
        .hyperspace_route(&world.cache, &route, false)
        .unwrap_err();
    assert!(
        error.contains("Insufficient Fuel") || error.contains("Out of Range"),
        "{error}"
    );
    assert_eq!(world.nav.current_system_code(), code);
}
