use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::galaxy_map::{date_text, GalaxyMap};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::screen::{MapScene, Screen};

/// The map as the original shows it at a new game (Ross 154, sector -1,0); frames are kept when
/// `FFE_SHOT_DIR` is set.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_map_draws() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let bytes = cache.asset("base:world/galaxy-tables").unwrap();
    let tables = ffe_core::sector::Tables::new(bytes).unwrap();
    let home = [
        0x1717_6a00u32 as i32,
        0x1524_3200u32 as i32,
        0xffff_cc00u32 as i32,
    ];
    let mut map = GalaxyMap::new(home, 1000);
    map.update_selection(&tables);
    assert!(map.selected.is_some());
    let shapes = map.render(&viewer, &tables).unwrap();
    let details_bytes = cache.asset("base:world/system-tables").unwrap();
    let details = ffe_core::galaxy::Tables::new(details_bytes).unwrap();
    let text = map.describe(&viewer, Some(&details)).unwrap();
    // What the original's console says of the star at the centre of a new game's map.
    assert_eq!(
        text,
        "Ross 154 Dist. 0.00 light years\rType'M'red star: Outdoor ice and water world. Tourism and fishing."
    );
    assert!(shapes.len() > 100, "{} shapes", shapes.len());
    let mut screen = Screen::new(&cache).unwrap();
    screen.galaxy = Some(MapScene {
        shapes,
        sector: map.sector_text(),
        fuel: None,
        date: date_text(5.0),
        page: None,
        info: "Ross 154 Dist. 0.00 light years\rType M red star: Outdoor ice and water world. Tourism and fishing.".into(),
    });
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for p in screen.logical_pixels() {
            ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join("galaxy-map.ppm"), ppm).unwrap();
    }
}

/// The F834/F840 console page in its six original states, plus the map-local
/// key map's effect. Frames are kept when `FFE_SHOT_DIR` is set.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_map_console_page_states() {
    use ffe_rs::console::MapConsole;
    use ffe_rs::map_keys::{self, MapAction};

    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let home = [
        0x1717_6a00u32 as i32,
        0x1524_3200u32 as i32,
        0xffff_cc00u32 as i32,
    ];
    let mut map = GalaxyMap::new(home, 1000);
    map.update_selection(&tables);
    let scene = MapScene {
        shapes: map.render(&viewer, &tables).unwrap(),
        sector: map.sector_text(),
        fuel: None,
        date: date_text(5.0),
        page: None,
        info: map.describe(&viewer, None).unwrap_or_default(),
    };
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    let mut screen = Screen::new(&cache).unwrap();
    screen.galaxy = Some(scene);
    let save = |screen: &Screen, name: &str| {
        if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
            let mut out = b"P6\n320 200\n255\n".to_vec();
            for p in screen.logical_pixels() {
                out.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            std::fs::write(std::path::Path::new(&dir).join(name), out).unwrap();
        }
    };
    // F834's entry page: only the map icon is lit.
    screen.map_console = MapConsole::default();
    screen.draw(&state, ViewState::default(), Some(&viewer), false, "");
    save(&screen, "map-page-controls.ppm");
    // F6/F7/F8 select the original radio group-1 images.
    for (key, action, name) in [
        (6, MapAction::Select, "map-page-select.ppm"),
        (7, MapAction::Details, "map-page-details.ppm"),
        (8, MapAction::Picture, "map-page-picture.ppm"),
    ] {
        assert_eq!(map_keys::action(key), Some(action));
        screen.map_console = map_keys::page(action).unwrap();
        screen.draw(&state, ViewState::default(), Some(&viewer), false, "");
        save(&screen, name);
    }
    // F9/F10 keep details and light the group-10 next/previous image.
    for (key, action, name) in [
        (9, MapAction::Next, "map-page-next.ppm"),
        (10, MapAction::Previous, "map-page-previous.ppm"),
    ] {
        assert_eq!(map_keys::action(key), Some(action));
        let page = map_keys::page(action).unwrap();
        assert!(page.details && (page.next || page.previous));
        screen.map_console = page;
        screen.draw(&state, ViewState::default(), Some(&viewer), false, "");
        save(&screen, name);
    }
    // F5 returns to the controls page; F11/F12 are the original zoom keys.
    screen.map_console = map_keys::page(MapAction::Page).unwrap();
    screen.draw(&state, ViewState::default(), Some(&viewer), false, "");
    save(&screen, "map-page-f5-controls.ppm");
    let before = map.zoom;
    assert_eq!(map_keys::action(11), Some(MapAction::ZoomIn));
    map.zoom_by(-0x1f40);
    assert!(map.zoom < before);
    let zoomed_in = map.zoom;
    assert_eq!(map_keys::action(12), Some(MapAction::ZoomOut));
    map.zoom_by(0x1f40);
    assert!(map.zoom > zoomed_in);
    // The port's global keys keep their meanings outside the map's page.
    use ffe_rs::navigation::View;
    assert!(map_keys::owns(View::Galaxy));
    assert!(!map_keys::owns(View::System));
    assert!(!map_keys::owns(View::Places));
    assert!(!map_keys::owns(View::Closed));
}

