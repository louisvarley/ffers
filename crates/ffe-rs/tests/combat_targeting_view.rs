//! Local-only frame checks for the Combat Targeting Computer (`enhancement-10`/`11`): the
//! dashboard button in the escape capsule's slot, the navigation-style list, the red target
//! square and the edge arrow.
//!
//! `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test
//! combat_targeting_view -- --ignored --nocapture`

use ffe_rs::camera::ViewState;
use ffe_rs::combat_computer::{CombatLabel, LabelColour};
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::screen::{Screen, TargetHud};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

fn flight() -> Flight {
    let mut state = Flight {
        mode: Mode::Flight,
        position: [0.0; 3],
        velocity: [0.0, 0.0, -100.0],
        system_flight: true,
        ..Flight::default()
    };
    state.axes = Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    state
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

fn label(ship: u32, name: &str, range: i32, colour: LabelColour) -> CombatLabel {
    CombatLabel {
        name: name.to_owned(),
        registration: format!("A{ship}-1"),
        centre: [0.0; 3],
        ship,
        range,
        colour,
    }
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_list_button_square_and_arrow_draw() {
    let cache = cache();
    let mut screen = Screen::new(&cache).unwrap();
    let state = flight();
    screen.draw(&state, ViewState::default(), None, true, "");
    let plain = screen.logical_pixels().to_vec();

    screen.npc_button = true;
    screen.draw(&state, ViewState::default(), None, true, "");
    save(&screen, "ctc-button.ppm");
    let button = screen.logical_pixels().to_vec();
    let changed = |a: &[u32], b: &[u32], x0: usize, x1: usize, y0: usize, y1: usize| {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| y * 320 + x))
            .filter(|at| a[*at] != b[*at])
            .count()
    };
    assert!(changed(&plain, &button, 237, 262, 174, 199) > 100);
    assert_eq!(changed(&plain, &button, 0, 237, 0, 200), 0);

    let open_before = screen.logical_pixels().to_vec();
    screen.npc_list_open = true;
    screen.npc_list_selected = Some(2);
    screen.combat_labels = vec![
        label(1, "Moray Starboat 3", 1240, LabelColour::Green),
        label(2, "Lanner Cobra Mk III", 3050, LabelColour::Red),
        label(3, "Police Viper", 8120, LabelColour::Blue),
    ];
    for k in 4..14 {
        screen
            .combat_labels
            .push(label(k, "Sidewinder", 9000 + k as i32, LabelColour::Red));
    }
    screen.draw(&state, ViewState::default(), None, true, "");
    save(&screen, "ctc-list.ppm");
    // The button is drawn pressed while the list is open.
    assert!(changed(&open_before, screen.logical_pixels(), 237, 262, 174, 199) > 20);

    screen.npc_list_open = false;
    screen.target_hud = Some(TargetHud {
        x: 160,
        y: 79,
        value: 12,
        au: false,
        behind: false,
        offscreen: false,
        colour: ffe_rs::targeter::COMBAT_COLOUR,
    });
    screen.draw(&state, ViewState::default(), None, true, "");
    save(&screen, "ctc-square.ppm");

    screen.target_hud = None;
    for (name, point) in [
        ("right", [4.0, 0.0, 1.0]),
        ("upleft", [-3.0, 3.0, 1.0]),
        ("down", [0.0, -4.0, 1.0]),
        ("corner", [9.0, -7.0, 1.0]),
        ("left", [-4.0, 0.3, 1.0]),
    ] {
        screen.edge_indicator =
            ffe_rs::targeter::edge_indicator(true, point, ffe_rs::targeter::COMBAT_COLOUR);
        screen.draw(&state, ViewState::default(), None, true, "");
        save(&screen, &format!("ctc-arrow-{name}.ppm"));
    }
}
