//! Plotting a course through systems (`enhancement-12`): given where the player is and a
//! star beyond one jump's range, find the stars to hop through.
//!
//! The drive's range limits one jump, so a far star takes several. The plotter finds the
//! course with the fewest jumps (each costs fuel, time and a one-in-128 chance of a
//! mis-jump), then the shortest in total distance, and prefers inhabited stops (where fuel
//! can be bought) and quieter ones (fewer pirate patrols) among courses that are otherwise
//! equal. It is an A* search over the galaxy's stars, so the search is generic over a
//! `near` function that lists the stars one jump from a star; [`plot_in_galaxy`] supplies the
//! real sector tables, and the tests supply a line of synthetic stars.

use ffe_core::sector::{self, Tables as SectorTables};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// A star on a course: its system code and 16.16 position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    pub code: u32,
    pub position: [i32; 3],
}

/// Why no course was plotted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseError {
    /// Even in a straight line the star is more than `max_hops` jumps away.
    TooFar { jumps: usize, max_hops: usize },
    /// No chain of stars, each within range of the last, reaches it (a gap in the
    /// stars wider than the drive's range, or the search gave up).
    NoCourse,
}

impl std::fmt::Display for CourseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFar { jumps, max_hops } => write!(
                formatter,
                "Too far to plot: at least {jumps} jumps ({max_hops} at most)."
            ),
            Self::NoCourse => write!(
                formatter,
                "No course: no chain of stars within your drive's range reaches it."
            ),
        }
    }
}

/// The most jumps a plotted course may have.
pub const MAX_HOPS: usize = 8;
/// A cost of one jump; everything else (distance, penalties) is far smaller, so the
/// fewest jumps always wins.
const HOP: i64 = 1_000_000;
/// The stars a search looks at before it gives up.
const MAX_EXPANSIONS: usize = 30_000;

/// The jump count a straight line to `to` needs at least.
pub fn jumps_needed(from: [i32; 3], to: [i32; 3], reach: i32) -> usize {
    let distance = sector::distance(from, to);
    if reach <= 0 {
        return usize::MAX;
    }
    usize::try_from((distance + reach - 1) / reach)
        .unwrap_or(usize::MAX)
        .max(1)
}

/// Plot the stops from `from` (not included) to `to` (the last). `reach` is the drive's range
/// in the distance units of `sector::distance`; `near` lists the stars within one jump of
/// a star with each one's penalty (small: a preference between equal courses).
pub fn plot(
    from: Stop,
    to: Stop,
    reach: i32,
    max_hops: usize,
    near: &mut dyn FnMut(&Stop) -> Vec<(Stop, i64)>,
) -> Result<Vec<Stop>, CourseError> {
    if from.code == to.code {
        return Ok(Vec::new());
    }
    let minimum = jumps_needed(from.position, to.position, reach);
    if minimum > max_hops {
        return Err(CourseError::TooFar {
            jumps: minimum,
            max_hops,
        });
    }
    if sector::distance(from.position, to.position) <= reach {
        return Ok(vec![to]);
    }
    let heuristic = |star: &Stop| {
        if star.code == to.code {
            0
        } else {
            jumps_needed(star.position, to.position, reach) as i64 * HOP
        }
    };
    // code -> (cost so far, hops, the star it was reached from)
    let mut best: HashMap<u32, (i64, usize, u32)> = HashMap::new();
    let mut stops: HashMap<u32, Stop> = HashMap::new();
    best.insert(from.code, (0, 0, from.code));
    stops.insert(from.code, from);
    let mut queue = BinaryHeap::new();
    queue.push(Reverse((heuristic(&from), 0i64, from.code)));
    let mut expanded = 0;
    while let Some(Reverse((_, cost, code))) = queue.pop() {
        let Some(&(known, hops, _)) = best.get(&code) else {
            continue;
        };
        if cost > known {
            continue;
        }
        if code == to.code {
            let mut path = Vec::new();
            let mut at = code;
            while at != from.code {
                path.push(stops[&at]);
                at = best[&at].2;
            }
            path.reverse();
            return Ok(path);
        }
        expanded += 1;
        if expanded > MAX_EXPANSIONS || hops >= max_hops {
            continue;
        }
        let here = stops[&code];
        for (star, penalty) in near(&here) {
            if star.code == code {
                continue;
            }
            let leg = i64::from(sector::distance(here.position, star.position));
            if leg > i64::from(reach) {
                continue;
            }
            // A penalty only counts for a stop on the way, not for the star asked for.
            let penalty = if star.code == to.code { 0 } else { penalty };
            let next = cost + HOP + leg + penalty;
            if best.get(&star.code).is_none_or(|&(old, _, _)| next < old) {
                best.insert(star.code, (next, hops + 1, code));
                stops.insert(star.code, star);
                queue.push(Reverse((next + heuristic(&star), next, star.code)));
            }
        }
    }
    Err(CourseError::NoCourse)
}

