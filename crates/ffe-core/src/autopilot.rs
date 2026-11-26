//! Source autopilot arithmetic: F587, F589, F592/F612 and F597.
//! Positions are system millimetres; velocities retain F1511's native units.
use crate::model::multiply;
use crate::model_animation::vector_msb;
use crate::model_math::table_root;
use crate::physics::Motion;
use crate::thrust::{clamp, Limits};

/// F1020: target present, state below 0x28, and object +0xca bit 0x20.
pub fn can_engage(mode: u8, has_target: bool, equipment: u32) -> bool {
    mode < 0x28 && has_target && equipment & (1 << 21) != 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guidance {
    pub thrust: [i16; 3],
    pub cap: [i32; 3],
    pub turns: [i16; 3],
    pub distance_shift: i32,
}

/// F612: normalise by the OR of the absolute components, not Euclidean length.
pub fn normalise_velocity(vector: &mut [i32; 3]) -> i32 {
    let bits = vector.iter().fold(0u32, |n, v| n | v.wrapping_abs() as u32);
    let shift = 31u32.saturating_sub(bits.leading_zeros()) as i32 - 14;
    for value in vector {
        *value = if shift < 0 {
            value.wrapping_shl((-shift) as u32)
        } else {
            *value >> shift
        };
    }
    shift
}

/// F587, including its half-turn escape when the target is behind the ship.
pub fn face_target(matrix: [[i32; 3]; 3], target: [i16; 3]) -> [i16; 3] {
    let dot = |row: [i32; 3]| {
        row.into_iter().zip(target).fold(0i32, |sum, (a, b)| {
            sum.wrapping_add((a >> 16).wrapping_mul(i32::from(b)))
        })
    };
    let pitch = (dot(matrix[1]).wrapping_mul(2).wrapping_neg() >> 16) as i16;
    let yaw = (dot(matrix[0]).wrapping_mul(2) >> 16) as i16;
    if dot(matrix[2]) < 0 {
        [
            if pitch < 0 { -0x4000 } else { 0x4000 },
            0,
            if yaw < 0 { -0x4000 } else { 0x4000 },
        ]
    } else {
        [pitch, 0, yaw]
    }
}

/// F589: equalise roll, or all three axes. Both matrices must share a frame.
pub fn match_orientation(matrix: [[i32; 3]; 3], target: [[i32; 3]; 3], all: bool) -> [i16; 3] {
    let dot = |a: [i32; 3], b: [i32; 3]| {
        a.into_iter().zip(b).fold(0i32, |sum, (a, b)| {
            sum.wrapping_add((a >> 16).wrapping_mul(b >> 16))
        })
    };
    let roll = (dot(matrix[0], target[1]).wrapping_neg() >> 16) as i16;
    if all {
        [
            (dot(matrix[1], target[2]) >> 16) as i16,
            roll,
            (dot(matrix[2], target[0]) >> 16) as i16,
        ]
    } else {
        [0, roll, 0]
    }
}

/// F597's ordinary guidance branch. Returns local thrust request and world caps.
/// The accelerated near-offset frame-match branch belongs to the world adapter.
pub fn request(
    matrix: [[i32; 3]; 3],
    mut velocity: [i32; 3],
    mut velocity_shift: i32,
    mut position: [i32; 3],
    distance_shift: i32,
    modifier: i32,
) -> ([i32; 3], [i32; 3]) {
    let mut scale = distance_shift.wrapping_add(modifier).wrapping_sub(8);
    if scale >= 0 {
        if scale & 1 != 0 {
            scale = (scale >> 1).wrapping_add(1);
            position = position.map(|v| v >> 1);
        } else {
            scale >>= 1;
        }
        let sum = position
            .iter()
            .fold(0i32, |s, v| s.wrapping_add(v.wrapping_mul(*v)));
        let length = table_root(sum as u32);
        let root = table_root(length.wrapping_shl(16));
        if root != 0 {
            position = position.map(|v| v.wrapping_shl(15) / root as i32);
        }
    }
    let shift = velocity_shift.wrapping_sub(scale.wrapping_sub(2));
    if shift >= 0 {
        velocity = std::array::from_fn(|k| velocity[k].wrapping_add(position[k] >> (shift & 31)));
    } else {
        velocity =
            std::array::from_fn(|k| (velocity[k] >> ((-shift) & 31)).wrapping_add(position[k]));
        velocity_shift = velocity_shift.wrapping_sub(shift);
    }
    if velocity_shift > 12 {
        velocity_shift = 12;
    }
    let cap = if velocity_shift > 0 {
        velocity.map(|v| v.wrapping_shl(velocity_shift as u32))
    } else {
        velocity
    };
    let local: [i32; 3] = matrix.map(|row| {
        row.into_iter()
            .zip(velocity)
            .fold(0i32, |sum, (a, b)| sum.wrapping_add(multiply(a, b)))
    });
    let scale = velocity_shift.wrapping_add(2);
    let local = if scale < 0 {
        local.map(|v| v >> ((-scale) & 31))
    } else {
        local.map(|v| v.wrapping_shl(scale.min(2) as u32))
    };
    (local, cap)
}

/// F592/F596 followed by F597/F599 and F587. No direct orientation changes,
/// keyboard-speed emulation or long-distance time/position transfers.
pub fn guide(
    motion: &Motion,
    position: [i64; 3],
    target: [i64; 3],
    target_velocity: [i32; 3],
    limits: &Limits,
    modifier: i32,
) -> Guidance {
    guide_to_radius(
        motion,
        position,
        target,
        target_velocity,
        limits,
        modifier,
        0,
    )
}

/// F610's large-body radius subtraction before the ordinary F597 branch.
pub fn guide_to_radius(
    motion: &Motion,
    position: [i64; 3],
    target: [i64; 3],
    target_velocity: [i32; 3],
    limits: &Limits,
    modifier: i32,
    radius: i64,
) -> Guidance {
    guide_detail(
        motion,
        position,
        target,
        target_velocity,
        limits,
        modifier,
        radius,
    )
    .guidance
}

/// What F610 leaves for F753: the guidance F597 stored plus the direction words F610 writes at
/// `+0x108..+0x10c` and the local request F598 and F599 read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detail {
    pub guidance: Guidance,
    pub direction: [i32; 3],
    pub local: [i32; 3],
    /// F612's shift of the relative velocity (F597's `+0x1c`).
    pub velocity_shift: i32,
}

