//! The original's 3D system map: module 5 of `DATA_007239` (F963-F986 in
//! `jjffe-master/ffe.asm`), the page with the "Time point:" line, the orbit
//! curves and the body labels. The galaxy map reaches it with F6 (this system,
//! F966) or `n`/`r` (the selected star's system, F974); its F5 and F7-F10 hand
//! back to the galaxy map's pages. Docs: `docs/notes/system-map.md`.
//!
//! What is the original's here: the page's keys and hot areas (F982), the time
//! point and the slider that moves it (F980: the offset from the slider's
//! centre, squared for this system and cubed for another, shifted left 18 into
//! the 2^32-per-day clock), the orbit of every object that has a parent (F986),
//! the name labels that skip starports and the view's right and bottom edges
//! (F975/F978), the target marker, and the click that targets an object within
//! eight pixels of its label point (F983). The view is the original's too: a camera object
//! parented to the focus (`DATA_008979`) that stands `mantissa * 2^(exponent - 16)`
//! millimetres back along the third row of F1676's matrix (the mouse accumulators
//! as its angles), projected by F1680/F1479 (`160 + 256 x / z`). What is the port's
//! own: the orbit is the circle F1507 places the object on, drawn as a polyline
//! instead of F986's ring model. See `docs/DEVIATIONS.md`.

use crate::navigation::{Body, Kind};
use ffe_core::ffp::{multiply, normalise, Ffp};
use ffe_core::model_animation::{
    setup_object, Camera as ObjectCamera, Descriptor, Draw, ObjectState,
};

/// The page's drawing height: the cabin view above the console (F977 clips at
/// y `0x95`).
pub const VIEW_HEIGHT: i32 = 158;
/// `DATA_007241`'s hot area (x0, y0, x1, y1): F982's action `0x80` slider.
pub const SLIDER: (i32, i32, i32, i32) = (268, 156, 319, 177);
/// F982 measures the slider's click from this x (`0x12c`).
const SLIDER_CENTRE: i32 = 0x12c;
/// One astronomical unit in millimetres, for the details.
const MM_PER_AU: f64 = 1.495_978_707e14;
/// Seconds in a game day (the clock's high dword counts days).
const DAY: f64 = 86_400.0;
/// F1479's centre of the view.
const CENTRE: (f64, f64) = (160.0, 79.0);
/// F1479's focal length: `x * 256 / z`.
const FOCAL: f64 = 256.0;
/// F983's pick distance (the sum of the pixel offsets, `cmp edx,8`).
const PICK: i32 = 8;
/// F969 starts the mouse accumulators at 0x100 each: a quarter turn about each axis.
const TURN_START: f64 = 256.0;
/// The accumulators are shifted left by 6 into a 16-bit binary angle (F982), so 1024 of
/// them make a whole turn.
const TURN_UNITS: f64 = 1024.0;
/// F963 starts the camera's distance at this mantissa and exponent.
const START_ZOOM: Ffp = Ffp::new(0x7fff, 0x2a);
/// The model F986 draws an orbit with (`DATA_008968`'s object has `+0x82` set to `0x96`).
const RING_MODEL: u32 = 0x96;
/// The largest ring, in pixels of radius, the page draws.
const MAX_RING_PIXELS: f64 = 1500.0;
/// F982's F12 stops the zoom at exponent `0x3f`.
const MAX_EXPONENT: i16 = 0x3f;

/// The text colours F842 uses: green orbit lines, yellow, white.
pub const GREEN: u8 = 17;
pub const YELLOW: u8 = 18;
pub const WHITE: u8 = 15;

/// What the page remembers between visits (`DATA_008804 + 0x408` and `DATA_008983`):
/// the camera's distance and the object it is parented to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    /// The distance as an ffp number: the camera stands `mantissa * 2^(exponent - 16)`
    /// millimetres from the focus.
    pub zoom: Ffp,
    /// The focus's slot in the system's object list (`DATA_008979`).
    pub slot: usize,
}

impl View {
    /// F963 at a new game: `0x7fff` at exponent `0x2a`, and the object the start type
    /// names (`0x6c`, `0x6f`, `0x60`, `0x70` for start types 0, 1, 2 and 3; any other
    /// start type `0x6f`): the world the player starts at.
    pub fn new_game(start_type: u8) -> Self {
        let slot = match start_type {
            0 => 0x6c,
            2 => 0x60,
            3 => 0x70,
            _ => 0x6f,
        };
        View {
            zoom: START_ZOOM,
            slot,
        }
    }
}

