//! Local-only frames and checks for the hit particles, explosion and debris
//! (F939/F942/F926), plus the model-155 fragment gate.
//!
//! `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test
//! effects_view -- --ignored --nocapture`

use ffe_core::effects::{self, Kind};
use ffe_core::laser;
use ffe_core::npc::Role;
use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::{AnimationInputs, ModelViewer};
use ffe_rs::navigation::{Navigator, NEW_GAME_SHIP};
use ffe_rs::screen::{Screen, SystemShip};

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

fn save(screen: &Screen, name: &str) {
    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for pixel in screen.logical_pixels() {
            ppm.extend([(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join(name), ppm).unwrap();
    }
}

fn changed(before: &[u32], after: &[u32]) -> usize {
    before.iter().zip(after).filter(|(a, b)| a != b).count()
}

#[test]
#[ignore = "Local-only flight background render; needs FFE_WORLD_CACHE"]
fn flight_background_model315_reaches_frame() {
    use ffe_rs::camera::Camera;
    use ffe_rs::model_viewer::Shape;
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let body = nav
        .bodies
        .iter()
        .find(|body| body.kind == ffe_rs::navigation::Kind::Star)
        .expect("the start system has a star");
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let mut screen = Screen::new(&cache).unwrap();
    screen.discs = vec![ffe_rs::screen::Disc {
        system_type: Some(body.system_type),
        centre: body.position.map(|value| value as f64 / unit),
        radius: body.radius as f64 / unit,
        light: [0.0, 0.0, 1.0],
        colours: (17, 17),
        star: true,
        temperature: u16::try_from(body.temperature).ok(),
        model: Some((body.model, body.seed)),
        orientation: body.orientation,
    }];
    let mut state = Flight {
        mode: Mode::Flight,
        position: [0.0; 3],
        velocity: [0.0, 0.0, 100.0],
        system_flight: true,
        ship: NEW_GAME_SHIP,
        ..Flight::default()
    };
    let mut start_axes = nav.bodies[nav.docked.unwrap()].orientation;
    start_axes[0] = start_axes[0].map(|value| -value);
    start_axes[2] = start_axes[2].map(|value| -value);
    state.axes = Some(start_axes);
    let views = ViewState::default();
    let scale = ffe_rs::screen::model_scale(&viewer).unwrap();
    let camera = Camera::new(&state, &views, None, scale);
    let rows = body.orientation.map(|axis| camera.direction(axis));
    let origin = camera
        .direction(camera.axes[2])
        .map(|component| component * 0x2000 as f64);
    let mut inputs = AnimationInputs::original(0, 0, &viewer.shades());
    inputs.depth_detail = true;
    let frame = viewer
        .render_scene_with(0x13b, rows, origin, viewer.light_direction(), &inputs)
        .unwrap();
    assert!(frame.shapes.iter().any(|shape| matches!(
        shape,
        Shape::Textured(t) if matches!(t.bitmap & 0xfff, 53..=58)
    )));
    assert!(frame.shapes.iter().any(|shape| matches!(
        shape,
        Shape::Spans(spans, _) if !spans.is_empty()
    )));

    screen.draw(&state, views, Some(&viewer), true, "");
    let with_background = screen.logical_pixels().to_vec();
    save(&screen, "flight-background-source-transform.ppm");

    screen.stars_visible = false;
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    let without_background = screen.logical_pixels();
    assert!(
        changed(&with_background, without_background) > 0,
        "F1775's model 315 background did not reach the flight frame"
    );

    // The Graphic Detail page's "Black" leaves the stars drawn over the black sky.
    screen.sky = ffe_rs::prefs::space_colour_rgb(0);
    screen.stars_visible = true;
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    let black_with = screen.logical_pixels().to_vec();
    save(&screen, "flight-background-black-sky.ppm");
    screen.stars_visible = false;
    screen.draw(&state, ViewState::default(), Some(&viewer), true, "");
    let view = 320 * 158;
    let stars = changed(&black_with[..view], &screen.logical_pixels()[..view]);
    eprintln!("black sky: {stars} star pixels");
    assert!(stars > 0, "the stars are not drawn over black space");
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(v: [f64; 3]) -> [f64; 3] {
    let length = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    v.map(|x| x / length)
}

/// The ship axes that look from `from` at `to` (flight units).
fn look_at(from: [f64; 3], to: [f64; 3]) -> [[f64; 3]; 3] {
    let forward = normalise(std::array::from_fn(|i| to[i] - from[i]));
    let right = normalise(cross([0.0, 1.0, 0.0], forward));
    let up = cross(forward, right);
    [right, up, forward]
}

fn stage(screen: &mut Screen, nav: &Navigator) {
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    screen.ships = nav
        .ships
        .iter()
        .map(|ship| SystemShip {
            model: ship.model,
            centre: ship.position.map(|v| v as f64 / unit),
            axes: ship.axes(),
            flash: ship.flash > 0.0,
            dead: !ship.alive(),
        })
        .collect();
    screen.effects = nav
        .effects
        .iter()
        .map(|effect| ffe_rs::screen::SystemEffect::from_effect(None, effect, unit))
        .collect();
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn hit_particle_explosion_and_frames() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    // F829/F998's new game has no traffic; the capture asks F688 for the
    // guaranteed pirate at the station's launch point.
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.respawn_traffic(&cache, at, forward);
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let index = nav
        .ships
        .iter()
        .position(|ship| ship.role == Role::Pirate)
        .unwrap_or(0);
    let target = nav.ships[index].position;
    // 20 m from the target, looking along +z at it.
    let from_mm = [target[0], target[1], target[2] - 20_000];
    let from = from_mm.map(|v| v as f64 / unit);
    let mut state = Flight {
        mode: Mode::Flight,
        position: from,
        velocity: [0.0; 3],
        system_flight: true,
        ship: NEW_GAME_SHIP,
        ..Flight::default()
    };
    state.axes = Some(look_at(from, target.map(|v| v as f64 / unit)));
    let views = ViewState::default();
    nav.lock_ship(nav.ships[index].id);
    stage(&mut screen, &nav);
    screen.draw(&state, views, Some(&viewer), true, "");
    let empty = screen.logical_pixels().to_vec();

    // F939: one model-0x9e particle at the hit point.
    let id = nav.ships[index].id;
    // F939 spawns the particle at F817's hit point on the interaction box, not
    // at the ship's centre: put it a few metres off the hull.
    let hit_point = [target[0], target[1] + 2_000, target[2]];
    assert_eq!(
        nav.damage_ship_at(id, 50, Some(hit_point))
            .map(|attack| attack.damage),
        Some(laser::Damage::Shields)
    );
    assert_eq!(
        nav.effects
            .iter()
            .filter(|effect| effect.kind == Kind::Hit)
            .count(),
        1
    );
    let particle = nav.effects.iter().find(|e| e.kind == Kind::Hit).unwrap();
    assert_eq!(particle.model, effects::HIT_MODEL);
    // The hit particle sits on the ship's hull and the shield flash is on.
    assert!(nav.ships[index].flash > 0.0);
    stage(&mut screen, &nav);
    screen.draw(&state, views, Some(&viewer), true, "");
    save(&screen, "effects-hit.ppm");
    let hit_frame = screen.logical_pixels().to_vec();
    assert!(
        changed(&empty, &hit_frame) > 20,
        "the hit frame did not change"
    );
    // At 20 m model 158 is a small spark: its shapes are one screen pixel, so
    // the frame check is the shield flash around it. The model itself is
    // checked by `model_155_fragments_are_gated_on_global_4`.
    assert!(!screen.effects.is_empty());
    // F942/F926: the kill spawns the model-0x9b explosion and F926 debris.
    assert_eq!(
        // Enough for any hull the spawn picks (the shared random pair
        // chooses the model).
        nav.damage_ship_at(id, 1_000_000, Some(hit_point))
            .map(|attack| attack.damage),
        Some(laser::Damage::Destroyed)
    );
    assert!(
        nav.effects
            .iter()
            .any(|effect| effect.kind == Kind::Explosion),
        "F942 spawned no explosion object"
    );
    // F926's pod counts are random and can both be zero (F922(6) - 2 and
    // F922(min(4, cargo >> 1)) - 1); any pod carries its cargo for F714.
    assert!(nav
        .effects
        .iter()
        .filter(|effect| effect.kind == Kind::Debris)
        .all(|effect| effect.cargo.is_some()));
    stage(&mut screen, &nav);
    screen.draw(&state, views, Some(&viewer), true, "");
    save(&screen, "effects-kill.ppm");
    let kill_frame = screen.logical_pixels().to_vec();
    assert!(
        changed(&hit_frame, &kill_frame) > 3,
        "the kill frame did not change"
    );

    // The effects live for their own lifetimes (debris the longest), then the
    // frame is clean again.
    // F712 sparks a pod for seven day ticks; its last sparks live 2.6 s.
    for _ in 0..((7.0 + 3.0) / 0.05) as usize {
        nav.tick_ships(0.05, 2_485, from_mm, 10_000);
    }
    assert!(
        nav.effects.iter().all(|effect| effect.kind == Kind::Debris),
        "the explosion did not expire: {:?}",
        nav.effects
            .slots()
            .map(|(slot, effect)| (slot, effect.kind, effect.timer, effect.countdown))
            .collect::<Vec<_>>()
    );
    stage(&mut screen, &nav);
    screen.draw(&state, views, Some(&viewer), true, "");
    save(&screen, "effects-kill-after.ppm");
    let after = screen.logical_pixels().to_vec();
    assert!(
        changed(&kill_frame, &after) > 3,
        "the explosion did not disappear"
    );
}

/// F924/F928: the navigator keeps the source's effect slots, so a hit
/// particle's freed slot is the one the next particle takes.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn f924_a_freed_effect_slot_is_reused() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.respawn_traffic(&cache, at, forward);
    let id = nav
        .ships
        .iter()
        .find(|ship| ship.alive())
        .map(|ship| ship.id)
        .expect("a live ship to hit");
    nav.effects.clear();
    // F939's particle takes F928's topmost free slot.
    assert!(nav.damage_ship(id, 1).is_some());
    assert_eq!(hit_slot(&nav), Some(0x72));
    assert_eq!(nav.effects.counted(), 1, "a hit is a counted +0x20 object");
    // F709 ends the particle within 2.6 s (0xffff at 497 a 20 ms update).
    for _ in 0..150 {
        nav.tick_effects(0.02, 994);
    }
    assert!(
        nav.effects.is_empty(),
        "F924 did not free the dead particle"
    );
    // The next F939 object takes the freed slot, not a fresh one.
    assert!(nav.damage_ship(id, 1).is_some());
    assert_eq!(hit_slot(&nav), Some(0x72));
    assert_eq!(nav.effects.counted(), 1);
}

fn hit_slot(nav: &Navigator) -> Option<usize> {
    nav.effects
        .slots()
        .find(|(_, effect)| effect.kind == Kind::Hit)
        .map(|(slot, _)| slot)
}

/// A model-155 fragment (model 320, one of F942's twelve child calls) is an
/// F1721 complex polygon. Drawn by the F6 viewer it must reach the F1560/F1559/
/// F1613 span rasteriser and cover pixels, not only its outline. At 2,000 units
/// the fragment is large enough to queue polygons rather than the textured path.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn explosion_fragment_polygons_fill() {
    use ffe_core::model_fill::fill_polygon;
    use ffe_rs::model_viewer::Shape;
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let rows = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let frame = viewer
        .render_scene_with(
            320,
            rows,
            [0.0, 0.0, 2_000.0],
            [0.0, 0.0, -1.0],
            &AnimationInputs::original(0, 0, &viewer.shades()),
        )
        .unwrap();
    let polygons: Vec<_> = frame
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Polygon(edges, _) => Some(edges),
            _ => None,
        })
        .collect();
    assert!(
        !polygons.is_empty(),
        "the model-155 fragment must queue polygons, got {:?}",
        frame
            .shapes
            .iter()
            .map(std::mem::discriminant)
            .collect::<Vec<_>>()
    );
    let mut filled = 0usize;
    for edges in polygons {
        let before = filled;
        fill_polygon(edges, |_, _| filled += 1).unwrap();
        assert!(filled > before, "the polygon span fill wrote no pixels");
    }
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn model_155_fragments_are_gated_on_global_4() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let rows = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let draw = |mass: u16| {
        let mut inputs = AnimationInputs::original(0, 0, &viewer.shades());
        inputs.globals[effects::EXPLOSION_MASS_GLOBAL] = mass;
        viewer
            .render_scene_with(
                effects::EXPLOSION_MODEL,
                rows,
                [0.0, 0.0, 1.0e6],
                [0.0, 0.0, -1.0],
                &inputs,
            )
            .map(|frame| frame.shapes.len())
    };
    // F942 writes 0xffffffff into +0xa4: the model selects no fragment.
    assert_eq!(draw(0xffff), Ok(0));
    // FFP 1.0 selects the twelve fragment calls.
    assert!(draw(0x4000).is_ok_and(|shapes| shapes >= 12));
    // Over F708's countdown the explosion shows fragments for most of its
    // life.
    let lit = (0..=0xffffu32)
        .step_by(0x100)
        .filter(|timer| draw(*timer as u16).is_ok_and(|shapes| shapes > 0))
        .count();
    eprintln!("model 155 draws at {lit} of 256 timer steps");
    assert!(lit > 64, "{lit}");
    // The fireball (320/322/323 -> 329..331) is a lit call inside an unlit
    // child; every child must run so the orange fire is drawn.
    for timer in [0x8000u16, 0xc000, 0xe69b] {
        let mut inputs = AnimationInputs::original(0, 0, &viewer.shades());
        inputs.globals[effects::EXPLOSION_MASS_GLOBAL] = timer;
        let frame = viewer
            .render_scene_with(
                effects::EXPLOSION_MODEL,
                rows,
                [0.0, 0.0, 3000.0],
                [0.0, 0.0, -1.0],
                &inputs,
            )
            .unwrap();
        assert_eq!(frame.children_error, None, "timer {timer:#x}");
    }
    // The hit particle (model 0x9e) is not gated and draws its six shapes.
    let frame = viewer
        .render_scene(
            effects::HIT_MODEL,
            rows,
            [0.0, 0.0, 1.0e6],
            [0.0, 0.0, -1.0],
        )
        .unwrap();
    assert!(frame.shapes.len() >= 6, "{}", frame.shapes.len());
}

