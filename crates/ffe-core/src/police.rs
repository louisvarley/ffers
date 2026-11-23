//! The police response to a wanted player: the original's F238 and F240, with F239's launch
//! described as data (`ffe.asm:18939-19170`).
//!
//! Nothing in the original makes existing ships hostile when the player becomes wanted. F236
//! (the offence) sets the wanted bit (`DATA_008646` bit 0) and remembers the starport that
//! witnessed it (`DATA_008804+0x50dc`, the "offended" object). Two handlers of the module
//! table `DATA_005038` then act on that starport's record:
//!
//! The two flag bytes are the police module's registration for F152 (`DATA_008623`/`8624`
//! `+ 8 * module`): `DATA_008646` bit 0 (the wanted bit, set by F236) enables the day event and
//! `DATA_008647` (the chase flag) enables the frame event, so F238 only runs while the player is
//! wanted and F240 only while a chase is running or dying away. The executed original's
//! police stop rolling the day the wanted bit is cleared (by F783, F233 or an empty starport).
//!
//! - **F238** (event 10, run by F152 once per call that finds a whole day, `0xc22e` game tics,
//!   elapsed, while the wanted bit is set) starts the chase and launches the police. It is a
//!   no-op while the chase flag `DATA_008647` is not zero.
//!   - An orbital station or closed starport (`+0x14c` bit `0x40` clear) with police left in
//!     `+0xdc` and its traffic record idle (`+0xc9 == 0`) launches up to two police (F239
//!     indices 1 then 0, state `0x22`), takes `+0xdc` down by one each, sets the record's mode
//!     to `0xff` (the door opens) and the chase flag to 1. With the record busy it is reset
//!     instead, when it is not handling another ship. With no police left it clears the wanted
//!     bit.
//!   - An open-air starport (bit `0x40` set) waits for the clock `+0xde` to be zero to start the
//!     chase, then each day has a one in four chance (F1530's word below `0x4000`) of launching
//!     one police ship (index `F922(4)`, state 1) and using up one of `+0xdc`; running out ends
//!     the chase (flag `0xff`) and clears the wanted bit.
//! - **F240** (event 3, every frame) steps `+0xde` by a quarter of the frame's tics: up while the
//!   chase flag is positive (a full clock ends the chase), down while it is negative (an empty
//!   clock ends it). The executed original leaves the clock alone while the flag is zero (it
//!   stays 0 between the offence and the first F238, and 65535 once a chase has filled it), so
//!   the first day after an offence starts the chase and the days after that roll for launches.
//!
//! F239 itself allocates a police ship (F772(0xe)), places it at the starport and gives it the
//! state it was asked for; when no slot is free it gives the count back. A police ship in state
//! `0x22` (F748) waits until the starport's mode is `0xfe` or `0` (the door open), then attacks
//! (state 1). F783 clears the chase and the wanted bit when a police ship (`+0x118 == 0xe`) is
//! removed; see `npc::Legal::clear_alert`.

use crate::npc::Random;
use crate::traffic::Starport;

/// `DATA_008647` while the chase runs.
pub const CHASE: i8 = 1;
/// `DATA_008647` while the chase is dying away (`0xff`): F240 counts `+0xde` down.
pub const ENDING: i8 = -1;
/// F772's type for a police ship.
pub const POLICE_TYPE: u8 = 0xe;
/// F239's state for a ship waiting in the starport (F748's `+0xff`).
pub const WAITING: u8 = 0x22;
/// The attack state F748 and F238's roll hand a launched ship.
pub const ATTACK: u8 = 1;
/// F238's roll: F1530's low word must be below this for the daily launch.
const ROLL: u32 = 0x4000;