/// F597's accelerated-branch test: `distance - max(2, velocity) - (t + t / 4)`, where `t` is
/// `DATA_009013` (0 at 1x, 3, 7, 10 and 12 up to 10000x). `None` at 1x or when the result is
/// over 8 (the ordinary guidance); otherwise the result: at 8 or less F597 clears the ship's
/// frame timer, and below 6 (with a waypoint set) it places the ship at its aim point.
pub fn accelerated_reach(distance_shift: i32, velocity_shift: i32, time_shift: i32) -> Option<i32> {
    if time_shift == 0 {
        return None;
    }
    let reach = distance_shift - velocity_shift.max(2) - (time_shift + (time_shift >> 2));
    (reach <= 8).then_some(reach)
}

/// F588: the turn away from the direction words, the mirror of [`face_target`]. A target in front
/// (`dot >= 0`) turns each rate to `(rate >> 8) ^ 0x3fff`, a hard turn.
pub fn face_away(matrix: [[i32; 3]; 3], target: [i16; 3]) -> [i16; 3] {
    let dot = |row: [i32; 3]| {
        row.into_iter().zip(target).fold(0i32, |sum, (a, b)| {
            sum.wrapping_add((a >> 16).wrapping_mul(i32::from(b)))
        })
    };
    let pitch = (dot(matrix[1]).wrapping_mul(2) >> 16) as i16;
    let yaw = (dot(matrix[0]).wrapping_mul(2).wrapping_neg() >> 16) as i16;
    if dot(matrix[2]) < 0 {
        [pitch, 0, yaw]
    } else {
        [(pitch >> 8) ^ 0x3fff, 0, (yaw >> 8) ^ 0x3fff]
    }
}