/// F1035's dust is model 0x6d drawn with its distance word (`+0x7e`, -7..-4)
/// as a shift on the model's own scale: each cloud is 0.5..4 km of F1766
/// chain dots ahead of the ship. At the model's native scale (65 km) the
/// camera sits inside every cloud and almost nothing is drawn.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn dust_is_drawn_ahead_at_speed() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let sine = ffe_core::model_math::SineTable::from_bytes(cache.asset("base:math/sine").unwrap())
        .unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let speed_mm = 5_000_000.0;
    let velocity = (speed_mm / ffe_rs::original_flight::MM_PER_SECOND) as i64;
    let mut position = [7.0e15f64, 3.0e15, 1.0e15];
    for _ in 0..50 {
        position[2] += speed_mm * 0.02;
        let at = position.map(|v| v as i64);
        nav.tick_dust(
            0.02,
            &ffe_core::dust::Spawn::deep_space(at, [0, 0, velocity], true),
        );
    }
    let state = Flight {
        mode: Mode::Flight,
        position: position.map(|v| v / unit),
        system_flight: true,
        ship: NEW_GAME_SHIP,
        axes: Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        ..Flight::default()
    };
    let views = ViewState::default();
    screen.draw(&state, views, Some(&viewer), true, "");
    let empty = screen.logical_pixels().to_vec();
    screen.effects = nav
        .dust
        .particles()
        .iter()
        .map(|particle| {
            let rows = particle.orientation(&sine);
            ffe_rs::screen::SystemEffect {
                model: particle.model,
                centre: particle.position.map(|v| v as f64 / unit),
                axes: std::array::from_fn(|r| {
                    std::array::from_fn(|c| f64::from(rows[3 * r + c]) / 2_147_483_648.0)
                }),
                timer: 0,
                size: 0,
                scale: viewer
                    .find(particle.model)
                    .map(|model| (model.scale as i32 + particle.distance_bias).max(0) as u32),
            }
        })
        .collect();
    screen.draw(&state, views, Some(&viewer), true, "");
    save(&screen, "effects-dust.ppm");
    let dots = changed(&empty, screen.logical_pixels());
    assert!(dots > 50, "only {dots} dust pixels");
}

