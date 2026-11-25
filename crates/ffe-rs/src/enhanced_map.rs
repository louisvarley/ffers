//! The enhanced (flat) galaxy map (`enhancement-12`): a 2D top-down starfield
//! that replaces the original's 3D scene, with a Sol indicator and multi-step
//! `X` routes. The projection is pure so the geometry is unit-testable; the
//! screen draws the [`Plot`] list through `Screen::draw_enhanced_galaxy`.
//!
//! Docs: `docs/notes/galaxy-map.md` and `docs/notes/enhancements.md`.

use crate::course;
use crate::galaxy_map::{sector_code, Selected};
use crate::navigation::{RouteCheck, RouteProblem};
use ffe_core::sector::{self, Tables as SectorTables};

/// Pixels per 16.16 unit at the opening zoom: one sector (0x1_0000) is 128
/// pixels, so about two and a half sectors span the 320-pixel view.
pub const DEFAULT_ZOOM: f64 = 1.0 / 512.0;
/// The smallest and largest zoom, in pixels per unit (roughly 8x in and 32x out).
pub const MIN_ZOOM: f64 = 1.0 / 8192.0;
pub const MAX_ZOOM: f64 = 1.0 / 16.0;
/// The centre of the 320x158 view.
pub const VIEW_CX: f64 = 160.0;
pub const VIEW_CY: f64 = 79.0;
pub const VIEW_W: i32 = 320;
pub const VIEW_H: i32 = 158;
/// Sol's sector, the origin the map's corner "Sector: x,y" text is relative to.
pub const SOL_SECTOR: [i32; 2] = [0x1718, 0x1524];
/// The Sol star itself (star 0 of its sector, F856's 16.16 position): half a sector
/// in from the sector's corner, not on it.
pub const SOL_POSITION: [i32; 3] = [0x1718_8000, 0x1524_a000, 0xffff_9400u32 as i32];

/// One star of the flat map, projected into logical view coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plot {
    /// The system code (sector + star number), for selection and jumps.
    pub code: u32,
    /// The star's 16.16 position (F856).
    pub position: [i32; 3],
    pub x: i32,
    pub y: i32,
    /// Dot radius in pixels; a depth cue from the star's z (lower = smaller).
    pub size: i32,
    pub kind: u16,
}

/// One line of the map's grid, in view pixels (see [`EnhancedMap::grid_lines`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridLine {
    /// A vertical line (at an x) rather than a horizontal one (at a y).
    pub vertical: bool,
    pub at: i32,
    /// A strong line (a sector edge or a multiple of it) rather than a faint one.
    pub major: bool,
}

/// One step of the multi-step route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waypoint {
    pub code: u32,
    pub position: [i32; 3],
    /// A stop the course plotter put in to reach the star clicked after it, rather than a
    /// star the player clicked.
    pub auto: bool,
}

/// The enhanced map's state: the pan centre, zoom, the selection and the route.
#[derive(Debug, Clone)]
pub struct EnhancedMap {
    /// The view centre in 16.16 sectors (`+0x23a`/`+0x23e` of the original).
    pub centre: [i32; 2],
    pub zoom: f64,
    /// The player's current system (16.16), for recentre and route legs.
    pub home: [i32; 3],
    /// Its system code (`u32::MAX` until the game says), for plotting a course from it.
    pub home_code: u32,
    /// The stars of the sectors already generated: (code, 16.16 position, type word).
    #[allow(clippy::type_complexity)]
    stars: std::cell::RefCell<std::collections::HashMap<(u16, u16), Vec<(u32, [i32; 3], u16)>>>,
    /// The drive's range in its units (`DATA_008907`).
    pub reach: i32,
    pub selected: Option<Selected>,
    /// The `X` route, in the order it was marked.
    pub route: Vec<Waypoint>,
}

impl EnhancedMap {
    pub fn new(home: [i32; 3], reach: i32) -> Self {
        Self {
            centre: [home[0], home[1]],
            zoom: DEFAULT_ZOOM,
            home,
            home_code: u32::MAX,
            stars: std::cell::RefCell::default(),
            reach,
            selected: None,
            route: Vec::new(),
        }
    }

    /// `Home`/`C`: back to the player's system, level, at the opening zoom.
    pub fn recentre(&mut self) {
        self.centre = [self.home[0], self.home[1]];
        self.zoom = DEFAULT_ZOOM;
    }

    /// Move the centre by `x`/`y` (16.16 sectors): the right-drag pan.
    pub fn pan(&mut self, x: i32, y: i32) {
        self.centre = [
            self.centre[0].wrapping_add(x),
            self.centre[1].wrapping_add(y),
        ];
    }

    /// Move the view by `right`/`up` screen pixels (the arrow keys): the stars
    /// slide the other way, so Right looks further right.
    pub fn pan_pixels(&mut self, right: f64, up: f64) {
        self.pan(
            (right / self.zoom).round() as i32,
            (up / self.zoom).round() as i32,
        );
    }

    /// Select `star` (a click on it). Only a click selects: panning the view, with
    /// the keys or the mouse, never does, and the selection holds until another
    /// star is clicked.
    pub fn pick(&mut self, code: u32, position: [i32; 3], kind: u16) {
        self.selected = Some(Selected {
            position,
            code,
            kind,
        });
    }

    /// Scale the zoom by `factor` (over 1 zooms in).
    pub fn zoom_by(&mut self, factor: f64) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Project a 16.16 position into view coordinates: the galaxy's x runs right
    /// and y runs up (screen y is inverted), and the star's z sizes its dot.
    pub fn project(&self, position: [i32; 3]) -> (i32, i32, i32) {
        let x = (VIEW_CX + (position[0] - self.centre[0]) as f64 * self.zoom).round() as i32;
        let y = (VIEW_CY - (position[1] - self.centre[1]) as f64 * self.zoom).round() as i32;
        let t = ((position[2] + 0x1_0000) as f64 / 0x2_0000 as f64).clamp(0.0, 1.0);
        let size = (1.0 + t * 2.0).round() as i32;
        (x, y, size)
    }

