//! The enhanced (flat) galaxy map (`enhancement-12`): a 2D top-down starfield
//! that replaces the original's 3D scene, with a Sol indicator and multi-step
//! `X` routes. The projection is pure so the geometry is unit-testable; the
//! screen draws the [`Plot`] list through `Screen::draw_enhanced_galaxy`.
//!
//! Docs: `docs/notes/galaxy-map.md` and `docs/notes/enhancements.md`.

use crate::galaxy_map::{sector_code, Selected};
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

/// One step of the multi-step route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waypoint {
    pub code: u32,
    pub position: [i32; 3],
}

/// The enhanced map's state: the pan centre, zoom, the selection and the route.
#[derive(Debug, Clone)]
pub struct EnhancedMap {
    /// The view centre in 16.16 sectors (`+0x23a`/`+0x23e` of the original).
    pub centre: [i32; 2],
    pub zoom: f64,
    /// The player's current system (16.16), for recentre and route legs.
    pub home: [i32; 3],
    /// The drive's range in its units (`DATA_008907`).
    pub reach: i32,
    pub selected: Option<Selected>,
    /// The `X` route, in the order it was marked.
    pub route: Vec<Waypoint>,
    /// The centre moved: the selection is looked for again.
    pub dirty: bool,
}

impl EnhancedMap {
    pub fn new(home: [i32; 3], reach: i32) -> Self {
        Self {
            centre: [home[0], home[1]],
            zoom: DEFAULT_ZOOM,
            home,
            reach,
            selected: None,
            route: Vec::new(),
            dirty: true,
        }
    }

    /// `Home`/`C`: back to the player's system, level, at the opening zoom.
    pub fn recentre(&mut self) {
        self.centre = [self.home[0], self.home[1]];
        self.zoom = DEFAULT_ZOOM;
        self.dirty = true;
    }

    /// Move the centre by `x`/`y` (16.16 sectors): the right-drag pan.
    pub fn pan(&mut self, x: i32, y: i32) {
        self.centre = [
            self.centre[0].wrapping_add(x),
            self.centre[1].wrapping_add(y),
        ];
        self.dirty = true;
    }

    /// Move the view by `right`/`up` screen pixels (the arrow keys): the stars
    /// slide the other way, so Right looks further right.
    pub fn pan_pixels(&mut self, right: f64, up: f64) {
        self.pan(
            (right / self.zoom).round() as i32,
            (up / self.zoom).round() as i32,
        );
    }

    /// Select `star` directly (a click on it). The selection holds until the
    /// view is panned, when it goes back to the star nearest the centre.
    pub fn pick(&mut self, code: u32, position: [i32; 3], kind: u16) {
        self.selected = Some(Selected {
            position,
            code,
            kind,
        });
        self.dirty = false;
    }

    /// Scale the zoom by `factor` (over 1 zooms in).
    pub fn zoom_by(&mut self, factor: f64) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// The star nearest the centre, exactly the original's rule (the same as the
    /// 3D map): the star of the centre sector with the smallest `|dx| + |dy|`,
    /// within a quarter sector.
    pub fn select(&mut self, tables: &SectorTables) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let (sx, sy) = ((self.centre[0] >> 16) as u16, (self.centre[1] >> 16) as u16);
        let fraction =
            |value: i32| i32::from((((value & 0xfe00) as u16).wrapping_add(0x8000)) as i16) >> 9;
        let (fx, fy) = (fraction(self.centre[0]), fraction(self.centre[1]));
        let stars = tables.stars(sx, sy);
        let mut best = 0x20;
        let mut found = None;
        for (index, star) in stars.iter().enumerate() {
            let miss = (i32::from(star.vector[4] as i8) - fx).abs()
                + (i32::from(star.vector[3] as i8) - fy).abs();
            if miss < best {
                best = miss;
                found = Some(index);
            }
        }
        self.selected = found.map(|index| {
            let star = &stars[index];
            Selected {
                position: sector::position(sx, sy, &star.vector),
                code: sector_code(sx, sy) | (index as u32) << 26,
                kind: star.kind,
            }
        });
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