/// Model 0x6d's chains are detail tiers (opcodes 11/12 through F1751): the original compares
/// each against the cloud's own view depth and scale, so a far cloud draws only its first
/// dots and a near one all of them. The viewer's default (nearest depth, full detail) drew
/// every cloud's densest tier, which clumped the far clouds' dots.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn dust_clouds_drop_dots_with_distance() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let model = viewer.find(ffe_core::dust::MODEL).unwrap();
    let scale = model.scale.saturating_sub(5);
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let dots = |z: f64, depth_detail: bool| {
        let mut inputs = AnimationInputs::original(0, 0, &viewer.shades());
        inputs.depth_detail = depth_detail;
        viewer
            .render_scene_with_scale(
                ffe_core::dust::MODEL,
                Some(scale),
                identity,
                [0.0, 0.0, z],
                [0.0, 0.0, 1.0],
                &inputs,
            )
            .unwrap()
            .shapes
            .len()
    };
    // A far cloud keeps only its first chain (8 dots); the viewer default draws every tier.
    let (full, far) = (dots(1.0e9, false), dots(1.0e9, true));
    assert_eq!(far, 8);
    assert!(full > far, "full {full}, far {far}");
}

/// At cruising speed the dust must stay on screen at every time step. F1517 leads each cloud
/// `2^shift` times farther ahead, which at 1000x and 10000x is beyond the viewer's 32-bit view
/// space; the original shifts such an object's view position and scale down together (F1680).
/// Dropping those clouds left no dust at the top steps.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn dust_stays_on_screen_at_every_time_step() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let sine = ffe_core::model_math::SineTable::from_bytes(cache.asset("base:math/sine").unwrap())
        .unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let speed_mm = 3_000_000.0;
    let velocity = (speed_mm / ffe_rs::original_flight::MM_PER_SECOND) as i64;
    for step in 0..5 {
        let mut nav = Navigator::new_game(&cache, 1).unwrap();
        let mut screen = Screen::new(&cache).unwrap();
        let mut position = [7.0e15f64, 3.0e15, 1.0e15];
        let (mut lit, mut frames) = (0, 0);
        for frame in 0..400 {
            // The ship goes 1, 10, 100, 1000 and 10000 times as far each frame.
            position[2] += speed_mm * [1.0, 10.0, 100.0, 1000.0, 10000.0][step] * 0.02;
            let mut spawn = ffe_core::dust::Spawn::deep_space(
                position.map(|v| v as i64),
                [0, 0, velocity],
                true,
            );
            spawn.speed = ffe_core::dust::ACCELERATION_SHIFT[step];
            nav.tick_dust(0.02, &spawn);
            if frame < 200 || frame % 5 != 0 {
                continue;
            }
            let state = Flight {
                mode: Mode::Flight,
                position: position.map(|v| v / unit),
                system_flight: true,
                ship: NEW_GAME_SHIP,
                axes: Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                ..Flight::default()
            };
            let views = ViewState::default();
            screen.effects.clear();
            screen.draw(&state, views, Some(&viewer), true, "");
            let empty = screen.logical_pixels().to_vec();
            screen.effects = nav
                .dust
                .particles()
                .iter()
                .map(|particle| {
                    let rows = particle.orientation(&sine);
                    ffe_rs::screen::SystemEffect {
                        model: particle.model,
                        centre: particle.position.map(|v| v as f64 / unit),
                        axes: std::array::from_fn(|r| {
                            std::array::from_fn(|c| f64::from(rows[3 * r + c]) / 2_147_483_648.0)
                        }),
                        timer: 0,
                        size: 0,
                        scale: viewer.find(particle.model).map(|model| {
                            (model.scale as i32 + particle.distance_bias).max(0) as u32
                        }),
                    }
                })
                .collect();
            screen.draw(&state, views, Some(&viewer), true, "");
            if changed(&empty, screen.logical_pixels()) > 5 {
                lit += 1;
            }
            frames += 1;
        }
        eprintln!("step {step}: dust on screen in {lit} of {frames} frames");
        assert!(
            lit * 2 > frames,
            "step {step}: dust in only {lit} of {frames} frames"
        );
    }
}

