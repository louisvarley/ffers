//! The playable missile field: launch, tracking, the F953 damage hand-off and
//! the F1120 pylon words (guide: `docs/notes/combat.md`).
//!
//! [`ffe_core::missile`] holds the original's tables and the missile object.
//! This module owns the live list, turns the cache's 48-byte model statistics
//! into a [`Spec`], and applies a hit through `NpcShip::hit` (F953's
//! shields→hull→death path) so the missile and the laser share one damage
//! implementation.
use ffe_core::laser::Damage;
use ffe_core::missile::{self, Event, Missile, Spec, State, Target, TickOptions};
use ffe_core::npc::NpcShip;

use crate::ships::ShipStats;

/// The live missiles of the current system.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MissileField {
    pub missiles: Vec<Missile>,
    /// The next F941 object id. The core `Missile` has no object slots, so the
    /// field hands each launch an id; `DATA_008877` refers to it.
    next_id: u32,
}

/// F1054's chaff object: an inert point a F1057-retargeted missile chases.
/// The source gives the object model 0xa4 and no movement of its own beyond
/// F1514's initial separation, so the port keeps a position and the model's
/// collision radius. `id` is in [`missile::DECOY_BASE`]'s range.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Decoy {
    pub id: u32,
    pub position: [i64; 3],
    pub radius: i64,
}

impl Decoy {
    pub fn target(&self) -> Target {
        Target {
            id: self.id,
            position: self.position,
            velocity: [0.0; 3],
            radius: self.radius,
        }
    }
}

/// A missile F718's ECM destroyed, for F719's explosion and the message log.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcmLoss {
    pub id: u32,
    /// The missile model (`+0x82`), to explain which ECM defeated it.
    pub model: u32,
    /// The system-frame position (millimetres) F719 puts the model-0x9b object
    /// at.
    pub position: [i64; 3],
    /// The missile's axes, for the explosion object's random matrix.
    pub axes: [[f64; 3]; 3],
}

/// What a tick did, for the message log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissileEvent {
    /// F720/F721/F953: the hit ship and the outcome.
    Hit {
        /// F941's missile object id, for F780's notification to an evading ship.
        id: u32,
        target: u32,
        name: String,
        damage: u32,
        outcome: Damage,
    },
    /// F720/F721/F953 on the player. The caller applies the player's shields and
    /// hull (`Flight::take_damage`).
    HitPlayer { id: u32, damage: u32 },
    /// F718/F721: the missile reached a chaff object. No ship takes damage.
    HitDecoy { id: u32, decoy: u32 },
    /// F758's timeout, F719's explosion without damage.
    Expired { id: u32, target: Option<u32> },
}

impl MissileEvent {
    /// F941's object id of the missile this event consumed, for F780's
    /// notification to an evading ship.
    pub fn id(&self) -> u32 {
        match self {
            MissileEvent::Hit { id, .. }
            | MissileEvent::HitPlayer { id, .. }
            | MissileEvent::HitDecoy { id, .. }
            | MissileEvent::Expired { id, .. } => *id,
        }
    }
}

impl MissileField {
    /// Validate native-save state before any missile physics or rendering.
    pub fn validate(&self) -> bool {
        let mut ids = std::collections::HashSet::new();
        self.missiles.len() <= 4096
            && self.missiles.iter().all(|missile| {
                let spec = missile.spec;
                ids.insert(missile.id)
                    && missile.id != self.next_id
                    && (4..=13).contains(&spec.model)
                    && spec.radius >= 0
                    && [spec.forward, spec.reverse, spec.side, spec.turn]
                        .iter()
                        .all(|v| v.is_finite())
                    && spec.forward >= 0.0
                    && spec.reverse <= 0.0
                    && spec.side >= 0.0
                    && spec.turn >= 0.0
                    && missile
                        .velocity
                        .iter()
                        .chain(missile.axes.iter().flatten())
                        .chain(&missile.thrust)
                        .all(|v| v.is_finite())
                    && missile.age.is_finite()
                    && missile.age >= 0.0
                    && missile.guidance.is_finite()
                    && missile.guidance >= 0.0
                    && matches!(
                        missile.ai_state,
                        missile::AI_MISSILE | missile::AI_MINE | missile::AI_MINE_MODEL_4
                    )
            })
    }