    /// Every star on the view, projected: those of every sector the view covers (the stars
    /// of a sector are generated once and kept).
    pub fn plots(&self, tables: &SectorTables) -> Vec<Plot> {
        let half_width = (VIEW_CX / self.zoom) as i32 + 0x1_0000;
        let half_height = (VIEW_CY / self.zoom) as i32 + 0x1_0000;
        let sectors = |centre: i32, half: i32| {
            let low = centre.wrapping_sub(half) >> 16;
            let high = centre.wrapping_add(half) >> 16;
            // The view never covers more than the zoom limits allow; keep a bad value from
            // looping for ever.
            low..=high.min(low + 80)
        };
        let mut cache = self.stars.borrow_mut();
        let mut out = Vec::new();
        for x in sectors(self.centre[0], half_width) {
            for y in sectors(self.centre[1], half_height) {
                let key = (x as u16, y as u16);
                let stars = cache.entry(key).or_insert_with(|| {
                    tables
                        .stars(key.0, key.1)
                        .iter()
                        .enumerate()
                        .map(|(index, star)| {
                            (
                                sector_code(key.0, key.1) | (index as u32) << 26,
                                sector::position(key.0, key.1, &star.vector),
                                star.kind,
                            )
                        })
                        .collect()
                });
                for (code, position, kind) in stars.iter() {
                    let (px, py, size) = self.project(*position);
                    if (0..VIEW_W).contains(&px) && (0..VIEW_H).contains(&py) {
                        out.push(Plot {
                            code: *code,
                            position: *position,
                            x: px,
                            y: py,
                            size,
                            kind: *kind,
                        });
                    }
                }
            }
        }
        out
    }

    /// Sol's projected position (possibly off-screen) and its distance from the
    /// player's system, in the drive's units. The screen draws an edge arrow
    /// when the position is off-view.
    pub fn sol(&self) -> (i32, i32, i32) {
        let (x, y, _) = self.project(SOL_POSITION);
        (x, y, sector::distance(self.home, SOL_POSITION))
    }

    /// The drive's range as a screen radius (the reach circle about the player's
    /// system), derived from the distance of one sector.
    pub fn range_radius(&self) -> i32 {
        let sector_distance = sector::distance([0, 0, 0], [0x1_0000, 0, 0]);
        (self.reach as f64 / sector_distance as f64 * 0x1_0000 as f64 * self.zoom).round() as i32
    }

    /// The map's grid, as lines across the view: one per sector edge, or per 2, 4, 8...
    /// sectors when zoomed out so they never crowd, each a strong line, with fainter
    /// ones between them when there is room (zoomed in). Like Elite Dangerous' galaxy
    /// map, the grid sits under the stars and moves and scales with them.
    pub fn grid_lines(&self) -> Vec<GridLine> {
        const SECTOR: f64 = 65_536.0;
        // The strong lines stay between this many and twice this many pixels apart.
        const MIN_GAP: f64 = 40.0;
        // Fainter lines come in only when they would be at least this far apart.
        const MIN_FAINT_GAP: f64 = 10.0;
        let mut step = 1.0;
        while SECTOR * step * self.zoom < MIN_GAP {
            step *= 2.0;
        }
        while SECTOR * step * 0.5 * self.zoom >= MIN_GAP {
            step *= 0.5;
        }
        let parts = if SECTOR * step * self.zoom / 4.0 >= MIN_FAINT_GAP {
            4
        } else {
            1
        };
        let unit = SECTOR * step / f64::from(parts);
        let mut lines = Vec::new();
        for (vertical, centre, size, mid) in [
            (true, self.centre[0], f64::from(VIEW_W), VIEW_CX),
            (false, self.centre[1], f64::from(VIEW_H), VIEW_CY),
        ] {
            let half = size / 2.0 / self.zoom;
            let first = ((f64::from(centre) - half - unit) / unit).floor() as i64;
            let last = ((f64::from(centre) + half + unit) / unit).ceil() as i64;
            for index in first..=last {
                let world = index as f64 * unit;
                let offset = (world - f64::from(centre)) * self.zoom;
                let at = if vertical { mid + offset } else { mid - offset };
                if (-1.0..=size).contains(&at) {
                    lines.push(GridLine {
                        vertical,
                        at: at.round() as i32,
                        major: index.rem_euclid(i64::from(parts)) == 0,
                    });
                }
            }
        }
        lines
    }

    /// The corner "Sector: x,y" line, relative to Sol (the same as the 3D map's).
    pub fn sector_text(&self) -> String {
        let x = (self.centre[0] >> 16) - SOL_SECTOR[0];
        let y = (self.centre[1] >> 16) - SOL_SECTOR[1];
        format!("Sector: {x},{y}")
    }

    /// The distance of each route leg from the previous point, the player's
    /// system for the first leg.
    pub fn legs(&self) -> Vec<i32> {
        let mut prev = self.home;
        self.route
            .iter()
            .map(|waypoint| {
                let distance = sector::distance(prev, waypoint.position);
                prev = waypoint.position;
                distance
            })
            .collect()
    }

    /// Add a waypoint; a star already at the end of the route is not re-added.
    pub fn add_waypoint(&mut self, code: u32, position: [i32; 3]) {
        if self.route.last().is_none_or(|last| last.code != code) {
            self.route.push(Waypoint {
                code,
                position,
                auto: false,
            });
        }
    }

    /// Remove the waypoint with `code` — and every waypoint after it — so a
    /// `X` press on a mid-route star shortens the route from there. Returns how
    /// many waypoints were removed (0 when the code is not in the route).
    pub fn remove_waypoint_from(&mut self, code: u32) -> usize {
        match self.route.iter().position(|waypoint| waypoint.code == code) {
            Some(index) => {
                let removed = self.route.len() - index;
                self.route.truncate(index);
                removed
            }
            None => 0,
        }
    }

    pub fn clear_route(&mut self) {
        self.route.clear();
    }

    /// The stops of the route that go together with the one at `index`: the star the
    /// player clicked and the stops the plotter put in on the way to it.
    fn group(&self, index: usize) -> std::ops::Range<usize> {
        let mut start = index;
        while start > 0 && self.route[start - 1].auto {
            start -= 1;
        }
        let mut end = index;
        while self.route[end].auto && end + 1 < self.route.len() {
            end += 1;
        }
        // A plotted star the player clicked after the stops it needed is the group's last.
        start..end + 1
    }

    /// Pan and zoom so the player's system and every stop of the route are in view, in the
    /// part of the view the legend and the notes leave clear. The selection is not moved.
    pub fn fit_route(&mut self) {
        if self.route.is_empty() {
            return;
        }
        let points = std::iter::once(self.home).chain(self.route.iter().map(|w| w.position));
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for point in points {
            min_x = min_x.min(point[0]);
            max_x = max_x.max(point[0]);
            min_y = min_y.min(point[1]);
            max_y = max_y.max(point[1]);
        }
        let (width, height) = (
            f64::from(max_x.wrapping_sub(min_x)).max(1.0),
            f64::from(max_y.wrapping_sub(min_y)).max(1.0),
        );
        // The clear area: x 20..300, y 34..108.
        let zoom = (280.0 / width).min(74.0 / height);
        self.zoom = zoom.clamp(MIN_ZOOM, DEFAULT_ZOOM);
        // The clear area's middle is a little above the view's, so the centre sits below the
        // route's by that much.
        self.centre = [
            min_x.wrapping_add(max_x.wrapping_sub(min_x) / 2),
            min_y
                .wrapping_add(max_y.wrapping_sub(min_y) / 2)
                .wrapping_sub((8.0 / self.zoom) as i32),
        ];
    }