    /// Every star of the nine sectors around the centre, projected.
    pub fn plots(&self, tables: &SectorTables) -> Vec<Plot> {
        let mut out = Vec::new();
        for (along, across) in [
            (0, 0),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ] {
            let (x, y) = (
                (self.centre[0].wrapping_add(along << 16) >> 16) as u16,
                (self.centre[1].wrapping_add(across << 16) >> 16) as u16,
            );
            for (index, star) in tables.stars(x, y).iter().enumerate() {
                let position = sector::position(x, y, &star.vector);
                let (px, py, size) = self.project(position);
                if (0..VIEW_W).contains(&px) && (0..VIEW_H).contains(&py) {
                    out.push(Plot {
                        code: sector_code(x, y) | (index as u32) << 26,
                        position,
                        x: px,
                        y: py,
                        size,
                        kind: star.kind,
                    });
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

    /// The corner "Sector: x,y" line, relative to Sol (the same as the 3D map's).
    pub fn sector_text(&self) -> String {
        let x = (self.centre[0] >> 16) - SOL_SECTOR[0];
        let y = (self.centre[1] >> 16) - SOL_SECTOR[1];
        format!("Sector: {x},{y}")
    }

    /// F828's fuel line for the selected star, from the hyperdrive's report
    /// (`Fuel Required: n`, `Insufficient Fuel (n required)` or `Out of Range`).
    pub fn fuel_text(&self, report: Option<&ffe_core::hyperspace::Report>) -> Option<String> {
        let selected = self.selected.as_ref()?;
        if sector::distance(self.home, selected.position) == 0 {
            return None;
        }
        let report = report?;
        Some(if report.result < 0 {
            if report.fuel_needed < 0 {
                "Out of Range".to_owned()
            } else {
                format!("Insufficient Fuel ({}t required)", report.fuel_needed)
            }
        } else {
            format!("Fuel Required: {}t", report.fuel_needed)
        })
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
            self.route.push(Waypoint { code, position });
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

    /// A click on a star: add it to the route, or take it off again when it is
    /// already on it (only that star; the legs re-plan through the rest). The
    /// player's own system is not a destination.
    pub fn toggle_waypoint(&mut self, code: u32, position: [i32; 3]) -> Toggled {
        if sector::distance(self.home, position) == 0 {
            return Toggled::Ignored;
        }
        if let Some(index) = self.route.iter().position(|waypoint| waypoint.code == code) {
            self.route.remove(index);
            Toggled::Removed
        } else {
            self.route.push(Waypoint { code, position });
            Toggled::Added
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
    /// Tonnes of fuel in the hold.
    pub drive_fuel: i32,
    pub date: String,
    /// The mouse, in view coordinates, when it is over the screen.
    pub mouse: Option<(i32, i32)>,
    /// The station/port note for a star the mouse is on (looked up lazily by the caller).
    pub station_note: &'a mut dyn FnMut(&Plot) -> Option<String>,
}

/// The controls line along the top of the view.
pub const HINT: &str = "Click star: route  H: jump  Arrows: pan  Hover: info";

/// Build everything the screen draws of the enhanced map for this frame.
pub fn scene(map: &mut EnhancedMap, ctx: &mut SceneContext<'_>) -> crate::screen::EnhancedGalaxy {
    map.select(ctx.sectors);
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
        if inhabited
            || selected_plot.is_some_and(|s| s.code == plot.code)
            || hovered_plot.is_some_and(|h| h.code == plot.code)
        {
            if let Some(name) = (ctx.name_of)(plot.code) {
                labels.push((plot.x, plot.y, name));
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
    // The selected star's fuel line and its facts.
    let report = map
        .selected
        .as_ref()
        .and_then(|selected| (ctx.report_for)(sector::distance(map.home, selected.position)));
    let fuel = map.fuel_text(report.as_ref());
    let info = star_info(map, ctx);
    // Each leg: can the range and the fuel left fly it? Waypoints that leave the
    // player stranded (uninhabited, too little fuel for the next leg) get a warning.
    let legs = map.legs();
    let mut leg_ok = Vec::with_capacity(legs.len());
    let mut warnings = Vec::new();
    let (mut used, mut total_fuel, mut all_ok) = (0, 0, true);
    for (index, distance) in legs.iter().enumerate() {
        let leg = (ctx.report_for)(*distance);
        let leg_fuel = leg.as_ref().map_or(0, |report| report.fuel_needed.max(0));
        let ok = leg
            .as_ref()
            .is_some_and(|report| report.result >= 0 && used + leg_fuel <= ctx.drive_fuel);
        leg_ok.push(ok);
        all_ok &= ok;
        used += leg_fuel;
        total_fuel += leg_fuel;
        let next = legs.get(index + 1).map_or(1, |d| {
            (ctx.report_for)(*d).map_or(0, |r| r.fuel_needed.max(0))
        });
        let inhabited = ctx
            .systems
            .is_some_and(|tables| tables.market(map.route[index].code).population != 0);
        if !inhabited && ctx.drive_fuel - used < next {
            if let Some(point) = route.get(index + 1) {
                warnings.push(*point);
            }
        }
    }
    let route_summary = (!map.route.is_empty()).then(|| {
        let total: i32 = legs.iter().sum();
        let text = format!(
            "Route: {} jump{}, {}.{:02} ly, {} t fuel{}",
            legs.len(),
            if legs.len() == 1 { "" } else { "s" },
            total / 100,
            total % 100,
            total_fuel,
            if all_ok { "" } else { " - cannot be flown" },
        );
        (text, if all_ok { 15 } else { THREAT_HIGH })
    });
    let hover = hovered_plot.map_or_else(Vec::new, |plot| hover_lines(&plot, ctx));
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
        fuel,
        date: ctx.date.clone(),
        info,
        route_summary,
        hover,
        legend,
        hint: HINT.to_owned(),
    }
}

/// What hovering a star says: its name and whether anyone lives there, how
/// dangerous it is to arrive in, and whether it has a station or port.
fn hover_lines(plot: &Plot, ctx: &mut SceneContext<'_>) -> Vec<(String, u8)> {
    let name = (ctx.name_of)(plot.code).unwrap_or_default();
    let mut lines = Vec::new();
    match ctx.systems {
        Some(tables) => {
            let market = tables.market(plot.code);
            if market.population == 0 {
                lines.push((format!("{name}: uninhabited"), UNINHABITED));
            } else {
                let faction = match market.allegiance >> 6 {
                    1 => "Imperial",
                    2 => "Federal",
                    3 => "Alliance",
                    _ => "Independent",
                };
                lines.push((
                    format!("{name}: inhabited ({faction}, pop {})", market.population),
                    15,
                ));
                if let Some(threat) = system_threat(plot.code, tables) {
                    lines.push((format!("Danger: {}", threat.label()), threat.colour()));
                }
            }
        }
        None => lines.push((name, 15)),
    }
    if let Some(note) = (ctx.station_note)(plot) {
        lines.push((note, 15));
    }
    lines
}

/// The console panel for the selected star: its name and distance, its faction
/// and population, how dangerous it is, then F869's description.
fn star_info(map: &EnhancedMap, ctx: &SceneContext<'_>) -> Vec<(String, u8)> {
    let Some(selected) = map.selected.as_ref() else {
        return Vec::new();
    };
    let name = (ctx.name_of)(selected.code).unwrap_or_default();
    let distance = sector::distance(map.home, selected.position);
    let mut lines = vec![(
        format!("{name}  {}.{:02} ly", distance / 100, distance % 100),
        15,
    )];
    if let Some(tables) = ctx.systems {
        let market = tables.market(selected.code);
        let faction = match market.allegiance >> 6 {
            1 => "Imperial",
            2 => "Federal",
            3 => "Alliance",
            _ => "Independent",
        };
        if market.population == 0 {
            lines.push(("Uninhabited".to_owned(), UNINHABITED));
        } else {
            lines.push((format!("{faction}, population {}", market.population), 15));
            if let Some(threat) = system_threat(selected.code, tables) {
                lines.push((format!("Danger: {}", threat.label()), threat.colour()));
            }
        }
        let code = tables.details(selected.code).words[0] as u16;
        if let Some(raw) = (ctx.string_of)(u32::from(code)) {
            let description: String =
                ffe_core::system_text::expand(&raw, &[0; 5], &|c| (ctx.string_of)(u32::from(c)))
                    .into_iter()
                    .map(char::from)
                    .collect();
            if !description.is_empty() {
                lines.push((description.replace('\r', " "), 15));
            }
        }
    }
    lines
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
        assert!(map.dirty);
        let before = map.centre;
        map.pan_pixels(0.0, 10.0);
        assert_eq!(map.centre[0], before[0]);
        assert!(map.centre[1] > before[1], "up raises the galaxy's y");
    }

    #[test]
    fn a_picked_star_stays_selected_until_the_map_moves() {
        let mut map = EnhancedMap::new([0, 0, 0], 0);
        map.pick(7, [0x1_0000, 0x2_0000, 0], 3);
        assert!(!map.dirty);
        assert_eq!(map.selected.as_ref().map(|s| s.code), Some(7));
        map.pan_pixels(5.0, 0.0);
        assert!(
            map.dirty,
            "a pan looks for the star nearest the centre again"
        );
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
}