/// The original queues every object's primitives and sorts the whole frame's queue by depth
/// key (F1680, F1789), so a ship in front of a dust cloud hides the dots behind it. Classic
/// once drew the effects in a pass after the ships and put the dust over them.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_ship_hides_the_dust_behind_it() {
    let cache = cache();
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let viewer = ModelViewer::new(&cache).unwrap();
    let sine = ffe_core::model_math::SineTable::from_bytes(cache.asset("base:math/sine").unwrap())
        .unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    // A cruise at 3 km/s: the clouds sit about 4 km ahead, all behind the ship below.
    let speed_mm = 3_000_000.0;
    let velocity = (speed_mm / ffe_rs::original_flight::MM_PER_SECOND) as i64;
    let mut position = [7.0e15f64, 3.0e15, 1.0e15];
    for _ in 0..100 {
        position[2] += speed_mm * 0.02;
        nav.tick_dust(
            0.02,
            &ffe_core::dust::Spawn::deep_space(position.map(|v| v as i64), [0, 0, velocity], true),
        );
    }
    let state = Flight {
        mode: Mode::Flight,
        position: position.map(|v| v / unit),
        system_flight: true,
        ship: NEW_GAME_SHIP,
        axes: Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        ..Flight::default()
    };
    // A ship 40 m dead ahead (mm / 9700 per flight unit), filling the middle of the view.
    let mut ahead = position.map(|v| v / unit);
    ahead[2] += 40_000.0 / unit;
    let views = ViewState::default();
    let mut screen = Screen::new(&cache).unwrap();
    screen.ships = vec![SystemShip {
        model: NEW_GAME_SHIP,
        centre: ahead,
        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        flash: false,
        dead: false,
    }];
    screen.draw(&state, views, Some(&viewer), true, "");
    let ship_only = screen.logical_pixels().to_vec();
    screen.ships.clear();
    screen.draw(&state, views, Some(&viewer), true, "");
    let empty = screen.logical_pixels().to_vec();
    let covered: Vec<usize> = (0..320 * 158)
        .filter(|&i| ship_only[i] != empty[i])
        .collect();
    assert!(
        covered.len() > 500,
        "the ship covers only {} pixels",
        covered.len()
    );
    screen.ships = vec![SystemShip {
        model: NEW_GAME_SHIP,
        centre: ahead,
        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        flash: false,
        dead: false,
    }];
    screen.effects = nav
        .dust
        .particles()
        .iter()
        .map(|particle| {
            let rows = particle.orientation(&sine);
            ffe_rs::screen::SystemEffect {
                model: particle.model,
                centre: particle.position.map(|v| v as f64 / unit),
                axes: std::array::from_fn(|r| {
                    std::array::from_fn(|c| f64::from(rows[3 * r + c]) / 2_147_483_648.0)
                }),
                timer: 0,
                size: 0,
                scale: viewer
                    .find(particle.model)
                    .map(|model| (model.scale as i32 + particle.distance_bias).max(0) as u32),
            }
        })
        .collect();
    screen.draw(&state, views, Some(&viewer), true, "");
    let both = screen.logical_pixels().to_vec();
    save(&screen, "ship-and-dust.ppm");
    let dust_pixels = changed(&ship_only, &both);
    assert!(dust_pixels > 5, "no dust is drawn at all");
    let over_ship = covered.iter().filter(|&&i| ship_only[i] != both[i]).count();
    assert_eq!(
        over_ship, 0,
        "{over_ship} dust pixels were drawn over the ship"
    );
}

