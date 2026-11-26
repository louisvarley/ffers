//! `enhancement-15`'s sound: three clips the owner supplied for the streaking hyperspace.
//!
//! The ramp up plays as the drive starts charging: its first [`LEAD`] seconds are the
//! charge, and the jump starts as it rises (the streaks gather speed over the next 2.2 s).
//! The ramp down is started so that it ends as the ship arrives (the streaks slow over its
//! last 2.2 s), and the drone fills the time between them: it fades in over the last second
//! of the ramp up and out over the first second of the ramp down, repeating if the jump is
//! longer than it. A cancelled charge stops the ramp up. With the clips missing the
//! original hyperspace sample is used instead.
//!
//! The clips are the owner's own, embedded in the game (`assets/hyperspace/`); they are not
//! original game data.

use ffe_audio::Audio;
use std::sync::Arc;

/// The embedded clips: ramp up, drone, ramp down.
const RAMP_UP_WAV: &[u8] = include_bytes!("../assets/hyperspace/ramp_up.wav");
const DRONE_WAV: &[u8] = include_bytes!("../assets/hyperspace/drone.wav");
const RAMP_DOWN_WAV: &[u8] = include_bytes!("../assets/hyperspace/ramp_down.wav");
/// The seconds the drone overlaps each ramp, fading.
pub const OVERLAP: f64 = 1.0;
/// The level the clips play at (the effects scale, `0..=63`); the effects volume applies.
const VOLUME: u8 = 63;
/// The mixer's rate.
const RATE: u32 = ffe_audio::mixer::SAMPLE_RATE;
/// The mixer layers used: one a clip.
const UP: usize = 0;
const DRONE: usize = 1;
const DOWN: usize = 2;
/// A jump entered later than this (a game loaded mid-jump) does not start the ramp up.
const LATE_START: f64 = 0.5;
/// The ramp up's lead-in, played while the drive charges: the owner's clip has this much
/// before its rise.
pub const LEAD: f64 = crate::hyperspace_streaks::CHARGE_SECONDS;

/// The three clips as mono PCM at the mixer's rate.
#[derive(Debug, Clone)]
pub struct Clips {
    pub up: Arc<Vec<i16>>,
    pub drone: Arc<Vec<i16>>,
    pub down: Arc<Vec<i16>>,
}

impl Clips {
    /// Decode the embedded clips.
    pub fn embedded() -> Result<Self, String> {
        let read = |name: &str, bytes: &[u8]| -> Result<Arc<Vec<i16>>, String> {
            read_wav(bytes)
                .map(Arc::new)
                .map_err(|e| format!("{name}: {e}"))
        };
        Ok(Self {
            up: read("ramp up", RAMP_UP_WAV)?,
            drone: read("drone", DRONE_WAV)?,
            down: read("ramp down", RAMP_DOWN_WAV)?,
        })
    }

    pub fn up_seconds(&self) -> f64 {
        self.up.len() as f64 / f64::from(RATE)
    }

    /// The lead-in's length, no longer than the clip.
    pub fn lead_seconds(&self) -> f64 {
        LEAD.min(self.up_seconds())
    }

    /// Seconds the ramp up still plays once the jump starts.
    pub fn up_after_charge(&self) -> f64 {
        self.up_seconds() - self.lead_seconds()
    }

    pub fn down_seconds(&self) -> f64 {
        self.down.len() as f64 / f64::from(RATE)
    }
}

