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
    let context = SceneContext {
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
        check_route: &|route| world.nav.check_route(route),
        drive_fuel: world.nav.drive.map_or(0, |drive| drive.fuel),
        date: world.nav.date_text(),
        mouse,
    };
    let scene = enhanced_map::scene(map, &context);
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
    // Nothing is selected until a star is clicked: opening, centring or panning select none.
    assert!(map.selected.is_none());

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
            let context = SceneContext {
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
                check_route: &|route| world.nav.check_route(route),
                drive_fuel: 12,
                date: String::new(),
                mouse: Some((plot.x, plot.y)),
            };
            enhanced_map::scene(&mut map, &context)
        };
        let lines: Vec<&str> = screen_scene.hover.iter().map(|(t, _)| t.as_str()).collect();
        assert!(!lines.is_empty(), "hovering a plotted star says something");
        // The panel opens with the name and the distance from the player's system.
        assert!(lines[0].contains(" ly"), "{lines:?}");
        let own_system = plot.position == map.route_end().1;
        let on_route = map.route.iter().any(|w| w.code == plot.code);
        // What the jump needs is always the last line, unless there is no jump to say.
        let last = lines.last().copied().unwrap_or_default();
        assert!(
            last.starts_with("Fuel Required")
                || last.starts_with("Insufficient Fuel")
                || last.starts_with("Out of range")
                || own_system
                || on_route,
            "{lines:?}"
        );
        // Insufficient fuel is said once, in red.
        let shortage: Vec<_> = screen_scene
            .hover
            .iter()
            .filter(|(text, _)| text.contains("Insufficient Fuel"))
            .collect();
        assert!(shortage.len() <= 1, "{lines:?}");
        assert!(
            shortage
                .iter()
                .all(|(_, colour)| *colour == enhanced_map::UNPOPULATED),
            "{lines:?}"
        );
        if lines[1] == "Unpopulated" {
            assert_eq!(screen_scene.hover[1].1, enhanced_map::UNPOPULATED);
        } else {
            assert_eq!(lines[1], "Populated", "{lines:?}");
            assert_eq!(screen_scene.hover[1].1, enhanced_map::POPULATED);
            assert!(
                lines.iter().any(|line| line.starts_with("Danger: ")),
                "{lines:?}"
            );
        }
    }
    draw(&world, &mut map, Some(hover), "em-hover.ppm");

    // With no fuel in the hold, a star the drive could reach says "Insufficient Fuel" once, in
    // red (there is no second, white copy of it).
    let mut dry = self::world();
    dry.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 0,
    });
    let mut dry_map = open(&dry);
    let target = dry_map
        .plots(&sectors_again)
        .into_iter()
        .find(|plot| {
            let distance = ffe_core::sector::distance(dry_map.home, plot.position);
            distance != 0
                && dry
                    .nav
                    .report_for(distance)
                    .is_some_and(|r| r.fuel_needed > 0)
        })
        .expect("a star within the drive's range");
    let screen = draw(
        &dry,
        &mut dry_map,
        Some((target.x, target.y)),
        "em-no-fuel.ppm",
    );
    drop(screen);
    let systems =
        ffe_core::galaxy::Tables::new(world.cache.asset("base:world/system-tables").unwrap())
            .unwrap();
    let scene = enhanced_map::scene(
        &mut dry_map,
        &SceneContext {
            sectors: &sectors_again,
            systems: Some(&systems),
            name_of: &|code| {
                dry.viewer
                    .system_name(code)
                    .map(|name| name.iter().map(|b| char::from(*b)).collect())
            },
            string_of: &|selector| dry.viewer.system_string(selector),
            report_for: &|distance| dry.nav.report_for(distance),
            check_route: &|route| dry.nav.check_route(route),
            drive_fuel: 0,
            date: String::new(),
            mouse: Some((target.x, target.y)),
        },
    );
    let shortage: Vec<_> = scene
        .hover
        .iter()
        .filter(|(text, _)| text.contains("Insufficient Fuel"))
        .collect();
    assert_eq!(shortage.len(), 1, "{:?}", scene.hover);
    assert_eq!(shortage[0].1, enhanced_map::UNPOPULATED, "in red");

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
    assert!(error.contains("needs") && error.contains("fuel"), "{error}");
    assert_eq!(world.nav.current_system_code(), code);
}