    /// Take the star `code` off the route, with the stops that were plotted to reach it
    /// (the other stars of the route are untouched). Returns how many stops went.
    pub fn remove_star(&mut self, code: u32) -> usize {
        let Some(index) = self.route.iter().position(|waypoint| waypoint.code == code) else {
            return 0;
        };
        let group = self.group(index);
        let removed = group.len();
        self.route.drain(group);
        removed
    }

    /// Where the route has got to: its last stop, or the player's system.
    pub fn route_end(&self) -> (u32, [i32; 3]) {
        self.route
            .last()
            .map_or((self.home_code, self.home), |last| {
                (last.code, last.position)
            })
    }

    /// A click on a star: add it to the route, or take it off again when it is
    /// already on it. The player's own system is not a destination.
    pub fn toggle_waypoint(&mut self, code: u32, position: [i32; 3]) -> Toggled {
        if sector::distance(self.home, position) == 0 {
            return Toggled::Ignored;
        }
        if self.remove_star(code) > 0 {
            Toggled::Removed
        } else {
            self.route.push(Waypoint {
                code,
                position,
                auto: false,
            });
            Toggled::Added
        }
    }

    /// A click on a star, with a course plotted when it is beyond one jump of where the
    /// route has got to: a star in range is added as it is; a star farther away is
    /// reached by the stops [`course::plot_in_galaxy`] finds, which are marked `auto`
    /// so a second click on the star takes them all off again.
    pub fn click_star(&mut self, star: course::Stop, plan: &PlanContext<'_>) -> Clicked {
        if sector::distance(self.home, star.position) == 0 {
            return Clicked::Ignored;
        }
        let removed = self.remove_star(star.code);
        if removed > 0 {
            return Clicked::Removed(removed);
        }
        let (from_code, from_position) = self.route_end();
        let from = course::Stop {
            code: from_code,
            position: from_position,
        };
        if self.reach <= 0 || sector::distance(from_position, star.position) <= self.reach {
            self.route.push(Waypoint {
                code: star.code,
                position: star.position,
                auto: false,
            });
            return Clicked::Added;
        }
        match course::plot_in_galaxy(plan.sectors, plan.systems, from, star, self.reach) {
            Ok(stops) => {
                let last = stops.len() - 1;
                for (index, stop) in stops.iter().enumerate() {
                    self.route.push(Waypoint {
                        code: stop.code,
                        position: stop.position,
                        auto: index != last,
                    });
                }
                Clicked::Plotted(stops)
            }
            Err(error) => Clicked::NoCourse(error),
        }
    }

    /// The line from the middle of the view to the player's own system: it ends
    /// at the system when that is on view and at the view's edge when it is not,
    /// so there is always a way back.
    pub fn home_pointer(&self) -> Pointer {
        let (x, y, _) = self.project(self.home);
        let on_view = (0..VIEW_W).contains(&x) && (0..VIEW_H).contains(&y);
        let tip = if on_view { (x, y) } else { edge_point((x, y)) };
        let centre = [self.centre[0], self.centre[1], self.home[2]];
        Pointer {
            tip,
            on_view,
            distance: sector::distance(self.home, centre),
        }
    }
}

/// Why a course cannot be flown, in a line: a fuel shortage says what the whole course
/// needs against the hold (`total` tonnes, `have` in the hold), not just the leg it ran
/// out at; a range problem names the leg and both distances.
pub fn course_problem(problem: &RouteProblem, total: i32, have: i32) -> String {
    match problem {
        RouteProblem::Fuel { .. } => {
            format!("Cannot fly this course yet: it needs {total} t of fuel, the hold has {have} t")
        }
        other => format!("Cannot fly this course: {other}"),
    }
}

/// What the map needs to plot a course.
pub struct PlanContext<'a> {
    pub sectors: &'a SectorTables<'a>,
    pub systems: Option<&'a ffe_core::galaxy::Tables<'a>>,
}

/// What [`EnhancedMap::click_star`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clicked {
    /// The star was one jump on and went into the route as it is.
    Added,
    /// The star (and the stops plotted to reach it) came off the route; how many stops.
    Removed(usize),
    /// A course was plotted through these stops, the star last.
    Plotted(Vec<course::Stop>),
    NoCourse(course::CourseError),
    /// The player's own system.
    Ignored,
}

impl Clicked {
    /// The line the console says for the click on the star called `name`.
    pub fn message(
        &self,
        name: &str,
        route_len: usize,
        name_of: &dyn Fn(u32) -> Option<String>,
    ) -> String {
        match self {
            Self::Added => format!(
                "{name} added: route of {route_len} jump(s). Click it again to remove it; H flies the route."
            ),
            Self::Removed(1) => format!("{name} removed: route of {route_len} jump(s)."),
            Self::Removed(stops) => format!(
                "{name} and the {} stop(s) plotted to it removed: route of {route_len} jump(s).",
                stops - 1
            ),
            Self::Plotted(stops) => {
                let via: Vec<String> = stops[..stops.len() - 1]
                    .iter()
                    .map(|stop| name_of(stop.code).unwrap_or_else(|| "?".to_owned()))
                    .collect();
                format!(
                    "Course to {name}: {} jumps via {}. H flies it; click {name} again to clear it.",
                    stops.len(),
                    via.join(", ")
                )
            }
            Self::NoCourse(error) => format!("{name}: {error}"),
            Self::Ignored => format!("{name} is the system you are in."),
        }
    }
}

/// What a click on a star did to the route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggled {
    Added,
    Removed,
    /// The star is the player's own system.
    Ignored,
}

/// A pointer from the middle of the view toward a point, ending on the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pointer {
    /// Where the line ends: the point itself when it is on view.
    pub tip: (i32, i32),
    pub on_view: bool,
    /// The distance from the map's centre to the point, in the drive's units.
    pub distance: i32,
}

/// F828's fuel line from the hyperdrive's report for a jump: `Fuel Required: n t`,
/// `Insufficient Fuel (n t required)` or `Out of Range`.
pub fn fuel_line(report: &ffe_core::hyperspace::Report) -> String {
    if report.result < 0 {
        if report.fuel_needed < 0 {
            "Out of Range".to_owned()
        } else {
            format!("Insufficient Fuel ({}t required)", report.fuel_needed)
        }
    } else {
        format!("Fuel Required: {}t", report.fuel_needed)
    }
}