/// A PCM WAV (8- or 16-bit, mono or stereo, any rate) as mono 16-bit at the mixer's rate.
pub fn read_wav(bytes: &[u8]) -> Result<Vec<i16>, String> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE file".to_owned());
    }
    let mut position = 12usize;
    let mut format = None;
    let mut data = None;
    while position + 8 <= bytes.len() {
        let id = &bytes[position..position + 4];
        let size = u32::from_le_bytes([
            bytes[position + 4],
            bytes[position + 5],
            bytes[position + 6],
            bytes[position + 7],
        ]) as usize;
        let start = position + 8;
        // A data chunk that claims more than the file holds is read to the end.
        let end = start.saturating_add(size).min(bytes.len());
        match id {
            b"fmt " if end - start >= 16 => {
                let f = &bytes[start..start + 16];
                let tag = u16::from_le_bytes([f[0], f[1]]);
                let channels = u16::from_le_bytes([f[2], f[3]]);
                let rate = u32::from_le_bytes([f[4], f[5], f[6], f[7]]);
                let bits = u16::from_le_bytes([f[14], f[15]]);
                if tag != 1 {
                    return Err(format!("format {tag} is not PCM"));
                }
                if !(1..=2).contains(&channels) || !matches!(bits, 8 | 16) || rate == 0 {
                    return Err(format!("{channels} channel(s), {bits}-bit, {rate} Hz"));
                }
                format = Some((channels as usize, bits, rate));
            }
            b"data" => data = Some(&bytes[start..end]),
            _ => {}
        }
        position = end + (size & 1);
    }
    let (channels, bits, rate) = format.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    let width = usize::from(bits / 8) * channels;
    let samples: Vec<f64> = data
        .chunks_exact(width)
        .map(|frame| {
            let channel = |c: usize| -> f64 {
                if bits == 8 {
                    (f64::from(frame[c]) - 128.0) * 256.0
                } else {
                    f64::from(i16::from_le_bytes([frame[c * 2], frame[c * 2 + 1]]))
                }
            };
            (0..channels).map(channel).sum::<f64>() / channels as f64
        })
        .collect();
    if rate == RATE || samples.is_empty() {
        return Ok(samples.iter().map(|s| *s as i16).collect());
    }
    // Linear resampling to the mixer's rate.
    let length = (samples.len() as u64 * u64::from(RATE) / u64::from(rate)) as usize;
    let step = f64::from(rate) / f64::from(RATE);
    Ok((0..length)
        .map(|i| {
            let at = i as f64 * step;
            let index = (at as usize).min(samples.len() - 1);
            let next = samples[(index + 1).min(samples.len() - 1)];
            let value = samples[index] + (next - samples[index]) * (at - index as f64);
            value.clamp(-32768.0, 32767.0) as i16
        })
        .collect())
}

/// The drone's fade at `t` seconds into a transit of `whole`: in over the last
/// [`OVERLAP`] of the ramp up (`up` seconds long), out over the first of the ramp down
/// (`down` long, ending with the transit); 0 when the ramps leave no gap for it.
pub fn drone_fade(t: f64, whole: f64, up: f64, down: f64) -> f64 {
    let start = up - OVERLAP;
    let end = whole - down + OVERLAP;
    if end <= start {
        return 0.0;
    }
    ((t - start) / OVERLAP)
        .clamp(0.0, 1.0)
        .min(((end - t) / OVERLAP).clamp(0.0, 1.0))
}

/// When the ramp down starts so that it ends with a transit of `whole` seconds.
pub fn down_start(whole: f64, down: f64) -> f64 {
    (whole - down).max(0.0)
}

/// What is playing for the current charge and jump.
#[derive(Debug, Default)]
pub struct HyperspaceSound {
    clips: Option<Clips>,
    /// The ramp up started with a charge.
    charging: bool,
    in_jump: bool,
    drone: bool,
    down: bool,
}

impl HyperspaceSound {
    pub fn new(clips: Option<Clips>) -> Self {
        Self {
            clips,
            ..Self::default()
        }
    }

    /// Whether the clips are there to play (else the original sample is used).
    pub fn ready(&self) -> bool {
        self.clips.is_some()
    }

