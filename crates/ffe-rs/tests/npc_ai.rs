//! Local-only sequence for the NPC flight/combat AI: a pirate fires its 1MW
//! pulse at the player (F931/F930/F953), the player's hull takes the damage, and
//! a clean kill is witnessed by the starport, whose F238 then sends police.
//!
//! `FFE_WORLD_CACHE=<data.bin> FFE_SHOT_DIR=<dir> cargo test -p ffe-rs --test
//! npc_ai -- --ignored --nocapture`
//!
//! Frames (when `FFE_SHOT_DIR` is set): `npc-ai-before.ppm` (the pirate in front
//! of the player), `npc-ai-hit.ppm` (the damaged player), `npc-ai-police.ppm`
//! (the wanted player and the hostile police) and `npc-ai-restored.ppm` (a pirate
//! carried through a jump).

use ffe_core::laser;
use ffe_core::npc::{Role, State};
use ffe_rs::camera::ViewState;
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::model_viewer::ModelViewer;
use ffe_rs::navigation::{Navigator, ShipEvent, DEFAULT_CASH, DEFAULT_SHIP, NEW_GAME_SHIP};
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

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(v: [f64; 3]) -> [f64; 3] {
    let length = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-9);
    v.map(|x| x / length)
}

fn look_at(from: [f64; 3], to: [f64; 3]) -> [[f64; 3]; 3] {
    let forward = normalise(std::array::from_fn(|i| to[i] - from[i]));
    let right = normalise(cross([0.0, 1.0, 0.0], forward));
    let up = cross(forward, right);
    [right, up, forward]
}

fn ships(screen: &mut Screen, nav: &Navigator) {
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
}

/// A frame of the player at `player` (system mm) looking at the pirate.
fn frame(
    screen: &mut Screen,
    viewer: &ModelViewer,
    state: &Flight,
    nav: &Navigator,
    message: &str,
) {
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    ships(screen, nav);
    screen.labels = nav
        .ships
        .iter()
        .map(|ship| ffe_rs::screen::BodyLabel {
            name: ship.name.clone(),
            centre: ship.position.map(|v| v as f64 / unit),
        })
        .collect();
    screen.lasers = [laser::PULSE_1MW, 0, 0, 0];
    let mut views = ViewState::default();
    views.console.labels = true;
    screen.draw(state, views, Some(viewer), true, message);
}

