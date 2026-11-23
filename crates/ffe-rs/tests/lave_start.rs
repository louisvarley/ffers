//! The Lave Station start (`--start 3`, F998's type 3) begins docked and must be
//! able to launch: a docking sequence left over from the start made
//! `begin_launch` answer "Already launching." so the Launch button did nothing.
//!
//! Local-only: needs the original game files.
//! `FFE_GAME=<game folder> cargo test -p ffe-rs --test lave_start -- --ignored`
//! (default: the checkout's `game` folder).

use ffe_rs::navigation::Navigator;

fn cache() -> ffe_content::VerifiedCache {
    let game = std::env::var_os("FFE_GAME").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game"),
        std::path::PathBuf::from,
    );
    ffe_content::read(&ffe_tools::cache::build_bytes(&game).unwrap()).unwrap()
}

#[test]
#[ignore = "Local-only: needs the original game files (FFE_GAME)"]
fn the_lave_start_is_docked_and_can_launch() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 3).unwrap();
    let index = nav.docked.expect("the Lave start is docked");
    assert_eq!(nav.bodies[index].kind, ffe_rs::navigation::Kind::Station);
    assert!(
        nav.sequence.is_none(),
        "docked is the end of the docking sequence"
    );
    nav.begin_launch().expect("the docked start can launch");
    let sequence = nav.sequence.expect("the launch stages begin");
    assert!(sequence.launching);
    assert_eq!(sequence.station, index);
}
