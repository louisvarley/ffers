//! F2's status screen docked in a station. The executed original (JJFFE, start key
//! 0x04, docked at Lave, state 0x2a; `tools/reference` with F2 = scancode 0x3c)
//! shows the Ship Equipment Status page over the cabin backdrop, with the flight
//! console, exactly as it does on a pad. The port only handled F2 in flight, so the
//! clipboard button did nothing in a station.
//!
//! Local-only: `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs
//! --test status_docked -- --ignored`.

use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::screen::{EquipmentStatus, Screen, StatusBackdrop};
use ffe_rs::station::Status;

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
fn the_status_page_draws_over_the_docked_view() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let state = Flight {
        mode: Mode::Docked,
        ..Flight::default()
    };
    let views = ViewState::default();
    screen.draw(&state, views, None, true, "");
    let plain = screen.logical_pixels().to_vec();
    save(&screen, "status-docked-before.ppm");

    screen.status_backdrop = Some(StatusBackdrop {
        hold: Status::default(),
        cargo_block: false,
        ship: state.ship,
        tics: 0,
        spin: 0,
        gear: 0,
    });
    screen.equipment_status = Some(EquipmentStatus {
        title: "SHIP EQUIPMENT STATUS - Cobra Mk III".to_owned(),
        status: vec!["Drive System:\tClass 2 Hyperdrive".to_owned()],
        equipment: vec![("Scanner".to_owned(), false)],
        lasers: vec![(
            "Front Gun Mounting:".to_owned(),
            "1MW Pulse Laser".to_owned(),
            108,
        )],
        scroll: 0,
    });
    screen.draw(&state, views, None, true, "");
    save(&screen, "status-docked-after.ppm");
    let shown = screen.logical_pixels();
    let changed = plain
        .iter()
        .zip(shown)
        .take(320 * 158)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > 500,
        "F2's status page did not draw docked: {changed} view pixels changed"
    );
    // The page is drawn: its title's top row (the 0x9c04 line at y 5) is not the
    // cabin's.
    assert!(
        plain[5 * 320..6 * 320]
            .iter()
            .zip(&shown[5 * 320..6 * 320])
            .any(|(a, b)| a != b),
        "the status title is missing"
    );
}