fn setup(cache: &ffe_content::VerifiedCache) -> (Navigator, ModelViewer, Screen, Flight) {
    let mut nav = Navigator::new_game(cache, 1).unwrap();
    let viewer = ModelViewer::new(cache).unwrap();
    let screen = Screen::new(cache).unwrap();
    // The new game's pad has no traffic (F829/F998 do not call F688). This
    // fixture is the scripted flight capture, so ask F688 for the port's
    // guaranteed trader/pirate group at the station's launch point.
    let at = nav.station().map(|(_, at)| at).expect("a start station");
    let forward = nav.bodies[nav.docked.expect("docked")].orientation[2];
    nav.respawn_traffic(cache, at, forward);
    assert!(!nav.ships.is_empty(), "the capture spawned no traffic");
    // The player has launched: hostile ships hold fire at a docked player.
    nav.docked = None;
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    // The player sits 300 km in front of the pirate, looking back at it. The
    // pirate's default forward is +z, so it is already aimed at the player: its
    // first F931 shot lands without a turn. (The port's renderer draws NPC models
    // in flight units, so the evidence frame needs the ship this close.)
    let pirate = nav.ships.iter().find(|s| s.role == Role::Pirate).unwrap();
    let at = pirate.position;
    let player = [at[0], at[1], at[2] + 80_000];
    let mut state = Flight {
        mode: Mode::Flight,
        position: player.map(|v| v as f64 / unit),
        velocity: [0.0; 3],
        system_flight: true,
        ship: NEW_GAME_SHIP,
        ..Flight::default()
    };
    state.axes = Some(look_at(state.position, at.map(|v| v as f64 / unit)));
    (nav, viewer, screen, state)
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_pirate_fires_and_the_player_takes_f953_damage() {
    let cache = cache();
    let (mut nav, viewer, mut screen, mut state) = setup(&cache);
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let player = state.position.map(|v| (v * unit).round() as i64);
    let radius = viewer.model_radius(NEW_GAME_SHIP).unwrap();
    let radius = i64::from(radius);
    // The player's hull starts untracked; the first hit records mass * 4.
    let mass = viewer.ship_stats(NEW_GAME_SHIP).unwrap().mass;

    frame(&mut screen, &viewer, &state, &nav, "A pirate closes.");
    save(&screen, "npc-ai-before.ppm");

    let mut first: Option<(String, u32, bool, laser::Damage)> = None;
    'frames: for _ in 0..600 {
        let events = nav.tick_ships(0.02, 994, player, radius);
        for event in events {
            if let ShipEvent::Hit {
                name, damage, beam, ..
            } = event
            {
                let outcome = state.take_damage(damage, mass);
                let message = format!("{name} hits: hull {}%", state.hull[0]);
                screen.hud.hull = state.hull;
                frame(&mut screen, &viewer, &state, &nav, &message);
                save(&screen, "npc-ai-hit.ppm");
                first = Some((name, damage, beam, outcome));
                break 'frames;
            }
        }
    }
    let (name, damage, beam, outcome) = first.expect("the pirate never hit the player");
    assert_eq!(
        damage,
        50,
        "{name} fired a {} laser",
        if beam { "beam" } else { "pulse" }
    );
    assert!(!beam);
    assert_eq!(outcome, laser::Damage::Hull);
    let full = mass * 4;
    assert_eq!(state.hull, [full - 50, full]);
    // A second 1MW pulse uses up a 25 t hull (F953 then F942).
    assert_eq!(state.take_damage(50, mass), laser::Damage::Destroyed);
    assert!(state.destroyed());
    // The hull line the HUD draws is the damaged one, not the model's default;
    // the hits started F1102's 30-second line.
    assert_eq!(state.hull_display, 30);
    screen.hud.hull_shown = state.hull_display != 0;
    screen.hud.hull = [0, full];
    frame(&mut screen, &viewer, &state, &nav, "Hull damaged.");
    let damaged = screen.logical_pixels().to_vec();
    screen.hud.hull = [full, full];
    frame(&mut screen, &viewer, &state, &nav, "Hull damaged.");
    let repaired = screen.logical_pixels().to_vec();
    assert!(
        damaged != repaired,
        "the damaged hull did not reach the HUD"
    );
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_pirate_attack_alerts_the_player_and_lights_the_combat_lamp() {
    use ffe_rs::audio::{AudioEngine, MusicLibrary, NullMidiSink, RecordingSampleSink};
    use ffe_rs::camera::ViewMode;
    use ffe_rs::console::{Console, ConsoleState, ShipSystems};
    use ffe_rs::sound::{sample, GameAudio};

    let cache = cache();
    let (mut nav, viewer, _screen, state) = setup(&cache);
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let player = state.position.map(|v| (v * unit).round() as i64);
    let radius = i64::from(viewer.model_radius(NEW_GAME_SHIP).unwrap());

    // The fixture pirate is 80 m from the player, already inside F610's combat
    // range, so F750's alert must fire on the first AI tick.
    assert!(!nav.combat_mode(), "combat mode before the first tick");
    let events = nav.tick_ships(0.02, 994, player, radius);
    let alert = events.iter().find_map(|event| match event {
        ShipEvent::UnderAttack { name } => Some(name.clone()),
        _ => None,
    });
    assert!(alert.is_some(), "F750's alert never fired: {events:?}");

    // F753's combat mode lights console record 0x37 (bitmap 213) for as long as
    // the attacker is in range; F151 would clear it at combat end.
    assert!(
        nav.combat_mode(),
        "F753's combat mode did not follow the attack"
    );
    let attacked = ShipSystems {
        combat: nav.combat_mode(),
        ..ShipSystems::default()
    };
    let mut console = Console::new();
    console.apply_flight(ViewMode::Front, &ConsoleState::default(), &attacked);
    assert_eq!(
        console.image(0x37),
        Some(213),
        "the under-attack lamp is dark"
    );

    // F750's klaxon is sample 0x1e and still sounds when the fight's music was
    // already started by the player's own first hit.
    let sink = RecordingSampleSink::new();
    let mut sound = GameAudio::new(AudioEngine::new(
        MusicLibrary::new("missing-music"),
        NullMidiSink,
        sink.clone(),
    ));
    sound.start_battle();
    sound.attack_alert();
    assert_eq!(
        sink.calls()
            .iter()
            .map(|call| call.code)
            .collect::<Vec<_>>(),
        vec![sample::SIREN, sample::SIREN]
    );
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_clean_kill_makes_the_police_hostile() {
    let cache = cache();
    let (mut nav, viewer, mut screen, mut state) = setup(&cache);
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let player = state.position.map(|v| (v * unit).round() as i64);
    let radius = i64::from(viewer.model_radius(NEW_GAME_SHIP).unwrap());
    // A clean trader: the F931 crime flag and the wanted state.
    let trader = nav
        .ships
        .iter()
        .find(|ship| ship.role == Role::Trader)
        .expect("no trader");
    let id = trader.id;
    let name = trader.name.clone();
    assert!(!nav.legal.wanted);
    // The traffic tick records where the player is (F236's witness test).
    nav.tick_ships(0.02, 994, player, radius);
    let attack = nav.player_attack(id, 10_000).expect("the trader");
    assert!(attack.crime);
    assert!(attack.destroyed);
    assert!(nav.legal.wanted);
    assert_eq!(nav.legal.offences, 1);
    // F236 remembered the starport that saw it (DATA_008804+0x50dc); nothing makes
    // the existing police hostile: F238 launches them from that starport.
    assert!(nav.offended.is_some(), "no starport witnessed {name}");
    assert!(!attack.police);
    assert!(nav
        .ships
        .iter()
        .filter(|ship| ship.role == Role::Police)
        .all(|ship| !ship.is_hostile()));
    // F240 and F238 run on that starport's record. An open-air starport (Hope's) has
    // already stepped its clock (`+0xde`) when the first game day ends, so F238 rolls:
    // one in four each day it launches a police ship straight into the attack and
    // uses one of the starport's police (`+0xdc`).
    // (The dead trader is recycled meanwhile, so count the new ids, not the length.)
    let first_new = nav.ships.iter().map(|ship| ship.id + 1).max().unwrap_or(0);
    let index = nav.offended.unwrap();
    let before = nav.starport(index).police_left();
    for _ in 0..6000 {
        nav.tick_ships(0.02, 994, player, radius);
        nav.spawn_police(&cache, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        if nav.ships.iter().any(|ship| ship.id >= first_new) {
            break;
        }
    }
    let sent = nav.ships.iter().filter(|ship| ship.id >= first_new).count();
    assert!(sent > 0, "no police were launched");
    assert!(nav
        .ships
        .iter()
        .filter(|ship| ship.id >= first_new)
        .all(|ship| ship.role == Role::Police && ship.is_hostile()));
    assert_eq!(
        before - nav.starport(index).police_left(),
        sent as i16,
        "each launch uses one of the starport's police"
    );
    // A pirate is not a clean ship: attacking it is not a crime (F931's illegal
    // fire flag is the port's `legal.wanted`, which is already set here).
    let pirate = nav
        .ships
        .iter()
        .find(|ship| ship.role == Role::Pirate)
        .map(|ship| ship.id);
    if let Some(id) = pirate {
        let pirate_hit = nav.player_attack(id, 10_000).unwrap();
        assert!(!pirate_hit.crime, "a pirate is not a clean ship");
    }
    // The police frame: labels show the hostile ships and the HUD the wanted
    // state is reported in the message line.
    // The police frame: put the camera beside a police ship (the traffic spawned
    // around the station, not around the pirate the first test used).
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let police_at = nav
        .ships
        .iter()
        .find(|ship| ship.role == Role::Police)
        .map(|ship| ship.position)
        .unwrap();
    state.position = [police_at[0], police_at[1], police_at[2] + 250_000].map(|v| v as f64 / unit);
    state.axes = Some(look_at(state.position, police_at.map(|v| v as f64 / unit)));
    let message = "WANTED: police are responding.";
    frame(&mut screen, &viewer, &state, &nav, message);
    save(&screen, "npc-ai-police.ppm");
    let with = screen.logical_pixels().to_vec();
    screen.ships.clear();
    screen.labels.clear();
    screen.draw(&state, ViewState::default(), Some(&viewer), true, message);
    assert!(
        with != screen.logical_pixels(),
        "the police traffic did not draw"
    );
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_pirate_close_to_the_player_follows_a_jump() {
    let cache = cache();
    let (mut nav, viewer, mut screen, mut state) = setup(&cache);
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let player = state.position.map(|v| (v * unit).round() as i64);
    // Bring a pirate within F675's carry range.
    let pirate = nav
        .ships
        .iter_mut()
        .find(|ship| ship.role == Role::Pirate)
        .expect("no pirate");
    let name = pirate.name.clone();
    let model = pirate.model;
    pirate.position = [player[0], player[1], player[2] + 1_000_000];
    nav.tick_ships(0.02, 994, player, 1_000);
    // Jump to the next system in the list and enter it (F680).
    assert!(nav.system_count() > 1, "only one system");
    let next = (nav.current_system() + 1) % nav.system_count();
    let arrival = nav.jump(&cache, next).unwrap();
    nav.spawn_traffic(&cache, arrival, [0.0, 0.0, 1.0]);
    let restored = nav
        .ships
        .iter()
        .find(|ship| ship.role == Role::Pirate && ship.name == name);
    assert!(restored.is_some(), "{name} did not follow the jump");
    let restored = restored.unwrap();
    assert_eq!(restored.model, model);
    assert!(restored.is_hostile());
    let at = restored.position;
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    state.position = [at[0], at[1], at[2] + 80_000].map(|v| v as f64 / unit);
    state.axes = Some(look_at(state.position, at.map(|v| v as f64 / unit)));
    let message = "The pirate followed the jump.";
    frame(&mut screen, &viewer, &state, &nav, message);
    save(&screen, "npc-ai-restored.ppm");
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn firing_at_hope_is_fined_and_shows_f236s_message() {
    let cache = cache();
    // F829 record 0 / F998's default arm: Gateway [-1,4], planet Hope,
    // starport Old Blackelk, state 0x30, Saker Mk III, 10,000 CR (EXP-019
    // changes the hold, not the cash).
    let mut nav = Navigator::new_game(&cache, 0).unwrap();
    let new_game = nav.new_game.expect("the new game state");
    assert_eq!(new_game.start_type, 0);
    assert_eq!(new_game.player_state, 0x30);
    assert_eq!(new_game.cash, DEFAULT_CASH);
    assert_eq!(new_game.ship, DEFAULT_SHIP);
    let seat = new_game.seat;
    let docked = nav.docked.expect("docked at Old Blackelk");
    let starport = &nav.bodies[docked];
    let name = starport.name.clone();
    assert_eq!(name, "Old Blackelk");
    assert_eq!(nav.system_name(), "Gateway");
    // The fine ledger's DATA_008996 index is the ordinal among starports.
    let ordinal = nav.bodies[..docked]
        .iter()
        .filter(|body| body.kind == ffe_rs::navigation::Kind::Starport)
        .count();
    assert_eq!(nav.starport_in_range(seat), Some(ordinal), "{name}");

    let mut state = Flight {
        mode: Mode::Flight,
        position: seat.map(|v| v as f64 / ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT),
        system_flight: true,
        ship: DEFAULT_SHIP,
        credits: u32::try_from(DEFAULT_CASH).unwrap(),
        ..Flight::default()
    };
    state.axes = Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    assert!(!nav.legal.wanted);
    // The F931 gates (refire/heat/energy) pass before the legal branch runs.
    let mut combat = ffe_rs::combat::Combat::new_game();
    assert!(combat.fire(0, 994).is_ok(), "the first shot fires");

    let report = nav.player_fired(seat).expect("recorded at Hope");
    assert_eq!(
        report.offence.message,
        "Unlawful discharge of weapon is not permitted here. Pay a $600 fine."
    );
    assert_eq!(report.offence.fine, 600);
    assert_eq!(report.offence.charged, 6_000);
    assert!(nav.legal.wanted);
    // F236 sets only the wanted bit (`DATA_008646`). The chase flag
    // (`DATA_008647`) is F238's, set while a police object exists, and the
    // executed original's default start has none, so nothing is alerted.
    assert!(!nav.legal.alert);
    // F236's ledger index is DATA_008996, the system's jurisdiction (not the
    // witness ordinal used by the proximity check above).
    assert_eq!(nav.legal.fines[nav.jurisdiction], 6_000);
    // F236 is a debt: cash is untouched (10,000 CR on the default start).
    assert_eq!(state.credits, u32::try_from(DEFAULT_CASH).unwrap());
    // The recorded offence turns any police and military ships hostile (the
    // default start has none, so no ship answers).
    assert!(!report.police);
    assert!(nav
        .ships
        .iter()
        .filter(|ship| ship.role == Role::Police)
        .all(|ship| ship.is_hostile()));

    let viewer = ModelViewer::new(&cache).unwrap();
    let mut screen = Screen::new(&cache).unwrap();
    // F271/F351: the changed message becomes the newest log entry.
    screen.clock = nav.date_text();
    screen.messages_visible = true;
    screen.system = nav.system_name().to_owned();
    screen.station = name.clone();
    screen
        .messages
        .push(report.offence.message.clone(), nav.date_text());
    frame(&mut screen, &viewer, &state, &nav, &report.offence.message);
    save(&screen, "legal-hope.ppm");

    // F783: the police object is removed and clears the alert. The default
    // start has no police, so the scripted capture spawn supplies one, in
    // pursuit as F238 would leave it.
    nav.respawn_traffic(&cache, seat, [0.0, 0.0, 1.0]);
    for ship in &mut nav.ships {
        ship.provoke();
    }
    nav.legal.alert = true;
    let police = nav
        .ships
        .iter_mut()
        .find(|ship| ship.role == Role::Police)
        .expect("police");
    police.state = State::Escaped;
    let radius = viewer.model_radius(DEFAULT_SHIP).unwrap();
    let events = nav.tick_ships(0.02, 994, seat, i64::from(radius));
    assert!(!nav.legal.wanted);
    assert!(!nav.legal.alert);
    assert!(events.contains(&ShipEvent::AlertCleared));
    // The illegal-fire flag is the system's, not the chase's.
    assert!(nav.legal.illegal_fire);
}

/// F11's combat computer view (6): the chase camera behind the player and
/// F1109's amber readout (range, hull/shields, type) beside the pirate.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_combat_computer_reads_out_the_pirate() {
    use ffe_rs::camera::ViewMode;
    use ffe_rs::combat_computer::{readout, CombatCamera, COLOUR};
    let cache = cache();
    let (nav, viewer, mut screen, state) = setup(&cache);
    let unit = ffe_rs::original_flight::UNITS_PER_FLIGHT_UNIT;
    let player = state.position.map(|v| (v * unit).round() as i64);
    let pirate = nav.ships.iter().find(|s| s.role == Role::Pirate).unwrap();
    let info = readout(pirate, player, unit).expect("the pirate qualifies");
    // 80 m: 0.08 km, F1109's Write2dpFrac.
    assert_eq!(info.range, 8);
    assert_eq!(info.lines[0], "0.08km");
    assert!(info.lines[1].ends_with('%'));
    // F1109 prints the type's name (`stats+0xe`), not the port's numbering.
    assert_eq!(info.lines[2], pirate.type_name);
    assert!(pirate.name.starts_with(&pirate.type_name));
    ships(&mut screen, &nav);
    screen.combat_readouts = nav
        .ships
        .iter()
        .filter_map(|ship| readout(ship, player, unit))
        .collect();
    let mut views = ViewState {
        mode: ViewMode::Combat,
        combat: CombatCamera::enter(),
        ..ViewState::default()
    };
    views.combat.step(Some(info.range), 0);
    screen.draw(&state, views, Some(&viewer), true, "");
    save(&screen, "combat-computer.ppm");
    let amber = screen.palette()[usize::from(COLOUR)];
    let lit = screen
        .logical_pixels()
        .iter()
        .take(320 * 158)
        .filter(|pixel| **pixel == amber)
        .count();
    assert!(lit > 20, "no F1109 readout drawn ({lit} amber pixels)");
}

/// What the executed original does at Hope (`--flight idle 12000 --start-key=0x02` with the laser
/// fired at frame 200 and `FFE_POLICE_TRACE`'s `phull`): the police launch, fly to the pad and shoot
/// the landed player's hull from 100 to 12, first hit on frame 1252. Its pirates stay on their
/// orbit (20,000 frames at up to 10000x) and never reach a player on the pad.
struct PadRun {
    /// Hits by frame, in thousands of frames (the original: 12 hits by frame 1,800, then none).
    hits_by_1000: [u32; 8],
    wanted_at_end: bool,
    first_hit: Option<u32>,
    police_hits: u32,
    pirate_hits: u32,
    police: usize,
}

/// Fire the laser at Hope and run `frames` frames of `tics` game tics. `launched` lifts the player
/// 2 km off the pad (not landed), as after takeoff.
fn hope_run(cache: &ffe_content::VerifiedCache, frames: u32, tics: u32, launched: bool) -> PadRun {
    let mut nav = Navigator::new_game(cache, 0).unwrap();
    let viewer = ModelViewer::new(cache).unwrap();
    let seat = nav.new_game.expect("the new game state").seat;
    assert!(nav.landed_view(), "the default start is on the pad");
    nav.player_fired(seat).expect("recorded at Hope");
    let mut player = seat;
    if launched {
        nav.docked = None;
        nav.new_game = None;
        player[1] += 2_000_000;
    }
    let radius = i64::from(viewer.model_radius(DEFAULT_SHIP).unwrap());
    eprintln!("player radius {radius}");
    let seconds = f64::from(tics) / ffe_core::npc::TICS_PER_SECOND;
    let mut run = PadRun {
        hits_by_1000: [0; 8],
        wanted_at_end: false,
        first_hit: None,
        police_hits: 0,
        pirate_hits: 0,
        police: 0,
    };
    for frame in 0..frames {
        for event in nav.tick_ships(seconds, tics, player, radius) {
            if let ShipEvent::Hit { ship, .. } = event {
                run.first_hit.get_or_insert(frame);
                run.hits_by_1000[(frame as usize / 1000).min(7)] += 1;
                match nav.ships.iter().find(|s| s.id == ship).map(|s| s.role) {
                    Some(Role::Pirate) => run.pirate_hits += 1,
                    Some(Role::Police) => run.police_hits += 1,
                    _ => {}
                }
            }
        }
        nav.spawn_police(cache, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        run.police = run
            .police
            .max(nav.ships.iter().filter(|s| s.role == Role::Police).count());
    }
    run.wanted_at_end = nav.legal.wanted;
    run
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_police_shoot_a_landed_player_at_hope() {
    let cache = cache();
    let run = hope_run(&cache, 6000, 1043, false);
    eprintln!(
        "1x pad: first hit {:?}, police hits {}, pirate hits {}, police {}, hits per 1000 frames {:?}, wanted at end {}",
        run.first_hit, run.police_hits, run.pirate_hits, run.police, run.hits_by_1000, run.wanted_at_end
    );
    assert!(run.police > 0, "no police were launched");
    assert!(
        run.police_hits > 0,
        "the police never hit the landed player"
    );
    assert_eq!(run.pirate_hits, 0, "the pirates hold still over the pad");
    // Fast-forward: the executed original burns through the police at 100x to 10000x within a few
    // frames, the wanted bit clears and the hull stays at 100 for the 5,800 frames traced
    // (`FFE_ACCEL_AT=300:3`), so nothing shoots the player.
    let fast = hope_run(&cache, 600, 1043 * 100, false);
    eprintln!(
        "100x pad: police hits {}, wanted at end {}",
        fast.police_hits, fast.wanted_at_end
    );
    assert_eq!(fast.pirate_hits, 0);
    assert!(
        !fast.wanted_at_end,
        "the police stock is spent and the wanted bit clears"
    );
}

/// The executed original puts a new system's pirate group on F702's far orbit round the star
/// (`--npc-trace`: 1.3e15 from the root, further out than Hope's planet), so a player who
/// launches at Hope is not attacked within minutes. The port used to lurk the group 28 km out and
/// was attacked 25 seconds after takeoff.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn pirates_do_not_reach_a_player_who_has_just_launched_at_hope() {
    let cache = cache();
    let nav = Navigator::new_game(&cache, 0).unwrap();
    let seat = nav.new_game.expect("the new game state").seat;
    let pirates: Vec<_> = nav
        .ships
        .iter()
        .filter(|ship| ship.role == Role::Pirate)
        .collect();
    assert!(!pirates.is_empty(), "the default start has a pirate group");
    for pirate in &pirates {
        let range: f64 = (0..3)
            .map(|k| ((pirate.position[k] - seat[k]) as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            range > 1.0e14,
            "{} starts {range} from the player, not on F702's far orbit",
            pirate.name
        );
    }
    let run = hope_run(&cache, 6000, 1043, true);
    eprintln!(
        "1x flight: first hit {:?}, police hits {}, pirate hits {}",
        run.first_hit, run.police_hits, run.pirate_hits
    );
    assert_eq!(run.pirate_hits, 0, "the pirates are system-wide away");
}