/// The rectangle a pointer to an off-view point stops at: inside the view, below
/// the legend and controls line along the top and above the notes at the bottom
/// left, so a pointer's label never lands on other text.
pub const EDGE_LEFT: f64 = 14.0;
pub const EDGE_RIGHT: f64 = 306.0;
pub const EDGE_TOP: f64 = 30.0;
pub const EDGE_BOTTOM: f64 = 112.0;

/// Where the ray from the view's centre to `point` leaves [`EDGE_LEFT`]..[`EDGE_BOTTOM`].
pub fn edge_point(point: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = (f64::from(point.0) - VIEW_CX, f64::from(point.1) - VIEW_CY);
    if dx == 0.0 && dy == 0.0 {
        return (VIEW_CX as i32, VIEW_CY as i32);
    }
    let tx = if dx > 0.0 {
        (EDGE_RIGHT - VIEW_CX) / dx
    } else if dx < 0.0 {
        (EDGE_LEFT - VIEW_CX) / dx
    } else {
        f64::INFINITY
    };
    let ty = if dy > 0.0 {
        (EDGE_BOTTOM - VIEW_CY) / dy
    } else if dy < 0.0 {
        (EDGE_TOP - VIEW_CY) / dy
    } else {
        f64::INFINITY
    };
    let scale = tx.min(ty);
    (
        (VIEW_CX + dx * scale).round() as i32,
        (VIEW_CY + dy * scale).round() as i32,
    )
}

/// The plotted star under (`x`, `y`): the nearest within a few pixels of its
/// dot, so a click does not need to be exact.
pub fn hit(plots: &[Plot], x: i32, y: i32) -> Option<&Plot> {
    plots
        .iter()
        .filter(|plot| {
            (plot.x - x).abs() <= HIT + plot.size && (plot.y - y).abs() <= HIT + plot.size
        })
        .min_by_key(|plot| (plot.x - x).pow(2) + (plot.y - y).pow(2))
}

/// How far from a dot's centre a click still counts as on it, in pixels.
pub const HIT: i32 = 5;

/// The classic FFE low-palette colours the map's inhabited systems use, by
/// allegiance (`Market.allegiance >> 6`): red for the Empire, blue for the
/// Federation, green for the Alliance and amber for the independents.
pub const EMPIRE: u8 = 0x10;
pub const FEDERATION: u8 = 0x43;
pub const ALLIANCE: u8 = 0x11;
pub const INDEPENDENT: u8 = 0x5f;
/// Uninhabited systems (population zero) are drawn in plain white.
pub const UNINHABITED: u8 = 15;
/// The reach circle, the route and the line home, in the low palette.
pub const REACH_COLOUR: u8 = 10;
pub const ROUTE_COLOUR: u8 = 13;
pub const HOME_COLOUR: u8 = 0x75;

/// How dangerous a system is to arrive in.
///
/// The measure is the system record's patrol byte (`+6`, `Details::bytes[1]`),
/// the very value F688 draws a new arrival's pirate groups from
/// (`RND(patrol * 4)` groups, each a lone raider or a gang), so a "Safe" system is
/// one where no pirate group is ever drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Threat {
    Safe,
    Low,
    Moderate,
    High,
}

/// The threat colours (the low palette): green, yellow, orange and red.
pub const THREAT_SAFE: u8 = 0x11;
pub const THREAT_LOW: u8 = 18;
pub const THREAT_MODERATE: u8 = 0x58;
pub const THREAT_HIGH: u8 = 0x10;

impl Threat {
    /// The rating of a patrol byte: none, 1-2, 3-4, 5 and over.
    pub fn from_patrol(patrol: u32) -> Self {
        match patrol {
            0 => Self::Safe,
            1..=2 => Self::Low,
            3..=4 => Self::Moderate,
            _ => Self::High,
        }
    }

    /// 0 (safe) to 3 (high), for ordering courses.
    pub fn rank(self) -> u32 {
        match self {
            Self::Safe => 0,
            Self::Low => 1,
            Self::Moderate => 2,
            Self::High => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Low => "Low",
            Self::Moderate => "Moderate",
            Self::High => "High",
        }
    }

    pub fn colour(self) -> u8 {
        match self {
            Self::Safe => THREAT_SAFE,
            Self::Low => THREAT_LOW,
            Self::Moderate => THREAT_MODERATE,
            Self::High => THREAT_HIGH,
        }
    }
}

/// A system's danger, or `None` for an uninhabited one (no traffic, no pirates:
/// nothing to rate).
pub fn system_threat(code: u32, tables: &ffe_core::galaxy::Tables) -> Option<Threat> {
    if tables.market(code).population == 0 {
        return None;
    }
    Some(Threat::from_patrol(tables.details(code).bytes[1]))
}

/// The dot colour and inhabited flag for a system, from its F870 market record:
/// the allegiance (`word >> 6`) picks the faction colour, and a zero population
/// is uninhabited. This is the map's faction/inhabited legend data.
pub fn system_marker(code: u32, tables: &ffe_core::galaxy::Tables) -> (u8, bool) {
    let market = tables.market(code);
    if market.population == 0 {
        return (UNINHABITED, false);
    }
    let colour = match market.allegiance >> 6 {
        1 => EMPIRE,
        2 => FEDERATION,
        3 => ALLIANCE,
        _ => INDEPENDENT,
    };
    (colour, true)
}

/// The navigator's route check, as the scene is given it.
pub type RouteChecker<'a> = dyn Fn(&[(u32, [i32; 3])]) -> Result<RouteCheck, RouteProblem> + 'a;

/// What [`scene`] needs from the game besides the map itself.
pub struct SceneContext<'a> {
    pub sectors: &'a SectorTables<'a>,
    pub systems: Option<&'a ffe_core::galaxy::Tables<'a>>,
    /// A system's name by its code.
    pub name_of: &'a dyn Fn(u32) -> Option<String>,
    /// A raw system string by its F830 selector (the description's source).
    pub string_of: &'a dyn Fn(u32) -> Option<Vec<u8>>,
    /// The hyperdrive's report for a distance (`Navigator::report_for`).
    pub report_for: &'a dyn Fn(i32) -> Option<ffe_core::hyperspace::Report>,
    /// The navigator's check of a route from the player's system
    /// (`Navigator::check_route`): the one place the range and the fuel are judged.
    pub check_route: &'a RouteChecker<'a>,
    /// Tonnes of fuel in the hold.
    pub drive_fuel: i32,
    pub date: String,
    /// The mouse, in view coordinates, when it is over the screen.
    pub mouse: Option<(i32, i32)>,
}

/// The controls line along the top of the view.
pub const HINT: &str = "Click star: route  H: jump  Arrows: pan  Hover: info";

/// The hover line's colours for a populated and an unpopulated system.
pub const POPULATED: u8 = 0x11;
pub const UNPOPULATED: u8 = 0x10;