    /// F941: a spec from a missile code, the model's statistics and the F1533
    /// collision radius. Models without statistics (4 and 5) fall back to a
    /// ballistic launch, as their words are zero in the cache.
    pub fn spec(code: u8, stats: Option<ShipStats>, radius: i64) -> Option<Spec> {
        let model = missile::model(code)?;
        Some(match stats {
            Some(stats) => {
                Spec::from_words(model, stats.forward_thrust, stats.reverse_thrust, radius)
            }
            None => Spec::ballistic(model, radius),
        })
    }

    /// F941/F1514: put a missile in flight. `pylon` is 1-based.
    #[allow(clippy::too_many_arguments)]
    pub fn launch(
        &mut self,
        spec: Spec,
        code: u8,
        owner: u32,
        target: Option<u32>,
        position: [i64; 3],
        axes: [[f64; 3]; 3],
        pylon: usize,
        launcher_radius: i64,
    ) {
        self.missiles.push(Missile::launch(
            spec,
            missile::ai_state(code),
            owner,
            target,
            position,
            axes,
            pylon,
            launcher_radius,
        ));
        if let Some(missile) = self.missiles.last_mut() {
            missile.id = self.next_id;
        }
        self.next_id = self.next_id.wrapping_add(1);
    }

    pub fn clear(&mut self) {
        self.missiles.clear();
    }

    /// The live missile with F941's object id (`+0x86`).
    pub fn get(&self, id: u32) -> Option<&Missile> {
        self.missiles
            .iter()
            .find(|missile| missile.id == id && missile.alive())
    }

    /// F718's ECM branch: destroy every live missile the source's model table
    /// defeats. The frame's `DATA_008835` bits are global, so there is no range
    /// or owner test. Returns the destroyed missiles for F719's explosion.
    /// This is [`Self::ecm_with`] with enhancement-8 off.
    pub fn ecm(&mut self, naval: bool) -> Vec<EcmLoss> {
        self.ecm_with(naval, false)
    }

    /// [`Self::ecm`] with `enhancement-8` (`enhancement.ecm_nn500`): when
    /// `nn500` is set, F718's immune model 8 (the NN500 Naval Missile, code
    /// `0x84`) joins the branch-0 models and either E.C.M. destroys it.
    pub fn ecm_with(&mut self, naval: bool, nn500: bool) -> Vec<EcmLoss> {
        let mut losses = Vec::new();
        for missile in &mut self.missiles {
            if missile.alive() && missile::ecm_destroys_with(missile.spec.model, naval, nn500) {
                missile.state = State::Spent;
                losses.push(EcmLoss {
                    id: missile.id,
                    model: missile.spec.model,
                    position: missile.position,
                    axes: missile.axes,
                });
            }
        }
        self.missiles.retain(Missile::alive);
        losses
    }

    /// F1057: pull a missile away from `from` to the chaff object `to`. Only
    /// models 6..=11 are affected, and the original rolls
    /// `F1530() & 0x1000` for each; `allow` is that per-missile gate.
    pub fn retarget(
        &mut self,
        from: u32,
        to: u32,
        mut allow: impl FnMut(&Missile) -> bool,
    ) -> usize {
        let mut count = 0;
        for missile in &mut self.missiles {
            if missile.alive()
                && missile.target == Some(from)
                && missile::retargetable(missile.spec.model)
                && allow(missile)
            {
                missile.target = Some(to);
                count += 1;
            }
        }
        count
    }

    /// Tick every missile over `seconds` of world time, then apply F953 to any
    /// ship a missile reached. F758's explosion on timeout does no damage.
    pub fn tick(&mut self, seconds: f64, ships: &mut [NpcShip]) -> Vec<MissileEvent> {
        self.tick_with_options(seconds, ships, None, &[], TickOptions::PORT_DEFAULT)
    }

    /// [`Self::tick`] with the player and the live chaff objects as targets.
    /// `player` is [`missile::PLAYER_TARGET`]'s object; a hit on it is reported
    /// as [`MissileEvent::HitPlayer`] so the caller can run F953 on the player.
    pub fn tick_with(
        &mut self,
        seconds: f64,
        ships: &mut [NpcShip],
        player: Option<Target>,
        decoys: &[Decoy],
    ) -> Vec<MissileEvent> {
        self.tick_with_options(seconds, ships, player, decoys, TickOptions::PORT_DEFAULT)
    }

