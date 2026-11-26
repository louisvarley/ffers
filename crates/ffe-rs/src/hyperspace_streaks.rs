//! `enhancement-15`: the streaking hyperspace view.
//!
//! While a jump's transit runs, the original draws F1066's tunnel (model 0xa5) in the view.
//! With this enhancement on, the view shows the stars and dust streaking past instead, over
//! the player's space colour: the ship gathers speed over 2.2 s (the ramp-up sound's rise)
//! while space brightens round a black hole in the middle; through the cruise the hole grows
//! a little and the brightness and the streaks dim; over the last 2.2 s (the ramp-down
//! sound's fall) the streaks slow, and a bright flash brings the new system.
//!
//! The jump starts from the view that was showing: the last flight frame's stars, nebula
//! and bodies stretch into streaks and rush outwards as the speed gathers, with the extra
//! field fading in among them ([`draw`]'s `before`). After the flash the new system's view
//! streaks in the same way, from near the centre out to where it rests ([`arrive`]).
//!
//! The picture is a pure function of the transit's elapsed and whole real seconds
//! ([`crate::navigation::Transit::real_seconds`]): it keeps no state and uses its own fixed
//! hash for the star field, so it never touches the simulation or its random numbers.

use std::f64::consts::TAU;
use std::sync::OnceLock;

/// The shortest transit, in real seconds, the streaks are given to play (a jump the original
/// would finish in a second is stretched to this): room for the ramp-up and ramp-down sounds
/// with the drone between them.
pub const MIN_SECONDS: f64 = 9.0;
/// The longest transit, however far the jump or route.
pub const MAX_SECONDS: f64 = 45.0;
/// Seconds the drive charges after H before the jump starts.
pub const CHARGE_SECONDS: f64 = 5.0;

/// How long, in real seconds, a jump of `distance` (hundredths of a light year) made in `legs`
/// jumps plays: 7 s and 0.35 s a light year, half a second more for each leg after the first,
/// between [`MIN_SECONDS`] and [`MAX_SECONDS`]. The game time the jump takes is unchanged.
pub fn transit_seconds(distance: i32, legs: usize) -> f64 {
    let light_years = f64::from(distance.max(0)) / 100.0;
    (7.0 + 0.35 * light_years + 0.5 * legs.saturating_sub(1) as f64).clamp(MIN_SECONDS, MAX_SECONDS)
}
/// Seconds to gather speed, and to slow down at the end: the sounds' ramps.
pub const RAMP_SECONDS: f64 = 2.2;

/// Stars that streak, and the fainter dust closer to the centre.
const STARS: usize = 1400;
const DUST: usize = 700;
/// The depth of the repeating field and the eye's near limit, in field units.
const DEPTH: f64 = 60.0;
const NEAR: f64 = 0.35;
/// The projection's focal length in logical pixels.
const FOCAL: f64 = 170.0;
/// A logical pixel is 1.2 times as tall as it is wide on a 4:3 display, so heights are
/// scaled by this to keep the field round.
const PIXEL_ASPECT: f64 = 1.0 / 1.2;
/// Field units a second: before the jump gathers speed, at full speed, and as it arrives.
pub const IDLE_SPEED: f64 = 0.6;
pub const CRUISE_SPEED: f64 = 110.0;
pub const ARRIVAL_SPEED: f64 = 2.0;
/// Seconds of travel each streak shows behind its star.
const TRAIL: f64 = 0.1;
/// The arrival flash, in the transit's last seconds.
const FLASH: f64 = 0.3;
/// The flash's peak, as the light added to every pixel.
const FLASH_PEAK: f32 = 1.6;
/// Seconds after the transit the new system's view takes to streak into place, and the
/// part of them the flash takes to clear.
pub const ARRIVAL_SECONDS: f64 = 2.0;
const ARRIVAL_FLASH: f64 = 0.5;
/// Seconds the extra field takes to fade in over the view that was showing.
const FIELD_APPEARS: f64 = 0.6;