/// What a stop costs to use as a stop: an uninhabited star has no port to refuel at,
/// and a patrolled one is dangerous to arrive in.
fn stop_penalty(code: u32, systems: Option<&ffe_core::galaxy::Tables>) -> i64 {
    let Some(tables) = systems else {
        return 0;
    };
    if tables.market(code).population == 0 {
        return 3_000;
    }
    i64::from(crate::enhanced_map::Threat::from_patrol(tables.details(code).bytes[1]).rank()) * 400
}

/// The galaxy's stars as [`plot`] wants them: the sectors' stars near a star, generated
/// once each.
pub struct Galaxy<'a> {
    sectors: &'a SectorTables<'a>,
    systems: Option<&'a ffe_core::galaxy::Tables<'a>>,
    reach: i32,
    cache: HashMap<(u16, u16), Vec<(Stop, i64)>>,
}

impl<'a> Galaxy<'a> {
    pub fn new(
        sectors: &'a SectorTables<'a>,
        systems: Option<&'a ffe_core::galaxy::Tables<'a>>,
        reach: i32,
    ) -> Self {
        Self {
            sectors,
            systems,
            reach,
            cache: HashMap::new(),
        }
    }

    /// The stars of the sector (`x`, `y`) with their penalties.
    fn sector(&mut self, x: u16, y: u16) -> &[(Stop, i64)] {
        let (sectors, systems) = (self.sectors, self.systems);
        self.cache.entry((x, y)).or_insert_with(|| {
            sectors
                .stars(x, y)
                .iter()
                .enumerate()
                .map(|(index, star)| {
                    let code = crate::galaxy_map::sector_code(x, y) | (index as u32) << 26;
                    (
                        Stop {
                            code,
                            position: sector::position(x, y, &star.vector),
                        },
                        stop_penalty(code, systems),
                    )
                })
                .collect()
        })
    }

    /// The stars within one jump of `here`.
    pub fn near(&mut self, here: &Stop) -> Vec<(Stop, i64)> {
        let unit = sector::distance([0, 0, 0], [0x1_0000, 0, 0]).max(1);
        let span = (self.reach / unit + 1).clamp(1, 12);
        let (sx, sy) = (here.position[0] >> 16, here.position[1] >> 16);
        let reach = self.reach;
        let mut found = Vec::new();
        for dx in -span..=span {
            for dy in -span..=span {
                let (x, y) = ((sx + dx) as u16, (sy + dy) as u16);
                for (star, penalty) in self.sector(x, y) {
                    if star.code != here.code
                        && sector::distance(here.position, star.position) <= reach
                    {
                        found.push((*star, *penalty));
                    }
                }
            }
        }
        found
    }
}

