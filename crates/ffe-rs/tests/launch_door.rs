//! Local-only: after a station launch the outer door stays open while the ship is still in
//! the slot (the navigator's `door_hold`) and closes once the game loop lets it go.
//!
//! `FFE_WORLD_CACHE=<data.bin> cargo test -p ffe-rs --test launch_door -- --ignored`
use ffe_rs::navigation::{Navigator, Traffic};

const FRAME_SECONDS: f64 = 0.021;
const FRAME_TICS: u32 = 1043;

#[test]
#[ignore = "Local-only: needs FFE_WORLD_CACHE"]
fn the_door_waits_open_for_the_launched_ship() {
    let cache = ffe_content::read(
        &std::fs::read(std::env::var_os("FFE_WORLD_CACHE").expect("FFE_WORLD_CACHE")).unwrap(),
    )
    .unwrap();
    let mut nav = Navigator::new_game(&cache, 3).unwrap();
    nav.begin_launch().unwrap();
    let mut done = None;
    for _ in 0..5000 {
        for event in nav.tick_traffic(FRAME_TICS, FRAME_SECONDS) {
            if let Traffic::LaunchDone(index) = event {
                done = Some(index);
            }
        }
        if done.is_some() {
            break;
        }
    }
    let station = done.expect("the launch completes");
    // The game loop holds the door as the launch completes.
    nav.door_hold = Some(station);
    assert_eq!(nav.door(station), 0, "the door is open");
    for _ in 0..400 {
        nav.tick_traffic(FRAME_TICS, FRAME_SECONDS);
    }
    assert_eq!(
        nav.door(station),
        0,
        "the door stays open while the ship is in the slot"
    );
    // Once the ship is clear the hold is dropped and the door closes behind it.
    nav.door_hold = None;
    for _ in 0..400 {
        nav.tick_traffic(FRAME_TICS, FRAME_SECONDS);
    }
    assert!(nav.door(station) > 0, "the door closes");
}