/// A scripted map interaction with the map-local keys: F6 selects, F7/F9
/// re-image, F5 returns to the controls page, and H's destination jump still
/// works. Frames are kept when `FFE_SHOT_DIR` is set.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_map_key_script_selects_reimages_and_jumps() {
    use ffe_rs::map_keys::{self, MapAction};
    use ffe_rs::navigation::{DriveState, Navigator};

    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let details =
        ffe_core::galaxy::Tables::new(cache.asset("base:world/system-tables").unwrap()).unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let target = (1..15)
        .find(|i| nav.range_to(*i) == Some(435))
        .expect("Alpha Centauri");
    let position = nav.system_position(target).unwrap();
    let mut map = GalaxyMap::new(nav.current_position().unwrap(), nav.reach().unwrap_or(0));
    map.centre = [position[0], position[1]];
    map.update_selection(&tables);
    let selected = map.selected.clone().expect("the target star");
    nav.set_destination(selected.code, selected.position);
    nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 3,
    });
    let mut screen = Screen::new(&cache).unwrap();
    screen.galaxy = Some(MapScene {
        shapes: map.render(&viewer, &tables).unwrap(),
        sector: map.sector_text(),
        fuel: map.fuel_text(nav.destination_report().as_ref()),
        date: nav.date_text(),
        page: None,
        info: map.describe(&viewer, Some(&details)).unwrap_or_default(),
    });
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    let save = |screen: &Screen, name: &str| {
        if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
            let mut out = b"P6\n320 200\n255\n".to_vec();
            for p in screen.logical_pixels() {
                out.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            std::fs::write(std::path::Path::new(&dir).join(name), out).unwrap();
        }
    };
    // The scripted keys, in the order the original processes them.
    let keys = [
        (6, MapAction::Select),
        (7, MapAction::Details),
        (9, MapAction::Next),
        (5, MapAction::Page),
    ];
    for (key, expected) in keys {
        let action = map_keys::action(key).expect("a map key");
        assert_eq!(action, expected);
        if let Some(page) = map_keys::page(action) {
            screen.map_console = page;
        }
        screen.draw(&state, ViewState::default(), Some(&viewer), false, "");
        save(&screen, &format!("map-script-f{key}.ppm"));
    }
    // The stored destination survives the page keys, and H's jump runs.
    assert!(nav.destination.is_some());
    save(&screen, "map-script-destination.ppm");
    let jumped = nav.hyperspace_to_destination(&cache).unwrap();
    nav.finish_transit(&cache);
    assert!(nav.system_name().starts_with("Alpha Centauri"));
    assert_eq!(nav.destination, None);
    assert_eq!(nav.locked, None);
    assert!(jumped.days > 0.0);
}