/// Plot a course across the real galaxy.
pub fn plot_in_galaxy(
    sectors: &SectorTables<'_>,
    systems: Option<&ffe_core::galaxy::Tables<'_>>,
    from: Stop,
    to: Stop,
    reach: i32,
) -> Result<Vec<Stop>, CourseError> {
    let mut galaxy = Galaxy::new(sectors, systems, reach);
    plot(from, to, reach, MAX_HOPS, &mut |here| galaxy.near(here))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of stars along x, one `step` apart (in the units of `sector::distance`
    /// for 16.16 positions: a sector is `0x1_0000`).
    fn line(count: u32, step: i32) -> Vec<Stop> {
        (0..count)
            .map(|i| Stop {
                code: i + 1,
                position: [i as i32 * step, 0, 0],
            })
            .collect()
    }

    fn near_of(
        stars: Vec<Stop>,
        penalties: HashMap<u32, i64>,
    ) -> impl FnMut(&Stop) -> Vec<(Stop, i64)> {
        move |here| {
            stars
                .iter()
                .filter(|star| star.code != here.code)
                .map(|star| (*star, penalties.get(&star.code).copied().unwrap_or(0)))
                .collect()
        }
    }

    #[test]
    fn a_star_in_one_jump_is_the_whole_course() {
        let stars = line(6, 0x8000);
        let reach = sector::distance([0, 0, 0], [0x1_0000, 0, 0]);
        let course = plot(
            stars[0],
            stars[2],
            reach,
            8,
            &mut near_of(stars.clone(), HashMap::new()),
        )
        .unwrap();
        assert_eq!(course, vec![stars[2]]);
        assert_eq!(
            plot(
                stars[1],
                stars[1],
                reach,
                8,
                &mut near_of(stars.clone(), HashMap::new())
            )
            .unwrap(),
            vec![]
        );
    }

    #[test]
    fn a_far_star_takes_the_fewest_jumps_each_within_range() {
        // Twenty stars half a sector apart; the drive reaches a sector and a half.
        let stars = line(20, 0x8000);
        let unit = sector::distance([0, 0, 0], [0x1_0000, 0, 0]);
        let reach = unit * 3 / 2 + 5;
        let (from, to) = (stars[0], stars[19]);
        let course = plot(
            from,
            to,
            reach,
            8,
            &mut near_of(stars.clone(), HashMap::new()),
        )
        .unwrap();
        // 9.5 sectors at 1.5 a jump: at least 7 jumps.
        assert_eq!(
            course.len(),
            jumps_needed(from.position, to.position, reach)
        );
        assert_eq!(*course.last().unwrap(), to);
        let mut at = from;
        for stop in &course {
            assert!(
                sector::distance(at.position, stop.position) <= reach,
                "{at:?} -> {stop:?}"
            );
            at = *stop;
        }
    }

    #[test]
    fn a_gap_wider_than_the_drive_has_no_course() {
        let mut stars = line(5, 0x8000);
        stars.extend(line(5, 0x8000).into_iter().map(|mut star| {
            star.code += 100;
            star.position[0] += 0x10_0000;
            star
        }));
        let reach = sector::distance([0, 0, 0], [0x1_0000, 0, 0]);
        let result = plot(
            stars[0],
            stars[9],
            reach,
            8,
            &mut near_of(stars.clone(), HashMap::new()),
        );
        // Sixteen sectors across at one a jump is more than the limit...
        assert!(
            matches!(result, Err(CourseError::TooFar { .. })),
            "{result:?}"
        );
        // ... and with a longer limit the empty gap still cannot be crossed.
        let result = plot(
            stars[0],
            stars[9],
            reach,
            40,
            &mut near_of(stars.clone(), HashMap::new()),
        );
        assert_eq!(result, Err(CourseError::NoCourse));
    }

    #[test]
    fn a_star_too_many_jumps_away_is_refused_at_once() {
        let stars = line(40, 0x8000);
        let reach = sector::distance([0, 0, 0], [0x1_0000, 0, 0]);
        let error = plot(
            stars[0],
            stars[39],
            reach,
            8,
            &mut near_of(stars.clone(), HashMap::new()),
        )
        .unwrap_err();
        assert_eq!(
            error,
            CourseError::TooFar {
                jumps: jumps_needed(stars[0].position, stars[39].position, reach),
                max_hops: 8
            }
        );
        assert!(error.to_string().contains("Too far"));
    }

    #[test]
    fn among_equal_courses_the_better_stop_wins() {
        // Two ways across: through star 10 or star 11, level with each other. Star 10 is
        // an empty, patrolled system (a big penalty): the course goes through 11.
        let unit = sector::distance([0, 0, 0], [0x1_0000, 0, 0]);
        let reach = unit * 3 / 2;
        let stars = vec![
            Stop {
                code: 1,
                position: [0, 0, 0],
            },
            Stop {
                code: 10,
                position: [0x1_0000, 0x0800, 0],
            },
            Stop {
                code: 11,
                position: [0x1_0000, -0x0800, 0],
            },
            Stop {
                code: 2,
                position: [0x2_0000, 0, 0],
            },
        ];
        let penalties = HashMap::from([(10, 3_400)]);
        let course = plot(
            stars[0],
            stars[3],
            reach,
            8,
            &mut near_of(stars.clone(), penalties),
        )
        .unwrap();
        assert_eq!(course.iter().map(|s| s.code).collect::<Vec<_>>(), [11, 2]);
        // Without the penalty either may do, but it is still two jumps.
        let course = plot(
            stars[0],
            stars[3],
            reach,
            8,
            &mut near_of(stars, HashMap::new()),
        )
        .unwrap();
        assert_eq!(course.len(), 2);
    }
}