/// F659's launch points for an open-air starport (bit `0x40`), `DATA_006202`
/// to `DATA_006205`. The table is chosen by the station's highest bay (`+0xce`,
/// the port's `Body::bays`): 1 uses `006202`, 3 uses `006203`, 2 (any other
/// non-zero value) uses `006204` and 0 uses `006205`. Each entry is the
/// launcher's local offset in 1/1024 mm, rotated by the starport's axes.
pub const OPEN_AIR_LAUNCH: [[[i32; 3]; 4]; 4] = [
    // bay 0: DATA_006205
    [
        [1660, 60, -1220],
        [1660, 60, -1220],
        [1740, 60, -1220],
        [1740, 60, -1220],
    ],
    // bay 1: DATA_006202
    [[-24, 30, 95], [-220, 30, 95], [-24, 60, 95], [-220, 60, 95]],
    // bay 2: DATA_006204
    [
        [970, 50, -1470],
        [-1030, 50, -1470],
        [1030, 50, -1470],
        [-970, 50, -1470],
    ],
    // bay 3: DATA_006203
    [
        [810, 50, -1270],
        [-870, 50, -1270],
        [870, 50, -1270],
        [-810, 50, -1270],
    ],
];

/// F659's launch points for a closed station (bit `0x20`), `DATA_006209`. The
/// two F238 launches (indices 1 and 0) start inside the station's lift.
pub const STATION_LAUNCH: [[i32; 3]; 4] = [[-40, 9, 40], [40, 9, 40], [-40, 9, -40], [40, 9, -40]];

/// F659's fourth branch (`DATA_006206`..`006208`), for a launcher that is
/// neither a starport nor a station: not rotated, and the z word quartered.
/// F239 only ever passes a starport, so this is documentation of the source,
/// not a path the port takes.
pub const OTHER_LAUNCH: [[i32; 3]; 4] = [
    [-25000, 0, 7500],
    [25000, 0, 7500],
    [-25000, 0, -1875],
    [25000, 0, -1875],
];

/// F659: the launch vector for `index`, in millimetres from the launcher's
/// centre. The source rotates the table entry by the launcher's orientation
/// (`F1669`), shifts it left 10 and returns it as an `Int64` vector. The port's
/// ships are all in the system frame, so the caller adds the launcher's
/// position (F239's station branch does the same).
pub fn launch_offset(open_air: bool, bays: u16, index: u8, axes: [[f64; 3]; 3]) -> [i64; 3] {
    let index = usize::from(index) & 3;
    if !open_air {
        // An indoor station; F238 asks for indices 1 and 0.
        let v = STATION_LAUNCH[index];
        return rotate(axes, v);
    }
    let table = match bays {
        1 => 1,
        3 => 3,
        0 => 0,
        _ => 2,
    };
    rotate(axes, OPEN_AIR_LAUNCH[table][index])
}

/// `F1669_VecMatMul` scaled by 1024: a table vector in the launcher's axes to
/// the system frame.
fn rotate(axes: [[f64; 3]; 3], v: [i32; 3]) -> [i64; 3] {
    std::array::from_fn(|k| {
        let sum = axes[0][k] * f64::from(v[0])
            + axes[1][k] * f64::from(v[1])
            + axes[2][k] * f64::from(v[2]);
        (sum * 1024.0).round() as i64
    })
}

/// One call of F239: a police ship to create at the offended starport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Launch {
    /// F239's second argument: which of the starport's launch places (F659's table).
    pub index: u8,
    /// F239's third argument: the ship's `+0xff`, [`WAITING`] or [`ATTACK`].
    pub state: u8,
}

/// F238 for the offended starport `port`. `law` is its `+0x14c` bit `0x40` (an open-air
/// starport), `chase` is `DATA_008647`, `wanted` is `DATA_008646` bit 0 and `player` the
/// traffic record's id for the player (`DATA_008857`). `rng` is the shared pair: F1530 and
/// F922 draw from it in the original's order. The launches are returned for the caller to
/// make with [`Launch`]; a ship it cannot make is given back with [`give_back`].
pub fn day(
    port: &mut Starport,
    law: bool,
    chase: &mut i8,
    wanted: &mut bool,
    player: u8,
    rng: &mut Random,
) -> Vec<Launch> {
    let mut launches = Vec::new();
    // The module's day event is enabled by the wanted bit alone (see the module notes).
    if !*wanted || *chase != 0 {
        return launches;
    }
    if law {
        if port.heat == 0 {
            *chase = CHASE;
        } else if rng.advance() & 0xffff < ROLL {
            let left = port.police_left().wrapping_sub(1);
            port.response = Some(left);
            if left < 0 {
                *chase = ENDING;
                *wanted = false;
                port.response = Some(0);
            } else {
                launches.push(Launch {
                    index: rng.bounded(4) as u8,
                    state: ATTACK,
                });
            }
        }
        return launches;
    }
    if port.police_left() <= 0 {
        port.mode = 1;
        *wanted = false;
        port.response = Some(0);
        return launches;
    }
    if port.mode != 0 {
        // Busy: only reset the record when it is not handling another ship.
        if port.current != 0 && i32::from(port.current as i8) != i32::from(player) {
            return launches;
        }
        port.mode = 1;
        port.current = 0;
        port.timeout = 0;
        return launches;
    }
    let mut index = 1i32;
    loop {
        launches.push(Launch {
            index: index as u8,
            state: WAITING,
        });
        let left = port.police_left().wrapping_sub(1);
        port.response = Some(left);
        if left == 0 {
            break;
        }
        index -= 1;
        if index < 0 {
            break;
        }
    }
    port.mode = 0xff;
    *chase = CHASE;
    port.heat = 0;
    launches
}