/// The full cached map route: choose a reachable star, store it as the destination,
/// render the map with the fuel line, jump, and render the arrival. Frames and the
/// state series are kept when `FFE_SHOT_DIR` is set; the same state renders twice
/// byte for byte.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_map_destination_and_jump_frames() {
    use ffe_rs::navigation::{DriveState, Navigator};
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let details =
        ffe_core::galaxy::Tables::new(cache.asset("base:world/system-tables").unwrap()).unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let target = (1..15)
        .find(|i| nav.range_to(*i) == Some(435))
        .expect("Alpha Centauri");
    let position = nav.system_position(target).unwrap();
    let code = ffe_rs::navigation::galaxy(&cache).unwrap()[target]
        .params
        .code;
    let mut map = GalaxyMap::new(nav.current_position().unwrap(), nav.reach().unwrap_or(0));
    map.centre = [position[0], position[1]];
    map.update_selection(&tables);
    let selected = map.selected.clone().expect("the target star");
    assert_eq!(selected.code, code);
    nav.set_destination(selected.code, selected.position);
    nav.drive = Some(DriveState {
        mass: 16,
        class: 10,
        fuel: 3,
    });
    let report = nav.destination_report().unwrap();
    let fuel = map.fuel_text(Some(&report));
    assert_eq!(fuel.as_deref(), Some("Fuel Required: 1t"));
    let shapes = map.render(&viewer, &tables).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    let save = |screen: &Screen, name: &str| {
        if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
            let mut out = b"P6\n320 200\n255\n".to_vec();
            for p in screen.logical_pixels() {
                out.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
            std::fs::write(std::path::Path::new(&dir).join(name), out).unwrap();
        }
    };
    screen.galaxy = Some(MapScene {
        shapes: shapes.clone(),
        sector: map.sector_text(),
        fuel: fuel.clone(),
        date: nav.date_text(),
        page: None,
        info: map.describe(&viewer, Some(&details)).unwrap_or_default(),
    });
    // The scene is deterministic: the same map state renders the same shapes.
    assert_eq!(shapes, map.render(&viewer, &tables).unwrap());
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    save(&screen, "galaxy-map-destination.ppm");
    let before_system = nav.system_name().to_owned();
    let before_clock = nav.clock;
    let jumped = nav.hyperspace_to_destination(&cache).unwrap();
    nav.finish_transit(&cache);
    assert!(nav.system_name().starts_with("Alpha Centauri"));
    assert_eq!(nav.locked, None);
    assert_eq!(nav.destination, None);
    assert!(nav.clock > before_clock);
    // The new system's map frame.
    let home = nav.current_position().unwrap();
    let mut arrival_map = GalaxyMap::new(home, nav.reach().unwrap_or(0));
    arrival_map.update_selection(&tables);
    screen.galaxy = Some(MapScene {
        shapes: arrival_map.render(&viewer, &tables).unwrap(),
        sector: arrival_map.sector_text(),
        fuel: None,
        date: nav.date_text(),
        page: None,
        info: arrival_map
            .describe(&viewer, Some(&details))
            .unwrap_or_default(),
    });
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    save(&screen, "hyperspace-arrival.ppm");
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        std::fs::write(
            std::path::Path::new(&dir).join("hyperspace-series.txt"),
            format!(
                "from={before_system}\nto={}\ndistance={}\nfuel={}\ndays={:.6}\nclock_before={before_clock:.3}\nclock_after={:.3}\nfuel_line={}\n",
                nav.system_name(),
                jumped.distance,
                jumped.fuel,
                jumped.days,
                nav.clock,
                fuel.as_deref().unwrap_or("")
            ),
        )
        .unwrap();
    }
}

