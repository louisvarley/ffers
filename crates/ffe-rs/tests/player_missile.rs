//! F1054/F1096's `M` launch, synthetic (no game-data cache): a new-game Eagle's
//! first pylon fires at the locked ship and the missile survives its launch
//! step next to the player's own object.
//!
//! Regression: the live game passes `ignore_launcher: false` when
//! `enhancement.missile_launch` is off, and the F718/F569 point contact test
//! used to honour that flag. The player's object is reported with F1686's cull
//! radius (17,600 mm for the Saker), which is wider than F941's launch
//! separation (9,116 mm), so the missile detonated on the player for 200 damage
//! on the first frame: `M` never produced a missile in flight.
use ffe_core::missile::{self, Target, TickOptions};
use ffe_rs::combat::Combat;
use ffe_rs::missile::MissileField;

fn axes() -> [[f64; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

/// The Saker Mk III's F1533 collision radius (`+0x140`, the launch separation)
/// and its F1686 cull radius (the player object's contact radius in `main`).
const LAUNCHER_RADIUS: i64 = 4_000;
const PLAYER_CONTACT_RADIUS: i64 = 17_600;

#[test]
fn m_fires_a_fitted_missile_at_a_locked_ship() {
    let mut combat = Combat::new_game();
    let locked = 7;
    let (pylon, code) = combat.first_missile().expect("two pylons are fitted");
    assert_eq!((pylon, code), (1, missile::PLAYER_MISSILE));

    // F941's launch: the pylon word is cleared and the object separates below
    // the ship (pylon 1 also pushes +0x2000 to the right).
    let spec = MissileField::spec(code, None, 2_100).expect("model 6");
    let mut field = MissileField::default();
    assert_eq!(combat.take_missile(pylon), Some(code));
    field.launch(
        spec,
        code,
        missile::PLAYER_TARGET,
        Some(locked),
        [0; 3],
        axes(),
        pylon,
        LAUNCHER_RADIUS,
    );
    assert_eq!(field.missiles.len(), 1);
    assert_eq!(field.missiles[0].target, Some(locked));
    assert_eq!(combat.first_missile().map(|(_, code)| code), Some(code));

    // One frame with config.cfg's defaults (`missile_launch` off, `missile_swept`
    // off): the player's own object is at the launch point with its cull radius.
    let player = Target {
        id: missile::PLAYER_TARGET,
        position: [0; 3],
        velocity: [0.0; 3],
        radius: PLAYER_CONTACT_RADIUS,
    };
    let mut ships = Vec::new();
    let events = field.tick_with_options(
        0.02,
        &mut ships,
        Some(player),
        &[],
        TickOptions {
            ignore_launcher: false,
            swept: false,
        },
    );
    assert!(
        events.is_empty(),
        "the missile hit its own launcher: {events:?}"
    );
    assert_eq!(
        field.missiles.len(),
        1,
        "the missile was consumed at launch"
    );
    assert!(field.missiles[0].alive());
}

#[test]
fn the_launcher_ray_enhancement_still_needs_the_toggle() {
    // enhancement-6 on, enhancement-2 off: the swept ray still meets the
    // launcher, because the ray is the one test the toggle guards.
    let spec = MissileField::spec(missile::PLAYER_MISSILE, None, 2_100).unwrap();
    let player = Target {
        id: missile::PLAYER_TARGET,
        position: [0; 3],
        velocity: [0.0; 3],
        radius: PLAYER_CONTACT_RADIUS,
    };
    let launch = |field: &mut MissileField| {
        field.launch(
            spec,
            missile::PLAYER_MISSILE,
            missile::PLAYER_TARGET,
            Some(7),
            [0; 3],
            axes(),
            1,
            LAUNCHER_RADIUS,
        );
    };
    let mut unguarded = MissileField::default();
    launch(&mut unguarded);
    let events = unguarded.tick_with_options(
        0.02,
        &mut Vec::new(),
        Some(player),
        &[],
        TickOptions {
            ignore_launcher: false,
            swept: true,
        },
    );
    assert!(matches!(
        events.as_slice(),
        [ffe_rs::missile::MissileEvent::HitPlayer { damage: 200, .. }]
    ));

    let mut guarded = MissileField::default();
    launch(&mut guarded);
    let events = guarded.tick_with_options(
        0.02,
        &mut Vec::new(),
        Some(player),
        &[],
        TickOptions {
            ignore_launcher: true,
            swept: true,
        },
    );
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(guarded.missiles.len(), 1);
}