    /// [`Self::tick_with`] with the `enhancement-2`/`enhancement-6` options the
    /// live game reads from `config.cfg`. The options are runtime-only.
    pub fn tick_with_options(
        &mut self,
        seconds: f64,
        ships: &mut [NpcShip],
        player: Option<Target>,
        decoys: &[Decoy],
        options: TickOptions,
    ) -> Vec<MissileEvent> {
        if self.missiles.is_empty() {
            return Vec::new();
        }
        let mut targets: Vec<Target> = ships
            .iter()
            .filter(|ship| ship.visible())
            .map(|ship| Target {
                id: ship.id,
                position: ship.position,
                velocity: ship.velocity,
                radius: ship.radius,
            })
            .collect();
        if let Some(player) = player {
            targets.push(player);
        }
        targets.extend(decoys.iter().map(Decoy::target));
        let mut events = Vec::new();
        for missile in &mut self.missiles {
            match missile.tick_with(seconds, &targets, options) {
                Some(Event::Hit { target, damage }) => {
                    if target == missile::PLAYER_TARGET {
                        events.push(MissileEvent::HitPlayer {
                            id: missile.id,
                            damage,
                        });
                    } else if missile::is_decoy(target) {
                        events.push(MissileEvent::HitDecoy {
                            id: missile.id,
                            decoy: target,
                        });
                    } else if let Some(ship) = ships.iter_mut().find(|ship| ship.id == target) {
                        let name = ship.name.clone();
                        let outcome = ship.hit(damage);
                        events.push(MissileEvent::Hit {
                            id: missile.id,
                            target,
                            name,
                            damage,
                            outcome,
                        });
                    }
                }
                Some(Event::Expired { target }) => {
                    events.push(MissileEvent::Expired {
                        id: missile.id,
                        target,
                    });
                }
                None => {}
            }
        }
        self.missiles.retain(|missile| missile.alive());
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffe_core::missile::AI_MISSILE;
    use ffe_core::npc::{Role, State};

    fn stats() -> ShipStats {
        let mut block = [0u8; 48];
        let word = |block: &mut [u8; 48], offset: usize, value: i16| {
            block[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        };
        word(&mut block, 0x00, 32700);
        word(&mut block, 0x02, -6540);
        word(&mut block, 0x06, 25);
        word(&mut block, 0x0e, 0x4014);
        word(&mut block, 0x12, 2);
        ShipStats::from_block(&block).unwrap()
    }

    fn ship(id: u32, position: [i64; 3]) -> NpcShip {
        NpcShip {
            id,
            slot: 0,
            model: 23,
            role: Role::Pirate,
            name: "Pirate 1".to_owned(),
            type_name: String::new(),
            position,
            velocity: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
            up: [0.0, 1.0, 0.0],
            radius: 10_000,
            damage: ffe_core::laser::DamageState {
                shields: 100,
                shields_max: 100,
                hull: 100,
                mass: 25,
            },
            shield_strength: 20,
            rating: 10,
            cargo: 8,
            state: State::Attack,
            anchor: [0; 3],
            orbit_radius: 0.0,
            phase: 0.0,
            max_speed: 0.0,
            accel: 0.0,
            turn_rate: 0.0,
            limits: None,
            motion: Default::default(),
            speed: 0.0,
            hostile: true,
            laser: None,
            guns: ffe_core::laser::State::default(),
            general: 0,
            pylons: [0; ffe_core::missile::PYLONS],
            missile_rng: ffe_core::RandomState::new(1, 2),
            control_clock: 0.0,
            think_tics: ffe_core::npc::THINK_TICS,
            scrambled: false,
            scramble_tics: 0,
            attack_phase: 0,
            in_fudge: false,
            pass_hits: 0,
            combat_velocity: [0; 3],
            waypoint: [0; 3],
            pursuit: ffe_core::npc::Pursuit::default(),
            placed: false,
            turned: false,
            missile_target: None,
            ecm_attempted: false,
            launch: None,
            ecm_burst: None,
            bounty: 0,
            leader: None,
            formation: [0; 3],
            pass: 0.0,
            flee: 0.0,
            responding: false,
            in_combat: false,
            alert: false,
            npc_target: None,
            contract: None,
            flash: 0.0,
            shot: None,
            death: 0.0,
            station: None,
            bay: None,
            pad: None,
            dock_timer: 0.0,
            dock_span: 0.0,
            dock_retry: 0.0,
            dock_wait: 0.0,
            seed: 0,
            script_target: 0,
            ai_code: None,
            class_flags: 0,
            collision_radius: 0,
            drive: 0,
            cargo_fuel: 0,
            damaged: 0,
            retro: 0,
            for_timeout: 0,
            smoke_timeout: 0,
            ecm_timeout: 0,
            avoid_timeout: 0,
            frame_parent: 0,
            frame_position: [0; 3],
            frame_due: false,
            avoid_due: false,
            avoid_target: None,
            jump: ffe_core::object_record::HyperspaceWords::default(),
            jump_clock: [0; 2],
            timer_clock: 0.0,
        }
    }

    fn axes() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    #[test]
    fn the_player_code_maps_to_model_6_and_200_damage() {
        let spec = MissileField::spec(0x82, Some(stats()), 2100).unwrap();
        assert_eq!(spec.model, 6);
        assert_eq!(spec.damage, 200);
        assert_eq!(spec.radius, 2100);
        assert!(spec.forward > 19_000.0);
        assert!(spec.turn > 3.0);
        assert_eq!(
            MissileField::spec(0xc0, Some(stats()), 1200).map(|spec| spec.model),
            Some(4)
        );
        assert!(MissileField::spec(0x8a, None, 0).is_none());
    }

    fn spec_for(model: u32) -> Spec {
        Spec::from_words(model, 32700, -6540, 2100)
    }

    #[test]
    fn launches_get_f941_object_ids_and_can_be_found() {
        let mut field = MissileField::default();
        field.launch(spec_for(6), 0x82, 0, None, [0; 3], axes(), 1, 5200);
        field.launch(spec_for(7), 0x83, 0, None, [0; 3], axes(), 2, 5200);
        assert_eq!(field.missiles[0].id, 0);
        assert_eq!(field.missiles[1].id, 1);
        assert!(field.get(0).is_some());
        assert_eq!(field.get(1).unwrap().spec.model, 7);
        assert!(field.get(2).is_none());
    }

    #[test]
    fn ecm_destroys_f718s_models_and_only_f719s_explosions_remain() {
        let mut field = MissileField::default();
        for model in [4, 5, 6, 7, 8, 9, 10, 11, 12, 13] {
            field.launch(spec_for(model), 0x82, 0, None, [0; 3], axes(), 1, 5200);
        }
        // The standard system defeats models 5 and 6; its DATA_008835 bit 2.
        let losses = field.ecm(false);
        assert_eq!(
            losses.iter().map(|loss| loss.model).collect::<Vec<_>>(),
            vec![5, 6]
        );
        assert!(losses.iter().all(|loss| loss.id < 10));
        // F719's object data is the missile's position and axes (the odd-pylon
        // separation offset from F941).
        assert_eq!(losses[0].position, [0x2000, -5200, 0]);
        assert_eq!(losses[0].axes, axes());
        // Naval E.C.M. adds 7, 9 and 10.
        let losses = field.ecm(true);
        assert_eq!(
            losses.iter().map(|loss| loss.model).collect::<Vec<_>>(),
            vec![7, 9, 10]
        );
        // 4, 8, 11, 12 and 13 are immune to both.
        assert_eq!(
            field
                .missiles
                .iter()
                .map(|missile| missile.spec.model)
                .collect::<Vec<_>>(),
            vec![4, 8, 11, 12, 13]
        );
        assert!(field.ecm(true).is_empty());
    }

    #[test]
    fn a_player_missile_does_not_contact_the_player_it_launched_from() {
        let mut field = MissileField::default();
        let spec = MissileField::spec(0x82, Some(stats()), 2100).unwrap();
        let player = Target {
            id: ffe_core::missile::PLAYER_TARGET,
            position: [0; 3],
            velocity: [0.0; 3],
            radius: 10_000,
        };
        // The player's missile is owned by PLAYER_TARGET in the field, so F569's
        // owner test skips the player's own object.
        field.launch(
            spec,
            0x82,
            ffe_core::missile::PLAYER_TARGET,
            Some(9),
            [0; 3],
            axes(),
            1,
            5200,
        );
        let mut ships = vec![ship(9, [0, 0, 10_000_000])];
        let events = field.tick_with(0.02, &mut ships, Some(player), &[]);
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(field.missiles.len(), 1);
    }

    #[test]
    fn the_launcher_is_a_ray_contact_only_when_the_toggle_is_off() {
        let spec = MissileField::spec(0x82, Some(stats()), 2100).unwrap();
        let launch = |field: &mut MissileField| {
            field.launch(spec, 0x82, 3, Some(9), [0; 3], axes(), 1, 20_000);
        };
        // F575 skips the missile's parent (the port's `owner`) in F718's point
        // contact test, so with enhancement-2 off the missile still flies out
        // of the launcher. This is the original launch path.
        let mut field = MissileField::default();
        launch(&mut field);
        // The first step's contact position is inside the launcher's hull.
        field.missiles[0].position = [0; 3];
        let mut ships = vec![ship(3, [0; 3])];
        ships[0].radius = 20_000;
        let events = field.tick_with_options(
            0.02,
            &mut ships,
            None,
            &[],
            TickOptions {
                ignore_launcher: false,
                swept: false,
            },
        );
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(field.missiles.len(), 1);

        // enhancement-6's ray starts at the launch point, so it does meet the
        // launcher; enhancement-2 removes the launcher from that ray.
        let mut field = MissileField::default();
        launch(&mut field);
        field.missiles[0].position = [0; 3];
        let mut ships = vec![ship(3, [0; 3])];
        ships[0].radius = 20_000;
        let events = field.tick_with_options(
            0.02,
            &mut ships,
            None,
            &[],
            TickOptions {
                ignore_launcher: false,
                swept: true,
            },
        );
        assert!(matches!(
            events.as_slice(),
            [MissileEvent::Hit { target: 3, .. }]
        ));
        assert!(field.missiles.is_empty());

        let mut field = MissileField::default();
        launch(&mut field);
        field.missiles[0].position = [0; 3];
        let mut ships = vec![ship(3, [0; 3])];
        ships[0].radius = 20_000;
        let events = field.tick_with_options(
            0.02,
            &mut ships,
            None,
            &[],
            TickOptions {
                ignore_launcher: true,
                swept: true,
            },
        );
        assert!(events.is_empty(), "{events:?}");
        assert_eq!(field.missiles.len(), 1);
    }

    #[test]
    fn the_swept_toggle_catches_a_thin_target_between_two_steps() {
        let spec = spec_for(6);
        let mut field = MissileField::default();
        field.launch(spec, 0x82, 0, Some(9), [0; 3], axes(), 1, 2100);
        field.missiles[0].position = [0; 3];
        field.missiles[0].velocity = [0.0, 0.0, 1_000_000.0];
        field.missiles[0].thrust = [0.0; 3];
        let mut ships = vec![ship(9, [0, 0, 10_000])];
        ships[0].radius = 100;
        let options = |swept| TickOptions {
            ignore_launcher: false,
            swept,
        };
        // Off: both endpoints are more than 2.2 m from the target.
        let mut point = field.clone();
        let mut point_ships = ships.clone();
        assert!(point
            .tick_with_options(0.02, &mut point_ships, None, &[], options(false))
            .is_empty());
        assert_eq!(point.missiles.len(), 1);
        // On: the swept path passes through it.
        let events = field.tick_with_options(0.02, &mut ships, None, &[], options(true));
        assert!(matches!(
            events.as_slice(),
            [MissileEvent::Hit { target: 9, .. }]
        ));
        assert!(field.missiles.is_empty());
    }

    #[test]
    fn the_ecm_toggle_destroys_only_the_nn500_in_the_field() {
        let mut field = MissileField::default();
        field.launch(spec_for(8), 0x84, 0, None, [0; 3], axes(), 1, 5200);
        // The source's table leaves model 8 immune to both systems.
        assert!(field.ecm(false).is_empty());
        assert!(field.ecm(true).is_empty());
        assert_eq!(field.missiles.len(), 1);
        // Enhancement-8 makes either burst destroy it.
        let losses = field.ecm_with(false, true);
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].model, ffe_core::missile::NN500_MODEL);
        assert!(field.missiles.is_empty());
    }