/// [`guide_to_radius`] with the intermediate vectors.
pub fn guide_detail(
    motion: &Motion,
    position: [i64; 3],
    target: [i64; 3],
    target_velocity: [i32; 3],
    limits: &Limits,
    modifier: i32,
    radius: i64,
) -> Detail {
    let relative: [i64; 3] = std::array::from_fn(|k| target[k].wrapping_sub(position[k]));
    let distance_shift = (vector_msb(relative) as i32 - 13).max(0);
    let mut direction = relative.map(|v| (v >> distance_shift) as i32);
    if radius >= 100_000_000 {
        let normal = crate::physics::normalise(direction).map(|v| v >> 16);
        let radius = (radius >> distance_shift) as i32;
        direction = std::array::from_fn(|k| {
            direction[k].wrapping_sub(multiply(normal[k].wrapping_shl(16), radius))
        });
    }
    let mut velocity = std::array::from_fn(|k| target_velocity[k].wrapping_sub(motion.velocity[k]));
    let velocity_shift = normalise_velocity(&mut velocity);
    let (request, cap) = request(
        motion.orientation,
        velocity,
        velocity_shift,
        direction,
        distance_shift,
        modifier,
    );
    Detail {
        guidance: Guidance {
            thrust: clamp(limits, request).words.map(|w| w as i16),
            cap,
            turns: face_target(motion.orientation, direction.map(|v| v as i16)),
            distance_shift,
        },
        direction,
        local: request,
        velocity_shift,
    }
}

/// Millimetres a velocity unit covers in one second (F1511's `v * t / 512`).
const MM_PER_VELOCITY_UNIT: f64 = crate::TICS_PER_SECOND / 512.0;

/// enhancement-4: F601's acceleration for one thrust word, in millimetres per
/// second squared, as a continuous value rather than one frame's change.
///
/// F601 adds `2 * (tics >> 3) * word >> 16` velocity units per step, so a
/// second's worth of tics gives `2 * (TICS_PER_SECOND / 8) / 65536` units per
/// second per word. The frame's time step, its shifts and its integer rounding
/// drop out, which is what makes the braking point stable at every time
/// acceleration.
pub fn thrust_acceleration(word: i16) -> f64 {
    let units_per_second = 2.0 * (crate::TICS_PER_SECOND / 8.0) / 65_536.0;
    units_per_second * f64::from(word) * MM_PER_VELOCITY_UNIT
}

/// enhancement-4: the thrust words that brake hardest along `closing` (a unit
/// vector in the world frame) and the deceleration they give in millimetres per
/// second squared.
///
/// The ship's axes are the orientation rows, so a world direction's component
/// on each axis is that row's dot product with it. A positive component calls
/// for that axis's minimum word and a negative one for its maximum, which is
/// F598's full-deflection direction and F599's limits. No cap is applied: the
/// caller asks for the ship's actual retro thrust, not the event's limited
/// velocity change.
pub fn retro_thrust(
    limits: &Limits,
    orientation: [[i32; 3]; 3],
    closing: [f64; 3],
) -> ([i16; 3], f64) {
    let mut words = [0i16; 3];
    let mut deceleration = 0.0;
    for axis in 0..3 {
        let component = orientation[axis]
            .iter()
            .zip(closing)
            .map(|(row, direction)| f64::from(*row) / 2_147_483_648.0 * direction)
            .sum::<f64>();
        let [maximum, minimum] = limits.axes[axis];
        words[axis] = if component > 0.0 {
            minimum
        } else if component < 0.0 {
            maximum
        } else {
            0
        };
        deceleration +=
            f64::from(words[axis].unsigned_abs()) * component.abs() * thrust_acceleration(1);
    }
    (words, deceleration)
}