/// Under time acceleration `DATA_009123` is the step's shift: the dust is led the same real
/// time ahead and its clouds grow with it, so at 1000x it is still drawn, spread across the
/// view. With the shift left at 0 (the port's old input) every cloud was culled as too far.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn dust_keeps_up_under_time_acceleration() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let sine = ffe_core::model_math::SineTable::from_bytes(cache.asset("base:math/sine").unwrap())
        .unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let speed_mm = 5_000_000.0;
    let velocity = (speed_mm / ffe_rs::original_flight::MM_PER_SECOND) as i64;
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let mut position = [7.0e15f64, 3.0e15, 1.0e15];
    for _ in 0..100 {
        // 1000x: the ship goes a thousand times as far each 20 ms frame.
        position[2] += speed_mm * 1000.0 * 0.02;
        let mut spawn =
            ffe_core::dust::Spawn::deep_space(position.map(|v| v as i64), [0, 0, velocity], true);
        spawn.speed = ffe_core::dust::ACCELERATION_SHIFT[3];
        nav.tick_dust(0.02, &spawn);
    }
    let state = Flight {
        mode: Mode::Flight,
        position: position.map(|v| v / unit),
        system_flight: true,
        ship: NEW_GAME_SHIP,
        axes: Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        ..Flight::default()
    };
    let views = ViewState::default();
    let mut screen = Screen::new(&cache).unwrap();
    screen.draw(&state, views, Some(&viewer), true, "");
    let empty = screen.logical_pixels().to_vec();
    screen.effects = nav
        .dust
        .particles()
        .iter()
        .map(|particle| {
            let rows = particle.orientation(&sine);
            ffe_rs::screen::SystemEffect {
                model: particle.model,
                centre: particle.position.map(|v| v as f64 / unit),
                axes: std::array::from_fn(|r| {
                    std::array::from_fn(|c| f64::from(rows[3 * r + c]) / 2_147_483_648.0)
                }),
                timer: 0,
                size: 0,
                scale: viewer
                    .find(particle.model)
                    .map(|model| (model.scale as i32 + particle.distance_bias).max(0) as u32),
            }
        })
        .collect();
    screen.draw(&state, views, Some(&viewer), true, "");
    let columns: Vec<usize> = empty
        .iter()
        .zip(screen.logical_pixels())
        .enumerate()
        .take(320 * 158)
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i % 320)
        .collect();
    let (left, right) = (
        columns.iter().min().copied().unwrap_or(0),
        columns.iter().max().copied().unwrap_or(0),
    );
    assert!(columns.len() > 10, "only {} dust pixels", columns.len());
    assert!(
        right - left > 100,
        "dust spread over columns {left}..{right}"
    );
}