    #[test]
    fn an_npc_missile_reaches_the_player_target_and_a_chaff_decoy() {
        let mut field = MissileField::default();
        let spec = MissileField::spec(0x82, Some(stats()), 2100).unwrap();
        let player = Target {
            id: ffe_core::missile::PLAYER_TARGET,
            position: [0, 0, 100_000],
            velocity: [0.0; 3],
            radius: 10_000,
        };
        field.launch(
            spec,
            0x82,
            3,
            Some(ffe_core::missile::PLAYER_TARGET),
            [0; 3],
            axes(),
            2,
            5200,
        );
        let events = field.tick_with(0.4, &mut [], Some(player), &[]);
        assert!(matches!(
            events.as_slice(),
            [MissileEvent::HitPlayer { damage: 200, .. }]
        ));

        let decoy = Decoy {
            id: ffe_core::missile::DECOY_BASE,
            position: [0, 0, 100_000],
            radius: 2_000,
        };
        field.launch(
            spec,
            0x82,
            3,
            Some(ffe_core::missile::PLAYER_TARGET),
            [0; 3],
            axes(),
            2,
            5200,
        );
        assert_eq!(
            field.retarget(ffe_core::missile::PLAYER_TARGET, decoy.id, |_| true),
            1
        );
        let events = field.tick_with(0.4, &mut [], None, &[decoy]);
        assert!(matches!(
            events.as_slice(),
            [MissileEvent::HitDecoy { decoy: id, .. }] if *id == decoy.id
        ));
    }