/// The speed through the field over a transit: a slow ease in, a cruise and a quick ease out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Profile {
    /// Seconds spent gathering speed.
    pub accelerate: f64,
    /// Seconds spent slowing at the end.
    pub decelerate: f64,
    /// The whole transit.
    pub whole: f64,
}

impl Profile {
    /// The profile for a transit of `whole` real seconds: [`RAMP_SECONDS`] to gather speed
    /// and as long to slow down (each at most half of a shorter transit).
    pub fn new(whole: f64) -> Self {
        let whole = if whole.is_finite() {
            whole.max(0.0)
        } else {
            0.0
        };
        let accelerate = RAMP_SECONDS.min(whole * 0.5);
        let decelerate = RAMP_SECONDS.min(whole * 0.5);
        Self {
            accelerate,
            decelerate,
            whole,
        }
    }

    fn cruise_end(&self) -> f64 {
        self.whole - self.decelerate
    }

    /// Field units a second at `t` seconds into the transit.
    pub fn speed(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, self.whole);
        if t < self.accelerate {
            let s = t / self.accelerate;
            IDLE_SPEED + (CRUISE_SPEED - IDLE_SPEED) * s * s
        } else if t < self.cruise_end() {
            CRUISE_SPEED
        } else if self.decelerate > 0.0 {
            let s = ((t - self.cruise_end()) / self.decelerate).min(1.0);
            ARRIVAL_SPEED + (CRUISE_SPEED - ARRIVAL_SPEED) * (1.0 - s) * (1.0 - s)
        } else {
            ARRIVAL_SPEED
        }
    }

    /// Field units travelled by `t` seconds: the integral of [`Self::speed`].
    pub fn distance(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, self.whole);
        let a = self.accelerate;
        let rise = CRUISE_SPEED - IDLE_SPEED;
        if t < a {
            return IDLE_SPEED * t + rise * t * t * t / (3.0 * a * a);
        }
        let at_cruise = IDLE_SPEED * a + rise * a / 3.0;
        let end = self.cruise_end();
        if t < end {
            return at_cruise + CRUISE_SPEED * (t - a);
        }
        let at_end = at_cruise + CRUISE_SPEED * (end - a);
        let b = self.decelerate;
        if b <= 0.0 {
            return at_end;
        }
        let s = ((t - end) / b).min(1.0);
        let fall = CRUISE_SPEED - ARRIVAL_SPEED;
        at_end + ARRIVAL_SPEED * (t - end) + fall * b * (1.0 - (1.0 - s).powi(3)) / 3.0
    }

    /// How far the speed has risen towards the cruise, 0 to 1.
    fn rush(&self, t: f64) -> f64 {
        ((self.speed(t) - IDLE_SPEED) / (CRUISE_SPEED - IDLE_SPEED)).clamp(0.0, 1.0)
    }

    /// How far through the cruise `t` is, 0 to 1 (1 from the start of the slowing on).
    fn cruised(&self, t: f64) -> f64 {
        let length = self.cruise_end() - self.accelerate;
        if length <= 0.0 {
            return if t >= self.accelerate { 1.0 } else { 0.0 };
        }
        ((t - self.accelerate) / length).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct Particle {
    angle: f64,
    radius: f64,
    depth: f64,
    colour: [f32; 3],
    brightness: f32,
    /// Dust turns slowly round the centre as the ship moves.
    swirl: f64,
}

/// A fixed hash (SplitMix64) for the field: the same every run, separate from the game's.
fn hash(index: u64) -> u64 {
    let mut z = index.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn unit(index: u64) -> f64 {
    (hash(index) >> 11) as f64 / (1u64 << 53) as f64
}

fn particles() -> &'static [Particle] {
    static FIELD: OnceLock<Vec<Particle>> = OnceLock::new();
    FIELD.get_or_init(|| {
        let mut field = Vec::with_capacity(STARS + DUST);
        for index in 0..(STARS + DUST) as u64 {
            let draw = |n: u64| unit(index * 8 + n);
            let star = (index as usize) < STARS;
            let angle = draw(0) * TAU;
            let (radius, colour, brightness, swirl) = if star {
                let pick = draw(3);
                let colour = if pick < 0.7 {
                    [0.80, 0.88, 1.0]
                } else if pick < 0.9 {
                    [1.0, 1.0, 1.0]
                } else {
                    [1.0, 0.78, 0.70]
                };
                (
                    2.0 + 6.0 * draw(1).powf(0.8),
                    colour,
                    (0.55 + 0.6 * draw(4)) as f32,
                    0.0,
                )
            } else {
                (
                    0.8 + 1.6 * draw(1),
                    [0.55, 0.60, 0.70],
                    (0.08 + 0.12 * draw(4)) as f32,
                    0.004,
                )
            };
            // The left of the tunnel is washed pink, as the destination's light.
            let pink = ((-angle.cos()).max(0.0).powi(2) * 0.55) as f32;
            let tint = [1.0, 0.55, 0.62];
            let colour = std::array::from_fn(|c| colour[c] * (1.0 - pink) + tint[c] * pink);
            field.push(Particle {
                angle,
                radius,
                depth: draw(2) * DEPTH,
                colour,
                brightness,
                swirl,
            });
        }
        field
    })
}

/// Add `light` of `colour` at the sub-pixel point (`x`, `y`), shared over its four pixels.
fn splat(light: &mut [[f32; 3]], width: usize, height: usize, x: f64, y: f64, colour: [f32; 3]) {
    let (fx, fy) = (x.floor(), y.floor());
    let (dx, dy) = ((x - fx) as f32, (y - fy) as f32);
    for (ox, oy, weight) in [
        (0, 0, (1.0 - dx) * (1.0 - dy)),
        (1, 0, dx * (1.0 - dy)),
        (0, 1, (1.0 - dx) * dy),
        (1, 1, dx * dy),
    ] {
        let (px, py) = (fx as i64 + ox, fy as i64 + oy);
        if px < 0 || py < 0 || px >= width as i64 || py >= height as i64 {
            continue;
        }
        let cell = &mut light[py as usize * width + px as usize];
        for (value, part) in cell.iter_mut().zip(colour) {
            *value += part * weight;
        }
    }
}

/// The part of the segment from `a` to `b` inside the view (with a pixel's margin), as the
/// start and end fractions along it.
fn clip(a: [f64; 2], b: [f64; 2], width: f64, height: f64) -> Option<(f64, f64)> {
    let (mut low, mut high) = (0.0f64, 1.0f64);
    let d = [b[0] - a[0], b[1] - a[1]];
    for (p, q) in [
        (-d[0], a[0] + 1.0),
        (d[0], width - a[0]),
        (-d[1], a[1] + 1.0),
        (d[1], height - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                low = low.max(r);
            } else {
                high = high.min(r);
            }
        }
    }
    (low <= high).then_some((low, high))
}

