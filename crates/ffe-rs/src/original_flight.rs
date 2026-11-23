//! The original flight model in the playable: the verified free-flight tick
//! (`ffe_core::player`) driven by real time and the original's own controls,
//! with the result copied back into [`Flight`] for the views, docking checks and
//! engine glows. Positions are the original's millimetres (about 1 per model
//! unit); one flight unit is 9,700 of them, the scale the station is drawn at.
//! Mouse steering is not wired (the mouse is the console's); the keys are the
//! original's. See `docs/notes/player.md`.
use crate::flight::Flight;
use crate::ships::ShipStats;
use ffe_core::collision::Contact;
use ffe_core::frames::{Bodies, Body, Words};
use ffe_core::keyboard::{Keyboard, ScanSet};
use ffe_core::objects::ModelInfo;
use ffe_core::physics::Motion;
use ffe_core::player::{Controls, Player, SteerKeys, ThrusterKeys};
use ffe_core::surface::SurfaceTools;
use ffe_core::thrust::Limits;

/// Position units (about a millimetre) per flight unit.
pub const UNITS_PER_FLIGHT_UNIT: f64 = 9_700.0;
/// The original's time units per second (`ms * 0x3e2 / 20` per millisecond).
pub const TICS_PER_SECOND: f64 = 49_700.0;
/// Millimetres per second per velocity unit (F1511: `v * t / 512`).
pub const MM_PER_SECOND: f64 = TICS_PER_SECOND / 512.0;
/// The Eagle's limits, for a cache without ship statistics.
pub const FALLBACK_LIMITS: Limits = Limits {
    axes: [[4905, -4905], [4905, -4905], [16350, -4905]],
};

/// The keys the original flight model reads, true while held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Keys {
    /// Return: raise the set speed.
    pub faster: bool,
    /// Shift: lower it.
    pub slower: bool,
    pub pitch_down: bool,
    pub pitch_up: bool,
    pub left: bool,
    pub right: bool,
    pub roll_left: bool,
    pub roll_right: bool,
    /// The mouse accumulator (`DATA_009188`) while [`Keys::mouse_enabled`] is
    /// set: x in the low word, y in the high word.
    pub mouse: u32,
    /// The Classic preset's right button is held. When false the accumulator is
    /// ignored and its previous value cleared, so a later press cannot jump.
    pub mouse_enabled: bool,
    /// Invert the mouse x and y axes (the original's Reverse controls options).
    pub mouse_flip: [bool; 2],
    /// Every key held, by the original's scancode ([`crate::input::scancode`]):
    /// the input read's last-key report and `DATA_007690`, which scales
    /// F1121's set-speed step, follow all keys, not only the flight ones.
    pub scancodes: ffe_core::keyboard::ScanSet,
}

/// The integer words of the free-flight model that the floating-point
/// [`Flight`] view cannot carry (PAR-18): everything the next frame reads that
/// is not rebuilt from the saved position, velocity and rows. Kept with the
/// save so a resume continues exactly where an uninterrupted flight would be.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResumeState {
    pub motion: Motion,
    pub mode: u8,
    pub flags: u8,
    pub altitude: i32,
    pub reference_counter: i8,
    pub combat: bool,
    pub combat_velocity: [i32; 3],
    pub scanner: ffe_core::physics::Scanner,
    pub mouse_previous: u32,
    pub view: u8,
    pub turret: [i16; 2],
    pub gravity_vector: [i32; 3],
    pub contact_flags: u8,
    pub gear_down: bool,
    pub gear_word: u16,
    pub speed_word: u32,
    pub collision_radius: i64,
}

impl ResumeState {
    /// The `Flight` values `OriginalFlight::apply` derives from these words.
    fn view_of(&self) -> ([f64; 3], [f64; 3], [[f64; 3]; 3]) {
        let motion = &self.motion;
        (
            motion.position.map(|p| p as f64 / UNITS_PER_FLIGHT_UNIT),
            motion
                .velocity
                .map(|v| f64::from(v) * MM_PER_SECOND / UNITS_PER_FLIGHT_UNIT),
            motion
                .orientation
                .map(|row| row.map(|v| f64::from(v) / 2_147_483_648.0)),
        )
    }

    /// Whether `flight` is still what these words produced: the same ship, and
    /// the position, velocity and rows unchanged since `apply` wrote them.
    fn matches(&self, flight: &Flight) -> bool {
        let (position, velocity, axes) = self.view_of();
        self.motion.model == flight.ship
            && flight.position == position
            && flight.velocity == velocity
            && flight.axes == Some(axes)
            && flight.set_speed.is_some()
    }
}

/// A ship's position, velocity and rows (`player.motion`'s three frame-dependent
/// fields).
type PadState = ([i64; 3], [i32; 3], [[i32; 3]; 3]);