/// F239's failure: no slot for the police ship, so the count is given back.
pub fn give_back(port: &mut Starport) {
    port.response = Some(port.police_left().wrapping_add(1));
}

/// F240 for one frame of `tics` game tics.
pub fn frame(port: &mut Starport, chase: &mut i8, tics: u32) {
    if *chase == 0 {
        return;
    }
    let step = (tics >> 2) as i32;
    let heat = i32::from(port.heat);
    if *chase < 0 {
        let heat = heat - step;
        if heat <= 0 {
            *chase = 0;
            port.heat = 0;
        } else {
            port.heat = heat as u16;
        }
    } else {
        let heat = heat + step;
        if heat >= 0xffff {
            *chase = 0;
            port.heat = 0xffff;
        } else {
            port.heat = heat as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The traffic record's id for the player (`DATA_008857`).
    const PLAYER: u8 = 1;

    fn port(bays: u16) -> Starport {
        Starport::new(bays)
    }

    /// The executed original (`--flight idle 8000 --start-key=0x02`, the laser fired at frame
    /// 200, `FFE_RNG_TRACE`): at Hope (face 1, 16 police) F238's first roll with a full clock
    /// (frame 532) draws F1530 = 63904 and stops; the roll at frame 1151 draws F1530 = 5089 from
    /// the state below, then F922(4) = 0 from the state after it, and launches one police ship.
    #[test]
    fn the_roll_reproduces_the_executed_originals_draws() {
        let mut rng = Random::new(0x2214_d408, 0x69ab_ba80);
        let mut p = port(1);
        p.heat = 0xffff;
        let (mut chase, mut wanted) = (0, true);
        let launches = day(&mut p, true, &mut chase, &mut wanted, PLAYER, &mut rng);
        assert_eq!(
            launches,
            [Launch {
                index: 0,
                state: ATTACK
            }]
        );
        assert_eq!(p.police_left(), 15);
        assert_eq!((chase, wanted), (0, true));
        // F1530 left the state the original's F922(4) row starts from.
        let mut again = Random::new(0x2214_d408, 0x69ab_ba80);
        assert_eq!(again.advance() & 0xffff, 5089);
        assert_eq!(again, Random::new(0x210c_13e1, 0x9a81_0442));
        // A missed roll draws exactly one F1530 and launches nothing.
        let (mut w, mut c) = (true, 0);
        for k in 0..32u32 {
            let mut r = Random::new(k.wrapping_mul(0x9e37_79b9), 0x1111_1111);
            let start = r;
            let mut port = port(1);
            port.heat = 0xffff;
            if day(&mut port, true, &mut c, &mut w, PLAYER, &mut r).is_empty() {
                let mut one = start;
                one.advance();
                assert_eq!(r, one, "a missed roll draws exactly one F1530");
                assert_eq!(port.police_left(), 16);
                return;
            }
        }
        panic!("no missed roll in 32 tries");
    }

    #[test]
    fn the_response_starts_at_eight_per_bay_plus_eight() {
        assert_eq!(port(3).police_left(), 32);
        assert_eq!(port(7).police_left(), 64);
        assert_eq!(port(1).police_left(), 16);
    }

    #[test]
    fn nothing_happens_while_a_chase_is_running() {
        let mut p = port(3);
        let mut rng = Random::new(1, 2);
        let (mut chase, mut wanted) = (CHASE, true);
        assert!(day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng).is_empty());
        assert_eq!((p.mode, chase, wanted), (0, CHASE, true));
        assert_eq!(rng, Random::new(1, 2));
    }

    #[test]
    fn nothing_happens_unless_the_player_is_wanted() {
        let mut p = port(3);
        let mut rng = Random::new(1, 2);
        let (mut chase, mut wanted) = (0, false);
        assert!(day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng).is_empty());
        assert_eq!((p.mode, p.police_left(), chase), (0, 32, 0));
        assert_eq!(rng, Random::new(1, 2));
    }

    #[test]
    fn an_idle_station_launches_two_waiting_police_and_opens_its_door() {
        let mut p = port(3);
        let mut rng = Random::new(1, 2);
        let (mut chase, mut wanted) = (0, true);
        let launches = day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng);
        assert_eq!(
            launches,
            [
                Launch {
                    index: 1,
                    state: WAITING
                },
                Launch {
                    index: 0,
                    state: WAITING
                }
            ]
        );
        assert_eq!((p.mode, p.police_left(), p.heat), (0xff, 30, 0));
        assert_eq!((chase, wanted), (CHASE, true));
        // F238 drew nothing.
        assert_eq!(rng, Random::new(1, 2));
    }

    #[test]
    fn the_launch_stops_when_the_count_reaches_zero() {
        let mut p = port(3);
        p.response = Some(1);
        let mut rng = Random::new(1, 2);
        let (mut chase, mut wanted) = (0, true);
        let launches = day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng);
        assert_eq!(launches.len(), 1);
        assert_eq!(p.police_left(), 0);
    }

    #[test]
    fn a_station_with_no_police_clears_the_wanted_bit() {
        let mut p = port(3);
        p.response = Some(0);
        let mut rng = Random::new(1, 2);
        let (mut chase, mut wanted) = (0, true);
        assert!(day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng).is_empty());
        assert_eq!((p.mode, wanted, chase), (1, false, 0));
    }

    #[test]
    fn a_busy_record_is_reset_unless_another_ship_holds_it() {
        let mut rng = Random::new(1, 2);
        // Another ship (id 9) is being handled: nothing changes.
        let mut p = port(3);
        p.mode = 0xff;
        p.current = 9;
        let (mut chase, mut wanted) = (0, true);
        assert!(day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng).is_empty());
        assert_eq!((p.mode, p.current), (0xff, 9));
        // The player's own handling, or none, is reset.
        for current in [PLAYER, 0] {
            let mut p = port(3);
            p.mode = 0xff;
            p.current = current;
            p.timeout = 0x7080;
            day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng);
            assert_eq!((p.mode, p.current, p.timeout), (1, 0, 0));
        }
    }

    #[test]
    fn an_open_air_starport_starts_the_chase_when_its_clock_is_zero() {
        let mut p = port(3);
        let mut rng = Random::new(1, 2);
        let (mut chase, mut wanted) = (0, true);
        assert!(day(&mut p, true, &mut chase, &mut wanted, PLAYER, &mut rng).is_empty());
        assert_eq!(chase, CHASE);
        assert_eq!(rng, Random::new(1, 2));
    }

    #[test]
    fn an_open_air_starport_rolls_each_day_for_one_police_ship() {
        // The roll is F1530's low word below 0x4000: count launches over many days.
        let mut rng = Random::new(0x1234_5678, 0x9abc_def0);
        let mut launched = 0;
        let mut days = 0;
        for _ in 0..4000 {
            let mut p = port(7);
            p.heat = 100;
            let (mut chase, mut wanted) = (0, true);
            let launches = day(&mut p, true, &mut chase, &mut wanted, PLAYER, &mut rng);
            days += 1;
            if let [launch] = launches[..] {
                launched += 1;
                assert_eq!(launch.state, ATTACK);
                assert!(launch.index < 4);
                assert_eq!(p.police_left(), 63);
            } else {
                assert!(launches.is_empty());
                assert_eq!(p.police_left(), 64);
            }
        }
        let share = f64::from(launched) / f64::from(days);
        assert!((0.2..0.3).contains(&share), "one in four, got {share}");
    }

    #[test]
    fn an_open_air_starport_out_of_police_ends_the_chase() {
        let mut rng = Random::new(1, 2);
        // Find a roll that succeeds, with no police left to launch.
        for _ in 0..64 {
            let mut p = port(3);
            p.heat = 5;
            p.response = Some(0);
            let (mut chase, mut wanted) = (0, true);
            let before = rng;
            let launches = day(&mut p, true, &mut chase, &mut wanted, PLAYER, &mut rng);
            let rolled = before != rng;
            assert!(rolled);
            if chase == ENDING {
                assert!(launches.is_empty());
                assert!(!wanted);
                assert_eq!(p.police_left(), 0);
                return;
            }
        }
        panic!("the roll never came up in 64 days");
    }

    #[test]
    fn a_failed_launch_gives_the_count_back() {
        let mut p = port(3);
        let (mut chase, mut wanted) = (0, true);
        let mut rng = Random::new(1, 2);
        let launches = day(&mut p, false, &mut chase, &mut wanted, PLAYER, &mut rng);
        assert_eq!(p.police_left(), 32 - launches.len() as i16);
        give_back(&mut p);
        assert_eq!(p.police_left(), 31);
    }

    #[test]
    fn the_clock_grows_with_a_chase_and_ends_it_when_full() {
        let mut p = port(3);
        let mut chase = CHASE;
        // 994 tics a frame: a quarter is 248.
        frame(&mut p, &mut chase, 994);
        assert_eq!((p.heat, chase), (248, CHASE));
        for _ in 0..300 {
            frame(&mut p, &mut chase, 994);
        }
        assert_eq!((p.heat, chase), (0xffff, 0));
        // With no chase the clock is left alone.
        frame(&mut p, &mut chase, 994);
        assert_eq!((p.heat, chase), (0xffff, 0));
        let mut idle = port(3);
        let mut none = 0;
        frame(&mut idle, &mut none, 994);
        assert_eq!(idle.heat, 0);
    }

    #[test]
    fn the_clock_runs_down_while_the_chase_is_ending() {
        let mut p = port(3);
        p.heat = 600;
        let mut chase = ENDING;
        frame(&mut p, &mut chase, 994);
        assert_eq!((p.heat, chase), (352, ENDING));
        frame(&mut p, &mut chase, 994);
        frame(&mut p, &mut chase, 994);
        assert_eq!((p.heat, chase), (0, 0));
    }

    #[test]
    fn f659s_open_air_launch_vectors_follow_the_bay_table() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        // DATA_006202 (one bay): F238's F922(4) index 0 is (-24, 30, 95).
        assert_eq!(
            launch_offset(true, 1, 0, identity),
            [-24 * 1024, 30 * 1024, 95 * 1024]
        );
        // DATA_006203 (three bays): index 2 is (870, 50, -1270).
        assert_eq!(
            launch_offset(true, 3, 2, identity),
            [870 * 1024, 50 * 1024, -1270 * 1024]
        );
        // DATA_006204 (any other non-zero bay count) and DATA_006205 (zero).
        assert_eq!(
            launch_offset(true, 2, 1, identity),
            [-1030 * 1024, 50 * 1024, -1470 * 1024]
        );
        assert_eq!(
            launch_offset(true, 0, 3, identity),
            [1740 * 1024, 60 * 1024, -1220 * 1024]
        );
        // The table is rotated into the starport's frame, then shifted by 10.
        let turned = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            launch_offset(true, 1, 2, turned),
            [-60 * 1024, -24 * 1024, 95 * 1024]
        );
    }

    #[test]
    fn f659s_closed_station_vectors_start_in_the_lift() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        // DATA_006209 index 1: F238's first launch.
        assert_eq!(
            launch_offset(false, 3, 1, identity),
            [40 * 1024, 9 * 1024, 40 * 1024]
        );
        assert_eq!(
            launch_offset(false, 3, 0, identity),
            [-40 * 1024, 9 * 1024, 40 * 1024]
        );
    }
}