/// enhancement-4: the distance in millimetres needed to stop `speed`
/// (millimetres per second) with `retro` deceleration (millimetres per second
/// squared) and `gravity` pulling the ship along its path (positive toward the
/// destination, where gravity makes the stop longer). Infinite when the
/// gravity leaves no stopping power at all; zero when not closing.
pub fn braking_distance(speed: f64, retro: f64, gravity: f64) -> f64 {
    if speed <= 0.0 {
        return 0.0;
    }
    let net = retro - gravity;
    if net <= 0.0 {
        return f64::INFINITY;
    }
    speed * speed / (2.0 * net)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        axes: [[4905, -4905], [4905, -4905], [16350, -4905]],
    };

    fn identity() -> [[i32; 3]; 3] {
        [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]]
    }

    /// F601's continuous acceleration: an Eagle's 4905-word reverse thrust is
    /// about 90 m/s², the value the port's old prototype used as a constant.
    #[test]
    fn a_reverse_thrust_word_is_ninety_metres_per_second_squared() {
        let accel = thrust_acceleration(4905);
        assert!((accel - 90_300.0).abs() < 200.0, "{accel}");
    }

    /// enhancement-4: the hard-braking words follow the world direction into
    /// the ship's own axes and use the limits' opposite side.
    #[test]
    fn the_retro_words_oppose_the_closing_direction() {
        let (words, accel) = retro_thrust(&LIMITS, identity(), [0.0, 0.0, 1.0]);
        assert_eq!(words, [0, 0, -4905]);
        assert!((accel - 90_300.0).abs() < 200.0, "{accel}");
        // A target behind the ship brakes with the forward word.
        let (words, _) = retro_thrust(&LIMITS, identity(), [0.0, 0.0, -1.0]);
        assert_eq!(words, [0, 0, 16350]);
        // The direction turns with the ship: facing backwards, the same world
        // direction needs the forward axis.
        let turned = [[-i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, -i32::MAX]];
        let (words, _) = retro_thrust(&LIMITS, turned, [0.0, 0.0, 1.0]);
        assert_eq!(words, [0, 0, 16350]);
    }

    /// enhancement-4: the acceleration does not depend on the frame's time
    /// step. F601's own arithmetic for the same word over 20 ms at 1x, 10x and
    /// 100x gives the same continuous value (before the fraction accumulator's
    /// sub-unit carry).
    #[test]
    fn the_braking_acceleration_is_the_same_at_every_time_step() {
        let continuous = thrust_acceleration(4905) / MM_PER_VELOCITY_UNIT;
        for tics in [994u32, 9_940, 49_700, 99_400] {
            let time = f64::from(tics >> 3);
            let per_step = 2.0 * time * 4905.0 / 65_536.0;
            let seconds = f64::from(tics) / crate::TICS_PER_SECOND;
            let from_step = per_step / seconds;
            assert!(
                (from_step - continuous).abs() / continuous < 0.01,
                "{tics} tics: {from_step} vs {continuous}"
            );
        }
    }

    /// enhancement-4: the stopping distance is the continuous `v² / 2a`, with
    /// gravity subtracted from the retro thrust and infinite when it wins.
    #[test]
    fn the_braking_distance_accounts_for_retro_thrust_and_gravity() {
        // 900 m/s with the Eagle's 90.3 m/s²: 4,485 m.
        let distance = braking_distance(900_000.0, 90_300.0, 0.0);
        assert!((distance - 4_485_049.8).abs() < 100.0, "{distance}");
        let with_gravity = braking_distance(900_000.0, 90_300.0, 9_000.0);
        assert!(with_gravity > distance, "{with_gravity} <= {distance}");
        assert!(braking_distance(900_000.0, 90_300.0, 100_000.0).is_infinite());
        assert_eq!(braking_distance(0.0, 90_300.0, 0.0), 0.0);
    }
}
