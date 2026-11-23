//! F439's View page of a hull, drawn from the real cache over the one-row ship
//! list (`docs/notes/station.md`). Local-only: `FFE_WORLD_CACHE=<data.bin>
//! FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test ship_view -- --ignored`.
use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::screen::{Screen, ShipViewFrame};
use ffe_rs::station::{self, DockRow, DockView, Screen as StationScreen};

fn save(screen: &Screen, name: &str) {
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for pixel in screen.logical_pixels() {
            ppm.extend([(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join(name), ppm).unwrap();
    }
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_view_page_shows_the_hull_and_its_statistics() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let state = Flight {
        mode: Mode::Docked,
        ..Flight::default()
    };
    let list = |screen: &mut Screen| {
        screen.dock = Some(DockView {
            screen: StationScreen::Ships,
            title: station::title(StationScreen::Ships, "Lave Station"),
            rows: vec![DockRow {
                main: "Turner Class".to_owned(),
                detail: None,
                columns: vec![
                    (station::SHIP_COST_X, "$1531000".to_owned(), 0x7d),
                    (station::SHIP_EXCHANGE_X, "$1500000".to_owned(), 0x7d),
                    (station::SHIP_CAPACITY_X, "1243t".to_owned(), 0x7d),
                ],
                fitted: false,
            }],
            total: 1,
            ..DockView::default()
        });
    };
    list(&mut screen);
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    let plain = screen.logical_pixels().to_vec();
    for (name, model, seed) in [
        ("turner", 0x2e_u32, 7u16),
        ("saker", 31, 2),
        ("eagle", 23, 9),
    ] {
        list(&mut screen);
        let page = ffe_rs::ship_view::page(&cache, model, seed).expect("the page");
        screen.ship_view = Some(ShipViewFrame {
            page,
            tics: 0,
            spin: 65_536,
        });
        screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
        save(&screen, &format!("ship-view-{name}.ppm"));
        let changed = plain
            .iter()
            .zip(screen.logical_pixels())
            .take(320 * 158)
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed > 800, "{name}: the View page drew {changed} pixels");
        // The model is in the left window and the text at the right.
        let side = |range: std::ops::Range<usize>| {
            (0..158usize)
                .flat_map(|y| range.clone().map(move |x| y * 320 + x))
                .filter(|at| plain[*at] != screen.logical_pixels()[*at])
                .count()
        };
        assert!(side(0..130) > 100, "{name}: no model at the left");
        assert!(side(135..320) > 300, "{name}: no text at the right");
    }
}