/// A star not among the systems on offer is still a valid jump destination:
/// the original's galaxy is procedural, so the port generates its system on
/// demand (F832/F860/F905) and enters it.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_jump_to_a_star_beyond_the_offer_generates_its_system() {
    use ffe_rs::navigation::{DriveState, Navigator};
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let here = nav.current_position().unwrap();
    let (sx, sy) = ((here[0] >> 16) as u16, (here[1] >> 16) as u16);
    // A nearby star that is not one of the systems on offer.
    let mut chosen = None;
    'outer: for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (x, y) = (sx.wrapping_add(dx as u16), sy.wrapping_add(dy as u16));
            let stars = tables.stars(x, y);
            for (index, star) in stars.iter().enumerate() {
                let code = ffe_rs::galaxy_map::sector_code(x, y) | (index as u32) << 26;
                if nav.entry_for_code(code).is_none() {
                    chosen = Some((code, ffe_core::sector::position(x, y, &star.vector)));
                    break 'outer;
                }
            }
        }
    }
    let (code, position) = chosen.expect("a nearby star beyond the offer");
    assert!(nav.entry_for_code(code).is_none());
    nav.set_destination(code, position);
    // A tiny ship with a big drive reaches any neighbour; the range gate is
    // covered elsewhere, this test is for the on-demand generation.
    nav.drive = Some(DriveState {
        mass: 1,
        class: 10,
        fuel: 9999,
    });
    assert!(nav.jump_possible());
    let jumped = nav.hyperspace_to_destination(&cache).unwrap();
    nav.finish_transit(&cache);
    assert_eq!(nav.current_code(), code);
    assert!(nav.entry_for_code(code).is_some());
    assert_eq!(nav.destination, None);
    assert!(jumped.days > 0.0);
    // The generated system also survives a save/load round trip.
    let saved = nav.save_state();
    let reloaded = Navigator::from_save(&cache, &saved).unwrap();
    assert_eq!(reloaded.current_code(), code);
}

/// The enhanced (flat) map projects the local stars, selects the nearest, and
/// measures a route, all from the real sector tables.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_enhanced_map_plots_stars_and_routes() {
    use ffe_rs::enhanced_map::EnhancedMap;
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let home = [
        0x1717_6a00u32 as i32,
        0x1524_3200u32 as i32,
        0xffff_cc00u32 as i32,
    ];
    let mut map = EnhancedMap::new(home, 1000);
    map.select(&tables);
    assert!(map.selected.is_some());
    let plots = map.plots(&tables);
    assert!(!plots.is_empty(), "the local sectors have stars");
    for plot in &plots {
        assert!(
            (0..320).contains(&plot.x) && (0..158).contains(&plot.y),
            "{plot:?}"
        );
        assert!((1..=3).contains(&plot.size), "{plot:?}");
    }
    // The selection is one of the plotted stars, and the Sol pointer is fixed.
    let selected = map.selected.as_ref().unwrap();
    assert!(plots.iter().any(|plot| plot.code == selected.code));
    assert_eq!(
        map.sol().2,
        ffe_core::sector::distance(home, ffe_rs::enhanced_map::SOL_POSITION)
    );
    assert!(map.range_radius() > 0);
    // A two-step route measures two legs.
    map.add_waypoint(plots[0].code, plots[0].position);
    map.add_waypoint(plots[1].code, plots[1].position);
    assert_eq!(map.legs().len(), 2);
    assert_eq!(
        map.legs()[0],
        ffe_core::sector::distance(home, plots[0].position)
    );
}

/// Scrolling picks the star nearest the centre; a long way from any star picks none.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_centre_selects_the_nearest_star() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let home = [
        0x1717_6a00u32 as i32,
        0x1524_3200u32 as i32,
        0xffff_cc00u32 as i32,
    ];
    let mut map = GalaxyMap::new(home, 1000);
    map.update_selection(&tables);
    let here = map.selected.clone().unwrap();
    assert_eq!(viewer.system_name(here.code).unwrap(), b"Ross 154");
    // The selection holds until the centre moves.
    map.centre[0] += 0x300;
    map.update_selection(&tables);
    assert_eq!(map.selected.as_ref(), Some(&here));
    // Into the empty part of the sector (a quarter of a sector from every star).
    map.scroll(0x4000, 0x4000);
    let mut seen = std::collections::BTreeSet::new();
    for step in 0..40 {
        map.scroll(0x1800, 0x0900);
        map.update_selection(&tables);
        seen.insert(map.selected.as_ref().map(|s| s.code));
        let _ = step;
    }
    assert!(seen.len() > 2, "{seen:?}");
    assert!(seen.contains(&None), "some centres are far from every star");
    // The corner text follows the centre, in sectors from Sol's.
    map.recentre();
    assert_eq!(map.sector_text(), "Sector: -1,0");
    assert_eq!(map.centre, [home[0], home[1]]);
    // Fuel lines: the same star needs none; a far one is out of range.
    assert_eq!(map.fuel_text(None), None);
}