/// The pad world's authoritative ship state, in the planet's rotating frame,
/// and the linear view last written to the player's motion. F601's per-axis
/// floor rounding, F603's gravity and the fraction accumulators all depend on
/// the frame's axis signs, so they must run in the original's frame (the
/// planet's rows turn the axes: Ross 154's are diag(-1, 1, -1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PadLocal {
    state: PadState,
    shown: PadState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OriginalFlight {
    player: Player,
    /// The keyboard's accumulators by scancode: `DATA_007690` for F1121.
    keyboard: Keyboard,
    /// The same accumulators for the six steering actions, in `SteerKeys`
    /// order (whichever key the preset gives each): F1038's
    /// `DATA_KeyAccumTable` entries.
    steering: Keyboard,
    /// The real-time clock in microseconds, so whole milliseconds carry from
    /// frame to frame as `TimerGetTimeStamp`'s do.
    clock_us: u64,
    forward_limit: i16,
    /// The time acceleration (`DATA_009012`, per 20 ms): 0x3e2 is real time.
    acceleration: u32,
    /// The two-body world of an open-air takeoff: the ship is body 1 and the
    /// starport body 2, so F570's frame switches (and their position, velocity
    /// and orientation changes) persist from frame to frame while the ship is
    /// on the pad. Dropped when the takeoff clears.
    pad_world: Option<Bodies>,
    /// The ship's state in the pad world's planet-rotating frame (`+0x57`), the
    /// frame the original integrates it in, while `player.motion` shows the
    /// planet-linear view the rest of the port reads.
    pad_local: Option<PadLocal>,
    guidance: Option<ffe_core::autopilot::Guidance>,
    /// The ship model's collision radius (`+0x140`), the lift F994 gives a pad
    /// seat and F612's radius distance: the descent aims at the pad's surface
    /// point raised by it, so the collision sphere touches rather than sinks.
    collision_radius: i64,
    /// The undercarriage command, `player+0xe9` bit 1 clear (down) or set
    /// (raised): F1056's `u` toggles it and F1008 sets it under the
    /// autopilot. A new game starts with it down.
    gear_down: bool,
    /// `ship+0x9e`, F1008's undercarriage extension: 0 raised, 0xffff down,
    /// moved half the frame's tics a frame toward the command. The models
    /// read it as animation global 1 and F575's landing test needs 0xffff.
    gear_word: u16,
    /// F1099's speed word (`player+0xd8`) from the end of the last frame, which
    /// F1014 reads.
    speed_word: u32,
    /// This frame's F1014 ran (F1021 in mode 0, F1019 in mode 2, after the
    /// scan): the speed word its heat uses.
    atmosphere: Option<u32>,
    /// `ship+0x101`, the AI's guidance modifier F742/F746 hand to F610: the
    /// creation code (`ffe.asm:3601-3617`) sets 2 for a ship whose x limit
    /// (`+0xbc`) is at least 0x1329, 1 for one of at least 0xaa5 and 0 below.
    /// Damage changes the limits later, not this byte.
    ai_modifier: i32,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn to_q31(value: f64) -> i32 {
    (value * 2_147_483_647.0)
        .round()
        .clamp(-2_147_483_648.0, 2_147_483_647.0) as i32
}

impl OriginalFlight {
    /// F998/F994: the common new-commander entry, including after Game Over.
    /// Build the parked simulation from the same delivered equipment and pad
    /// state used by the cockpit; missing source data must fail explicitly.
    pub fn new_commander(
        cache: &ffe_content::VerifiedCache,
        nav: &crate::navigation::Navigator,
    ) -> Result<(Flight, Self), String> {
        let start = nav.new_game.ok_or("No source new-game record.")?;
        let index = nav.docked.ok_or("New commander has no starting port.")?;
        let station = nav.bodies.get(index).ok_or("Starting port is missing.")?;
        let planet = station
            .parent
            .and_then(|i| nav.bodies.get(i))
            .unwrap_or(station);
        let records = cache
            .asset("base:model/records")
            .ok_or("Missing model records.")?;
        let stats = cache
            .asset("base:ship/stats")
            .ok_or("Missing ship statistics.")?;
        let info = |model| ModelInfo::read(records, stats, model).ok_or("Missing new-game model.");
        let ship_info = info(start.ship)?;
        let port_info = info(station.model)?;
        let planet_info = info(planet.model)?;
        let stats = ship_info
            .stats
            .as_ref()
            .and_then(|block| ShipStats::from_block(block))
            .ok_or("Missing new-game ship statistics.")?;
        let spec = crate::boot::StartSpec::get(start.start_type)?;
        let mut hold = vec![0; ffe_core::market::NAMES];
        hold[ffe_core::hyperspace::HYDROGEN] = spec.fuel;
        let combat = spec.combat();
        let loadout = spec.loadout();
        let mut state = Flight {
            hold,
            credits: u32::try_from(start.cash).map_err(|_| "Invalid starting cash.")?,
            ship: start.ship,
            mode: if spec.mode == 0x2a {
                crate::flight::Mode::Docked
            } else {
                crate::flight::Mode::Flight
            },
            system_flight: true,
            position: start.seat.map(|v| v as f64 / UNITS_PER_FLIGHT_UNIT),
            velocity: [0.0; 3],
            axes: Some(station.orientation),
            combat,
            loadout,
            ..Flight::default()
        };
        // F998's type-0 arm calls F997 after F994: reverse the right and
        // forward rows, preserving up. Without it the cockpit faces the nearby
        // hangar wall instead of looking out across Hope's city.
        if !matches!(start.start_type, 1..=3) {
            if let Some(axes) = state.axes.as_mut() {
                axes[0] = axes[0].map(|v| -v);
                axes[2] = axes[2].map(|v| -v);
            }
        }
        let mut model = Self::from_flight(&state, Some(&stats));
        model.set_mode(start.player_state);
        if spec.mode != 0x2a {
            model.begin_pad_takeoff(
                &crate::navigation::landing_body(planet, &planet_info),
                planet.position,
                &crate::navigation::landing_body(station, &port_info),
                station.position,
                &ship_info,
            );
        }
        model.apply(&mut state);
        Ok((state, model))
    }

    /// Start from a flight state: its position, velocity and orientation, its
    /// saved signed set speed (`DATA_008918`; only a state from an older save
    /// without one falls back to the speed along the nose), and the ship's
    /// limits and gun mounts when the cache has them. F1018's autopilot cancel
    /// is separate: it sets the set speed from `DATA_008866`, which only the
    /// new game writes (0).
    pub fn from_flight(flight: &Flight, stats: Option<&ShipStats>) -> Self {
        let axes = crate::camera::ship_axes(flight);
        let to_units = |v: f64| v * UNITS_PER_FLIGHT_UNIT / MM_PER_SECOND;
        let velocity = flight.velocity.map(|v| to_units(v) as i32);
        let forward_speed = dot(flight.velocity, axes[2]);
        let limits = flight.limits(stats);
        let mut player = Player {
            motion: Motion {
                orientation: axes.map(|row| row.map(to_q31)),
                velocity,
                position: flight
                    .position
                    .map(|p| (p * UNITS_PER_FLIGHT_UNIT).round() as i64),
                model: flight.ship,
                gun_mounts: stats.map_or(0, |s| s.gun_mounts),
                drift: 0x3c,
                craft: 1,
                ..Motion::default()
            },
            limits,
            set_speed: flight
                .set_speed
                .unwrap_or_else(|| to_units(forward_speed).round() as i32),
            altitude: i32::MAX,
            reference_counter: 0xd,
            active: true,
            keys_enabled: true,
            view: 4,
            ..Player::default()
        };
        player.scanner.player = 0;
        let mut flown = Self {
            player,
            keyboard: Keyboard::new(),
            steering: Keyboard::new(),
            clock_us: 0,
            forward_limit: limits.axes[2][0],
            acceleration: crate::console::TIME_ACCELERATION[0],
            pad_world: None,
            pad_local: None,
            guidance: None,
            collision_radius: 0,
            gear_down: true,
            gear_word: 0xffff,
            speed_word: 0,
            atmosphere: None,
            ai_modifier: match limits.axes[0][0] {
                0x1329.. => 2,
                0xaa5.. => 1,
                _ => 0,
            },
        };
        if let Some(resume) = flight.resume.as_ref().filter(|r| r.matches(flight)) {
            flown.restore(resume, stats);
        }
        flown
    }

    /// The integer words a resume continues from (see [`ResumeState`]).
    pub fn resume_state(&self) -> ResumeState {
        let player = &self.player;
        ResumeState {
            motion: player.motion,
            mode: player.mode,
            flags: player.flags,
            altitude: player.altitude,
            reference_counter: player.reference_counter,
            combat: player.combat,
            combat_velocity: player.combat_velocity,
            scanner: player.scanner,
            mouse_previous: player.mouse_previous,
            view: player.view,
            turret: player.turret,
            gravity_vector: player.gravity_vector,
            contact_flags: player.contact_flags,
            gear_down: self.gear_down,
            gear_word: self.gear_word,
            speed_word: self.speed_word,
            collision_radius: self.collision_radius,
        }
    }

    fn restore(&mut self, resume: &ResumeState, stats: Option<&ShipStats>) {
        let player = &mut self.player;
        player.motion = resume.motion;
        if let Some(stats) = stats {
            player.motion.gun_mounts = stats.gun_mounts;
        }
        player.mode = resume.mode;
        player.flags = resume.flags;
        player.altitude = resume.altitude;
        player.reference_counter = resume.reference_counter;
        player.combat = resume.combat;
        player.combat_velocity = resume.combat_velocity;
        player.scanner = resume.scanner;
        player.mouse_previous = resume.mouse_previous;
        player.view = resume.view;
        player.turret = resume.turret;
        player.gravity_vector = resume.gravity_vector;
        player.contact_flags = resume.contact_flags;
        self.gear_down = resume.gear_down;
        self.gear_word = resume.gear_word;
        self.speed_word = resume.speed_word;
        self.collision_radius = resume.collision_radius;
    }

    /// JJFFE's `FUNC_SetupCombatVelocity` (F750's and F770's hails): the
    /// combat reference velocity (`DATA_008839..8841`) is the player's own,
    /// or zero below 0x9c40 of altitude (`DATA_008867`) or when the player's
    /// frame has class bit 0x10 (`sheltered`).
    pub fn setup_combat_velocity(&mut self, sheltered: bool) {
        self.player.combat_velocity = if sheltered || self.player.altitude < 0x9c40 {
            [0; 3]
        } else {
            self.player.motion.velocity
        };
    }

    /// The combat reference velocity.
    pub fn combat_velocity(&self) -> [i32; 3] {
        self.player.combat_velocity
    }

    /// The same model at a time acceleration step.
    pub fn gear_down(&self) -> bool {
        self.gear_down
    }

    /// The undercarriage command (`u`, action 0x75, or F1008's autopilot).
    pub fn set_gear(&mut self, down: bool) {
        self.gear_down = down;
    }

    /// `ship+0x9e`: the undercarriage's extension, 0 raised to 0xffff down.
    pub fn gear_word(&self) -> u16 {
        self.gear_word
    }

    /// F1008's per-frame undercarriage: the extension moves half of
    /// `DATA_009155` (the frame's tics) toward the command.
    pub fn animate_gear(&mut self) {
        let step = (self.player.frame_tics >> 1).min(0xffff) as u16;
        self.gear_word = if self.gear_down {
            self.gear_word.saturating_add(step)
        } else {
            self.gear_word.saturating_sub(step)
        };
    }

    pub fn with_time_acceleration(mut self, step: usize) -> Self {
        self.set_time_acceleration(step);
        self
    }

    /// Choose the time acceleration step (0 = 1x .. 4 = 10000x): the world
    /// advances by the real time times it, while key amounts stay in real time.
    pub fn set_time_acceleration(&mut self, step: usize) {
        self.acceleration = crate::console::TIME_ACCELERATION[step.min(4)];
    }

    /// F1042 and the launch button: zero the velocity, the turn rates, the
    /// thrust words and the set speed, and put the ship in F1013's climb
    /// (`DATA_008870` `0x24`). The caller has already placed it on the pad.
    pub fn begin_takeoff(&mut self) {
        self.player.mode = ffe_core::traffic::state::TAKEOFF;
        self.player.set_speed = 0;
        self.player.motion.velocity = [0; 3];
        self.player.motion.rate = [0; 3];
        self.player.motion.thrust = [0; 3];
    }

    /// Set the player's mode word (`DATA_008870`): the takeoff machine's
    /// states are copied here each frame so F1041's `0x24` up-thrust and the
    /// ordinary flight paths apply.
    pub fn set_mode(&mut self, mode: u8) {
        self.player.mode = mode;
    }

    /// Whether the last step ran F1014 (an atmosphere contact below 20 km in
    /// mode 0 or 2), and the speed word its heat uses
    /// ([`crate::damage::atmosphere_heat`]). Taken once, so a frame without a
    /// step does not heat the hull again.
    pub fn take_atmosphere(&mut self) -> Option<u32> {
        self.atmosphere
            .take()
            .filter(|_| self.player.altitude <= ffe_core::player::ATMOSPHERE_ALTITUDE)
    }

    /// Current DATA_008870 after the flight handler's same-frame transitions.
    pub fn mode(&self) -> u8 {
        self.player.mode
    }

    /// F1021's landing: F1042 zeroes the velocity, the turn rates, the thrust
    /// words and the set speed, and `DATA_008870` becomes the frame's `0x32`
    /// (the caller's `Navigator::landing_frame` then sets `0x34`). The contact
    /// scan has already settled the ship on the surface.
    pub fn settle_landing(&mut self) {
        self.player.mode = ffe_core::traffic::state::LANDING;
        self.player.set_speed = 0;
        self.player.motion.velocity = [0; 3];
        self.player.motion.rate = [0; 3];
        self.player.motion.thrust = [0; 3];
    }

    /// F1019/F1048's open-air starport touch: F1042 zeroes the motion and the
    /// mode is the frame's `0x2e` (the caller's `Navigator::landing_frame` then
    /// sets `0x30`).
    pub fn settle_starport_landing(&mut self) {
        self.player.mode = ffe_core::traffic::state::STARPORT_LANDING;
        self.player.set_speed = 0;
        self.player.motion.velocity = [0; 3];
        self.player.motion.rate = [0; 3];
        self.player.motion.thrust = [0; 3];
    }

    /// The set speed (`DATA_008918`) in the original's velocity units.
    pub fn set_speed_units(&self) -> i32 {
        self.player.set_speed
    }

    /// `player+0xe9` bit 0 (`DATA_008871`) clear: F1041 drives the ship to the
    /// set speed (F607). Set, the engines are off and F1043's thrusters alone
    /// move it.
    pub fn engines_on(&self) -> bool {
        self.player.flags & 1 == 0
    }

    /// F1056's action 9 / F1020: switch the set-speed engines on or off.
    pub fn set_engines_on(&mut self, on: bool) {
        if on {
            self.player.flags &= !1;
        } else {
            self.player.flags |= 1;
        }
    }

    /// Set the set speed (`DATA_008918`, `player+0x1b8`) in velocity units.
    pub fn set_set_speed_units(&mut self, units: i32) {
        self.player.set_speed = units;
    }

    /// Replace the six thrust limits the flight step clamps against
    /// (`ship+0xbc..+0xc6`): a damaged or failed thruster changes how much of
    /// the request F598/F600 can deliver.
    pub fn set_limits(&mut self, limits: Limits) {
        self.player.limits = limits;
        self.forward_limit = limits.axes[2][0];
    }

    /// The three thrust words F938 sums for the energy cost (`+0xb6`/`+0xb8`/
    /// `+0xba`, the clamp's result written each step).
    pub fn thrust_words(&self) -> [i16; 3] {
        self.player.motion.thrust
    }

    /// Start the persistent three-body world of an open-air takeoff. The
    /// original's F994 parents the player to the starport's parent (the planet)
    /// — `+0x56` is the object two slots below the starport — and the starport
    /// is a sibling. That is what gives F603 its gravity and F1033 its altitude
    /// above the terrain. Body 1 is the ship, body 2 the planet it is parented
    /// to and body 3 the starport, so F570's frame switches (and their position,
    /// velocity and orientation changes) persist from frame to frame while the
    /// ship is on the pad. Dropped when the takeoff clears.
    pub fn begin_pad_takeoff(
        &mut self,
        planet: &Body,
        planet_position: [i64; 3],
        station: &Body,
        station_position: [i64; 3],
        player_model: &ModelInfo,
    ) {
        let motion = self.player.motion;
        let vector: [i64; 3] =
            std::array::from_fn(|k| motion.position[k].wrapping_sub(planet_position[k]));
        let (outer_radius, inner_radius) = player_model.radii();
        self.collision_radius = inner_radius;
        let mut world = Bodies {
            stamp: 1,
            frame_tics: self.player.frame_tics,
            player: 1,
            ..Bodies::default()
        };
        world.live[1] = true;
        world.physical[1] = true;
        world.live[2] = true;
        world.physical[2] = true;
        let mut words = Words::default();
        // The landing gear (`+0x9e` word 1 = 0xffff when down).
        words.0[1] = self.gear_word;
        world.body[1] = Body {
            position: vector,
            velocity: motion.velocity,
            parent: 2,
            index: 1,
            kind: 0,
            orientation: std::array::from_fn(|k| motion.orientation[k / 3][k % 3]),
            outer_radius,
            inner_radius,
            shape: player_model.class(),
            model: motion.model,
            words,
            ..Body::default()
        };
        // F994's parent: the planet the starport orbits, with F880's gravity
        // word (`+0xa4`) for F603.
        let mut parent = *planet;
        parent.position = planet_position;
        parent.parent = 0;
        parent.rotating = 0;
        parent.index = 2;
        parent.delay = 0;
        parent.stamp = 0;
        world.body[2] = parent;
        // The starport is a sibling in the planet's frame, so F576's pad contact
        // and its `0x20` effect region still report against the ship.
        if station_position != planet_position || station.model != planet.model {
            let mut port = *station;
            port.position =
                std::array::from_fn(|k| station_position[k].wrapping_sub(planet_position[k]));
            port.parent = 2;
            port.rotating = 0;
            port.index = 3;
            port.delay = 0;
            port.stamp = 0;
            world.body[3] = port;
            world.live[3] = true;
            world.physical[3] = true;
        }
        self.player.motion.position = vector;
        self.player.motion.parent = 2;
        self.player.motion.rotating = 0;
        // The port's planet orientation is fixed; keep the ship in the linear
        // frame so F572/F573's fixed-point round trip cannot move it when the
        // 13-tick selector runs during the climb.
        self.player.motion.switch_delay = 0x7f;
        self.player.motion.updated = 0;
        // The ship stands on the pad: F1033's last contact altitude is the
        // pad's, so F1013 climbs from 0x24 instead of reading a new model's
        // "no contact" as already above 0x30d40.
        self.player.altitude = 0;
        // F994 puts the ship and the starport in the planet's rotating frame
        // (`+0x57`): turn them into it once (F572), as the original does, and
        // integrate there from now on.
        let shown: PadState = (
            self.player.motion.position,
            self.player.motion.velocity,
            self.player.motion.orientation,
        );
        if world.live[3] {
            world.to_rotating(3, 2);
        }
        let state = Self::pad_turn(&world, shown, true);
        self.pad_world = Some(world);
        self.pad_local = Some(PadLocal { state, shown });
    }

    /// Turn a ship state between the pad world's planet-linear frame and the
    /// planet's rotating frame (F572 when `into_rotating`, else F573).
    fn pad_turn(world: &Bodies, state: PadState, into_rotating: bool) -> PadState {
        let mut scratch = world.clone();
        let body = &mut scratch.body[1];
        body.position = state.0;
        body.velocity = state.1;
        body.orientation = std::array::from_fn(|k| state.2[k / 3][k % 3]);
        body.parent = 2;
        body.rotating = u8::from(!into_rotating);
        if into_rotating {
            scratch.to_rotating(1, 2);
        } else {
            scratch.to_linear(1, 2);
        }
        let body = scratch.body[1];
        (
            body.position,
            body.velocity,
            std::array::from_fn(|r| std::array::from_fn(|c| body.orientation[3 * r + c])),
        )
    }

    /// The ship's state in the rotating frame. Whoever changed the linear view
    /// since it was last published (a placement, a teleport) wins; otherwise the
    /// stored state is used unchanged, so the frame turn's rounding never
    /// accumulates.
    fn pad_state(&self) -> Option<PadState> {
        let world = self.pad_world.as_ref()?;
        let local = self.pad_local?;
        let motion = &self.player.motion;
        let now: PadState = (motion.position, motion.velocity, motion.orientation);
        if now == local.shown {
            Some(local.state)
        } else {
            Some(Self::pad_turn(world, now, true))
        }
    }

    /// Store the ship's rotating-frame state and show its linear view.
    fn pad_publish(&mut self, state: PadState) {
        let Some(world) = self.pad_world.as_ref() else {
            return;
        };
        let shown = Self::pad_turn(world, state, false);
        let motion = &mut self.player.motion;
        motion.position = shown.0;
        motion.velocity = shown.1;
        motion.orientation = shown.2;
        motion.rotating = 0;
        self.pad_local = Some(PadLocal { state, shown });
    }

    /// F1011/F1012: a parked ship reads only F1039's turret steering.
    /// F1014/F1015 are atmosphere handlers, not the F1037/F1041 flight tick.
    /// No gravity, thrust integration or collision movement runs until launch.
    pub fn step_landed(&mut self, elapsed: f64, keys: &Keys, mode: u8) {
        self.player.mode = mode;
        // The input read still sees every key (the original reads the
        // keyboard whatever the mode); only the turret views steer.
        let turret_keys = if matches!(self.player.view, 2 | 3) {
            Keys {
                faster: false,
                slower: false,
                ..*keys
            }
        } else {
            Keys {
                scancodes: keys.scancodes,
                ..Keys::default()
            }
        };
        self.prepare(elapsed, &turret_keys);
        self.player.altitude = 0;
    }

    /// One frame on the pad in that world: the same F1041/F601/F1512/F1033
    /// tick as [`OriginalFlight::step_landing`], with the planet as the
    /// player's parent (F603's gravity and F1033's altitude) and the starport a
    /// sibling (F1031's `0x20` effect region and the pad contact).
    pub fn step_pad(
        &mut self,
        elapsed: f64,
        keys: &Keys,
        tools: Option<&SurfaceTools>,
    ) -> Result<Contact, String> {
        let (_, world_tics) = self.prepare(elapsed, keys);
        let state = self.pad_state().ok_or("No pad takeoff world to step.")?;
        {
            let motion = &mut self.player.motion;
            motion.position = state.0;
            motion.velocity = state.1;
            motion.orientation = state.2;
            motion.parent = 2;
            motion.rotating = 1;
            motion.switch_delay = 0x7f;
        }
        let world = self
            .pad_world
            .as_mut()
            .ok_or("No pad takeoff world to step.")?;
        // F1677's absolute-position cache is keyed by DATA_008811, the frame
        // stamp, so it must change every frame.
        world.stamp = world.stamp.wrapping_add(1);
        world.frame_tics = world_tics;
        let contact = tick_guided(&mut self.player, self.guidance, world_tics, world, tools);
        let motion = &self.player.motion;
        self.pad_publish((motion.position, motion.velocity, motion.orientation));
        contact
    }

    /// Move the pad world's planet and starport to their current navigator
    /// positions, keeping their orientations. See [`Self::move_pad_bodies`].
    pub fn set_pad_bodies(&mut self, planet_position: [i64; 3], station_position: [i64; 3]) {
        let (planet_rows, station_rows) =
            self.pad_world.as_ref().map_or(([0; 9], [0; 9]), |world| {
                (world.body[2].orientation, world.body[3].orientation)
            });
        self.move_pad_bodies(planet_position, planet_rows, station_position, station_rows);
    }

    /// Keep a parked ship at F994's current seat after the clock moves the port.
    /// The ship and the starport are fixed in the planet's rotating frame, so the
    /// seat turns with the planet by itself (see [`Self::move_pad_bodies`]) and
    /// there is nothing to pin; the arguments are the old linear-frame carry's.
    pub fn pin_pad_seat(&mut self, _seat: [i64; 3], _rows: [i32; 9], _reverse: bool) {
        if let Some(world) = self.pad_world.as_mut() {
            world.stamp = world.stamp.wrapping_add(1);
        }
    }

    /// Move the pad world's planet and starport to their current navigator
    /// positions and rows. The planet's rows turn its terrain: they must follow
    /// the planet's spin with the starport, or the ground under a ship carried
    /// with the port is terrain from kilometres away (the navigator's spin step
    /// at Old Blackelk moved it 10 km, and the contact then reported a hill
    /// 237 m higher under a hovering ship). F994 puts the player in the planet's
    /// rotating frame (`+0x57`), so a ship on or over the pad turns with the
    /// planet; this world keeps the ship linear (one exact frame), so it is
    /// carried rigidly with the starport instead: the same turn and shift for
    /// its position, velocity and orientation. A shift alone (the old carry)
    /// left the pad surface swinging away as the planet turned, 80 m in three
    /// seconds at Old Blackelk, which the first contact then made up at once.
    pub fn move_pad_bodies(
        &mut self,
        planet_position: [i64; 3],
        planet_rows: [i32; 9],
        _station_position: [i64; 3],
        _station_rows: [i32; 9],
    ) {
        let Some(state) = self.pad_state() else {
            return;
        };
        if let Some(world) = self.pad_world.as_mut() {
            // The ship and the starport keep their rotating-frame coordinates;
            // only the planet's place and rows move, and the linear view that
            // the rest of the port reads follows them.
            world.body[2].position = planet_position;
            world.body[2].orientation = planet_rows;
            // F1677's absolute-position cache is keyed by the frame stamp.
            world.stamp = world.stamp.wrapping_add(1);
        }
        self.pad_publish(state);
    }

    /// The ship model's collision radius (`+0x140`): F994 lifts a pad seat by
    /// it and F612 measures the distance to a body's surface from it.
    pub fn collision_radius(&self) -> i64 {
        self.collision_radius
    }

    pub fn set_collision_radius(&mut self, radius: i64) {
        self.collision_radius = radius;
    }

    /// F746/F599's thrust request for the next tick (body axes), or `None` to
    /// return to the set-speed path. The starport descent sets it each frame.
    pub fn set_thrust_override(&mut self, request: Option<[i32; 3]>) {
        self.player.thrust_override = request;
    }

    /// The ship is still in the pad world.
    pub fn on_pad(&self) -> bool {
        self.pad_world.is_some()
    }

    /// The player's F603 fraction words (`+0xec..+0xf0`): F926's pods clone them.
    pub fn velocity_fraction(&self) -> [u16; 3] {
        self.player.motion.fraction
    }

    /// F1677's absolute position in millimetres (the player's own position
    /// outside the pad world, where it is already absolute).
    pub fn absolute_position(&self) -> [i64; 3] {
        match (&self.pad_world, self.pad_state()) {
            (Some(world), Some(state)) => {
                // Resolve the live motion without depending on a previous
                // collision pass's cached body/stamp.
                let mut world = world.clone();
                world.stamp = world.stamp.wrapping_add(1);
                let body = &mut world.body[1];
                body.position = state.0;
                body.velocity = state.1;
                body.orientation = std::array::from_fn(|k| state.2[k / 3][k % 3]);
                body.parent = 2;
                body.rotating = 1;
                // A parked body may not have run a collision tick for 256 frames.
                // Invalidate explicitly: incrementing an 8-bit stamp can alias its
                // old cache stamp and return an obsolete absolute position.
                body.stamp = world.stamp.wrapping_sub(1);
                world.absolute(1).unwrap_or(self.player.motion.position)
            }
            _ => self.player.motion.position,
        }
    }

    /// Leave the pad world: F573 turns the ship back into the linear frame and
    /// F1677 gives its absolute position, so ordinary free flight continues.
    pub fn end_pad_takeoff(&mut self) {
        let Some(state) = self.pad_state() else {
            self.pad_world = None;
            return;
        };
        let Some(mut world) = self.pad_world.take() else {
            return;
        };
        self.pad_local = None;
        world.stamp = world.stamp.wrapping_add(1);
        let body = &mut world.body[1];
        body.position = state.0;
        body.velocity = state.1;
        body.orientation = std::array::from_fn(|k| state.2[k / 3][k % 3]);
        body.parent = 2;
        body.rotating = 1;
        // F1677's absolute position first: it already inverts a rotating frame,
        // so the one-frame hand-over to the system frame does not move the ship
        // by a frame-rotation-sized step.
        world.body[1].stamp = world.stamp.wrapping_sub(1);
        let absolute = world.absolute(1).unwrap_or(state.0);
        world.to_linear(1, 2);
        let body = world.body[1];
        self.player.motion.position = absolute;
        self.player.motion.velocity = body.velocity;
        self.player.motion.orientation =
            std::array::from_fn(|k| std::array::from_fn(|c| body.orientation[3 * k + c]));
        self.player.motion.parent = 0;
        self.player.motion.rotating = 0;
        self.player.motion.switch_delay = 0;
        self.player.motion.updated = 0;
    }

    /// Put the ship at a position (millimetres) with its velocity cleared and the
    /// set speed zero: a jump arrival.
    pub fn teleport(&mut self, position: [i64; 3]) {
        self.shift(position);
        self.player.motion.velocity = [0; 3];
        self.player.set_speed = 0;
    }

    /// Turn toward a point by at most `rate` radians per second over `seconds`: the
    /// original's autopilot sets the turn rates itself rather than pressing keys.
    pub fn turn_toward(&mut self, target: [i64; 3], seconds: f64, rate: f64) {
        let at = self.absolute_position();
        let want: [f64; 3] = std::array::from_fn(|k| (target[k] - at[k]) as f64);
        let length = want.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length < 1.0 {
            return;
        }
        let want = want.map(|v| v / length);
        let rows: [[f64; 3]; 3] = self
            .player
            .motion
            .orientation
            .map(|row| row.map(|v| f64::from(v) / 2_147_483_648.0));
        let norm = |v: [f64; 3]| {
            let l = v.iter().map(|c| c * c).sum::<f64>().sqrt().max(1e-12);
            v.map(|c| c / l)
        };
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let forward = norm(rows[2]);
        let cosine = dot(forward, want).clamp(-1.0, 1.0);
        let angle = cosine.acos();
        if angle < 1e-9 {
            return;
        }
        let mut axis = cross(forward, want);
        if axis.iter().map(|c| c * c).sum::<f64>() < 1e-18 {
            // Dead astern: turn about the ship's up axis.
            axis = norm(rows[1]);
        }
        let axis = norm(axis);
        let step = angle.min(rate * seconds);
        let (sin, cos) = step.sin_cos();
        // Rodrigues' rotation of each row about the axis.
        let turned = rows.map(|v| {
            let c = cross(axis, v);
            let d = dot(axis, v);
            std::array::from_fn(|k| v[k] * cos + c[k] * sin + axis[k] * d * (1.0 - cos))
        });
        // Re-orthonormalise: forward first, then right and up from it.
        let forward = norm(turned[2]);
        let right = norm(cross(turned[1], forward));
        let up = cross(forward, right);
        let q = |v: f64| (v * 2_147_483_647.0) as i32;
        self.player.motion.orientation = [right, up, forward].map(|row| row.map(q));
    }

    /// F746's copy of the target's orientation: the ship is set level with the
    /// starport's rows for the last metres of the pad descent (the source copies
    /// the target's nine dwords when its distance reaches zero).
    pub fn align_to(&mut self, rows: [[f64; 3]; 3]) {
        let q = |v: f64| (v * 2_147_483_647.0) as i32;
        self.player.motion.orientation = rows.map(|row| row.map(q));
    }

    /// F1520 on the ship's rows, as the docking stages leave them: the right and forward
    /// rows normalised through F1519 (0.992 of unit length, the constant `0x3f800000`), the
    /// up row their cross product. A ship launched from an orbital station keeps these rows
    /// (F1088 only zeroes its motion), which is why its thrust is 0.8% weaker than a unit
    /// matrix's.
    pub fn docked_rows(&mut self) {
        self.player.motion.renormalise();
    }

    /// Turn the ship to face a point (keeping it level against the system's y axis).
    pub fn face(&mut self, target: [i64; 3]) {
        let at = self.absolute_position();
        let mut forward: [f64; 3] = std::array::from_fn(|k| (target[k] - at[k]) as f64);
        let length = forward.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length < 1.0 {
            return;
        }
        forward = forward.map(|v| v / length);
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let unit = |v: [f64; 3]| {
            let l = v.iter().map(|c| c * c).sum::<f64>().sqrt().max(1e-12);
            v.map(|c| c / l)
        };
        let reference = if forward[1].abs() > 0.99 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let right = unit(cross(reference, forward));
        let up = cross(forward, right);
        let q = |v: f64| (v * 2_147_483_647.0) as i32;
        self.player.motion.orientation = [right, up, forward].map(|row| row.map(q));
    }

    /// Turn the free-flying ship with a body's frame: `turn` takes a point fixed
    /// in the body's frame at the previous frame to the same point now (the
    /// body's rows' change). F572 puts a ship near a spinning station in the
    /// station's rotating frame, where it stays at rest unless it thrusts; the
    /// port keeps the ship in the system frame, so it carries the ship's offset
    /// from `centre`, its velocity and its rows by the same turn.
    pub fn co_rotate(&mut self, centre: [i64; 3], turn: [[f64; 3]; 3]) {
        if self.pad_world.is_some() {
            return;
        }
        let apply = |v: [f64; 3]| -> [f64; 3] {
            std::array::from_fn(|i| (0..3).map(|j| turn[i][j] * v[j]).sum())
        };
        let motion = &mut self.player.motion;
        let offset: [f64; 3] = std::array::from_fn(|k| (motion.position[k] - centre[k]) as f64);
        let turned = apply(offset);
        motion.position = std::array::from_fn(|k| centre[k] + turned[k].round() as i64);
        motion.velocity = apply(motion.velocity.map(f64::from)).map(|v| v.round() as i32);
        for row in &mut motion.orientation {
            *row = apply(row.map(f64::from)).map(|v| v.round() as i32);
        }
        motion.renormalise();
    }

    /// Move the ship without touching its velocity or set speed (a change of frame).
    pub fn shift(&mut self, position: [i64; 3]) {
        let before = self.absolute_position();
        if let Some(world) = self.pad_world.as_mut() {
            // Navigator follows the moving planet in system coordinates. Carry
            // the local world's origin with it, retaining the ship/pad contact.
            for (axis, value) in position.iter().enumerate() {
                world.body[2].position[axis] =
                    world.body[2].position[axis].wrapping_add(value.wrapping_sub(before[axis]));
            }
            world.stamp = world.stamp.wrapping_add(1);
        } else {
            self.player.motion.position = position;
        }
    }

    pub fn acceleration_step(&self) -> usize {
        crate::console::TIME_ACCELERATION
            .iter()
            .position(|a| *a == self.acceleration)
            .unwrap_or(0)
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    /// The set speed in metres per second.
    pub fn set_speed_metres(&self) -> f64 {
        f64::from(self.player.set_speed) * MM_PER_SECOND / 1000.0
    }

    /// F1018's autopilot cancel: the set speed falls to `DATA_008866`, which the
    /// flight model starts at 0, so the engines brake the ship instead of it
    /// coasting on at the speed the autopilot had chosen. The target and the
    /// navigator's leg are the caller's to clear ([`crate::navigation::Navigator::cancel`]).
    /// F1094's hyperspace exit: the velocity in the system frame, thrust and
    /// the thrust fractions cleared (`+0x8c..0x94`, `+0xf2..0xfa`).
    pub fn leave_hyperspace(&mut self, velocity: [i32; 3]) {
        self.player.motion.velocity = velocity;
        self.player.motion.thrust = [0; 3];
        self.player.motion.fraction = [0; 3];
    }

    pub fn halt(&mut self) {
        self.player.set_speed = 0;
    }

    /// Tell the model which view is showing (2 and 3 are the turrets, whose
    /// angles the steering keys then turn) and the turret angles.
    pub fn set_view(&mut self, view: u8, turret: [i16; 2]) {
        self.player.view = view;
        self.player.turret = turret;
    }

    pub fn view(&self) -> (u8, [i16; 2]) {
        (self.player.view, self.player.turret)
    }

    /// Install the source AI's thrust, velocity caps and F587/F589 turn words.
    pub fn apply_guidance(&mut self, guidance: ffe_core::autopilot::Guidance) {
        self.guidance = Some(guidance);
        self.player.mode = 2;
    }

    /// F746/F671's accelerated pad completion: copy the station rows and seat.
    pub fn settle_autopilot_pad(&mut self, position: [i64; 3], rows: [i32; 9]) {
        self.teleport(position);
        self.player.motion.orientation =
            std::array::from_fn(|r| std::array::from_fn(|c| rows[3 * r + c]));
        self.player.motion.thrust = [0; 3];
        self.player.motion.cap = [0; 3];
        self.player.motion.fraction = [0; 3];
        self.player.mode = 0x2e;
        self.guidance = None;
        // F1008 lowers the undercarriage for AI states past 0x1e, the pad
        // descent and the landed state included; the accelerated touchdown
        // leaves no frames to animate it, so it is down on the pad.
        self.gear_down = true;
        self.gear_word = 0xffff;
    }

    /// F994's pad seat: the ship at `position` with the port's `rows` as its
    /// axes, at rest.
    pub fn place(&mut self, position: [i64; 3], rows: [i32; 9]) {
        self.teleport(position);
        self.player.motion.orientation =
            std::array::from_fn(|r| std::array::from_fn(|c| rows[3 * r + c]));
        self.player.motion.rate = [0; 3];
        self.player.motion.thrust = [0; 3];
    }

    /// A landed ship's pose in a body's axes (`rows`, Q31, one axis per row)
    /// relative to the body at `origin`: the position offset and the ship's
    /// three axes, for [`Self::pin_to_body`].
    pub fn pose_in_body(&self, origin: [i64; 3], rows: [i32; 9]) -> LandedPose {
        let at = self.absolute_position();
        let local = |v: [f64; 3]| -> [f64; 3] {
            std::array::from_fn(|i| (0..3).map(|j| q31(rows[3 * i + j]) * v[j]).sum())
        };
        let offset = local(std::array::from_fn(|k| {
            at[k].wrapping_sub(origin[k]) as f64
        }));
        let axes = self
            .player
            .motion
            .orientation
            .map(|axis| local(axis.map(q31)));
        LandedPose { offset, axes }
    }

    /// Place a landed ship back on its body: F1012's landed handlers keep the
    /// player in the body's rotating frame (`+0x57`), so the ship turns with
    /// the planet (and its port) however far the clock jumps.
    pub fn pin_to_body(&mut self, pose: &LandedPose, origin: [i64; 3], rows: [i32; 9]) {
        let world = |v: [f64; 3]| -> [f64; 3] {
            std::array::from_fn(|j| (0..3).map(|i| q31(rows[3 * i + j]) * v[i]).sum())
        };
        let offset = world(pose.offset);
        self.teleport(std::array::from_fn(|k| {
            origin[k].wrapping_add(offset[k].round() as i64)
        }));
        self.player.motion.orientation = pose.axes.map(|axis| {
            world(axis).map(|v| {
                (v * 2_147_483_648.0)
                    .round()
                    .clamp(-2_147_483_647.0, 2_147_483_647.0) as i32
            })
        });
        self.player.motion.rate = [0; 3];
    }

    pub fn clear_guidance(&mut self) {
        if self.guidance.take().is_some() && self.player.mode == 2 {
            // F1018: resume manual flight with actual speed as the set speed.
            self.player.set_speed = ffe_core::model_math::length(self.player.motion.velocity)
                .min(i32::MAX as u32) as i32;
            self.player.mode = 0;
        }
    }

    pub fn guidance(
        &self,
        target: [i64; 3],
        velocity: [i32; 3],
        modifier: i32,
    ) -> ffe_core::autopilot::Guidance {
        ffe_core::autopilot::guide(
            &self.player.motion,
            self.absolute_position(),
            target,
            velocity,
            &self.player.limits,
            modifier,
        )
    }

    /// The command the AI last installed, which F742 leaves in place on an
    /// event that only runs a state change (F745's clearance request): it
    /// applies F599 and F587 only while F610's distance shift is above 6.
    pub fn held_guidance(&self) -> ffe_core::autopilot::Guidance {
        self.guidance.unwrap_or(ffe_core::autopilot::Guidance {
            thrust: self.player.motion.thrust,
            cap: self.player.motion.cap,
            turns: self.player.motion.autopilot,
            distance_shift: 0,
        })
    }

    /// `ship+0x101`: F742's modifier for F610. F746's final leg is the constant -2.
    pub fn ai_modifier(&self) -> i32 {
        self.ai_modifier
    }

    pub fn body_guidance(
        &self,
        target: [i64; 3],
        velocity: [i32; 3],
        radius: i64,
    ) -> ffe_core::autopilot::Guidance {
        ffe_core::autopilot::guide_to_radius(
            &self.player.motion,
            self.absolute_position(),
            target,
            velocity,
            &self.player.limits,
            self.ai_modifier,
            radius,
        )
    }

    /// Whether the flight module is the one showing (`DATA_008630 < 0`: F1003
    /// sets 0xff, F1006 sets 0x7f when another module such as the map or the
    /// status screen takes over). While it is not, F1037 runs neither the
    /// set-speed keys nor F1038's steering; the keyboard accumulators keep
    /// counting, as the original's input read and `Timer50Hz` do.
    pub fn set_flight_view(&mut self, active: bool) {
        self.player.active = active;
    }

    /// One frame of `elapsed` seconds with `keys` held: F1037/F1038 then F1041,
    /// F601/F602 and F1512 over the elapsed time in the original's units.
    pub fn step(&mut self, elapsed: f64, keys: &Keys) -> Result<(), String> {
        let (_, world_tics) = self.prepare(elapsed, keys);
        if let Some(guidance) = self.guidance {
            integrate_guidance(&mut self.player, guidance, world_tics, 0);
            Ok(())
        } else {
            self.player
                .step(&ThrusterKeys::default(), i64::from(world_tics))
        }
    }

    /// One frame with F569's collision scan against `parent` (a planet, moon or
    /// starport) as the player's frame of reference: the original's F570/F575
    /// contact code runs, so a soft touch settles the ship on the terrain or the
    /// starport and a crash bounces it. `parent.position` is the body's absolute
    /// position. The player is body 1 and the parent body 2 in a two-body world;
    /// the scan's object index in the returned [`Contact`] is 2.
    pub fn step_landing(
        &mut self,
        elapsed: f64,
        keys: &Keys,
        parent: &Body,
        parent_position: [i64; 3],
        player_model: &ModelInfo,
        tools: Option<&SurfaceTools>,
    ) -> Result<Contact, String> {
        self.step_scan(
            elapsed,
            keys,
            parent,
            parent_position,
            None,
            player_model,
            tools,
        )
    }

    /// One frame with F569's collision scan against a starport's pad: the
    /// original's F994 world has the player and the starport as siblings under
    /// the planet, so F569 tests the player's spheres against the starport's
    /// F576 collision stream (F580 children included) and F575's landing,
    /// crash and bounce code. The player stays in the system frame between
    /// frames; the three-body world is built for the tick and the absolute
    /// position is restored after it.
    #[allow(clippy::too_many_arguments)]
    pub fn step_starport_landing(
        &mut self,
        elapsed: f64,
        keys: &Keys,
        planet: &Body,
        planet_position: [i64; 3],
        port: &Body,
        port_position: [i64; 3],
        player_model: &ModelInfo,
        tools: Option<&SurfaceTools>,
    ) -> Result<Contact, String> {
        self.step_scan(
            elapsed,
            keys,
            planet,
            planet_position,
            Some((port, port_position)),
            player_model,
            tools,
        )
    }

    /// One frame with F569's collision scan against an orbital station, the
    /// player's frame: F1024/F1019's docking test reads the result (a soft
    /// touch, `0x40`, on the bay inside the station's region, `0x20`), and a
    /// hull strike is F575's crash and bounce. As for a planet the ship enters
    /// the station's rotating frame (its spinning rows) for the tick.
    pub fn step_station(
        &mut self,
        elapsed: f64,
        keys: &Keys,
        station: &Body,
        station_position: [i64; 3],
        player_model: &ModelInfo,
        tools: Option<&SurfaceTools>,
    ) -> Result<Contact, String> {
        self.step_scan(
            elapsed,
            keys,
            station,
            station_position,
            None,
            player_model,
            tools,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn step_scan(
        &mut self,
        elapsed: f64,
        keys: &Keys,
        parent: &Body,
        parent_position: [i64; 3],
        sibling: Option<(&Body, [i64; 3])>,
        player_model: &ModelInfo,
        tools: Option<&SurfaceTools>,
    ) -> Result<Contact, String> {
        let (_, world_tics) = self.prepare(elapsed, keys);
        let (outer_radius, inner_radius) = player_model.radii();
        let vector: [i64; 3] = std::array::from_fn(|k| {
            self.player.motion.position[k].wrapping_sub(parent_position[k])
        });
        // The playable keeps the ship's absolute position between frames; the scan
        // wants it in the parent's linear frame.
        self.player.motion.parent = 2;
        self.player.motion.rotating = 0;
        self.player.motion.updated = 0;
        self.player.motion.switch_delay = 0;
        self.player.motion.craft = 1;
        self.player.motion.position = vector;
        // The world is rebuilt every frame; do not let the original's 13-tick
        // frame selector run against it.
        self.player.reference_counter = 0xd;
        // A mathematically exact Q31 row set makes the landing dot product of two
        // unit vectors round past 0x7fffffff and wrap negative; the original's
        // integrated rows are a hair under unit length. Keep the same margin.
        for row in &mut self.player.motion.orientation {
            for value in row {
                *value = (f64::from(*value) * 0.999_99) as i32;
            }
        }

        let mut world = Bodies {
            stamp: 1,
            frame_tics: world_tics,
            player: 1,
            ..Bodies::default()
        };
        world.live[1] = true;
        world.physical[1] = true;
        world.live[2] = true;
        world.physical[2] = true;
        let motion = self.player.motion;
        let mut words = Words::default();
        // The landing gear (`+0x9e` word 1 = 0xffff when down): with it down a slow,
        // level touch lands instead of crashing.
        words.0[1] = self.gear_word;
        world.body[1] = Body {
            position: vector,
            velocity: motion.velocity,
            parent: 2,
            index: 1,
            kind: 0,
            orientation: std::array::from_fn(|k| motion.orientation[k / 3][k % 3]),
            outer_radius,
            inner_radius,
            shape: player_model.class(),
            model: motion.model,
            words,
            ..Body::default()
        };
        let mut planet = *parent;
        planet.position = parent_position;
        planet.parent = 0;
        planet.rotating = 0;
        planet.index = 2;
        world.body[2] = planet;
        if let Some((port, port_position)) = sibling {
            // F994: the starport is a sibling of the player in the planet's frame.
            let mut station = *port;
            station.position =
                std::array::from_fn(|k| port_position[k].wrapping_sub(parent_position[k]));
            station.parent = 2;
            station.rotating = 0;
            station.index = 3;
            station.delay = 0;
            station.stamp = 0;
            world.body[3] = station;
            world.live[3] = true;
            world.physical[3] = true;
        }

        // The original enters the planet's rotating frame once (F570/F572) and
        // stays there; this world is rebuilt every frame, so the ship enters it
        // and returns (F573) every frame. Each round trip truncates the
        // position to 28 bits and turns it by the planet's integrated rows, which
        // are a hair under unit length: at a surface port that pulled the ship
        // about 2 m a frame toward the planet's centre (the takeoff that settles
        // back on the ground). Make exactly one round trip a frame here, F572
        // before the tick (so F570 has no switch of its own to make, which with
        // a starport sibling it otherwise made twice) and F573 after it, and
        // take the error that pair makes on the start state back out.
        let round_trip = {
            let mut dry = world.clone();
            dry.to_rotating(1, 2);
            dry.to_linear(1, 2);
            let moved: [i64; 3] = std::array::from_fn(|k| dry.body[1].position[k] - vector[k]);
            let turned: [i32; 3] =
                std::array::from_fn(|k| dry.body[1].velocity[k] - motion.velocity[k]);
            (moved, turned)
        };
        world.to_rotating(1, 2);
        // F570's own switch waits while the delay runs, as after any switch.
        world.body[1].delay = 0x7f;
        world.body[1].give_motion(&mut self.player.motion);
        let contact = tick_guided(
            &mut self.player,
            self.guidance,
            world_tics,
            &mut world,
            tools,
        );
        // F1021 (mode 0) and F1019 (mode 2): F1014 after F1033's scan, on the
        // velocity in the scan's frame. Mode 0 needs the contact free of the
        // 0x20 region and the 0x40 touch; mode 2 only bit 0.
        if let Ok(contact) = &contact {
            let flags = contact.flags;
            let runs = flags & 1 != 0
                && match self.player.mode {
                    0 => flags & 0x60 == 0,
                    2 => true,
                    _ => false,
                };
            if runs {
                self.player.atmospheric_drag(self.speed_word);
                self.atmosphere = Some(self.speed_word);
            }
        }
        // Undo the per-frame landing margin so it does not accumulate, and put
        // the player back in the system frame even when the scan fails (an
        // undecodable starport pad must not leave it parent-relative).
        // Returning to root coordinates also requires F573's orientation/velocity
        // conversion; clearing only +0x57 silently leaves those vectors in the
        // old frame. Far from the planet F570 may already have made it.
        world.body[1].take_motion(&self.player.motion);
        if world.body[1].rotating != 0 {
            world.to_linear(1, 2);
        }
        let body = &mut world.body[1];
        for k in 0..3 {
            body.position[k] -= round_trip.0[k];
            body.velocity[k] = body.velocity[k].wrapping_sub(round_trip.1[k]);
        }
        world.body[1].give_motion(&mut self.player.motion);
        // F1520 is not run here: its F1519 normalises to 0.992 of unit length
        // (a factor 0.985 on the rows it rebuilds), and the original only runs
        // it when F611's drift counter goes negative, which the tick already
        // does. Divide the margin out instead; renormalising every frame cut the
        // thrust 1.5% from the first free-flight frame after the takeoff.
        for row in &mut self.player.motion.orientation {
            for value in row {
                *value = (f64::from(*value) / 0.999_99)
                    .round()
                    .clamp(-2_147_483_647.0, 2_147_483_647.0) as i32;
            }
        }
        // The scan may have cached the ship's absolute position mid-tick (F1677
        // for a pair across frames, the starport's); recompute it from the
        // corrected linear position rather than return that stale value.
        world.stamp = world.stamp.wrapping_add(1);
        world.body[1].stamp = world.stamp.wrapping_sub(1);
        let absolute = world.absolute(1).unwrap_or(vector);
        self.player.motion.position = absolute;
        self.player.motion.parent = 0;
        self.player.motion.rotating = 0;
        self.player.motion.updated = 0;
        self.player.motion.switch_delay = 0;
        contact
    }

    /// Align the input clock with an executed trace: `now` is the original's
    /// `tmLastTime` and `next_tick` its `tmNext50Hz`, in milliseconds.
    pub fn set_input_clock(&mut self, now: u32, next_tick: u32) {
        self.clock_us = u64::from(now) * 1000;
        self.keyboard.set_clock(now, next_tick);
        self.steering.set_clock(now, next_tick);
    }

    /// The whole milliseconds of real time this frame adds to the clock, with
    /// the frame clamped to 100 ms.
    fn frame_ms(&mut self, elapsed: f64) -> u32 {
        let before = self.clock_us / 1000;
        self.clock_us += (elapsed.clamp(0.0, 0.1) * 1e6).round() as u64;
        (self.clock_us / 1000 - before) as u32
    }

    /// The key state F1037/F1038 and set speed for one frame; returns the input
    /// tics and the accelerated world tics.
    fn prepare(&mut self, elapsed: f64, keys: &Keys) -> (u32, u32) {
        // F1099 left last frame's speed in `+0xd8`; F1014 reads it this frame.
        self.speed_word =
            ffe_core::model_math::length(self.player.motion.velocity).min(i32::MAX as u32);
        self.atmosphere = None;
        // TimerAdvance: whole milliseconds of real time this frame (at most
        // 100). The input time is `ms * 0x3e2 / 20` (994 per 20 ms) and the
        // world time `ms * acceleration / 20`, each truncated per frame.
        let ms = self.frame_ms(elapsed);
        let tics = ms * 0x3e2 / 20;
        let world_tics = (u64::from(ms) * u64::from(self.acceleration) / 20) as u32;
        // The input read at the end of the previous frame (the port polls the
        // keys at the start of this one, the same moment), then this frame's
        // 50 Hz ticks: F1037 and F1038 see the accumulators after both.
        let steering = [
            keys.pitch_down,
            keys.pitch_up,
            keys.left,
            keys.right,
            keys.roll_left,
            keys.roll_right,
        ];
        let roles: ScanSet = (0u8..6)
            .filter(|role| steering[usize::from(*role)])
            .collect();
        self.keyboard.read(&keys.scancodes);
        self.steering.read(&roles);
        self.keyboard.advance(ms);
        self.steering.advance(ms);
        self.player.speed_boost = self.keyboard.last_key_count();
        if self.guidance.is_some() {
            self.player.input_time = tics;
            self.player.frame_tics = world_tics;
            return (tics, world_tics);
        }
        let ticks: [u32; 6] = std::array::from_fn(|role| self.steering.held(role as u8));
        let (mouse, mouse_flip) = if keys.mouse_enabled {
            (keys.mouse, keys.mouse_flip)
        } else {
            self.player.mouse_previous = 0;
            (0, [false; 2])
        };
        let controls = Controls {
            mouse,
            faster: keys.faster,
            slower: keys.slower,
            steer: SteerKeys {
                pitch_down: ticks[0],
                pitch_up: ticks[1],
                left: ticks[2],
                right: ticks[3],
                roll_left: ticks[4],
                roll_right: ticks[5],
            },
        };
        self.player.mouse_flip = mouse_flip;
        self.player.input_time = tics;
        self.player.frame_tics = world_tics;
        self.player.update_set_speed(&controls);
        (tics, world_tics)
    }

    /// Copy the state into `flight`: position and velocity in flight units, the
    /// orientation, `yaw`/`pitch` for saves, and the forward thrust for the glows.
    /// In the pad world the player's position is relative to the starport, so
    /// F1677's absolute position is what the views and navigation see.
    pub fn apply(&mut self, flight: &mut Flight) {
        let position = self.absolute_position();
        let motion = &self.player.motion;
        let axes: [[f64; 3]; 3] = motion
            .orientation
            .map(|row| row.map(|v| f64::from(v) / 2_147_483_648.0));
        flight.set_speed = Some(self.player.set_speed);
        flight.gear = self.gear_word;
        flight.position = position.map(|p| p as f64 / UNITS_PER_FLIGHT_UNIT);
        flight.velocity = motion
            .velocity
            .map(|v| f64::from(v) * MM_PER_SECOND / UNITS_PER_FLIGHT_UNIT);
        flight.axes = Some(axes);
        let forward = axes[2];
        flight.yaw = forward[0]
            .atan2(forward[2])
            .rem_euclid(std::f64::consts::TAU);
        flight.pitch = forward[1].clamp(-1.0, 1.0).asin().clamp(-1.4, 1.4);
        let limit = f64::from(self.forward_limit.max(1));
        flight.thrust = (f64::from(motion.thrust[2]) / limit).clamp(-1.0, 1.0);
        flight.ticks = flight.ticks.wrapping_add(1);
        // The pad world and the autopilot hold state the words above do not
        // describe (the ship's frame, its guidance), so a resume then rebuilds.
        flight.resume =
            (self.pad_world.is_none() && self.pad_local.is_none() && self.guidance.is_none())
                .then(|| self.resume_state());
    }
}

/// F727: gravity, capped acceleration, F585 with the AI flag, then F1512.
fn integrate_guidance(
    player: &mut Player,
    guidance: ffe_core::autopilot::Guidance,
    tics: u32,
    mass: u32,
) {
    player.motion.thrust = guidance.thrust;
    player.motion.cap = guidance.cap;
    player.motion.autopilot = guidance.turns;
    player.motion.gravity(mass, tics as i32);
    player.motion.accelerate(tics);
    player
        .motion
        .orient(tics as i32, true, tics as i32, &mut player.scanner);
    player.motion.advance(i64::from(tics));
}

fn tick_guided(
    player: &mut Player,
    guidance: Option<ffe_core::autopilot::Guidance>,
    tics: u32,
    world: &mut Bodies,
    tools: Option<&SurfaceTools>,
) -> Result<Contact, String> {
    let Some(guidance) = guidance else {
        return player.tick_with(&ThrusterKeys::default(), i64::from(tics), world, 1, tools);
    };
    let mass = world.body[usize::from(player.motion.parent)].mass;
    integrate_guidance(player, guidance, tics, mass);
    world.body[1].take_motion(&player.motion);
    let contact = world
        .collide_with(1, tools, 0xff)
        .ok_or("The autopilot contact needs the models' collision records.")?;
    world.body[1].give_motion(&mut player.motion);
    player.altitude = contact.altitude();
    player.contact_flags = contact.flags;
    Ok(contact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::{Flight, Mode};

    /// Resume as `main` does: the model's state goes into the flight record,
    /// through the native file, and a new model is built from it. `keep` is
    /// whether the integer words travel with the save.
    fn resumed(model: &mut OriginalFlight, template: &Flight, keep: bool) -> OriginalFlight {
        let mut state = template.clone();
        model.apply(&mut state);
        if !keep {
            state.resume = None;
        }
        let bytes =
            crate::saves::encode_game(&crate::saves::SaveGame::from_flight(&state)).unwrap();
        let loaded = crate::saves::decode_game(&bytes).unwrap().state;
        OriginalFlight::from_flight(&loaded, None)
    }

    /// Fly with steering keys held 20 frames in every 50 (a save is made from a
    /// menu, with no key down), resuming at frame 35 of each block. Returns the
    /// first frame at which the resumed flight differs from the uninterrupted
    /// one, if any.
    fn first_difference(keep: bool) -> Option<usize> {
        let template = Flight {
            mode: Mode::Flight,
            velocity: [0.0, 0.0, 50.0],
            set_speed: Some(200_000),
            ..Flight::default()
        };
        let held = Keys {
            pitch_up: true,
            left: true,
            scancodes: [0x1eu8, 0x20].into_iter().collect(),
            ..Keys::default()
        };
        let mut straight = OriginalFlight::from_flight(&template, None);
        let mut cut = OriginalFlight::from_flight(&template, None);
        for frame in 0..400 {
            let keys = if frame % 50 < 20 {
                &held
            } else {
                &Keys::default()
            };
            if frame % 50 == 35 {
                cut = resumed(&mut cut, &template, keep);
            }
            straight.step(0.02, keys).unwrap();
            cut.step(0.02, keys).unwrap();
            if straight.player().motion != cut.player().motion {
                return Some(frame);
            }
        }
        None
    }

    #[test]
    fn a_resumed_free_flight_matches_the_uninterrupted_one_exactly() {
        // PAR-18: rates, sub-unit fractions, caps and thrust words are part of
        // the save, so eight resumes in 400 frames change nothing.
        assert_eq!(first_difference(true), None);
        // Without them (what a save was before) the next frame already differs.
        assert!(first_difference(false).is_some());
    }

    #[test]
    fn selected_signed_speed_survives_save_and_model_reconstruction() {
        for selected in [-1_000_000, -1, 0, 1, 1_000_000] {
            let state = Flight {
                mode: Mode::Flight,
                velocity: [0.0, 0.0, 50.0],
                set_speed: Some(selected),
                ..Flight::default()
            };
            let bytes =
                crate::saves::encode_game(&crate::saves::SaveGame::from_flight(&state)).unwrap();
            let mut loaded = crate::saves::decode_game(&bytes).unwrap().state;
            let mut model = OriginalFlight::from_flight(&loaded, None);
            assert_eq!(model.set_speed_units(), selected);
            model.apply(&mut loaded);
            assert_eq!(loaded.set_speed, Some(selected));
        }
    }

    /// F1018's cancel drops the set speed to DATA_008866 (0), so the model brakes
    /// the ship rather than coasting on at the autopilot's chosen speed.
    #[test]
    fn halt_zeroes_the_set_speed_and_the_ship_brakes() {
        let state = Flight {
            mode: Mode::Flight,
            velocity: [0.0, 0.0, 500.0],
            ..Flight::default()
        };
        let mut flight = OriginalFlight::from_flight(&state, None);
        assert!(flight.set_speed_metres() > 400.0);
        let start = flight.set_speed_metres();
        flight.halt();
        assert_eq!(flight.set_speed_units(), 0);
        for _ in 0..1_000 {
            flight.step(0.05, &Keys::default()).unwrap();
        }
        let velocity = flight.player().motion.velocity;
        let speed = (velocity.iter().map(|c| f64::from(*c).powi(2)).sum::<f64>()).sqrt()
            * MM_PER_SECOND
            / 1000.0;
        assert!(speed < start * 0.2, "{speed} m/s after halt from {start}");
    }

    /// F1037 with the engines off (`+0xe9` bit 0, F1056's action 9): the
    /// set-speed keys are skipped (`jnz JUMP_004289`) but F1038 still steers.
    #[test]
    fn engines_off_skips_the_set_speed_keys_but_still_steers() {
        let state = Flight {
            mode: Mode::Flight,
            set_speed: Some(500_000),
            ..Flight::default()
        };
        let mut flight = OriginalFlight::from_flight(&state, None);
        flight.set_engines_on(false);
        let orientation = flight.player().motion.orientation;
        for keys in [
            Keys {
                faster: true,
                pitch_up: true,
                scancodes: [0x1cu8, 0x1e].into_iter().collect(),
                ..Keys::default()
            },
            Keys {
                slower: true,
                pitch_up: true,
                scancodes: [0x36u8, 0x1e].into_iter().collect(),
                ..Keys::default()
            },
        ] {
            for _ in 0..50 {
                flight.step(0.02, &keys).unwrap();
            }
            assert_eq!(flight.set_speed_units(), 500_000);
        }
        assert_ne!(
            flight.player().motion.orientation,
            orientation,
            "F1038 steered"
        );
        flight.set_engines_on(true);
        let faster = Keys {
            faster: true,
            scancodes: [0x1cu8].into_iter().collect(),
            ..Keys::default()
        };
        flight.step(0.02, &faster).unwrap();
        assert!(flight.set_speed_units() > 500_000);
    }

    /// F1037 returns at once while `DATA_008630 >= 0` (F1006: another module
    /// is showing): Return, right Shift and the steering keys change nothing,
    /// and the slow-down latch (`+0xe9` bit 7) is not cleared as a frame with
    /// right Shift up would clear it. Back in the flight view the keys work.
    #[test]
    fn another_module_showing_stops_the_set_speed_and_steering_keys() {
        let state = Flight {
            mode: Mode::Flight,
            set_speed: Some(1_000_000),
            ..Flight::default()
        };
        let mut flight = OriginalFlight::from_flight(&state, None);
        // Right Shift held down to zero sets the latch.
        let slower = Keys {
            slower: true,
            scancodes: [0x36u8].into_iter().collect(),
            ..Keys::default()
        };
        for _ in 0..400 {
            flight.step(0.02, &slower).unwrap();
            if flight.set_speed_units() == 0 {
                break;
            }
        }
        assert_eq!(flight.set_speed_units(), 0);
        assert_ne!(flight.player().flags & 0x80, 0, "latched at zero");
        let orientation = flight.player().motion.orientation;
        flight.set_flight_view(false);
        let busy = Keys {
            faster: true,
            pitch_up: true,
            left: true,
            scancodes: [0x1cu8, 0x1e, 0x33].into_iter().collect(),
            ..Keys::default()
        };
        for _ in 0..50 {
            flight.step(0.02, &busy).unwrap();
        }
        assert_eq!(flight.set_speed_units(), 0, "no set-speed change");
        assert_ne!(flight.player().flags & 0x80, 0, "latch untouched");
        assert_eq!(
            flight.player().motion.orientation,
            orientation,
            "no steering"
        );
        flight.set_flight_view(true);
        flight.step(0.02, &busy).unwrap();
        assert!(flight.set_speed_units() > 0, "Return works again");
    }
}

/// A landed ship's position offset and axes in its body's axes; see
/// [`OriginalFlight::pin_to_body`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandedPose {
    pub offset: [f64; 3],
    pub axes: [[f64; 3]; 3],
}

fn q31(v: i32) -> f64 {
    f64::from(v) / 2_147_483_648.0
}