/// Build everything the screen draws of the enhanced map for this frame.
pub fn scene(map: &mut EnhancedMap, ctx: &SceneContext<'_>) -> crate::screen::EnhancedGalaxy {
    let plots = map.plots(ctx.sectors);
    let selected_plot = map.selected.as_ref().and_then(|selected| {
        plots
            .iter()
            .find(|plot| plot.code == selected.code)
            .copied()
    });
    let hovered_plot = ctx.mouse.and_then(|(x, y)| hit(&plots, x, y)).copied();
    let mut dots = Vec::with_capacity(plots.len());
    let mut labels = Vec::new();
    for plot in &plots {
        let (colour, inhabited) = ctx
            .systems
            .map_or((15u8, false), |tables| system_marker(plot.code, tables));
        dots.push((plot.x, plot.y, plot.size, colour));
        // The stops of the route are numbered in the order they are flown.
        let stop = map.route.iter().position(|w| w.code == plot.code);
        if (inhabited && map.zoom >= DEFAULT_ZOOM / 2.0)
            || stop.is_some()
            || selected_plot.is_some_and(|s| s.code == plot.code)
            || hovered_plot.is_some_and(|h| h.code == plot.code)
        {
            if let Some(name) = (ctx.name_of)(plot.code) {
                labels.push((
                    plot.x,
                    plot.y,
                    stop.map_or(name.clone(), |index| format!("{} {name}", index + 1)),
                ));
            }
        }
    }
    let project_point = |position: [i32; 3]| {
        let (x, y, _) = map.project(position);
        (x, y)
    };
    let home = project_point(map.home);
    let mut route = vec![home];
    route.extend(
        map.route
            .iter()
            .map(|waypoint| project_point(waypoint.position)),
    );
    // Each leg: can the range and the fuel left fly it? Waypoints that leave the
    // player stranded (uninhabited, too little fuel for the next leg) get a warning.
    let legs = map.legs();
    let pairs: Vec<(u32, [i32; 3])> = map
        .route
        .iter()
        .map(|waypoint| (waypoint.code, waypoint.position))
        .collect();
    let check = (ctx.check_route)(&pairs);
    // Legs before the one that cannot be flown are fine; it and the rest are red.
    let leg_ok: Vec<bool> = (0..legs.len())
        .map(|index| match &check {
            Ok(_) => true,
            Err(problem) => problem.leg().is_none_or(|leg| index + 1 < leg),
        })
        .collect();
    let mut warnings = Vec::new();
    if let Ok(flown) = &check {
        // Waypoints that would strand the player: uninhabited, with too little fuel left
        // for the next leg.
        let mut used = 0;
        for (index, distance) in legs.iter().enumerate() {
            used += (ctx.report_for)(*distance).map_or(0, |r| r.fuel_needed.max(0));
            let next = legs.get(index + 1).map_or(1, |d| {
                (ctx.report_for)(*d).map_or(0, |r| r.fuel_needed.max(0))
            });
            let inhabited = ctx
                .systems
                .is_some_and(|tables| tables.market(map.route[index].code).population != 0);
            if !inhabited && flown.have - used < next {
                if let Some(point) = route.get(index + 1) {
                    warnings.push(*point);
                }
            }
        }
    }
    let route_summary = (!map.route.is_empty()).then(|| match &check {
        Ok(flown) => {
            let total: i32 = flown.legs.iter().sum();
            (
                format!(
                    "Course OK: {} jump{}, {}.{:02} ly, {} of {} t fuel. H to jump",
                    flown.legs.len(),
                    if flown.legs.len() == 1 { "" } else { "s" },
                    total / 100,
                    total % 100,
                    flown.fuel,
                    flown.have,
                ),
                POPULATED,
            )
        }
        Err(problem) => {
            let total: i32 = legs
                .iter()
                .filter_map(|d| (ctx.report_for)(*d).map(|r| r.fuel_needed.max(0)))
                .sum();
            (course_problem(problem, total, ctx.drive_fuel), THREAT_HIGH)
        }
    });
    let hover = hovered_plot.map_or_else(Vec::new, |plot| hover_lines(&plot, map, &pairs, ctx));
    let legend: Vec<(u8, &'static str)> = vec![
        (FEDERATION, "Federation"),
        (EMPIRE, "Empire"),
        (ALLIANCE, "Alliance"),
        (INDEPENDENT, "Independent"),
    ];
    crate::screen::EnhancedGalaxy {
        dots,
        labels,
        selected: selected_plot.map(|plot| (plot.x, plot.y, plot.size)),
        hovered: hovered_plot.map(|plot| (plot.x, plot.y, plot.size)),
        route,
        leg_ok,
        warnings,
        range_radius: map.range_radius(),
        home,
        home_pointer: map.home_pointer(),
        sol: map.sol(),
        sector: map.sector_text(),
        grid: map.grid_lines(),
        date: ctx.date.clone(),
        route_summary,
        hover,
        legend,
        hint: HINT.to_owned(),
        page: None,
    }
}

/// The red the map uses for what cannot be done (the unpopulated colour).
const PROBLEM: u8 = UNPOPULATED;

/// What hovering a star says, in the right-hand panel: its name and distance from the
/// player's system, whether anyone lives there (green "Populated" or red "Unpopulated"),
/// for a populated system the original map's own text (its government, economy,
/// allegiance and population, the lines F843's people page writes) and how dangerous it
/// is to arrive in, then F869's description of the system and, last, what the jump
/// would need: the fuel, or why it cannot be made (in red).
fn hover_lines(
    plot: &Plot,
    map: &EnhancedMap,
    route: &[(u32, [i32; 3])],
    ctx: &SceneContext<'_>,
) -> Vec<(String, u8)> {
    let name = (ctx.name_of)(plot.code).unwrap_or_default();
    let distance = sector::distance(map.home, plot.position);
    let mut lines = vec![(
        format!("{name}  {}.{:02} ly", distance / 100, distance % 100),
        15,
    )];
    if let Some(tables) = ctx.systems {
        let market = tables.market(plot.code);
        let details = tables.details(plot.code);
        let expand = |selector: u16, vars: &[i32; 5]| -> Option<String> {
            let raw = (ctx.string_of)(u32::from(selector))?;
            let text =
                ffe_core::system_text::expand(&raw, vars, &|code| (ctx.string_of)(u32::from(code)));
            Some(plain_text(&text))
        };
        if market.population == 0 {
            lines.push(("Unpopulated".to_owned(), UNPOPULATED));
        } else {
            lines.push(("Populated".to_owned(), POPULATED));
            // F843's people page (`0x84b5`): the government (`0x84cb +`), economy
            // (`0x84bb +`, "Small Mining" and the like), allegiance (`0x84b7 +`) and
            // population (`0x84db +`).
            let word = details.allegiance as i32;
            let vars = [
                (word & 0x3f) + 0x84cb,
                (word & 0x3f) + 0x84bb,
                (word >> 6) + 0x84b7,
                market.population as i32 + 0x84db,
                0,
            ];
            if let Some(people) = expand(0x84b5, &vars) {
                for line in people.split(['\r', '\n']) {
                    let line = line.trim();
                    if !line.is_empty() {
                        lines.push((line.to_owned(), 15));
                    }
                }
            }
            if let Some(threat) = system_threat(plot.code, tables) {
                lines.push((format!("Danger: {}", threat.label()), threat.colour()));
            }
        }
        // F869's first dword: the system's description, as the original's console shows it.
        if let Some(description) = expand(details.words[0] as u16, &[0; 5]) {
            let description = description.replace('\r', " ");
            if !description.trim().is_empty() {
                lines.push((description, 15));
            }
        }
    }
    lines.extend(jump_line(plot, map, route, ctx));
    lines
}

