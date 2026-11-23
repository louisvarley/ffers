//! The Eagle's laser beam is the original's big wedge, not a stub. The executed
//! JJFFE (`tools/reference`, a shot taken the way the player's key takes it, after
//! F149 and before the draw: see `docs/KNOWN_ISSUES.md`) draws, in the Eagle's front
//! view, a solid wedge from the crosshair to both lower corners of the view with a
//! thin white ray down its middle. It is 13,206 red pixels for the 1MW pulse and the
//! 1MW beam, the same shape in yellow for the 20MW beam, and a cyan wedge twice the
//! size for the mining laser. The port used to draw model 156 at the ship's scale 8
//! instead of its own scale 17 (512 times too small) and showed a short stub that
//! pointed wherever the ship did.
//!
//! The wedge here is within 1% of the original's pixel count; its edges are a few
//! pixels off at the foot of the view (the original's right edge is wider), which is
//! recorded in the issue note and not asserted.
//!
//! `FFE_WORLD_CACHE=<data.bin> cargo test -p ffe-rs --test eagle_beam -- --ignored`

use ffe_rs::camera::ViewState;
use ffe_rs::combat;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::screen::Screen;

const VIEW_ROWS: usize = 158;

/// What the executed original draws for each laser, as (code, colour index of
/// the wedge in the view, its pixel count).
const WEDGES: [(u8, u32, usize); 3] = [
    (0x88, 0xe40404, 13_206),
    (0xb3, 0xe4e444, 13_206),
    (0xc6, 0x04e4e4, 41_746),
];

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_eagles_front_beam_is_the_originals_wedge() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let state = Flight {
        mode: Mode::Flight,
        system_flight: true,
        ship: 23,
        ..Flight::default()
    };
    let views = ViewState::default();
    let stats = viewer.ship_stats(state.ship);
    for (laser, colour, expected) in WEDGES {
        let mut screen = Screen::new(&cache).unwrap();
        screen.lasers = [laser, 0, 0, 0];
        screen.beam = Some(combat::beam(&state, stats.as_ref(), views.mode, [0, 0]));
        screen.draw(&state, views, Some(&viewer), true, "");
        let view = &screen.logical_pixels()[..VIEW_ROWS * 320];
        let wedge = view
            .iter()
            .filter(|pixel| **pixel & 0xffffff == colour)
            .count();
        assert!(
            wedge.abs_diff(expected) * 100 <= expected * 3,
            "laser {laser:#x}: {wedge} wedge pixels, the original draws {expected}"
        );
        // The apex is the crosshair's centre and the foot spans the whole view
        // (the mining laser's wedge is wider than the view and has no apex).
        let row = |y: usize| -> Vec<usize> {
            (0..320)
                .filter(|x| view[y * 320 + x] & 0xffffff == colour)
                .collect()
        };
        if laser != 0xc6 {
            let apex = row(80);
            assert!(
                apex.first().is_some_and(|x| (155..=160).contains(x))
                    && apex.last().is_some_and(|x| (158..=164).contains(x)),
                "laser {laser:#x}: the wedge does not start at the crosshair: {apex:?}"
            );
            let foot = row(157);
            assert_eq!(
                (foot.first().copied(), foot.last().copied()),
                (Some(0), Some(319)),
                "laser {laser:#x}: the wedge does not reach both lower corners"
            );
        }
    }
}