/// F738's departure cloud and F1094's exit cloud: model 0x9a at distance word
/// +8, global word 1 the size ramp. Both draw ahead of the camera, and a cloud
/// expires once its game time is spent (F763/F764).
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn hyperspace_clouds_draw_and_expire() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let at = [7_000_000_000_000_i64, 3_000_000_000_000, 1_000_000_000_000];
    let state = Flight {
        mode: Mode::Flight,
        position: at.map(|v| v as f64 / unit),
        system_flight: true,
        ship: NEW_GAME_SHIP,
        axes: Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        ..Flight::default()
    };
    let views = ViewState::default();
    screen.draw(&state, views, Some(&viewer), true, "");
    let empty = screen.logical_pixels().to_vec();
    let ahead = [at[0], at[1], at[2] + 20_000_000];
    for (name, mut cloud) in [
        (
            "departure",
            effects::departure_cloud(ahead, effects::TICS_PER_DAY),
        ),
        (
            "exit",
            effects::exit_cloud(ahead, u64::from(effects::THINK_TICS)),
        ),
    ] {
        // Half a second of F735's ramp.
        for _ in 0..25 {
            cloud.tick(0.02, 994);
        }
        screen.effects = vec![ffe_rs::screen::SystemEffect::from_effect(
            Some(&viewer),
            &cloud,
            unit,
        )];
        screen.draw(&state, views, Some(&viewer), true, "");
        save(&screen, &format!("effects-cloud-{name}.ppm"));
        let drawn = changed(&empty, screen.logical_pixels());
        assert!(drawn > 50, "{name} cloud drew {drawn} pixels");
        assert!(cloud.alive(), "{name} cloud expired early");
        for _ in 0..100 {
            cloud.tick(0.02, 994);
        }
        assert_eq!(cloud.alive(), name == "departure", "{name} cloud lifetime");
    }
}