/// The original's text as plain characters: its column-tab code (`0xfd` and the column
/// byte after it) becomes a space and the other control codes (except the line break
/// `\r`) are dropped, so "Government Type:<tab>Feudal" reads as a sentence.
pub fn plain_text(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len());
    let mut bytes = bytes.iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            0xfd => {
                bytes.next();
                text.push(' ');
            }
            b'\r' | b'\n' => text.push('\r'),
            0x20..=0x7e => text.push(char::from(byte)),
            _ => {}
        }
    }
    text
}

/// What the jump to `plot` from where the route has got to would need, or why it cannot
/// be made: out of range (a click plots a course), or too little fuel for the whole
/// route with this star on the end of it. `None` for the player's own system and for
/// a star already on the route (a click takes it off again).
fn jump_line(
    plot: &Plot,
    map: &EnhancedMap,
    route: &[(u32, [i32; 3])],
    ctx: &SceneContext<'_>,
) -> Option<(String, u8)> {
    let leg = sector::distance(map.route_end().1, plot.position);
    if leg == 0 || route.iter().any(|(code, _)| *code == plot.code) {
        return None;
    }
    let report = (ctx.report_for)(leg)?;
    if report.result < 0 && report.fuel_needed < 0 {
        return Some(("Out of range: click to plot a course".to_owned(), PROBLEM));
    }
    if report.result < 0 {
        return Some((fuel_line(&report), PROBLEM));
    }
    // The leg alone is fine; the earlier legs may have left too little for it.
    let mut trial = route.to_vec();
    trial.push((plot.code, plot.position));
    if let Err(RouteProblem::Fuel { needed, .. }) = (ctx.check_route)(&trial) {
        return Some((format!("Insufficient Fuel ({needed}t required)"), PROBLEM));
    }
    Some((fuel_line(&report), 15))
}

