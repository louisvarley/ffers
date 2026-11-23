//! The galaxy map's F7 description page (F842): Gateway's body diagram, the
//! "Stable system with 15 major bodies." line and the system description.
//!
//! `FFE_WORLD_CACHE=<data.bin> cargo test -p ffe-rs --test map_pages -- --ignored --nocapture`
//! (`FFE_SHOT_DIR=<dir>` also writes the frame).

use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::galaxy_map::GalaxyMap;
use ffe_rs::map_pages;
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::Navigator;
use ffe_rs::screen::{MapScene, Screen};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn gateways_description_page() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let tables = map_pages::sector_tables(&cache).unwrap();
    let mut map = GalaxyMap::new(nav.current_position().unwrap(), 0);
    map.update_selection(&tables);
    let page = map_pages::description(&cache, &viewer, map.selected.as_ref()).unwrap();
    let texts: Vec<String> = page
        .texts
        .iter()
        .map(|t| String::from_utf8_lossy(&t.bytes).into_owned())
        .collect();
    assert_eq!(texts[0], "Stable system with 15 major bodies.");
    assert!(texts[1].starts_with("Very high humidity and native life"));
    // The star and every body in the list that fits the view get a sprite; the
    // moons stand below their planets.
    assert_eq!(page.sprites[0].index, 0);
    assert_eq!(page.sprites[0].bitmap, 37);
    assert!(page.sprites.len() >= 15);

    let mut screen = Screen::new(&cache).unwrap();
    screen.galaxy = Some(MapScene {
        shapes: Vec::new(),
        sector: map.sector_text(),
        fuel: None,
        date: nav.date_text(),
        page: Some(page),
        info: map.describe(&viewer, None).unwrap_or_default(),
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
        std::fs::write(std::path::Path::new(&dir).join("map-description.ppm"), ppm).unwrap();
    }
}

/// With no star selected F842 shows only the unexplored-system text.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn no_selection_says_unexplored() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let page = map_pages::description(&cache, &viewer, None).unwrap();
    assert!(page.sprites.is_empty());
    let text = String::from_utf8_lossy(&page.texts[0].bytes).into_owned();
    assert!(text.starts_with("Unexplored System."), "{text}");
}

/// F9's economy and F10's people page for Gateway, as the owner's captures of the
/// original show them.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn gateways_economy_and_people_pages() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let tables = map_pages::sector_tables(&cache).unwrap();
    let mut map = GalaxyMap::new(nav.current_position().unwrap(), 0);
    map.update_selection(&tables);
    let selected = map.selected.clone().unwrap();
    let names = |index: usize| nav.market_tables()?.name(index).map(str::to_owned);
    let economy = map_pages::economy(&cache, &viewer, &selected, &names)
        .unwrap()
        .unwrap();
    let column = |x: i32| -> Vec<String> {
        economy
            .texts
            .iter()
            .skip(1)
            .filter(|t| t.x == x)
            .map(|t| String::from_utf8_lossy(&t.bytes).into_owned())
            .collect()
    };
    println!("imports {:?} {:?}", column(2), column(0x41));
    println!("exports {:?} {:?}", column(0x80), column(0xc1));
    println!("illegal {:?}", column(0x100));
    // The capture: heavy plastics, farm machinery and robots are the major imports,
    // narcotics, live animals and slaves the illegal goods.
    assert_eq!(column(2), ["Heavy Plastics", "Farm Machinery", "Robots"]);
    assert_eq!(column(0x41), ["Hand Weapons", "Industrial Parts"]);
    assert_eq!(
        column(0x80),
        ["Animal Meat", "Medicines", "Animal Skins", "Chaff"]
    );
    assert_eq!(
        column(0xc1),
        ["Water", "Liquid Oxygen", "Grain", "Liquor", "Luxury Goods"]
    );
    assert_eq!(column(0x100), ["Narcotics", "Live Animals", "Slaves"]);

    let people = map_pages::people(&cache, &viewer, &selected)
        .unwrap()
        .unwrap();
    let lines: Vec<String> = people
        .texts
        .iter()
        .map(|t| String::from_utf8_lossy(&t.bytes).into_owned())
        .collect();
    println!("{lines:?}");
    assert!(lines[0].contains("Alliance Democracy"), "{}", lines[0]);
    assert!(lines[0].contains("Capitalist"));
    assert!(lines[0].contains("100 Million-1 Billion"));
    assert!(
        lines[1].contains("-1,4") && lines[1].contains('6'),
        "{}",
        lines[1]
    );

    for (name, page) in [("economy", economy), ("people", people)] {
        let mut screen = Screen::new(&cache).unwrap();
        screen.galaxy = Some(MapScene {
            shapes: Vec::new(),
            sector: map.sector_text(),
            fuel: None,
            date: nav.date_text(),
            page: Some(page),
            info: map.describe(&viewer, None).unwrap_or_default(),
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
            let file = format!("map-{name}.ppm");
            std::fs::write(std::path::Path::new(&dir).join(file), ppm).unwrap();
        }
    }
}