/// F1751 adds the detail words `DATA_008812/8813` to the cloud's scale before comparing its
/// tiers with the view depth. The executed original reads `1, 1` (the default High radio), not
/// the `0, 0` the viewer assumed, so a mid-depth cloud draws more dots than at `0, 0`; at
/// `-1, -2` (Low) it never draws more.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn dust_detail_follows_the_shape_detail_words() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let scale = viewer.find(ffe_core::dust::MODEL).unwrap().scale;
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let dots = |z: f64, detail: [i32; 2]| {
        let mut inputs = AnimationInputs::original(0, 0, &viewer.shades());
        inputs.depth_detail = true;
        inputs.detail = detail;
        viewer
            .render_scene_with_scale(
                ffe_core::dust::MODEL,
                Some(scale),
                identity,
                [0.0, 0.0, z],
                [0.0, 0.0, 1.0],
                &inputs,
            )
            .unwrap()
            .shapes
            .len()
    };
    let (mut more, mut fewer) = (0, 0);
    for step in 0..30 {
        let z = 1.0e6 * 1.25f64.powi(step);
        let (low, medium, high) = (dots(z, [-1, -2]), dots(z, [0, 0]), dots(z, [1, 1]));
        more += usize::from(high > medium);
        fewer += usize::from(high < medium || medium < low);
    }
    assert!(more > 0, "High never drew more dots than Medium");
    assert_eq!(fewer, 0, "a higher detail level drew fewer dots");
}

