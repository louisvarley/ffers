//! `enhancement-15`: the streaking hyperspace view.
//!
//! While a jump's transit runs, the original draws F1066's tunnel (model 0xa5) in the view.
//! With this enhancement on, the view shows the stars and dust streaking past instead: the
//! ship gathers speed slowly, cruises with the stars drawn out into long lines round a dark
//! centre and a swirl of dust, then slows quickly as it reaches the new system, ending on a
//! short flash.
//!
//! The picture is a pure function of the transit's elapsed and whole real seconds
//! ([`crate::navigation::Transit::real_seconds`]): it keeps no state and uses its own fixed
//! hash for the star field, so it never touches the simulation or its random numbers.

use std::f64::consts::TAU;
use std::sync::OnceLock;

/// The shortest transit, in real seconds, the streaks are given to play (a jump the original
/// would finish in a second is stretched to this).
pub const MIN_SECONDS: f64 = 6.0;

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
/// The background, near black with a little blue.
const BACKGROUND: [f32; 3] = [1.0, 2.0, 6.0];

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
    /// The profile for a transit of `whole` real seconds: half of it (at most four seconds)
    /// to gather speed, a fifth of it (half a second to 1.2 s) to slow down.
    pub fn new(whole: f64) -> Self {
        let whole = if whole.is_finite() {
            whole.max(0.0)
        } else {
            0.0
        };
        let mut accelerate = (whole * 0.5).min(4.0);
        let mut decelerate = (whole * 0.2).clamp(0.5, 1.2);
        let both = accelerate + decelerate;
        if both > whole {
            let scale = if both > 0.0 { whole / both } else { 0.0 };
            accelerate *= scale;
            decelerate *= scale;
        }
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

/// Draw the streaking view for `elapsed` of `whole` real seconds of transit into the top
/// `height` rows of `frame` (`width` wide, 0x00RRGGBB pixels).
pub fn draw(frame: &mut [u32], width: usize, height: usize, elapsed: f64, whole: f64) {
    let height = height.min(frame.len() / width.max(1));
    if width == 0 || height == 0 {
        return;
    }
    let profile = Profile::new(whole);
    let t = if elapsed.is_finite() {
        elapsed.clamp(0.0, profile.whole)
    } else {
        0.0
    };
    let travelled = profile.distance(t);
    let trail = profile.speed(t) * TRAIL;
    let rush = profile.rush(t) as f32;
    let (cx, cy) = (width as f64 / 2.0, height as f64 / 2.0);
    let mut light = vec![[0.0f32; 3]; width * height];

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
        let colour = particle.colour.map(|c| c * intensity);
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
                &mut light,
                width,
                height,
                point[0],
                point[1],
                colour.map(|c| c * fade),
            );
        }
        if high >= 1.0 {
            splat(
                &mut light,
                width,
                height,
                b[0],
                b[1],
                colour.map(|c| c * 0.8),
            );
        }
    }

    // The glow of the tunnel at speed: pink light to the left, a cool wash to the right and
    // a ring of dust cloud round the dark centre.
    let glow = |x: f64, y: f64, gx: f64, gy: f64, size: f64| {
        let d2 = ((x - gx) / size).powi(2) + ((y - gy) / size).powi(2);
        (-d2).exp() as f32
    };
    let flash = if profile.whole > 0.0 {
        let s = ((t - (profile.whole - FLASH)) / FLASH).clamp(0.0, 1.0) as f32;
        s * s * 1.6
    } else {
        0.0
    };
    let rotation = t * 0.9;
    for y in 0..height {
        for x in 0..width {
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            let mut cell = light[y * width + x];
            if rush > 0.0 {
                let pink = glow(fx, fy, -20.0, height as f64 * 0.2, 150.0) * 0.35 * rush;
                let cool =
                    glow(fx, fy, width as f64 + 20.0, height as f64 * 0.5, 120.0) * 0.12 * rush;
                let (dx, dy) = (fx - cx, (fy - cy) / PIXEL_ASPECT);
                let r = (dx * dx + dy * dy).sqrt();
                let a = dy.atan2(dx);
                let cloud = 0.55
                    + 0.25 * (3.0 * a + rotation).sin()
                    + 0.2 * (7.0 * a - 1.9 * rotation).sin();
                let ring = ((-((r - 30.0) / 16.0).powi(2)).exp() * cloud) as f32 * 0.4 * rush;
                for (c, value) in cell.iter_mut().enumerate() {
                    *value += pink * [1.0, 0.45, 0.5][c]
                        + cool * [0.6, 0.7, 0.9][c]
                        + ring * [0.55, 0.6, 0.68][c];
                }
            }
            let pixel = std::array::from_fn::<u32, 3, _>(|c| {
                let lit = 1.0 - (-(cell[c] + flash) * 1.3).exp();
                (BACKGROUND[c] + (255.0 - BACKGROUND[c]) * lit).clamp(0.0, 255.0) as u32
            });
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
        draw(&mut frame, W, H, elapsed, whole);
        frame
            .iter()
            .map(|p| u64::from((p >> 16) & 0xff) + u64::from((p >> 8) & 0xff) + u64::from(p & 0xff))
            .sum()
    }

    #[test]
    fn the_speed_gathers_slowly_cruises_and_falls_quickly() {
        let profile = Profile::new(8.0);
        assert_eq!(profile.accelerate, 4.0);
        assert!((profile.decelerate - 1.2).abs() < 1e-12);
        assert_eq!(profile.speed(0.0), IDLE_SPEED);
        // A quarter of the way through gathering speed it is still slow.
        assert!(profile.speed(1.0) < CRUISE_SPEED * 0.1);
        assert_eq!(profile.speed(5.0), CRUISE_SPEED);
        assert!((profile.speed(8.0) - ARRIVAL_SPEED).abs() < 1e-9);
        // Slowing takes far less time than speeding up.
        assert!(profile.decelerate < profile.accelerate / 2.0);
        // Half way through slowing down, most of the speed has gone.
        assert!(profile.speed(8.0 - 0.6) < CRUISE_SPEED * 0.3);
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
        let start = light(0.0, 8.0);
        let cruise = light(5.0, 8.0);
        assert!(cruise > start * 4, "start {start}, cruise {cruise}");
        // The arrival flash lights the last frame.
        assert!(light(8.0, 8.0) > light(8.0 - 0.6, 8.0));
    }

    #[test]
    fn the_same_moment_draws_the_same_picture() {
        let mut a = vec![0u32; W * H];
        let mut b = vec![7u32; W * H];
        draw(&mut a, W, H, 3.3, 7.0);
        draw(&mut b, W, H, 3.3, 7.0);
        assert_eq!(a, b);
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
            draw(&mut frame, W, H, elapsed, whole);
        }
        draw(&mut frame, W, 10_000, 1.0, 6.0);
        draw(&mut [], 0, 0, 1.0, 6.0);
    }
}