    /// Once a frame: `charging` is whether the drive is charging for a jump, and `transit`
    /// the jump's elapsed and whole real seconds; both are off when nothing is under way (or
    /// the enhancement is off).
    pub fn update(&mut self, audio: &mut dyn Audio, charging: bool, transit: Option<(f64, f64)>) {
        let Some(clips) = self.clips.as_ref() else {
            return;
        };
        let Some((t, whole)) = transit else {
            if charging {
                if !self.charging {
                    // The lead-in plays under the charge's countdown.
                    self.charging = true;
                    audio.stop_layer(DOWN);
                    audio.play_layer(UP, clips.up.clone(), VOLUME, 1.0, false);
                }
            } else if self.charging || self.in_jump {
                // A cancelled or refused charge stops the ramp up; after a jump the ramp
                // down ends by itself and the others stop.
                audio.stop_layer(UP);
                audio.stop_layer(DRONE);
                self.charging = false;
                self.in_jump = false;
            }
            return;
        };
        if !self.in_jump {
            self.in_jump = true;
            self.drone = false;
            self.down = false;
            // A jump without a charge (a game loaded just as one began) starts the ramp up
            // past its lead-in, so the rise still meets the speed-up.
            if !self.charging && t <= LATE_START {
                audio.stop_layer(DOWN);
                let skip = (clips.lead_seconds() * f64::from(RATE)) as usize;
                let rest = clips.up.get(skip..).unwrap_or_default().to_vec();
                audio.play_layer(UP, Arc::new(rest), VOLUME, 1.0, false);
            }
            self.charging = false;
        }
        let fade = drone_fade(t, whole, clips.up_after_charge(), clips.down_seconds());
        if self.drone {
            audio.set_layer_fade(DRONE, VOLUME, fade);
        } else if fade > 0.0 {
            audio.play_layer(DRONE, clips.drone.clone(), VOLUME, fade, true);
            self.drone = true;
        }
        if !self.down && t >= down_start(whole, clips.down_seconds()) {
            audio.play_layer(DOWN, clips.down.clone(), VOLUME, 1.0, false);
            self.down = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drone_fills_the_gap_with_a_second_of_overlap_each_side() {
        // Ramp up 5 s, ramp down 5 s, a 12 s jump: the down starts at 7 s.
        assert_eq!(down_start(12.0, 5.0), 7.0);
        assert_eq!(drone_fade(3.9, 12.0, 5.0, 5.0), 0.0);
        assert!((drone_fade(4.5, 12.0, 5.0, 5.0) - 0.5).abs() < 1e-9);
        assert_eq!(drone_fade(6.0, 12.0, 5.0, 5.0), 1.0);
        assert!((drone_fade(7.5, 12.0, 5.0, 5.0) - 0.5).abs() < 1e-9);
        assert_eq!(drone_fade(8.1, 12.0, 5.0, 5.0), 0.0);
        // No gap: no drone.
        assert_eq!(drone_fade(4.0, 7.0, 5.0, 5.0), 0.0);
    }

    #[test]
    fn eight_bit_wav_reads_as_sixteen_bit_and_other_rates_are_resampled() {
        fn wav(rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
            let mut out = b"RIFF".to_vec();
            out.extend((36 + data.len() as u32).to_le_bytes());
            out.extend(b"WAVEfmt ");
            out.extend(16u32.to_le_bytes());
            out.extend(1u16.to_le_bytes());
            out.extend(1u16.to_le_bytes());
            out.extend(rate.to_le_bytes());
            out.extend((rate * u32::from(bits / 8)).to_le_bytes());
            out.extend((bits / 8).to_le_bytes());
            out.extend(bits.to_le_bytes());
            out.extend(b"data");
            out.extend((data.len() as u32).to_le_bytes());
            out.extend(data);
            out
        }
        let pcm = read_wav(&wav(22_050, 8, &[128, 255, 0])).unwrap();
        assert_eq!(pcm, [0, 127 * 256, -128 * 256]);
        let pcm = read_wav(&wav(44_100, 16, &[0; 400])).unwrap();
        assert_eq!(pcm.len(), 100);
        assert!(read_wav(b"nonsense").is_err());
    }

    #[test]
    fn the_embedded_clips_decode_and_the_ramp_up_has_its_lead_in() {
        let clips = Clips::embedded().unwrap();
        assert!(clips.up_seconds() > LEAD, "{}", clips.up_seconds());
        assert!(!clips.drone.is_empty());
        assert!(clips.down_seconds() > 2.2);
    }
}