fn rgb(pixel: u32) -> [f32; 3] {
    [(pixel >> 16) & 0xff, (pixel >> 8) & 0xff, pixel & 0xff].map(|c| c as f32)
}

/// A view `source` (`width` by `height`) as seen at the point (`x`, `y`) when it is zoomed
/// by `zoom` about the centre and smeared by `smear`: each bright point becomes the head of
/// a trail running `smear` of its distance in towards the centre, fading along it, as the
/// field's streaks do. Light below `fill` (the space colour) does not smear, and points
/// beyond the view's edge are `fill`.
#[allow(clippy::too_many_arguments)]
fn streaked(
    source: &[u32],
    width: usize,
    height: usize,
    x: f64,
    y: f64,
    zoom: f64,
    smear: f64,
    fill: [f32; 3],
) -> [f32; 3] {
    let (cx, cy) = (width as f64 / 2.0, height as f64 / 2.0);
    let (dx, dy) = (x - cx, y - cy);
    let sample = |s: f64| {
        let scale = (1.0 + smear * s) / zoom;
        let (sx, sy) = (cx + dx * scale, cy + dy * scale);
        if sx < 0.0 || sy < 0.0 || sx >= width as f64 || sy >= height as f64 {
            fill
        } else {
            rgb(source[sy as usize * width + sx as usize])
        }
    };
    let mut out = sample(0.0);
    // One sample for every source pixel the trail crosses. A negative smear runs the trails
    // outwards instead (the arrival's, which is the departure reversed).
    let reach = (dx * dx + dy * dy).sqrt() * smear.abs() / zoom;
    let steps = ((reach * 1.5).ceil() as usize).min(96);
    for step in 1..=steps {
        let s = step as f64 / steps as f64;
        let weight = 1.0 - 0.8 * s as f32;
        let value = sample(s);
        for c in 0..3 {
            out[c] = out[c].max(fill[c] + (value[c] - fill[c]).max(0.0) * weight);
        }
    }
    out
}