/// A click on Gateway 5, the large gas giant the owner's capture of the original
/// shows: its name and type, mass, temperature, starports and orbit.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn clicking_a_body_shows_its_facts() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let tables = map_pages::sector_tables(&cache).unwrap();
    let mut map = GalaxyMap::new(nav.current_position().unwrap(), 0);
    map.update_selection(&tables);
    let selected = map.selected.clone().unwrap();
    let diagram = map_pages::description(&cache, &viewer, Some(&selected)).unwrap();
    // Gateway 5 is object 17: its hot square is about its pen, (159,35).
    let hit = map_pages::body_at(&diagram, 160, 36);
    assert_eq!(hit, Some(17));
    assert_eq!(map_pages::body_at(&diagram, 310, 120), None);
    let page = map_pages::body_info(&cache, &viewer, &selected, 17)
        .unwrap()
        .unwrap();
    let text = |t: &ffe_rs::map_pages::PageText| String::from_utf8_lossy(&t.bytes).into_owned();
    for t in &page.texts {
        println!("{} {} {:?}", t.x, t.y, text(t));
    }
    let all: Vec<String> = page.texts.iter().map(text).collect();
    assert!(
        all.contains(&"Gateway 5: Large gas giant".to_owned()),
        "{all:?}"
    );
    assert!(all[0].contains("299.02 Earth masses"), "{}", all[0]);
    assert!(
        all[0].contains("-99\u{5e}C") || all[0].contains("-99"),
        "{}",
        all[0]
    );
    assert!(all.contains(&"None".to_owned()));
    assert!(all.contains(&"104.7 years".to_owned()));
    assert!(all.iter().any(|t| t.contains("21.050 A.U.")));

    let mut screen = Screen::new(&cache).unwrap();
    screen.galaxy = Some(MapScene {
        shapes: Vec::new(),
        sector: map.sector_text(),
        fuel: None,
        date: nav.date_text(),
        page: Some(page),
        info: map.describe(&viewer, None).unwrap_or_default(),
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
        std::fs::write(std::path::Path::new(&dir).join("map-body.ppm"), ppm).unwrap();
    }
}

/// F8's picture mode: the star-density sky with the names of the nearby stars, as the
/// owner's capture of the original shows it with the map centred on Gateway.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn picture_mode_names_the_nearby_stars() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let home = nav.current_position().unwrap();
    let page = map_pages::picture(&cache, &viewer, [home[0], home[1]], 1).unwrap();
    let sky = page.background.as_ref().unwrap();
    assert_eq!(sky.len(), 320 * 158);
    assert!(sky.iter().any(|v| *v != 0) && sky.iter().all(|v| *v < 16));
    let found: Vec<(String, i32, i32)> = page
        .texts
        .iter()
        .map(|t| (String::from_utf8_lossy(&t.bytes).into_owned(), t.x, t.y))
        .collect();
    println!("{found:?}");
    // The capture shows exactly these nine names (at two screen pixels to a logical one,
    // measured by eye to within about six pixels).
    let mut names: Vec<&str> = found.iter().map(|(n, _, _)| n.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "Antares",
            "Betelgeuse",
            "Dumbell Nebula",
            "Hadar",
            "Pleiades",
            "Polaris",
            "Rigel",
            "Sol",
            "Spica"
        ]
    );
    for (name, x, y) in [
        ("Polaris", 162, 4),
        ("Dumbell Nebula", 91, 41),
        ("Pleiades", 194, 47),
        ("Betelgeuse", 222, 65),
        ("Rigel", 255, 73),
        ("Sol", 163, 78),
        ("Spica", 158, 96),
        ("Antares", 123, 113),
        ("Hadar", 156, 118),
    ] {
        let (_, fx, fy) = found.iter().find(|(n, _, _)| n == name).unwrap();
        assert!(
            (fx - x).abs() <= 8 && (fy - y).abs() <= 8,
            "{name} at {fx},{fy}, the capture has about {x},{y}"
        );
    }
    let mut screen = Screen::new(&cache).unwrap();
    screen.galaxy = Some(MapScene {
        shapes: Vec::new(),
        sector: String::new(),
        fuel: None,
        date: nav.date_text(),
        page: Some(page),
        info: String::new(),
    });
    let state = Flight {
        mode: Mode::Flight,
        ..Flight::default()
    };
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6
320 200
255
"
        .to_vec();
        for p in screen.logical_pixels() {
            ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join("map-picture.ppm"), ppm).unwrap();
    }
}
