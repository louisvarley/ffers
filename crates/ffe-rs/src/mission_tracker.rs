//! `enhancement-17`, the mission tracker: the most urgent active contract on the
//! flight view (where to go, how many jumps, how long is left), and every active
//! contract's destination marked on the enhanced galaxy map.
//!
//! It only reads the contracts (`ffe_core::missions::State`) and the clock; it
//! changes nothing in the game. A contract is tracked to its destination system;
//! a military photograph or bombing that has been carried out is tracked back to
//! the system it was accepted in, where it pays.

use ffe_core::missions::{Kind, Mission, State, Status};
use ffe_core::sector::{self, Tables as SectorTables};

/// Seconds in a game day.
const DAY: f64 = 86_400.0;

/// Where a contract sends the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub id: u32,
    /// The system to fly to.
    pub system: u32,
    /// The station or place named there.
    pub place: String,
    pub deadline_day: u32,
}

/// Where `mission` sends the player now, if it is active.
pub fn target(mission: &Mission) -> Option<Target> {
    if mission.status != Status::Active {
        return None;
    }
    let back = mission.kind == Kind::MilitaryStrike && mission.done;
    let (system, place) = if back {
        (mission.origin, mission.origin_name.clone())
    } else {
        (mission.destination, mission.destination_name.clone())
    };
    Some(Target {
        id: mission.id,
        system,
        place,
        deadline_day: mission.deadline_day,
    })
}

/// Every active contract's target, the most urgent (earliest deadline) first.
pub fn targets(missions: &State) -> Vec<Target> {
    let mut targets: Vec<Target> = missions.missions.iter().filter_map(target).collect();
    targets.sort_by_key(|target| (target.deadline_day, target.id));
    targets
}

/// Seconds until `deadline_day` runs out at `now_days` (the clock in fractional game
/// days, as `Navigator::day` counts them): a contract is overdue once the day after
/// its deadline begins.
pub fn seconds_left(deadline_day: u32, now_days: f64) -> f64 {
    (f64::from(deadline_day) + 1.0 - now_days) * DAY
}

/// The time left as the tracker says it: days and hours, or hours and minutes
/// inside a day, or "Overdue".
pub fn time_left_text(seconds: f64) -> String {
    if seconds <= 0.0 {
        return "Overdue".to_owned();
    }
    let minutes = (seconds / 60.0) as u64;
    let (days, hours, minutes) = (minutes / 1440, (minutes / 60) % 24, minutes % 60);
    if days > 0 {
        format!("{days}d {hours:02}h left")
    } else {
        format!("{hours}h {minutes:02}m left")
    }
}

/// The 16.16 position of system `code` (sector x in bits 0..12, y in 13..25, the star's
/// number in the sector in 26..31).
pub fn system_position(tables: &SectorTables<'_>, code: u32) -> Option<[i32; 3]> {
    let (x, y) = ((code & 0x1fff) as u16, ((code >> 13) & 0x1fff) as u16);
    let stars = tables.stars(x, y);
    let star = stars.get((code >> 26) as usize)?;
    Some(sector::position(x, y, &star.vector))
}

/// The tracker's two lines: the destination, then the jumps and the time left.
/// `jumps` is `None` when the drive's range is unknown, `Some(0)` in the
/// destination system.
pub fn hud_lines(place: &str, system: &str, jumps: Option<usize>, seconds: f64) -> [String; 2] {
    let first = if place.is_empty() || place == system {
        format!("Mission: {system}")
    } else {
        format!("Mission: {place}, {system}")
    };
    let route = match jumps {
        Some(0) => "In system".to_owned(),
        Some(1) => "1 jump".to_owned(),
        Some(n) if n < usize::MAX => format!("{n} jumps"),
        _ => "Out of range".to_owned(),
    };
    [first, format!("{route}  {}", time_left_text(seconds))]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mission(id: u32, deadline_day: u32, status: Status) -> Mission {
        Mission {
            id,
            kind: Kind::Parcel,
            code: 0x981b,
            origin: 1,
            origin_name: "Home Port".to_owned(),
            destination: 2,
            destination_name: "Far Port".to_owned(),
            reward: 100,
            accepted_day: 0,
            deadline_day,
            passengers: 0,
            cargo: String::new(),
            seed: 0,
            basic_data: 0,
            client: String::new(),
            status,
            settled_day: None,
            done: false,
            side: 0,
            overdue_noticed: false,
        }
    }

    #[test]
    fn the_most_urgent_active_contract_comes_first() {
        let state = State {
            missions: vec![
                mission(1, 30, Status::Active),
                mission(2, 10, Status::Complete),
                mission(3, 12, Status::Active),
            ],
            ..State::default()
        };
        let ids: Vec<u32> = targets(&state).iter().map(|t| t.id).collect();
        assert_eq!(ids, [3, 1]);
    }

    #[test]
    fn a_strike_that_is_done_goes_back_to_where_it_was_accepted() {
        let mut strike = mission(4, 9, Status::Active);
        strike.kind = Kind::MilitaryStrike;
        assert_eq!(target(&strike).unwrap().system, 2);
        strike.done = true;
        let back = target(&strike).unwrap();
        assert_eq!((back.system, back.place.as_str()), (1, "Home Port"));
    }

    #[test]
    fn the_time_left_counts_to_the_end_of_the_deadline_day() {
        // Day 5 at noon with the deadline on day 6: a day and a half.
        assert_eq!(seconds_left(6, 5.5), 1.5 * DAY);
        assert_eq!(time_left_text(1.5 * DAY), "1d 12h left");
        assert_eq!(time_left_text(5_400.0), "1h 30m left");
        assert_eq!(time_left_text(-1.0), "Overdue");
        assert_eq!(
            hud_lines("Far Port", "Lave", Some(2), 2.0 * DAY),
            [
                "Mission: Far Port, Lave".to_owned(),
                "2 jumps  2d 00h left".to_owned()
            ]
        );
        assert_eq!(
            hud_lines("", "Lave", Some(0), 60.0)[1],
            "In system  0h 01m left"
        );
    }
}