    #[test]
    fn f1057_retargets_only_the_models_it_names_and_only_from_the_chosen_target() {
        let mut field = MissileField::default();
        field.launch(spec_for(6), 0x82, 0, Some(1), [0; 3], axes(), 1, 5200);
        field.launch(spec_for(5), 0x81, 0, Some(1), [0; 3], axes(), 1, 5200);
        field.launch(spec_for(7), 0x83, 0, Some(1), [0; 3], axes(), 1, 5200);
        field.launch(spec_for(8), 0x84, 0, Some(2), [0; 3], axes(), 1, 5200);
        // F1530's 0x1000 gate is per missile; this one accepts.
        assert_eq!(field.retarget(1, 99, |_| true), 2);
        assert_eq!(field.missiles[0].target, Some(99));
        assert_eq!(field.missiles[1].target, Some(1), "model 5 is not named");
        assert_eq!(field.missiles[2].target, Some(99));
        assert_eq!(field.missiles[3].target, Some(2), "another target");
        // A failed roll leaves them all alone.
        assert_eq!(field.retarget(99, 0, |_| false), 0);
        assert_eq!(field.missiles[0].target, Some(99));
    }

    #[test]
    fn a_launched_missile_hits_a_nearby_ship_through_f953_and_is_removed() {
        let mut field = MissileField::default();
        let spec = MissileField::spec(0x82, Some(stats()), 2100).unwrap();
        let mut ships = vec![ship(3, [0, 0, 120_000])];
        ships[0].damage.shields = 300;
        ships[0].damage.shields_max = 300;
        field.launch(spec, 0x82, 0, Some(3), [0; 3], axes(), 1, 5200);
        assert_eq!(field.missiles.len(), 1);
        assert_eq!(field.missiles[0].ai_state, AI_MISSILE);
        // The missile's separation position (8192, -5200, 0) is inside the
        // 131 m fuse of a target 120 m away at the first guidance update.
        let events = field.tick(ffe_core::missile::GUIDANCE_INTERVAL, &mut ships);
        assert_eq!(events.len(), 1);
        match &events[0] {
            MissileEvent::Hit {
                target,
                name,
                damage,
                outcome,
                ..
            } => {
                assert_eq!((*target, name.as_str(), *damage), (3, "Pirate 1", 200));
                assert_eq!(*outcome, Damage::Shields);
            }
            other => panic!("{other:?}"),
        }
        assert!(field.missiles.is_empty());
        assert_eq!(ships[0].damage.shields, 100);
        assert_eq!(ships[0].damage.hull, 100);
    }