/// F828/F863's console strings are F1347/F830 templates: the sector and fuel
/// lines' number codes and the distance fraction expand from the cache's
/// system strings, and `describe` reads F869's description through the same
/// expansion. The map's compiled-in fuel copies match the cache.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_map_strings_expand_f1347_templates() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let tables =
        ffe_core::sector::Tables::new(cache.asset("base:world/galaxy-tables").unwrap()).unwrap();
    let details =
        ffe_core::galaxy::Tables::new(cache.asset("base:world/system-tables").unwrap()).unwrap();
    let home = [
        0x1717_6a00u32 as i32,
        0x1524_3200u32 as i32,
        0xffff_cc00u32 as i32,
    ];
    let mut map = GalaxyMap::new(home, 1000);
    map.update_selection(&tables);
    // The raw templates are the source ones, not pre-rendered text.
    for (selector, raw) in [
        (0x84ae, b"Sector: \xff0\x01,\xff0\x02".as_slice()),
        (0x84af, b"Fuel Required: \xff0\x1f".as_slice()),
        (0x84b0, b"Insufficient Fuel (\xff0\x1f required)".as_slice()),
        (0x84b1, b"Out of Range".as_slice()),
        (0x849e, b"Dist. \xff0\x07 light years".as_slice()),
    ] {
        assert_eq!(viewer.system_string(selector).as_deref(), Some(raw));
    }
    // The sector line expands the two integers from the centre.
    assert_eq!(
        map.sector_text_with(&viewer).as_deref(),
        Some("Sector: -1,0")
    );
    assert_eq!(map.sector_text(), "Sector: -1,0");
    // The fuel line expands the tonne writer, from the cache and from the
    // map's compiled-in copies. The line needs a non-zero distance, so move
    // the player's system off the selected star for these checks.
    let star = map.selected.as_ref().unwrap().position;
    map.home = [star[0] - 0x10000, star[1], star[2]];
    // 0x849e's 0xff 0x30 0x07 is F1377's two-place fraction writer.
    let distance = ffe_core::sector::distance(map.home, star);
    let expected = format!(
        "Ross 154 Dist. {}.{:02} light years\r",
        distance / 100,
        distance % 100
    );
    assert!(
        map.describe(&viewer, None).unwrap().starts_with(&expected),
        "0x849e must expand the distance fraction"
    );
    let report = |result, fuel_needed| ffe_core::hyperspace::Report {
        result,
        name: 0,
        fuel_needed,
        range_now: 0,
        jump_time: None,
        max_range: 0,
        flag: None,
        consumed: 0,
    };
    for (result, fuel_needed, line) in [
        (10, 1, "Fuel Required: 1t"),
        (-1, 2, "Insufficient Fuel (2t required)"),
        (-1, -1, "Out of Range"),
    ] {
        let report = report(result, fuel_needed);
        assert_eq!(
            map.fuel_text_with(&viewer, Some(&report)).as_deref(),
            Some(line)
        );
        assert_eq!(map.fuel_text(Some(&report)).as_deref(), Some(line));
    }
    // F863: the distance's 0x849e fraction and F869's description.
    map.home = home;
    assert_eq!(
        map.describe(&viewer, Some(&details)).as_deref(),
        Some(
            "Ross 154 Dist. 0.00 light years\rType'M'red star: Outdoor ice and water world. Tourism and fishing."
        )
    );
}