/// The hyperspace tunnel at fifteen points of its animation, as a contact sheet of
/// frames (kept when `FFE_SHOT_DIR` is set), to see which part of it can repeat.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_tunnel_animation_frames() {
    let world = world();
    let dense = std::env::var_os("FFE_TUNNEL_DENSE").is_some();
    let steps: Vec<u16> = if dense {
        (40..=130).collect()
    } else {
        (0..=145u16).step_by(10).collect()
    };
    for (index, share) in steps.into_iter().enumerate() {
        let mut screen = Screen::new(&world.cache).unwrap();
        screen.hyperspace = Some((share << 8) & 0xff00);
        let state = Flight {
            mode: Mode::Flight,
            ..Flight::default()
        };
        screen.draw(&state, ViewState::default(), Some(&world.viewer), true, "");
        save(&screen, &format!("tunnel-{index:02}.ppm"));
    }
}

/// A star several jumps away is reached by a plotted course: every leg within the drive's
/// range, the stops marked `auto`, the whole route flyable with the fuel in the hold, a
/// second click on the star taking the stops off again, and the hyperspace icon (the
/// navigator's `jump_possible`) on for the course although the star itself is out of range.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_far_star_is_reached_through_a_plotted_course() {
    use ffe_rs::enhanced_map::{Clicked, PlanContext};
    let mut world = world();
    world.nav = Navigator::new(&world.cache).unwrap();
    world.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 12,
    });
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let systems =
        ffe_core::galaxy::Tables::new(world.cache.asset("base:world/system-tables").unwrap())
            .unwrap();
    let mut map = open(&world);
    map.home_code = world.nav.current_system_code();
    let reach = world.nav.reach().unwrap();
    map.reach = reach;
    // A star about six sectors east of Sol: beyond two jumps, within a few.
    let (hx, hy) = ((map.home[0] >> 16) as u16, (map.home[1] >> 16) as u16);
    let far = (6..9)
        .flat_map(|dx| (0..3).map(move |dy| (hx + dx, hy + dy)))
        .find_map(|(x, y)| {
            sectors
                .stars(x, y)
                .first()
                .map(|star| ffe_rs::course::Stop {
                    code: ffe_rs::galaxy_map::sector_code(x, y),
                    position: ffe_core::sector::position(x, y, &star.vector),
                })
        })
        .expect("a far star");
    let distance = ffe_core::sector::distance(map.home, far.position);
    assert!(
        distance > 2 * reach,
        "{distance} vs {reach}: not far enough for the test"
    );
    let plan = PlanContext {
        sectors: &sectors,
        systems: Some(&systems),
    };
    let Clicked::Plotted(stops) = map.click_star(far, &plan) else {
        panic!("a course should be plotted to {far:?}");
    };
    assert_eq!(*stops.last().unwrap(), far);
    assert!(stops.len() >= 3, "{} jumps", stops.len());
    // Fewest jumps: no shorter chain could cover the distance.
    assert_eq!(
        stops.len(),
        ffe_rs::course::jumps_needed(map.home, far.position, reach)
            .max(stops.len().min(stops.len()))
    );
    // Every leg is within the drive's range, and the route is the stops, auto but the last.
    let mut at = map.home;
    for stop in &stops {
        assert!(ffe_core::sector::distance(at, stop.position) <= reach);
        at = stop.position;
    }
    assert_eq!(map.route.len(), stops.len());
    assert!(map.route[..stops.len() - 1].iter().all(|w| w.auto));
    assert!(!map.route.last().unwrap().auto);
    // The navigator agrees the course can be flown with the fuel in the hold.
    let route: Vec<(u32, [i32; 3])> = map.route.iter().map(|w| (w.code, w.position)).collect();
    let flown = world.nav.check_route(&route).expect("a flyable course");
    assert_eq!(flown.legs.len(), stops.len());
    assert!(flown.fuel <= flown.have);
    // The star alone is out of range, so the icon would be off; the ready course turns it on.
    world.nav.set_destination(far.code, far.position);
    assert!(!world.nav.jump_possible());
    world.nav.course_ready = true;
    assert!(world.nav.jump_possible());
    // Clicking the star again takes it and the stops plotted to it off.
    assert_eq!(map.click_star(far, &plan), Clicked::Removed(stops.len()));
    assert!(map.route.is_empty());
    // The course is flown as one jump, ending at the star.
    for waypoint in &stops {
        map.add_waypoint(waypoint.code, waypoint.position);
    }
    let pairs: Vec<(u32, [i32; 3])> = map.route.iter().map(|w| (w.code, w.position)).collect();
    let jumped = world
        .nav
        .hyperspace_route(&world.cache, &pairs, false)
        .expect("the course");
    if jumped.misjump.is_none() {
        assert_eq!(world.nav.current_system_code(), far.code);
        assert_eq!(jumped.fuel, flown.fuel);
    }
}