    #[test]
    fn a_kill_goes_through_f942_and_a_timeout_does_not_damage() {
        let mut field = MissileField::default();
        let spec = MissileField::spec(0x82, Some(stats()), 2100).unwrap();
        let mut ships = vec![ship(3, [0, 0, 120_000])];
        ships[0].damage.shields = 0;
        ships[0].damage.hull = 100;
        field.launch(spec, 0x82, 0, Some(3), [0; 3], axes(), 1, 5200);
        let events = field.tick(ffe_core::missile::GUIDANCE_INTERVAL, &mut ships);
        assert!(matches!(
            events.as_slice(),
            [MissileEvent::Hit {
                outcome: Damage::Destroyed,
                ..
            }]
        ));
        assert!(!ships[0].alive());
        assert_eq!(ships[0].state, State::Dead);

        let mut field = MissileField::default();
        field.launch(spec, 0x82, 0, Some(99), [0; 3], axes(), 1, 5200);
        let mut far = vec![ship(3, [0, 0, 10_000_000])];
        let mut events = Vec::new();
        for _ in 0..61 {
            events.extend(field.tick(ffe_core::missile::GUIDANCE_INTERVAL, &mut far));
        }
        assert!(matches!(events.as_slice(), [MissileEvent::Expired { .. }]));
        assert!(field.missiles.is_empty());
        assert_eq!(far[0].damage.hull, 100);
    }
}