/// Blend the flash's `light` over a colour.
fn flashed(colour: f32, light: f32) -> u32 {
    let lit = 1.0 - (-light * 1.3).exp();
    (colour + (255.0 - colour) * lit).clamp(0.0, 255.0) as u32
}

/// After the transit: the departure in reverse. The new system's view in the top `height`
/// rows of `frame`, already drawn, fades in over [`ARRIVAL_SECONDS`] (`since` the arrival)
/// as its stars and nebula draw in from the edges with their trails, shrinking to rest,
/// while the field the jump flew through fades out and the arrival flash clears. `whole` is
/// the transit's length, so the field carries on from where its last frame left it. Nothing
/// changes after that.
pub fn arrive(frame: &mut [u32], width: usize, height: usize, since: f64, whole: f64, space: u32) {
    let height = height.min(frame.len() / width.max(1));
    if width == 0 || height == 0 || !(0.0..ARRIVAL_SECONDS).contains(&since) {
        return;
    }
    let u = since / ARRIVAL_SECONDS;
    let left = (1.0 - u) * (1.0 - u);
    let keep = (u * u * (3.0 - 2.0 * u)) as f32;
    // Magnified at first, so every point it shows is inside the view, settling to 1; the
    // trails run outwards behind the inward drift and shrink away.
    let zoom = left.exp();
    let smear = -0.5 * left;
    let flash = FLASH_PEAK * (1.0 - (since / ARRIVAL_FLASH).min(1.0)).powi(2) as f32;
    let profile = Profile::new(if whole.is_finite() { whole } else { 0.0 });
    let mut light = vec![[0.0f32; 3]; width * height];
    field_light(
        &mut light,
        width,
        height,
        profile.distance(profile.whole) + ARRIVAL_SPEED * since,
        ARRIVAL_SPEED * TRAIL,
        // The cruise's dimming, then gone as the view takes over.
        0.55 * (1.0 - keep),
    );
    let fill = rgb(space);
    let source = frame[..width * height].to_vec();
    for y in 0..height {
        for x in 0..width {
            let value = streaked(
                &source,
                width,
                height,
                x as f64 + 0.5,
                y as f64 + 0.5,
                zoom,
                smear,
                fill,
            );
            let cell = light[y * width + x];
            let pixel = std::array::from_fn::<u32, 3, _>(|c| {
                flashed(fill[c] + (value[c] - fill[c]) * keep, cell[c] + flash)
            });
            frame[y * width + x] = pixel[0] << 16 | pixel[1] << 8 | pixel[2];
        }
    }
}