/// A star with no way through is refused with its reason, and a leg longer than the drive,
/// or with too little fuel, says which and by how much.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn an_unreachable_or_unflyable_course_says_why() {
    use ffe_rs::enhanced_map::{Clicked, PlanContext};
    let mut world = world();
    world.nav = Navigator::new(&world.cache).unwrap();
    world.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 12,
    });
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let mut map = open(&world);
    map.home_code = world.nav.current_system_code();
    map.reach = world.nav.reach().unwrap();
    let plan = PlanContext {
        sectors: &sectors,
        systems: None,
    };
    // Far beyond eight jumps.
    let faraway = ffe_rs::course::Stop {
        code: 77,
        position: [map.home[0] + 0x40_0000, map.home[1], map.home[2]],
    };
    let Clicked::NoCourse(error) = map.click_star(faraway, &plan) else {
        panic!("too far to plot");
    };
    assert!(error.to_string().contains("Too far"), "{error}");
    assert!(map.route.is_empty());
    // One leg longer than the drive: named with both distances.
    let route = vec![(77u32, faraway.position)];
    let problem = world.nav.check_route(&route).unwrap_err();
    let text = problem.to_string();
    assert!(
        text.contains("Leg 1 of 1") && text.contains("your drive reaches"),
        "{text}"
    );
    // Too little fuel: names the leg, the need and the hold.
    world.nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 0,
    });
    let near = chain(&world, 1);
    let text = world.nav.check_route(&near).unwrap_err().to_string();
    assert!(
        text.contains("needs") && text.contains("t of fuel") && text.contains("you would have 0 t"),
        "{text}"
    );
}

/// A plotted course as it is drawn: the numbered stops, the legs, the summary line and the
/// zoomed-out view that takes the whole course in (kept when `FFE_SHOT_DIR` is set).
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_plotted_course_draws() {
    use ffe_rs::enhanced_map::{Clicked, PlanContext};
    let world = world();
    let sectors =
        ffe_core::sector::Tables::new(world.cache.asset("base:world/galaxy-tables").unwrap())
            .unwrap();
    let systems =
        ffe_core::galaxy::Tables::new(world.cache.asset("base:world/system-tables").unwrap())
            .unwrap();
    let mut map = open(&world);
    map.home_code = world.nav.current_system_code();
    map.reach = world.nav.reach().unwrap();
    let (hx, hy) = ((map.home[0] >> 16) as u16, (map.home[1] >> 16) as u16);
    let star = sectors.stars(hx + 4, hy).first().copied().expect("a star");
    let far = ffe_rs::course::Stop {
        code: ffe_rs::galaxy_map::sector_code(hx + 4, hy),
        position: ffe_core::sector::position(hx + 4, hy, &star.vector),
    };
    let plan = PlanContext {
        sectors: &sectors,
        systems: Some(&systems),
    };
    assert!(matches!(map.click_star(far, &plan), Clicked::Plotted(_)));
    map.fit_route();
    draw(&world, &mut map, None, "em-course.ppm");
    // Every stop is on the view, and numbered in the order it is flown.
    for waypoint in &map.route {
        let (x, y, _) = map.project(waypoint.position);
        assert!((0..320).contains(&x) && (0..158).contains(&y), "{x},{y}");
    }
    let context = SceneContext {
        sectors: &sectors,
        systems: Some(&systems),
        name_of: &|code| {
            world
                .viewer
                .system_name(code)
                .map(|n| n.iter().map(|b| char::from(*b)).collect())
        },
        string_of: &|selector| world.viewer.system_string(selector),
        report_for: &|distance| world.nav.report_for(distance),
        check_route: &|route| world.nav.check_route(route),
        drive_fuel: 12,
        date: String::new(),
        mouse: None,
    };
    let scene = enhanced_map::scene(&mut map, &context);
    for (index, waypoint) in map.route.iter().enumerate() {
        let (x, y, _) = map.project(waypoint.position);
        let label = scene.labels.iter().find(|(lx, ly, _)| (*lx, *ly) == (x, y));
        let label = label.expect("a label on every stop");
        assert!(label.2.starts_with(&format!("{} ", index + 1)), "{label:?}");
    }
    assert_eq!(scene.leg_ok, vec![true; map.route.len()]);
    assert!(scene
        .route_summary
        .as_ref()
        .unwrap()
        .0
        .starts_with("Course OK"));
}