/// One system seen from outside, at a time point.
#[derive(Debug, Clone)]
pub struct SystemMap {
    /// The bodies at [`Self::clock`], in the order of the navigator's list.
    pub bodies: Vec<Body>,
    /// The system's name, for the details.
    pub system: String,
    /// The player's own system (F966, slider squared) rather than the star the
    /// galaxy map selected (F974, slider cubed).
    pub home: bool,
    /// The game clock when the page opened, in world seconds.
    pub now: f64,
    /// The time point: the clock the bodies stand at.
    pub clock: f64,
    /// The mouse accumulators (`DATA_008980`, `DATA_008981`): x turns the view's second
    /// angle and y its first (F982).
    pub turn: (f64, f64),
    /// The camera's distance from the focus (an ffp number, see [`View::zoom`]).
    pub zoom: Ffp,
    /// The lowest exponent the zoom may reach: F984's model size plus ten. Zero when the
    /// model's size is not known.
    pub floor: i16,
    /// The body the camera is parented to (`DATA_008979`): the view's centre.
    pub focus: usize,
    /// The body the last click picked (F983's target).
    pub selected: Option<usize>,
    /// `t`: the next click is a destination for the autopilot (F982's flag).
    pub navcomp: bool,
}

/// A far object's dot sprite as F1680 asks F1700 to draw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite {
    /// The object's model, whose descriptor names its colour and size class.
    pub model: u32,
    /// F1479's projection of the centre.
    pub screen: [i16; 2],
    /// F1680's `SpecialDiv` operands: the descriptor's radius shifted by the scale,
    /// and the view depth.
    pub radius: i32,
    pub depth: i32,
}

/// A body's dot as drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Dot {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    pub colour: u8,
    pub kind: Kind,
    /// The body's index in [`SystemMap::bodies`].
    pub index: usize,
    pub depth: f64,
    /// The picked body (F978's marker is drawn about it).
    pub marked: bool,
    /// The original's dot sprite, when the model's descriptor was known.
    pub sprite: Option<Sprite>,
}

/// A name written beside a dot (F978).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub x: i32,
    pub y: i32,
    pub text: String,
    /// The picked body, whose label carries F978's marker.
    pub marked: bool,
}

/// Everything the screen draws for one frame of the page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    pub orbits: Vec<Vec<(i32, i32)>>,
    pub dots: Vec<Dot>,
    pub labels: Vec<Label>,
    /// "Time point: ..." (F977's string 0, white at (10, 148)).
    pub time_point: String,
    /// The picked body's facts, bottom up from y 137.
    pub details: Vec<String>,
}