/// Add the streaking field's light to `light` (`width` by `height`) with the ship
/// `travelled` field units in and trails `trail` units long, scaled by `scale`.
fn field_light(
    light: &mut [[f32; 3]],
    width: usize,
    height: usize,
    travelled: f64,
    trail: f64,
    scale: f32,
) {
    if scale <= 0.0 {
        return;
    }
    let (cx, cy) = (width as f64 / 2.0, height as f64 / 2.0);
    for particle in particles() {
        let head = NEAR + (particle.depth - travelled).rem_euclid(DEPTH);
        let tail = head + trail;
        // Stars fade in from the far end of the field rather than appearing.
        let emerge = ((NEAR + DEPTH - head) / (DEPTH * 0.35)).clamp(0.0, 1.0);
        let near = 0.06 + 1.1 * (1.0 - head / (NEAR + DEPTH)).powf(2.5);
        let intensity = particle.brightness * (emerge * near) as f32;
        if intensity <= 0.0 {
            continue;
        }
        let angle = particle.angle + particle.swirl * travelled;
        let (sin, cos) = angle.sin_cos();
        let project = |depth: f64| {
            let scale = particle.radius * FOCAL / depth;
            [cx + cos * scale, cy + sin * scale * PIXEL_ASPECT]
        };
        let (a, b) = (project(tail), project(head));
        let colour = particle.colour.map(|c| c * intensity * scale);
        let Some((low, high)) = clip(a, b, width as f64, height as f64) else {
            continue;
        };
        let length = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let steps = ((length * (high - low) * 1.5).ceil() as usize).clamp(1, 2000);
        for step in 0..=steps {
            let along = low + (high - low) * step as f64 / steps as f64;
            // The tail fades out behind the bright head.
            let fade = (along * along) as f32;
            let point = [a[0] + (b[0] - a[0]) * along, a[1] + (b[1] - a[1]) * along];
            splat(
                light,
                width,
                height,
                point[0],
                point[1],
                colour.map(|c| c * fade),
            );
        }
        if high >= 1.0 {
            splat(light, width, height, b[0], b[1], colour.map(|c| c * 0.8));
        }
    }
}

/// The countdown's text for `seconds`: `XXX.XXX`, never below zero.
pub fn countdown_text(seconds: f64) -> String {
    let seconds = if seconds.is_finite() {
        seconds.clamp(0.0, 999.999)
    } else {
        0.0
    };
    format!("{seconds:07.3}")
}

/// Light the `ink` pixels (the countdown's text, drawn in the game's font) glowing red in
/// `frame` (`width` by `height`, 0x00RRGGBB), in the manner of Elite Dangerous's jump
/// countdown: a soft red halo, then the pixels themselves.
pub fn glow_ink(frame: &mut [u32], width: usize, height: usize, ink: &[(i32, i32)]) {
    let height = height.min(frame.len() / width.max(1));
    let (w, h) = (width as i32, height as i32);
    let mut glow = vec![0.0f32; width * height];
    for &(px, py) in ink {
        for oy in -2..=2 {
            for ox in -2..=2 {
                let (gx, gy) = (px + ox, py + oy);
                if gx < 0 || gy < 0 || gx >= w || gy >= h {
                    continue;
                }
                let d2 = (ox * ox + oy * oy) as f32;
                glow[(gy * w + gx) as usize] += (-d2 / 1.6).exp() * 0.22;
            }
        }
    }
    let add = |pixel: u32, rgb: [f32; 3]| -> u32 {
        let base = self::rgb(pixel);
        let out = std::array::from_fn::<u32, 3, _>(|c| (base[c] + rgb[c]).clamp(0.0, 255.0) as u32);
        out[0] << 16 | out[1] << 8 | out[2]
    };
    for (index, amount) in glow.iter().enumerate() {
        if *amount > 0.0 {
            let a = amount.min(1.0);
            frame[index] = add(frame[index], [200.0 * a, 28.0 * a, 16.0 * a]);
        }
    }
    for &(px, py) in ink {
        if px >= 0 && py >= 0 && px < w && py < h {
            let index = (py * w + px) as usize;
            frame[index] = add(frame[index], [255.0, 70.0, 45.0]);
        }
    }
}