/// Why a star was not added to the route, when the hold cannot pay for the course to it.
pub fn fuel_block_message(name: &str, total: i32, have: i32) -> String {
    format!("{name} not added: the course needs {total} t of fuel and the hold has {have} t.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> EnhancedMap {
        EnhancedMap::new([0, 0, 0], 0)
    }

    #[test]
    fn the_projection_is_a_flat_top_down_view() {
        let map = map();
        // The centre maps to the view centre; z 0 is the middle of the depth range.
        assert_eq!(map.project([0, 0, 0]), (160, 79, 2));
        // x runs right, y runs up (screen y falls), and z sizes the dot.
        let (x, y, _) = map.project([0x1_0000, 0, 0]);
        assert!(x > 160 && y == 79, "{x},{y}");
        let (_, y2, _) = map.project([0, 0x1_0000, 0]);
        assert!(y2 < 79, "{y2}");
        let (_, _, big) = map.project([0, 0, 0x1_0000]);
        let (_, _, small) = map.project([0, 0, -0x1_0000]);
        assert!(big > small);
    }

    #[test]
    fn recentre_returns_to_the_home_and_opening_zoom() {
        let mut map = map();
        map.pan(0x2_0000, -0x1_0000);
        map.zoom_by(2.0);
        map.recentre();
        assert_eq!(map.centre, [0, 0]);
        assert_eq!(map.zoom, DEFAULT_ZOOM);
    }

    #[test]
    fn route_legs_measure_from_the_home_then_each_previous_point() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        map.add_waypoint(1, [0x1_0000, 0, 0]);
        map.add_waypoint(2, [0x2_0000, 0, 0]);
        map.add_waypoint(2, [0x2_0000, 0, 0]); // duplicate last: not re-added
        let legs = map.legs();
        assert_eq!(legs.len(), 2);
        assert_eq!(legs[0], sector::distance([0, 0, 0], [0x1_0000, 0, 0]));
        assert_eq!(
            legs[1],
            sector::distance([0x1_0000, 0, 0], [0x2_0000, 0, 0])
        );
    }

    #[test]
    fn removing_a_mid_route_waypoint_removes_everything_after_it() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        map.add_waypoint(1, [0x1_0000, 0, 0]);
        map.add_waypoint(2, [0x2_0000, 0, 0]);
        map.add_waypoint(3, [0x3_0000, 0, 0]);
        // Removing the middle waypoint drops it and the one after it.
        assert_eq!(map.remove_waypoint_from(2), 2);
        assert_eq!(map.route.len(), 1);
        assert_eq!(map.route[0].code, 1);
        // Removing an absent code removes nothing.
        assert_eq!(map.remove_waypoint_from(99), 0);
        assert_eq!(map.route.len(), 1);
    }

    #[test]
    fn sol_is_at_its_sector_and_reports_the_home_distance() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        map.recentre();
        let (x, y, distance) = map.sol();
        let at = map.project(SOL_POSITION);
        assert_eq!((x, y), (at.0, at.1));
        assert_eq!(distance, sector::distance([0, 0, 0], SOL_POSITION));
    }

    #[test]
    fn a_click_adds_a_star_and_a_second_click_removes_only_that_star() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        let (a, b, c) = ([0x1_0000, 0, 0], [0x2_0000, 0, 0], [0x3_0000, 0, 0]);
        assert_eq!(map.toggle_waypoint(1, a), Toggled::Added);
        assert_eq!(map.toggle_waypoint(2, b), Toggled::Added);
        assert_eq!(map.toggle_waypoint(3, c), Toggled::Added);
        // Clicking the middle star again takes it off and leaves the others in order.
        assert_eq!(map.toggle_waypoint(2, b), Toggled::Removed);
        let codes: Vec<u32> = map.route.iter().map(|waypoint| waypoint.code).collect();
        assert_eq!(codes, [1, 3]);
        // The star the player is in is not a destination.
        assert_eq!(map.toggle_waypoint(9, [0, 0, 0]), Toggled::Ignored);
        assert_eq!(map.route.len(), 2);
    }

    #[test]
    fn the_arrow_keys_pan_by_screen_pixels() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        map.pan_pixels(10.0, 0.0);
        // Looking right moves the centre right: a star there slides toward the middle.
        let (x, _, _) = map.project([(10.0 / map.zoom) as i32, 0, 0]);
        assert!((x - 160).abs() <= 1, "{x}");
        let before = map.centre;
        map.pan_pixels(0.0, 10.0);
        assert_eq!(map.centre[0], before[0]);
        assert!(map.centre[1] > before[1], "up raises the galaxy's y");
    }

    #[test]
    fn the_grid_follows_the_sectors_and_never_crowds() {
        for zoom in [MIN_ZOOM, 1.0 / 2048.0, DEFAULT_ZOOM, 1.0 / 128.0, MAX_ZOOM] {
            let mut map = EnhancedMap::new([0, 0, 0], 0);
            map.zoom = zoom;
            let lines = map.grid_lines();
            for vertical in [true, false] {
                let mut strong: Vec<i32> = lines
                    .iter()
                    .filter(|line| line.vertical == vertical && line.major)
                    .map(|line| line.at)
                    .collect();
                strong.sort_unstable();
                assert!(strong.len() >= 2, "{zoom}: {strong:?}");
                // A line on the sector edge at the view's centre (the world's origin).
                let centre = if vertical { 160 } else { 79 };
                assert!(strong.contains(&centre), "{zoom}: {strong:?}");
                // The strong lines are 40 to 80 pixels apart, whatever the zoom.
                for pair in strong.windows(2) {
                    let gap = pair[1] - pair[0];
                    assert!((39..=81).contains(&gap), "{zoom}: gap {gap}");
                }
                // The faint lines, when there are any, lie between them at a quarter step.
                let faint = lines
                    .iter()
                    .filter(|line| line.vertical == vertical && !line.major)
                    .count();
                assert!(faint == 0 || faint >= strong.len(), "{zoom}: {faint}");
            }
        }
        // The grid moves with the view: panning a sector shifts every line by its width.
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        let before: Vec<i32> = map
            .grid_lines()
            .iter()
            .filter(|line| line.vertical && line.major)
            .map(|line| line.at)
            .collect();
        map.pan(0x1_0000, 0);
        let after: Vec<i32> = map
            .grid_lines()
            .iter()
            .filter(|line| line.vertical && line.major)
            .map(|line| line.at)
            .collect();
        let shift = (0x1_0000 as f64 * map.zoom).round() as i32;
        assert!(
            after
                .iter()
                .all(|at| before.contains(&(at + shift)) || *at + shift > 320 || *at + shift < 0),
            "{before:?} -> {after:?}"
        );
    }

    #[test]
    fn the_originals_tab_code_reads_as_a_space() {
        // F843's people text: "Government Type:", the column tab (0xfd, column 'A'), the
        // government, a line break, then the next heading.
        let raw = b"Government Type:\xfdAFeudal\rEconomy Type:\xfdASmall Mining\x18";
        assert_eq!(
            plain_text(raw),
            "Government Type: Feudal\rEconomy Type: Small Mining"
        );
    }

    #[test]
    fn only_a_click_selects_a_star_and_panning_never_does() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        assert!(map.selected.is_none(), "nothing is selected on opening");
        map.pan_pixels(50.0, 20.0);
        map.pan(0x1_0000, 0x1_0000);
        map.recentre();
        assert!(map.selected.is_none(), "keys and drags only move the view");
        map.pick(7, [0x1_0000, 0x2_0000, 0], 3);
        assert_eq!(map.selected.as_ref().map(|s| s.code), Some(7));
        // The picked star stays selected while the view moves.
        map.pan_pixels(5.0, 0.0);
        map.pan(0x2_0000, 0);
        assert_eq!(map.selected.as_ref().map(|s| s.code), Some(7));
    }

    #[test]
    fn the_line_home_ends_on_the_view_and_never_on_other_text() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        // Home at the centre: on view, the line is a stub at the centre.
        let near = map.home_pointer();
        assert!(near.on_view);
        assert_eq!(near.tip, (160, 79));
        assert_eq!(near.distance, 0);
        // Panned far away: off view, the tip is on the inset rectangle on the way home.
        map.pan_pixels(900.0, 300.0);
        let far = map.home_pointer();
        assert!(!far.on_view);
        assert!(far.distance > 0);
        let (x, y) = (f64::from(far.tip.0), f64::from(far.tip.1));
        assert!((EDGE_LEFT..=EDGE_RIGHT).contains(&x), "{x}");
        assert!((EDGE_TOP..=EDGE_BOTTOM).contains(&y), "{y}");
        // Home is left of and below the view's centre, so the tip is too.
        assert!(far.tip.0 < 160 && far.tip.1 > 79, "{:?}", far.tip);
    }

    #[test]
    fn an_edge_point_lies_on_the_inset_rectangle_in_every_direction() {
        for (dx, dy) in [
            (900, 0),
            (-900, 0),
            (0, 900),
            (0, -900),
            (500, 400),
            (-500, 400),
            (-300, -600),
            (700, -50),
        ] {
            let (x, y) = edge_point((160 + dx, 79 + dy));
            let (fx, fy) = (f64::from(x), f64::from(y));
            assert!(
                (EDGE_LEFT - 0.5..=EDGE_RIGHT + 0.5).contains(&fx),
                "{dx},{dy}: {x}"
            );
            assert!(
                (EDGE_TOP - 0.5..=EDGE_BOTTOM + 0.5).contains(&fy),
                "{dx},{dy}: {y}"
            );
            let on_edge = (fx - EDGE_LEFT).abs() < 0.6
                || (fx - EDGE_RIGHT).abs() < 0.6
                || (fy - EDGE_TOP).abs() < 0.6
                || (fy - EDGE_BOTTOM).abs() < 0.6;
            assert!(on_edge, "{dx},{dy}: {x},{y}");
        }
        assert_eq!(edge_point((160, 79)), (160, 79));
    }

    #[test]
    fn fitting_a_route_puts_every_stop_in_the_clear_part_of_the_view() {
        let mut map = EnhancedMap::new([0x1_0000, 0x1_0000, 0], 0);
        map.route.push(Waypoint {
            code: 1,
            position: [0x5_0000, 0x2_8000, 0],
            auto: true,
        });
        map.route.push(Waypoint {
            code: 2,
            position: [0x9_0000, -0x1_0000, 0],
            auto: false,
        });
        map.fit_route();
        for point in std::iter::once(map.home).chain(map.route.iter().map(|w| w.position)) {
            let (x, y, _) = map.project(point);
            assert!((20..=300).contains(&x), "x {x}");
            assert!((34..=108).contains(&y), "y {y}");
        }
        // A short route does not zoom in past the opening zoom.
        let mut near = EnhancedMap::new([0, 0, 0], 0);
        near.route.push(Waypoint {
            code: 1,
            position: [0x800, 0, 0],
            auto: false,
        });
        near.fit_route();
        assert_eq!(near.zoom, DEFAULT_ZOOM);
    }

    #[test]
    fn a_click_picks_the_nearest_dot_within_a_few_pixels() {
        let plot = |code, x, y| Plot {
            code,
            position: [0; 3],
            x,
            y,
            size: 2,
            kind: 0,
        };
        let plots = [plot(1, 100, 50), plot(2, 106, 50), plot(3, 200, 120)];
        assert_eq!(hit(&plots, 101, 50).map(|p| p.code), Some(1));
        assert_eq!(hit(&plots, 104, 50).map(|p| p.code), Some(2));
        assert_eq!(hit(&plots, 150, 90), None);
        assert_eq!(hit(&plots, 200 + HIT + 2, 120).map(|p| p.code), Some(3));
        assert_eq!(hit(&plots, 200 + HIT + 3, 120).map(|p| p.code), None);
    }

    #[test]
    fn danger_follows_the_patrol_byte_pirates_are_drawn_from() {
        assert_eq!(Threat::from_patrol(0), Threat::Safe);
        assert_eq!(Threat::from_patrol(1), Threat::Low);
        assert_eq!(Threat::from_patrol(2), Threat::Low);
        assert_eq!(Threat::from_patrol(3), Threat::Moderate);
        assert_eq!(Threat::from_patrol(4), Threat::Moderate);
        assert_eq!(Threat::from_patrol(5), Threat::High);
        assert_eq!(Threat::from_patrol(9), Threat::High);
        let colours = [
            Threat::Safe.colour(),
            Threat::Low.colour(),
            Threat::Moderate.colour(),
            Threat::High.colour(),
        ];
        for (i, a) in colours.iter().enumerate() {
            for b in &colours[i + 1..] {
                assert_ne!(a, b, "each rating has its own colour");
            }
        }
    }

    #[test]
    fn a_longer_distance_plays_a_longer_tunnel_and_sound() {
        use crate::navigation::{enhanced_transit_seconds, Transit, HYPERSPACE_RATE};
        let short = enhanced_transit_seconds(500, 1);
        let route = enhanced_transit_seconds(2000, 3);
        let far = enhanced_transit_seconds(9000, 1);
        assert!(
            short > 3.0 && short < route && route < far,
            "{short} {route} {far}"
        );
        // A very long route stops at twenty seconds.
        assert_eq!(enhanced_transit_seconds(100_000, 10), 20.0);
        // The same game time passes over the stretched real time.
        let transit = Transit {
            remaining: 2.0 * HYPERSPACE_RATE,
            total: 2.0 * HYPERSPACE_RATE,
            stretch: 4.0,
        };
        assert!((transit.real_seconds_left() - 8.0).abs() < 1.0e-9);
        let plain = Transit {
            stretch: 1.0,
            ..transit
        };
        assert!((plain.real_seconds_left() - 2.0).abs() < 1.0e-9);
    }

    #[test]
    fn a_long_tunnel_only_ever_plays_forward() {
        use crate::navigation::{Transit, HYPERSPACE_RATE};
        let natural = 2.0;
        let at = |stretch: f64, real: f64| {
            let total = natural * HYPERSPACE_RATE;
            Transit {
                total,
                remaining: total - real / stretch * HYPERSPACE_RATE,
                stretch,
            }
            .share()
        };
        // Unstretched, it is the original's share: elapsed over total, times 145.
        for real in [0.0, 0.3, 1.0, 1.5, 1.99] {
            assert!(
                (at(1.0, real) - real / natural * 145.0).abs() < 1.0e-6,
                "{real}"
            );
        }
        let rate = 145.0 / crate::navigation::TUNNEL_SECONDS;
        // Stretched to 8 s, and a long route's 12, 20 and 40 s: it starts at 0, ends at the
        // end of the tunnel, never moves backwards or jumps, runs at the tunnel's own pace in
        // the opening and closing and keeps the swirl inside its own shares.
        for whole in [8.0, 12.0, 20.0, 40.0] {
            let stretch = whole / natural;
            assert!(at(stretch, 0.0) < 1.0e-6);
            assert!(at(stretch, whole - 1.0e-6) > 144.9, "{whole}");
            let step = 0.01;
            let mut real = 0.0;
            while real + step < whole {
                let delta = at(stretch, real + step) - at(stretch, real);
                assert!(
                    delta >= -1.0e-9,
                    "{whole} s runs backwards at {real}: {delta}"
                );
                assert!(
                    delta <= rate * step * 1.0001 + 1.0e-6,
                    "{whole} s jumps by {delta} at {real}"
                );
                if (70.0..110.0).contains(&at(stretch, real)) {
                    assert!(delta < rate * step, "the swirl is slowed at {real}");
                }
                real += step;
            }
            // The opening and closing run at the tunnel's own pace.
            assert!((at(stretch, 0.5) - 0.5 * rate).abs() < 1.0e-6);
            assert!((at(stretch, whole - 0.5) - (145.0 - 0.5 * rate)).abs() < 1.0e-6);
        }
        // A jump of any length moves forward only, through the same shares.
        for seconds in [3.2, 3.9, 6.0, 12.0, 20.0] {
            let total = seconds * HYPERSPACE_RATE;
            let share = |real: f64| {
                Transit {
                    total,
                    remaining: total - real * HYPERSPACE_RATE,
                    stretch: 1.0,
                }
                .share()
            };
            let mut last = share(0.0);
            let mut real = 0.0;
            while real < seconds {
                let now = share(real);
                assert!(now >= last - 1.0e-9, "{seconds} s runs backwards at {real}");
                last = now;
                real += 0.007;
            }
            assert!(share(seconds - 1.0e-6) > 144.9);
        }
        // The animation word is the share, as before.
        let total = natural * HYPERSPACE_RATE;
        let word = Transit {
            total,
            remaining: total / 2.0,
            stretch: 1.0,
        }
        .animation();
        assert_eq!(word, ((72.0f64 as u16) << 8) & 0xff00);
        // A long transit's word keeps the share's fraction, so the slowed swirl is smooth.
        let long_total = 8.0 * HYPERSPACE_RATE;
        let long_word = |remaining: f64| {
            Transit {
                total: long_total,
                remaining,
                stretch: 1.0,
            }
            .animation()
        };
        let (early, later) = (
            long_word(long_total * 0.5),
            long_word(long_total * 0.5 - 1.0e4),
        );
        assert!(later >= early && later - early < 256, "{early} {later}");
    }
}