/// A fast ship's clouds are big: F1035's `+0x7e` word includes F1099's speed class
/// (`DATA_008865`), so at 9,994 km/s each cloud is thousands of kilometres across against a
/// lead of 13,000 km, and the dust fills the view instead of bunching round the line of flight.
/// Measured in the executed original at that speed: 45 clouds, dots on every part of the
/// screen, about one in five a "+" (a size-2 particle). Local-only: needs the cache.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn dust_fills_the_view_at_ten_thousand_kilometres_a_second() {
    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    let sine = ffe_core::model_math::SineTable::from_bytes(cache.asset("base:math/sine").unwrap())
        .unwrap();
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let speed_mm = 9_994.0e6;
    let velocity = (speed_mm / ffe_rs::original_flight::MM_PER_SECOND) as i64;
    let mut nav = Navigator::new_game(&cache, 1).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    let mut position = [7.0e15f64, 3.0e15, 1.0e15];
    for _ in 0..300 {
        position[2] += speed_mm * 0.02;
        nav.tick_dust(
            0.02,
            &ffe_core::dust::Spawn::deep_space(position.map(|v| v as i64), [0, 0, velocity], true),
        );
    }
    assert!(
        nav.dust.particles().len() > 20,
        "only {} clouds alive",
        nav.dust.particles().len()
    );
    let state = Flight {
        mode: Mode::Flight,
        position: position.map(|v| v / unit),
        system_flight: true,
        ship: NEW_GAME_SHIP,
        axes: Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        ..Flight::default()
    };
    let views = ViewState::default();
    screen.draw(&state, views, Some(&viewer), true, "");
    let empty = screen.logical_pixels().to_vec();
    screen.effects = nav
        .dust
        .particles()
        .iter()
        .map(|particle| {
            let rows = particle.orientation(&sine);
            ffe_rs::screen::SystemEffect {
                model: particle.model,
                centre: particle.position.map(|v| v as f64 / unit),
                axes: std::array::from_fn(|r| {
                    std::array::from_fn(|c| f64::from(rows[3 * r + c]) / 2_147_483_648.0)
                }),
                timer: 0,
                size: 0,
                scale: viewer
                    .find(particle.model)
                    .map(|model| (model.scale as i32 + particle.distance_bias).max(0) as u32),
            }
        })
        .collect();
    screen.draw(&state, views, Some(&viewer), true, "");
    save(&screen, "effects-dust-fast.ppm");
    let after = screen.logical_pixels().to_vec();
    let lit: Vec<usize> = (0..empty.len())
        .filter(|i| empty[*i] != after[*i])
        .collect();
    assert!(lit.len() > 60, "only {} dust pixels", lit.len());
    // Outside the middle fifth of the width and third of the height.
    let outside = lit
        .iter()
        .filter(|i| {
            let (x, y) = (*i % 320, *i / 320);
            !((128..192).contains(&x) && (53..105).contains(&y))
        })
        .count();
    assert!(
        outside * 10 > lit.len() * 8,
        "dust bunches at the centre: {outside} of {} pixels are outside it",
        lit.len()
    );
}