fn sub(a: [i64; 3], b: [i64; 3]) -> [f64; 3] {
    [
        (a[0] - b[0]) as f64,
        (a[1] - b[1]) as f64,
        (a[2] - b[2]) as f64,
    ]
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// F982's slider (action `0x80`): the game seconds one click at logical x moves
/// the time point; negative left of the slider's centre.
pub fn slider_step(x: i32, home: bool) -> f64 {
    let offset = x - SLIDER_CENTRE;
    let magnitude = i64::from(offset.abs());
    let value = if home {
        magnitude * magnitude
    } else {
        magnitude * magnitude * magnitude
    };
    // `value << 18` in a clock whose low dword is 2^32 to the day.
    let days = value as f64 / 16_384.0;
    days.copysign(f64::from(offset)) * DAY
}

/// Whether a logical point is on the slider's hot area.
pub fn on_slider(x: i32, y: i32) -> bool {
    (SLIDER.0..=SLIDER.2).contains(&x) && (SLIDER.1..=SLIDER.3).contains(&y)
}

/// The ffp number nearest a positive value, normalised (bit 14 of the mantissa set).
fn ffp_from(value: f64) -> Ffp {
    let exponent = value.log2().floor() as i32 + 1;
    let mantissa = (value / 2f64.powi(exponent) * 32768.0).round().min(32767.0);
    normalise(Ffp::new(mantissa as i16, exponent.clamp(-100, 100) as i16))
}

fn ffp_value(value: Ffp) -> f64 {
    f64::from(value.mantissa) / 32768.0 * 2f64.powi(i32::from(value.exponent))
}

impl SystemMap {
    pub fn new(bodies: Vec<Body>, system: String, home: bool, clock: f64) -> Self {
        Self::open(bodies, system, home, clock, &View::new_game(1))
    }

    /// F966 (this system) or F974 (another): the page opens with the camera where it was
    /// left (`view`). Another system starts at the highest slot there is (F974 sets
    /// `DATA_008979` to `0x72`, and F968 steps down to an object that is drawn).
    pub fn open(bodies: Vec<Body>, system: String, home: bool, clock: f64, view: &View) -> Self {
        let wanted = if home { view.slot } else { 0x72 };
        let focus = (0..=wanted)
            .rev()
            .find_map(|slot| {
                bodies
                    .iter()
                    .position(|b| b.slot == slot && b.kind != Kind::Starport)
            })
            .or_else(|| bodies.iter().position(|body| body.kind == Kind::Star))
            .unwrap_or(0);
        SystemMap {
            bodies,
            system,
            home,
            now: clock,
            clock,
            turn: (TURN_START, TURN_START),
            zoom: view.zoom,
            floor: 0,
            focus,
            selected: None,
            navcomp: false,
        }
    }

    /// What to remember when the page closes.
    pub fn view(&self) -> View {
        View {
            zoom: self.zoom,
            slot: self.bodies.get(self.focus).map_or(0x6f, |b| b.slot),
        }
    }

    /// The farthest orbit about the focus: the planets of a star, the moons and
    /// stations of a world; `None` for a body with nothing around it.
    fn reach(&self) -> Option<f64> {
        let focus = self.bodies.get(self.focus)?;
        self.bodies
            .iter()
            .enumerate()
            .filter(|(i, body)| {
                *i != self.focus
                    && body.kind != Kind::Starport
                    && if focus.kind == Kind::Star {
                        body.kind == Kind::Planet
                    } else {
                        body.parent == Some(self.focus)
                    }
            })
            .map(|(_, body)| length(sub(body.position, focus.position)))
            .fold(None, |best: Option<f64>, r| {
                Some(best.map_or(r, |b| b.max(r)))
            })
    }

    /// Set the camera's distance so the focus's farthest orbit fills most of the view
    /// (the port's own: the original keeps its distance when the focus changes).
    pub fn fit(&mut self) {
        let reach = self.reach().filter(|r| *r > 0.0).unwrap_or_else(|| {
            self.bodies
                .get(self.focus)
                .map_or(1e9, |body| body.radius as f64 * 4.0)
        });
        self.set_distance(reach * FOCAL / 66.0);
    }

    /// The camera's distance from the focus in millimetres.
    pub fn distance(&self) -> f64 {
        ffp_value(self.zoom) / 2.0
    }

    fn set_distance(&mut self, millimetres: f64) {
        let wanted = ffp_from((millimetres * 2.0).max(1.0));
        self.zoom = if wanted.exponent > MAX_EXPONENT {
            Ffp::new(0x7fff, MAX_EXPONENT)
        } else if wanted.exponent < self.floor {
            Ffp::new(0x4000, self.floor)
        } else {
            wanted
        };
    }

    /// Centre the page on a body (the distance is refitted to what orbits it).
    pub fn focus_on(&mut self, index: usize) {
        if index < self.bodies.len() {
            self.focus = index;
            self.fit();
        }
    }

    /// Back to the star (or the first body) the page opened on.
    pub fn reset_focus(&mut self) {
        let star = self
            .bodies
            .iter()
            .position(|body| body.kind == Kind::Star)
            .unwrap_or(0);
        self.focus_on(star);
    }

    /// Mouse turn (F982's accumulators): `dx` and `dy` are added to them.
    pub fn rotate(&mut self, dx: f64, dy: f64) {
        self.turn.0 += dx;
        self.turn.1 += dy;
    }

    /// F11 (zoom in): the distance times `0x7998` at exponent 0 (about 0.95), kept
    /// no closer than F984's size of the focus's model.
    pub fn zoom_in(&mut self) {
        let candidate = normalise(multiply(self.zoom, Ffp::new(0x7998, 0)));
        if self.floor <= candidate.exponent {
            self.zoom = candidate;
        }
    }

    /// F12 (zoom out): the distance times `0x435d` at exponent 1 (about 1.05), no
    /// farther than exponent `0x3f`.
    pub fn zoom_out(&mut self) {
        let candidate = normalise(multiply(self.zoom, Ffp::new(0x435d, 1)));
        self.zoom = if candidate.exponent > MAX_EXPONENT {
            Ffp::new(0x7fff, MAX_EXPONENT)
        } else {
            candidate
        };
    }

    /// Smooth zoom for the keys the port adds: scale the distance by `1 / factor`.
    pub fn zoom(&mut self, factor: f64) {
        if factor > 0.0 {
            self.set_distance(self.distance() / factor);
        }
    }

    /// Replace the bodies for a new time point.
    pub fn set_time(&mut self, bodies: Vec<Body>, clock: f64) {
        if bodies.len() == self.bodies.len() {
            self.bodies = bodies;
        }
        self.clock = clock;
    }

    /// F982's camera matrix (`MatBuildYZ`): rows are the view's x, y and z axes in the
    /// system's frame. The first angle is the y accumulator, the second the negated x.
    fn rows(&self) -> [[f64; 3]; 3] {
        let a = self.turn.1 * std::f64::consts::TAU / TURN_UNITS;
        let b = -self.turn.0 * std::f64::consts::TAU / TURN_UNITS;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        [
            [ca * cb, -(sa * cb), sb],
            [sa, ca, 0.0],
            [-(sb * ca), sa * sb, cb],
        ]
    }

    /// The camera's position relative to the focus: minus the distance along the matrix's
    /// third row (F982's `mantissa * (row >> 16) << (exponent - 31)`).
    fn camera_offset(&self, rows: &[[f64; 3]; 3]) -> [f64; 3] {
        let scale = f64::from(self.zoom.mantissa) * 2f64.powi(i32::from(self.zoom.exponent) - 31);
        std::array::from_fn(|i| -scale * (rows[2][i] * 32768.0).floor())
    }

    /// The camera as F1680 sees it: its matrix and its position in the system's frame.
    fn object_camera(&self) -> ObjectCamera {
        let rows = self.rows();
        let offset = self.camera_offset(&rows);
        let focus = self.bodies.get(self.focus).map_or([0; 3], |b| b.position);
        let q31 =
            |v: f64| (v * 2_147_483_648.0).clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        ObjectCamera {
            matrix: std::array::from_fn(|k| q31(rows[k / 3][k % 3])),
            position: std::array::from_fn(|i| focus[i].wrapping_add(offset[i].round() as i64)),
        }
    }

    /// A world position on the page (F1479's `160 + 256 x / z`): the logical pixel and
    /// the depth along the view; `None` behind the camera.
    pub fn project(&self, position: [i64; 3]) -> Option<(f64, f64, f64)> {
        let origin = self.bodies.get(self.focus).map_or([0; 3], |b| b.position);
        self.project_vector(sub(position, origin))
    }

    fn project_vector(&self, v: [f64; 3]) -> Option<(f64, f64, f64)> {
        let rows = self.rows();
        let offset = self.camera_offset(&rows);
        let relative = [v[0] - offset[0], v[1] - offset[1], v[2] - offset[2]];
        let view: [f64; 3] = std::array::from_fn(|k| {
            rows[k][0] * relative[0] + rows[k][1] * relative[1] + rows[k][2] * relative[2]
        });
        let depth = view[2];
        // The nearest the original projects: 64 units of the 32-bit view.
        if depth <= 0.0 {
            return None;
        }
        // F1479 divides in integers, which truncate toward zero.
        Some((
            CENTRE.0 + (FOCAL * view[0] / depth).trunc(),
            CENTRE.1 - (FOCAL * view[1] / depth).trunc(),
            depth,
        ))
    }

    /// The orbit of a body as F1507 places it: a circle about its parent in the
    /// plane through the system's z axis and the body, as polylines (the parts of it
    /// in front of the camera).
    fn orbit(&self, body: &Body) -> Vec<Vec<(i32, i32)>> {
        if body.kind == Kind::Starport {
            return Vec::new();
        }
        let Some(centre) = body.orbit_centre else {
            return Vec::new();
        };
        let relative = sub(body.position, centre);
        let radius = length(relative);
        if radius <= 0.0 {
            return Vec::new();
        }
        let flat = relative[0].hypot(relative[1]);
        let (hx, hy) = if flat > radius * 1e-9 {
            (relative[0] / flat, relative[1] / flat)
        } else {
            (1.0, 0.0)
        };
        let origin = self.bodies.get(self.focus).map_or([0; 3], |b| b.position);
        let base = sub(centre, origin);
        let steps = 192;
        let mut pieces: Vec<Vec<(i32, i32)>> = Vec::new();
        let mut piece: Vec<(i32, i32)> = Vec::new();
        for k in 0..=steps {
            let angle = std::f64::consts::TAU * f64::from(k) / f64::from(steps);
            let (s, c) = angle.sin_cos();
            let v = [
                base[0] + radius * hx * s,
                base[1] + radius * hy * s,
                base[2] + radius * c,
            ];
            match self.project_vector(v) {
                Some((x, y, _)) if x.abs() < 4000.0 && y.abs() < 4000.0 => {
                    piece.push((x.round() as i32, y.round() as i32));
                }
                _ => {
                    if piece.len() > 1 {
                        pieces.push(std::mem::take(&mut piece));
                    }
                    piece.clear();
                }
            }
        }
        if piece.len() > 1 {
            pieces.push(piece);
        }
        pieces
    }

    /// F1680's decision for F986's ring: the ring stands at its parent's position with
    /// `+0x7e` set to its radius's ffp exponent less `0x19`; at the far class, or when
    /// F1686 culls it, nothing is drawn.
    fn ring_is_drawn(
        &self,
        body: &Body,
        camera: &ObjectCamera,
        models: &dyn Fn(u32) -> Option<Descriptor>,
    ) -> bool {
        let (Some(centre), Some(descriptor)) = (body.orbit_centre, models(RING_MODEL)) else {
            return true;
        };
        let radius = length(sub(body.position, centre));
        if radius <= 0.0 {
            return false;
        }
        let exponent = ffp_from(radius).exponent;
        let object = ObjectState {
            position: centre,
            scale: i32::from(exponent) - 0x19,
            ..ObjectState::default()
        };
        if !matches!(
            setup_object(&object, camera, &descriptor, [0; 3], &[[0; 3]; 8]),
            Draw::Dot { .. } | Draw::Model { .. }
        ) {
            return false;
        }
        // The port's own: the executed original draws no line for a ring thousands of
        // pixels across (a star's orbit about its barycentre seen from close by), where
        // F1680's far class would still draw it.
        let depth = self.project(centre).map_or(0.0, |(_, _, depth)| depth);
        depth > 0.0 && radius * FOCAL / depth < MAX_RING_PIXELS
    }

    /// The pixel a body's dot is drawn at and its radius (F978/F975 skip the
    /// starports and what lies past the view's right and bottom edges). With the
    /// model's descriptor (`models`) the position and the sprite are F1680's.
    fn dot(
        &self,
        index: usize,
        camera: &ObjectCamera,
        models: &dyn Fn(u32) -> Option<Descriptor>,
    ) -> Option<Dot> {
        let body = self.bodies.get(index)?;
        if body.kind == Kind::Starport {
            return None;
        }
        let (colour, fallback) = match body.kind {
            Kind::Star => (YELLOW, (3, 12)),
            Kind::Planet => (WHITE, (2, 10)),
            Kind::Moon => (11, (1, 6)),
            _ => (GREEN, (1, 1)),
        };
        if let Some(descriptor) = models(body.model) {
            let object = ObjectState {
                position: body.position,
                ..ObjectState::default()
            };
            let (screen, sprite) =
                match setup_object(&object, camera, &descriptor, [0; 3], &[[0; 3]; 8]) {
                    Draw::Far { .. } => return None,
                    Draw::Dot {
                        screen,
                        depth,
                        radius,
                        ..
                    } => (
                        screen?,
                        Some(Sprite {
                            model: body.model,
                            screen: screen?,
                            radius,
                            depth,
                        }),
                    ),
                    Draw::Culled { screen, .. } | Draw::Model { screen, .. } => (screen?, None),
                };
            let (_, _, depth) = self.project(body.position).unwrap_or((0.0, 0.0, 0.0));
            let physical = if depth > 0.0 {
                (body.radius as f64 * FOCAL / depth).round() as i32
            } else {
                0
            };
            return Some(Dot {
                x: i32::from(screen[0]),
                y: i32::from(screen[1]),
                radius: physical.clamp(fallback.0, fallback.1),
                colour,
                kind: body.kind,
                index,
                depth,
                marked: self.selected == Some(index),
                sprite,
            });
        }
        let (x, y, depth) = self.project(body.position)?;
        let physical = (body.radius as f64 * FOCAL / depth).round() as i32;
        Some(Dot {
            x: x.round() as i32,
            y: y.round() as i32,
            radius: physical.clamp(fallback.0, fallback.1),
            colour,
            kind: body.kind,
            index,
            depth,
            marked: self.selected == Some(index),
            sprite: None,
        })
    }

    /// F983: the body whose label point is within eight pixels (the sum of the
    /// offsets) of a click, the nearest first.
    pub fn pick(&self, x: i32, y: i32) -> Option<usize> {
        let none = |_: u32| None;
        let camera = self.object_camera();
        (0..self.bodies.len())
            .filter_map(|i| {
                let dot = self.dot(i, &camera, &none)?;
                let far = (dot.x - x).abs() + (dot.y - y).abs();
                (far <= PICK.max(dot.radius)).then_some((far, i))
            })
            .min()
            .map(|(_, i)| i)
    }

    /// What the page draws now, with the port's own dots. `date` is the time point's
    /// text; `range` the picked body's distance from the ship when it is known.
    pub fn frame(&self, date: &str, range: Option<String>) -> Frame {
        self.frame_with(date, range, &|_| None)
    }

    /// [`Self::frame`] with the models' descriptors (`models`), so that the dots are
    /// F1680's and the distant bodies get the original's sprites.
    pub fn frame_with(
        &self,
        date: &str,
        range: Option<String>,
        models: &dyn Fn(u32) -> Option<Descriptor>,
    ) -> Frame {
        let mut frame = Frame {
            time_point: format!("Time point: {date}"),
            ..Frame::default()
        };
        let camera = self.object_camera();
        for body in &self.bodies {
            // F986 draws a ring for stars, planets and moons that orbit something, and the
            // ring model (`RING_MODEL`) is a far object like any other: F1680 does not draw
            // it past its far class, which the ring's size lowers (`+0x7e`).
            if !matches!(body.kind, Kind::Star | Kind::Planet | Kind::Moon)
                || !self.ring_is_drawn(body, &camera, models)
            {
                continue;
            }
            frame.orbits.extend(self.orbit(body));
        }
        let mut dots: Vec<Dot> = (0..self.bodies.len())
            .filter_map(|i| self.dot(i, &camera, models))
            .collect();
        dots.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        frame.dots = dots;
        for (index, body) in self.bodies.iter().enumerate() {
            // F978 names stars, planets and moons only.
            if !matches!(body.kind, Kind::Star | Kind::Planet | Kind::Moon) {
                continue;
            }
            let Some(dot) = self.dot(index, &camera, models) else {
                continue;
            };
            let marked = self.selected == Some(index);
            // F978 leaves out a name whose parent's dot is within ten pixels (the sum of the
            // offsets), which would only stack the two.
            let crowded = body
                .parent
                .and_then(|parent| self.dot(parent, &camera, models))
                .is_some_and(|p| (p.x - dot.x).abs() + (p.y - dot.y).abs() < 10)
                && !marked;
            // F975: the name is written at (x + 3, y - 4), and dropped past the view's right
            // and bottom edges.
            if crowded || dot.x + 3 >= 0x13c || dot.y - 4 >= 0x95 {
                continue;
            }
            frame.labels.push(Label {
                x: dot.x + 3,
                y: dot.y - 4,
                text: body.name.clone(),
                marked,
            });
        }
        if let Some(body) = self.selected.and_then(|i| self.bodies.get(i)) {
            frame.details = self.details(body, range);
        }
        frame
    }

    fn details(&self, body: &Body, range: Option<String>) -> Vec<String> {
        let mut lines = vec![format!("{}: {}", body.name, body.kind.label())];
        if let Some(centre) = body.orbit_centre {
            let radius = length(sub(body.position, centre));
            if radius >= 1e10 {
                lines.push(format!("Orbit radius {:.3} A.U.", radius / MM_PER_AU));
            } else if radius > 0.0 {
                lines.push(format!("Orbit radius {:.0} km", radius / 1e6));
            }
        }
        if body.kind != Kind::Station && body.kind != Kind::Starport {
            lines.push(format!("Radius {:.0} km", body.radius as f64 / 1e6));
        }
        if let Some(range) = range {
            lines.push(format!("Range {range}"));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(
        kind: Kind,
        name: &str,
        position: [i64; 3],
        parent: Option<usize>,
        slot: usize,
    ) -> Body {
        Body {
            name: name.to_owned(),
            kind,
            system_type: 0,
            position,
            velocity: [0; 3],
            radius: 70_000_000_000,
            inner_radius: 1_000_000,
            parent,
            model: 0,
            seed: 0,
            mass: 0,
            slot,
            rows: [0; 9],
            class: 0,
            open_air: false,
            orientation: [[0.0; 3]; 3],
            spin: 0,
            bays: 0,
            temperature: 0,
            size_word: 0,
            parent_frame: None,
            orbit_centre: parent.map(|_| [0; 3]),
        }
    }

    fn system() -> SystemMap {
        let au = MM_PER_AU as i64;
        let mut map = SystemMap::new(
            vec![
                body(Kind::Star, "Sun", [0; 3], None, 0x6f),
                body(Kind::Planet, "Near", [au, 0, 0], Some(0), 0x6e),
                body(Kind::Planet, "Far", [0, 0, 5 * au], Some(0), 0x6d),
            ],
            "Test".to_owned(),
            true,
            0.0,
        );
        map.fit();
        map
    }

    fn dot(map: &SystemMap, index: usize) -> Option<Dot> {
        map.dot(index, &map.object_camera(), &|_| None)
    }

    #[test]
    fn the_page_opens_as_the_original_does() {
        // F963's distance, and F982's camera 0x7fff << 26 above the focus: the executed
        // original's DATA_008973 block at the start, looking down the y axis.
        let map = SystemMap::new(Vec::new(), String::new(), true, 0.0);
        assert_eq!(map.zoom, Ffp::new(0x7fff, 0x2a));
        let camera = map.object_camera();
        assert_eq!(camera.position, [0, 0x1ff_fc00_0000, 0]);
        // The original's matrix at the start: rows (0,0,-1), (1,0,0), (0,-1,0).
        assert_eq!(
            camera.matrix.map(|v| v.signum()),
            [0, 0, -1, 1, 0, 0, 0, -1, 0]
        );
    }

    #[test]
    fn the_focus_is_the_object_the_start_type_names() {
        let bodies = || -> Vec<Body> {
            vec![
                body(Kind::Star, "A", [0; 3], None, 0x70),
                body(Kind::Planet, "B", [1, 0, 0], Some(0), 0x6f),
                body(Kind::Planet, "C", [2, 0, 0], Some(0), 0x60),
            ]
        };
        let open = |start: u8, home: bool| {
            SystemMap::open(bodies(), String::new(), home, 0.0, &View::new_game(start)).focus
        };
        assert_eq!(open(1, true), 1, "start types 1 and 4 focus slot 0x6f");
        assert_eq!(open(2, true), 2, "start type 2 focuses slot 0x60");
        assert_eq!(open(3, true), 0, "start type 3 focuses slot 0x70");
        assert_eq!(
            open(1, false),
            0,
            "another system starts at the highest slot"
        );
    }

    #[test]
    fn the_focus_is_the_centre_of_the_view() {
        let map = system();
        let (x, y, _) = map.project([0; 3]).expect("in front of the camera");
        assert_eq!((x.round() as i32, y.round() as i32), (160, 79));
    }

    #[test]
    fn the_farthest_planet_fits_the_view() {
        let map = system();
        let dot = dot(&map, 2).expect("the far planet");
        let reach = (f64::from(dot.x) - 160.0).hypot(f64::from(dot.y) - 79.0);
        assert!((50.0..=80.0).contains(&reach), "{reach}");
    }

    #[test]
    fn the_view_is_a_perspective_one() {
        // Raised halfway to the camera a body is twice as far from the centre: F1479's
        // 256 x / z with z the distance along the view.
        let mut map = system();
        let flat = dot(&map, 2).unwrap();
        map.bodies[2].position[1] = (map.distance() / 2.0) as i64;
        let near = dot(&map, 2).unwrap();
        let (a, b) = (
            (f64::from(flat.x) - 160.0).abs(),
            (f64::from(near.x) - 160.0).abs(),
        );
        assert!((b / a - 2.0).abs() < 0.05, "{a} -> {b}");
    }

    #[test]
    fn a_click_within_eight_pixels_picks_the_nearest_body() {
        let map = system();
        let dot = dot(&map, 1).unwrap();
        assert_eq!(map.pick(dot.x + 2, dot.y + 3), Some(1));
        assert_eq!(map.pick(dot.x + 30, dot.y), None);
    }

    #[test]
    fn an_orbit_is_a_circle_about_its_parent() {
        let map = system();
        let orbit: Vec<(i32, i32)> = map.orbit(&map.bodies[2]).into_iter().flatten().collect();
        assert!(orbit.len() > 100);
        let reach = 5.0 * MM_PER_AU * FOCAL / map.distance();
        for (x, y) in orbit {
            let r = (f64::from(x) - 160.0).hypot(f64::from(y) - 79.0);
            assert!((r - reach).abs() < 1.5, "{r} against {reach}");
        }
        // The star has no parent and so no orbit.
        assert!(map.orbit(&map.bodies[0]).is_empty());
    }

    #[test]
    fn f11_and_f12_step_the_distance_by_the_originals_factors() {
        let mut map = SystemMap::new(Vec::new(), String::new(), true, 0.0);
        let start = map.distance();
        map.zoom_in();
        assert!((map.distance() / start - f64::from(0x7998) / 32768.0).abs() < 1e-3);
        let mut map = SystemMap::new(Vec::new(), String::new(), true, 0.0);
        map.zoom_out();
        assert!((map.distance() / start - f64::from(0x435d) / 16384.0).abs() < 1e-3);
        for _ in 0..2000 {
            map.zoom_out();
        }
        assert_eq!(
            map.zoom,
            Ffp::new(0x7fff, 0x3f),
            "F12 stops at exponent 0x3f"
        );
        map.floor = 40;
        for _ in 0..2000 {
            map.zoom_in();
        }
        assert!(map.zoom.exponent >= 40, "F11 stops at F984's floor");
    }

    #[test]
    fn the_mouse_turns_the_view_about_the_focus() {
        let mut map = system();
        let before = dot(&map, 2).unwrap();
        map.rotate(64.0, 64.0);
        let after = dot(&map, 2).unwrap();
        assert_ne!((before.x, before.y), (after.x, after.y));
        // A whole turn comes back.
        map.rotate(1024.0 - 64.0, 1024.0 - 64.0);
        let again = dot(&map, 2).unwrap();
        assert!((again.x - before.x).abs() <= 1 && (again.y - before.y).abs() <= 1);
    }

    #[test]
    fn the_slider_squares_for_home_and_cubes_for_a_remote_system() {
        // 26 pixels left of the centre: 676 << 18 of a 2^32 day, about 59 minutes.
        let home = slider_step(SLIDER_CENTRE - 26, true);
        assert!((home + 676.0 / 16_384.0 * DAY).abs() < 1e-6, "{home}");
        let minutes = home.abs() / 60.0;
        assert!((minutes - 59.4).abs() < 0.1, "{minutes}");
        let remote = slider_step(SLIDER_CENTRE + 20, false);
        assert!((remote - 8_000.0 / 16_384.0 * DAY).abs() < 1e-6);
        assert!(on_slider(300, 160) && !on_slider(200, 160) && !on_slider(300, 120));
    }

    #[test]
    fn labels_are_written_at_x_plus_3_and_skip_the_edges() {
        let mut map = system();
        let frame = map.frame("1-Jan-3250 12:00:00", None);
        assert_eq!(frame.time_point, "Time point: 1-Jan-3250 12:00:00");
        let sun = frame
            .labels
            .iter()
            .find(|l| l.text == "Sun")
            .expect("the star's name");
        assert_eq!(
            (sun.x, sun.y),
            (163, 75),
            "F975: (x + 3, y - 4) of the dot at (160, 79)"
        );
        map.zoom(1000.0);
        let zoomed = map.frame("x", None);
        assert!(zoomed.labels.iter().all(|l| l.x < 0x13c && l.y < 0x95));
    }

    #[test]
    fn the_picked_body_has_details() {
        let mut map = system();
        map.selected = Some(1);
        let frame = map.frame("x", Some("1.0 A.U.".to_owned()));
        assert_eq!(frame.details[0], "Near: planet");
        assert!(frame
            .details
            .iter()
            .any(|l| l.starts_with("Orbit radius 1.000")));
        assert!(frame.details.iter().any(|l| l.starts_with("Range")));
    }
}