/// Draw the streaking view for `elapsed` of `whole` real seconds of transit into the top
/// `height` rows of `frame` (`width` wide, 0x00RRGGBB pixels), over `space` (the player's
/// space colour, 0x00RRGGBB). `before` is the view that was showing as the jump began (the
/// same size), whose stars and nebula streak away as the speed gathers.
pub fn draw(
    frame: &mut [u32],
    width: usize,
    height: usize,
    elapsed: f64,
    whole: f64,
    space: u32,
    before: Option<&[u32]>,
) {
    let height = height.min(frame.len() / width.max(1));
    if width == 0 || height == 0 {
        return;
    }
    let before = before.filter(|view| view.len() >= width * height);
    let profile = Profile::new(whole);
    let t = if elapsed.is_finite() {
        elapsed.clamp(0.0, profile.whole)
    } else {
        0.0
    };
    let travelled = profile.distance(t);
    let trail = profile.speed(t) * TRAIL;
    let rush = profile.rush(t) as f32;
    // Through the cruise the brightness and the streaks dim to about half.
    let dim = 1.0 - 0.45 * profile.cruised(t) as f32;
    let (cx, cy) = (width as f64 / 2.0, height as f64 / 2.0);
    // The black hole in the middle opens with the speed and grows a little in the cruise.
    let hole = f64::from(rush) * (8.0 + 7.0 * profile.cruised(t));
    let space = [(space >> 16) & 0xff, (space >> 8) & 0xff, space & 0xff].map(|c| c as f32);
    // Space at full speed: the space colour lifted a little towards blue (the owner found
    // the first, lighter blue too strong).
    let bright = space.map(|c| c * 1.6);
    let bright =
        std::array::from_fn::<f32, 3, _>(|c| (bright[c] + [14.0, 24.0, 44.0][c]).min(255.0));
    let mut light = vec![[0.0f32; 3]; width * height];
    // The view that was showing stays while the speed gathers, its light stretching into
    // streaks that rush outwards (the same smear as the field's trails), and has gone once
    // the cruise begins; the extra field fades in among it.
    let (keep, appear) = match before {
        Some(_) if profile.accelerate > 0.0 => {
            let s = (t / profile.accelerate).clamp(0.0, 1.0);
            (
                (1.0 - s * s * (3.0 - 2.0 * s)) as f32,
                (t / FIELD_APPEARS).min(1.0) as f32,
            )
        }
        _ => (0.0, 1.0),
    };
    let before_zoom = (travelled / 60.0).exp();
    let before_smear = 0.7 * f64::from(rush);

    field_light(&mut light, width, height, travelled, trail, dim * appear);

    // The glow of the tunnel at speed: pink light to the left, a cool wash to the right and
    // a ring of dust cloud round the dark centre.
    let glow = |x: f64, y: f64, gx: f64, gy: f64, size: f64| {
        let d2 = ((x - gx) / size).powi(2) + ((y - gy) / size).powi(2);
        (-d2).exp() as f32
    };
    let flash = if profile.whole > 0.0 {
        let s = ((t - (profile.whole - FLASH)) / FLASH).clamp(0.0, 1.0) as f32;
        s * s * FLASH_PEAK
    } else {
        0.0
    };
    let rotation = t * 0.9;
    let lift = rush * dim;
    for y in 0..height {
        for x in 0..width {
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            let mut cell = light[y * width + x];
            let (dx, dy) = (fx - cx, (fy - cy) / PIXEL_ASPECT);
            let r = (dx * dx + dy * dy).sqrt();
            // 0 in the black hole, 1 clear of it.
            let open = if hole > 0.5 {
                let s = ((r - hole * 0.55) / (hole * 1.1)).clamp(0.0, 1.0);
                (s * s * (3.0 - 2.0 * s)) as f32
            } else {
                1.0
            };
            // Space brightens with the speed, a little more towards the edges.
            let edge = (0.8 + 0.2 * (r / (width as f64 * 0.5)).min(1.0)) as f32;
            let mut base = std::array::from_fn::<f32, 3, _>(|c| {
                space[c] + (bright[c] - space[c]) * lift * edge
            });
            if let Some(view) = before.filter(|_| keep > 0.001) {
                let old = streaked(
                    view,
                    width,
                    height,
                    fx,
                    fy,
                    before_zoom,
                    before_smear,
                    space,
                );
                for c in 0..3 {
                    base[c] += (old[c] - base[c]) * keep;
                }
            }
            let sky = base.map(|c| c * (1.0 - rush + rush * open));
            if rush > 0.0 {
                let pink = glow(fx, fy, -20.0, height as f64 * 0.2, 150.0) * 0.22 * lift;
                let a = dy.atan2(dx);
                let cloud = 0.55
                    + 0.25 * (3.0 * a + rotation).sin()
                    + 0.2 * (7.0 * a - 1.9 * rotation).sin();
                let ring_r = hole * 1.9 + 6.0;
                let ring =
                    ((-((r - ring_r) / (hole + 8.0)).powi(2)).exp() * cloud) as f32 * 0.35 * lift;
                for (c, value) in cell.iter_mut().enumerate() {
                    *value += pink * [1.0, 0.45, 0.5][c] + ring * [0.55, 0.6, 0.68][c];
                    // Streaks crossing the hole are swallowed by it.
                    *value *= 0.25 + 0.75 * open;
                }
            }
            let pixel = std::array::from_fn::<u32, 3, _>(|c| flashed(sky[c], cell[c] + flash));
            frame[y * width + x] = pixel[0] << 16 | pixel[1] << 8 | pixel[2];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = 320;
    const H: usize = 158;

    fn light(elapsed: f64, whole: f64) -> u64 {
        let mut frame = vec![0u32; W * H];
        draw(&mut frame, W, H, elapsed, whole, 0x00_00_10, None);
        frame
            .iter()
            .map(|p| u64::from((p >> 16) & 0xff) + u64::from((p >> 8) & 0xff) + u64::from(p & 0xff))
            .sum()
    }

    #[test]
    fn the_speed_gathers_slowly_cruises_and_falls_quickly() {
        let profile = Profile::new(12.0);
        // The ramps last as long as the sounds' rise and fall.
        assert_eq!(profile.accelerate, RAMP_SECONDS);
        assert_eq!(profile.decelerate, RAMP_SECONDS);
        assert_eq!(profile.speed(0.0), IDLE_SPEED);
        // A fifth of the way through gathering speed it is still slow.
        assert!(profile.speed(0.45) < CRUISE_SPEED * 0.1);
        assert_eq!(profile.speed(5.0), CRUISE_SPEED);
        assert!((profile.speed(12.0) - ARRIVAL_SPEED).abs() < 1e-9);
        // Half way through slowing down, most of the speed has gone.
        assert!(profile.speed(12.0 - 1.1) < CRUISE_SPEED * 0.3);
        // A short transit splits itself between the two ramps.
        let short = Profile::new(3.0);
        assert_eq!((short.accelerate, short.decelerate), (1.5, 1.5));
    }

    #[test]
    fn the_distance_is_the_speed_integrated_and_never_goes_back() {
        for whole in [0.0, 0.4, 1.0, 3.0, 6.0, 20.0] {
            let profile = Profile::new(whole);
            assert!(profile.accelerate + profile.decelerate <= whole + 1e-12);
            let steps = 4000;
            let dt = whole / steps as f64;
            let mut sum = 0.0;
            let mut last = 0.0;
            for step in 0..steps {
                let t = step as f64 * dt;
                sum += profile.speed(t + dt / 2.0) * dt;
                let now = profile.distance(t + dt);
                assert!(now >= last, "{whole}: distance fell at {t}");
                last = now;
            }
            let end = profile.distance(whole);
            assert!(
                (sum - end).abs() <= 1e-3 * end.max(1.0),
                "{whole}: integrated {sum} against {end}"
            );
        }
    }

    #[test]
    fn the_view_is_dark_dots_at_the_start_and_bright_streaks_at_speed() {
        let start = light(0.0, 12.0);
        let cruise = light(3.0, 12.0);
        assert!(cruise > start * 4, "start {start}, cruise {cruise}");
        // The cruise dims towards its end.
        assert!(light(9.5, 12.0) < cruise, "{} {cruise}", light(9.5, 12.0));
        // The arrival flash lights the last frame.
        assert!(light(12.0, 12.0) > light(12.0 - 0.6, 12.0));
    }

    #[test]
    fn the_same_moment_draws_the_same_picture() {
        let mut a = vec![0u32; W * H];
        let mut b = vec![7u32; W * H];
        draw(&mut a, W, H, 3.3, 7.0, 0x10, None);
        draw(&mut b, W, H, 3.3, 7.0, 0x10, None);
        assert_eq!(a, b);
    }

    /// A dark view with one bright star off the centre.
    fn one_star() -> (Vec<u32>, usize) {
        let mut view = vec![0x00_00_10u32; W * H];
        let at = 40 * W + 60;
        view[at] = 0xff_ff_ff;
        (view, at)
    }

    #[test]
    fn the_jump_starts_from_the_view_that_was_showing() {
        let (view, at) = one_star();
        let mut frame = vec![0u32; W * H];
        draw(&mut frame, W, H, 0.0, 12.0, 0x00_00_10, Some(&view));
        // The first frame is the view itself: no extra field yet, no smear.
        assert_eq!(frame, view);
        // Gathering speed, the star has stretched into a streak of several pixels.
        draw(&mut frame, W, H, 1.0, 12.0, 0x00_00_10, Some(&view));
        let lit = frame.iter().filter(|p| (*p & 0xff) > 0x60).count();
        assert!(lit > 3, "{lit} lit pixels");
        assert_ne!(frame[at], view[at]);
    }

    #[test]
    fn a_longer_jump_plays_longer() {
        assert_eq!(transit_seconds(100, 1), MIN_SECONDS);
        assert!(transit_seconds(2000, 1) > transit_seconds(1000, 1));
        assert_eq!(transit_seconds(100_000, 1), MAX_SECONDS);
    }

    #[test]
    fn the_countdown_reads_seconds_to_the_thousandth() {
        assert_eq!(countdown_text(5.0), "005.000");
        assert_eq!(countdown_text(12.3456), "012.346");
        assert_eq!(countdown_text(-1.0), "000.000");
        let mut frame = vec![0u32; W * H];
        glow_ink(&mut frame, W, H, &[(100, 100), (101, 100)]);
        assert!(frame.iter().any(|p| (p >> 16) == 0xff));
        // Off the frame it draws nothing and does not panic.
        glow_ink(&mut frame, W, H, &[(-400, 500), (0, -1)]);
    }

    #[test]
    fn the_new_view_streaks_in_and_then_rests() {
        let (view, _) = one_star();
        let mut frame = view.clone();
        arrive(&mut frame, W, H, 0.0, 12.0, 0x00_00_10);
        // The flash is up as the transit's own last frame left it.
        assert!(frame.iter().all(|p| (p & 0xff) > 0xc0));
        let mut frame = view.clone();
        arrive(&mut frame, W, H, ARRIVAL_SECONDS * 0.6, 12.0, 0x00_00_10);
        let lit = frame.iter().filter(|p| (*p & 0xff) > 0x60).count();
        assert!(lit > 1, "{lit} lit pixels");
        let mut frame = view.clone();
        arrive(&mut frame, W, H, ARRIVAL_SECONDS, 12.0, 0x00_00_10);
        assert_eq!(frame, view);
    }

    #[test]
    fn odd_times_and_sizes_do_not_panic() {
        let mut frame = vec![0u32; W * H];
        for (elapsed, whole) in [
            (0.0, 0.0),
            (-1.0, 5.0),
            (9.0, 5.0),
            (f64::NAN, 5.0),
            (1.0, f64::INFINITY),
        ] {
            draw(&mut frame, W, H, elapsed, whole, 0, None);
            arrive(&mut frame, W, H, elapsed, whole, 0);
        }
        let short = vec![0u32; 10];
        draw(&mut frame, W, H, 1.0, 6.0, 0, Some(&short));
        draw(&mut frame, W, 10_000, 1.0, 6.0, 0, None);
        draw(&mut [], 0, 0, 1.0, 6.0, 0, None);
        arrive(&mut [], 0, 0, 0.1, 6.0, 0);
    }
}
