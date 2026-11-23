//! First-run HMP rendering and private, source-fingerprinted PCM cache.

use crate::library::{MusicLibrary, SONG_COUNT};
use crate::mixer::{decode_wav, EffectSample};
use ffe_core::hmp::{HmpEventKind, HmpSong};
use oxisynth::{MidiEvent, SoundFont, Synth, SynthDescriptor};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Cursor;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const MUSIC_SAMPLE_RATE: u32 = 22_050;
const RENDERER_VERSION: &[u8] = b"ffe-oxisynth-v2;mono-s16;22050;gain=0.2;tail=1s";
const GENERALUSER_GS_SF2: &[u8] = include_bytes!("../assets/soundfont/GeneralUser-GS.sf2");
const MUSIC_BIN_MAGIC: &[u8; 8] = b"FFEMUSC\0";
const MUSIC_BIN_VERSION: u32 = 1;
const MUSIC_BIN_HEADER_BYTES: usize = 20;
const MUSIC_BIN_ENTRY_BYTES: usize = 20;

/// PCM for all songs found in the original data/cache.
#[derive(Debug, Default, Clone)]
pub struct RenderedMusic {
    songs: BTreeMap<usize, Arc<Vec<i16>>>,
}

impl RenderedMusic {
    /// Generate one packed `music.bin` if it does not already exist. Temporary
    /// per-track WAV cache files are removed after the bundle is installed.
    pub fn ensure_music_bin(library: &MusicLibrary, path: &Path) -> Result<bool, String> {
        if path.exists() {
            return Ok(false);
        }
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|error| format!("create music bundle directory: {error}"))?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let work = parent.join(format!(".music-render-{}-{nonce}", std::process::id()));
        fs::create_dir(&work).map_err(|error| format!("create temporary music cache: {error}"))?;
        let result = (|| {
            let rendered = Self::prepare(library, &work)?;
            if rendered.len() != SONG_COUNT {
                return Err(format!(
                    "cannot build music.bin: found {} of {SONG_COUNT} songs",
                    rendered.len()
                ));
            }
            rendered.write_music_bin_if_missing(path)
        })();
        let _ = fs::remove_dir_all(&work);
        result
    }

    /// Load the packed PCM tracks created by [`Self::ensure_music_bin`].
    pub fn load_music_bin(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
        if bytes.len() < MUSIC_BIN_HEADER_BYTES || &bytes[..8] != MUSIC_BIN_MAGIC {
            return Err("invalid music.bin header".to_owned());
        }
        let version = read_u32(&bytes, 8)?;
        let sample_rate = read_u32(&bytes, 12)?;
        let count = read_u32(&bytes, 16)? as usize;
        if version != MUSIC_BIN_VERSION {
            return Err(format!("unsupported music.bin version {version}"));
        }
        if sample_rate != MUSIC_SAMPLE_RATE {
            return Err(format!("unsupported music.bin sample rate {sample_rate}"));
        }
        if count != SONG_COUNT {
            return Err(format!("music.bin contains {count} of {SONG_COUNT} tracks"));
        }
        let directory_end = MUSIC_BIN_HEADER_BYTES
            .checked_add(
                count
                    .checked_mul(MUSIC_BIN_ENTRY_BYTES)
                    .ok_or("music.bin directory overflow")?,
            )
            .ok_or("music.bin directory overflow")?;
        if bytes.len() < directory_end {
            return Err("truncated music.bin directory".to_owned());
        }

        let mut songs = BTreeMap::new();
        for entry in 0..count {
            let offset = MUSIC_BIN_HEADER_BYTES + entry * MUSIC_BIN_ENTRY_BYTES;
            let index = read_u32(&bytes, offset)? as usize;
            let data_offset = usize::try_from(read_u64(&bytes, offset + 4)?)
                .map_err(|_| "music.bin track offset exceeds this platform")?;
            let frames = usize::try_from(read_u64(&bytes, offset + 12)?)
                .map_err(|_| "music.bin track length exceeds this platform")?;
            if index >= SONG_COUNT || songs.contains_key(&index) {
                return Err(format!(
                    "invalid or duplicate music.bin track index {index}"
                ));
            }
            let data_bytes = frames
                .checked_mul(2)
                .ok_or("music.bin track length overflow")?;
            let end = data_offset
                .checked_add(data_bytes)
                .ok_or("music.bin track boundary overflow")?;
            if data_offset < directory_end || end > bytes.len() || frames == 0 {
                return Err(format!("invalid music.bin data range for track {index}"));
            }
            let pcm = bytes[data_offset..end]
                .chunks_exact(2)
                .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
                .collect();
            songs.insert(index, Arc::new(pcm));
        }
        if songs.len() != SONG_COUNT {
            return Err(format!("music.bin contains {} unique tracks", songs.len()));
        }
        Ok(Self { songs })
    }

    fn write_music_bin_if_missing(&self, path: &Path) -> Result<bool, String> {
        if path.exists() {
            return Ok(false);
        }
        let mut data_bytes = 0usize;
        for index in 0..SONG_COUNT {
            let pcm = self
                .songs
                .get(&index)
                .ok_or_else(|| format!("music.bin is missing track {index}"))?;
            data_bytes = data_bytes
                .checked_add(pcm.len().checked_mul(2).ok_or("music.bin size overflow")?)
                .ok_or("music.bin size overflow")?;
        }
        let directory_end = MUSIC_BIN_HEADER_BYTES
            .checked_add(SONG_COUNT * MUSIC_BIN_ENTRY_BYTES)
            .ok_or("music.bin directory overflow")?;
        let capacity = directory_end
            .checked_add(data_bytes)
            .ok_or("music.bin size overflow")?;
        let mut bytes = Vec::with_capacity(capacity);
        bytes.extend_from_slice(MUSIC_BIN_MAGIC);
        bytes.extend_from_slice(&MUSIC_BIN_VERSION.to_le_bytes());
        bytes.extend_from_slice(&MUSIC_SAMPLE_RATE.to_le_bytes());
        bytes.extend_from_slice(&(SONG_COUNT as u32).to_le_bytes());
        let mut data_offset = directory_end;
        for index in 0..SONG_COUNT {
            let pcm = &self.songs[&index];
            bytes.extend_from_slice(&(index as u32).to_le_bytes());
            bytes.extend_from_slice(&(data_offset as u64).to_le_bytes());
            bytes.extend_from_slice(&(pcm.len() as u64).to_le_bytes());
            data_offset += pcm.len() * 2;
        }
        for index in 0..SONG_COUNT {
            for sample in self.songs[&index].iter() {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }

        let temporary = path.with_file_name(format!(
            ".music-bin-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("create temporary music.bin: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("write temporary music.bin: {error}"))?;
        drop(file);
        if path.exists() {
            let _ = fs::remove_file(&temporary);
            return Ok(false);
        }
        match fs::hard_link(&temporary, path) {
            Ok(()) => {
                fs::remove_file(&temporary)
                    .map_err(|error| format!("remove temporary music.bin: {error}"))?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temporary);
                Ok(false)
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(format!("install music.bin: {error}"))
            }
        }
    }

    /// Render or reuse every numbered song. A song cache entry is named by the
    /// source HMP, SoundFont and renderer settings, so stale output is ignored.
    pub fn prepare(library: &MusicLibrary, cache_root: &Path) -> Result<Self, String> {
        let music_dir = cache_root.join("music");
        fs::create_dir_all(&music_dir)
            .map_err(|error| format!("create generated music cache: {error}"))?;
        let mut rendered = Self::default();
        let override_path = std::env::var_os("FFE_SOUNDFONT").map(PathBuf::from);
        let override_bytes = override_path
            .as_ref()
            .map(|path| {
                fs::read(path)
                    .map_err(|error| format!("read SoundFont override {}: {error}", path.display()))
            })
            .transpose()?;
        let soundfont_identity = override_bytes.as_deref().unwrap_or(GENERALUSER_GS_SF2);
        let soundfont_hash = Sha256::digest(soundfont_identity);
        let mut missing = Vec::new();
        for index in 0..SONG_COUNT {
            let source = match library.source_bytes(index) {
                Ok(source) => source,
                Err(crate::audio::AudioError::SongMissing { .. }) => continue,
                Err(error) => return Err(error.to_string()),
            };
            let song = HmpSong::parse(&source).map_err(|error| {
                format!(
                    "parse song {}: {error}",
                    MusicLibrary::name(index).unwrap_or("?")
                )
            })?;
            let mut key = Sha256::new();
            key.update(RENDERER_VERSION);
            key.update((index as u32).to_le_bytes());
            key.update(soundfont_hash);
            key.update(&source);
            let key = format!("{:x}", key.finalize());
            let path = music_dir.join(format!("{index:02}-{key}.wav"));

            if let Some(sample) = path
                .is_file()
                .then(|| fs::read(&path).ok())
                .flatten()
                .and_then(|bytes| decode_wav(&bytes).ok())
                .filter(|sample| sample.sample_rate == MUSIC_SAMPLE_RATE)
            {
                rendered.songs.insert(index, Arc::new(sample.pcm));
            } else {
                missing.push((index, song, path));
            }
        }
        if missing.is_empty() {
            return Ok(rendered);
        }

        let sf2 = override_bytes.unwrap_or_else(|| GENERALUSER_GS_SF2.to_vec());
        let mut synth = Synth::new(SynthDescriptor {
            sample_rate: MUSIC_SAMPLE_RATE as f32,
            audio_channels: 2,
            audio_groups: 1,
            gain: 0.2,
            ..SynthDescriptor::default()
        })
        .map_err(|error| format!("create MIDI synthesizer: {error}"))?;
        let font = SoundFont::load(&mut Cursor::new(sf2)).map_err(|error| {
            if override_path.is_some() {
                format!("load SoundFont override: {error}")
            } else {
                format!("load bundled GeneralUser GS SoundFont: {error}")
            }
        })?;
        synth.add_font(font, true);
        let missing_count = missing.len();
        for (position, (index, song, path)) in missing.into_iter().enumerate() {
            eprintln!(
                "Generating music cache {}/{}: {}",
                position + 1,
                missing_count,
                MusicLibrary::title(index).unwrap_or("unknown song")
            );
            let sample = render_song(&song, &mut synth).map_err(|error| {
                format!(
                    "render song {}: {error}",
                    MusicLibrary::title(index).unwrap_or("unknown song")
                )
            })?;
            write_wav_atomic(&path, &sample.pcm)?;
            rendered.songs.insert(index, Arc::new(sample.pcm));
        }
        Ok(rendered)
    }

    pub fn song(&self, index: usize) -> Option<Arc<Vec<i16>>> {
        self.songs.get(&index).cloned()
    }

    pub fn contains(&self, index: usize) -> bool {
        self.songs.contains_key(&index)
    }

    pub fn len(&self) -> usize {
        self.songs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }
}

/// The platform cache location. `FFE_AUDIO_CACHE_DIR` is useful for portable
/// installs and tests; otherwise follow the OS user's cache convention.
pub fn default_cache_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("FFE_AUDIO_CACHE_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("FFE Classic").join("audio");
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = std::env::var_os("HOME") {
        return PathBuf::from(path)
            .join("Library")
            .join("Caches")
            .join("FFE Classic")
            .join("audio");
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
            return PathBuf::from(path).join("ffe-classic").join("audio");
        }
        if let Some(path) = std::env::var_os("HOME") {
            return PathBuf::from(path)
                .join(".cache")
                .join("ffe-classic")
                .join("audio");
        }
    }
    std::env::temp_dir().join("ffe-classic").join("audio")
}

fn render_song(song: &HmpSong, synth: &mut Synth) -> Result<EffectSample, String> {
    synth
        .send_event(MidiEvent::SystemReset)
        .map_err(|error| format!("reset MIDI synthesizer: {error}"))?;
    let mut events: Vec<(u64, usize, usize, &HmpEventKind)> = Vec::new();
    for (track_index, track) in song.tracks.iter().enumerate() {
        for (event_index, event) in track.events.iter().enumerate() {
            events.push((event.tick, track_index, event_index, &event.kind));
        }
    }
    events.sort_by_key(|(tick, track, event, _)| (*tick, *track, *event));

    let mut pcm = Vec::new();
    let mut frame = 0u64;
    for (tick, _, _, event) in events {
        let target = tick_to_frame(tick, song.division, song.tempo_us_per_quarter);
        if target > frame {
            render_frames(synth, target - frame, &mut pcm)?;
            frame = target;
        }
        if let HmpEventKind::Midi {
            status,
            data1,
            data2,
        } = event
        {
            if let Some(event) = to_midi_event(*status, *data1, data2.unwrap_or(0)) {
                synth.send_event(event).map_err(|error| {
                    format!(
                        "send MIDI event {status:02x} {data1:02x} {:02x} while rendering: {error}",
                        data2.unwrap_or(0)
                    )
                })?;
            }
        }
    }
    let song_end = tick_to_frame(
        song.duration_ticks(),
        song.division,
        song.tempo_us_per_quarter,
    );
    if song_end > frame {
        render_frames(synth, song_end - frame, &mut pcm)?;
    }
    render_frames(synth, u64::from(MUSIC_SAMPLE_RATE), &mut pcm)?;
    Ok(EffectSample {
        pcm,
        sample_rate: MUSIC_SAMPLE_RATE,
    })
}

fn tick_to_frame(tick: u64, division: u16, tempo: u32) -> u64 {
    (u128::from(tick) * u128::from(tempo) * u128::from(MUSIC_SAMPLE_RATE)
        / (u128::from(division.max(1)) * 1_000_000)) as u64
}

fn render_frames(synth: &mut Synth, count: u64, pcm: &mut Vec<i16>) -> Result<(), String> {
    let extra = usize::try_from(count).map_err(|_| "rendered music is too long")?;
    pcm.reserve(extra);
    for _ in 0..count {
        let (left, right) = synth.read_next();
        let mono = ((left + right) * 0.5 * f32::from(i16::MAX))
            .round()
            .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
        pcm.push(mono);
    }
    Ok(())
}

fn to_midi_event(status: u8, data1: u8, data2: u8) -> Option<MidiEvent> {
    let channel = status & 0x0f;
    // MIDI data bytes are seven-bit. Some original HMP controller events use
    // 0x80 for maximum (notably CC7); clamp it to 127 instead of wrapping it
    // to zero, which would mute that whole channel.
    let data1 = data1.min(0x7f);
    let data2 = data2.min(0x7f);
    Some(match status & 0xf0 {
        0x80 => MidiEvent::NoteOff {
            channel,
            key: data1,
        },
        0x90 if data2 == 0 => MidiEvent::NoteOff {
            channel,
            key: data1,
        },
        0x90 => MidiEvent::NoteOn {
            channel,
            key: data1,
            vel: data2,
        },
        0xa0 => MidiEvent::PolyphonicKeyPressure {
            channel,
            key: data1,
            value: data2,
        },
        0xb0 if data1 == 120 => MidiEvent::AllSoundOff { channel },
        0xb0 if data1 == 123 => MidiEvent::AllNotesOff { channel },
        0xb0 => MidiEvent::ControlChange {
            channel,
            ctrl: data1,
            value: data2,
        },
        0xc0 => MidiEvent::ProgramChange {
            channel,
            program_id: data1,
        },
        0xd0 => MidiEvent::ChannelPressure {
            channel,
            value: data1,
        },
        0xe0 => MidiEvent::PitchBend {
            channel,
            value: u16::from(data1) | (u16::from(data2) << 7),
        },
        _ => return None,
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset.checked_add(4).ok_or("music.bin offset overflow")?;
    let value = bytes
        .get(offset..end)
        .ok_or("truncated music.bin integer")?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let end = offset.checked_add(8).ok_or("music.bin offset overflow")?;
    let value = bytes
        .get(offset..end)
        .ok_or("truncated music.bin integer")?;
    Ok(u64::from_le_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

fn write_wav_atomic(path: &Path, pcm: &[i16]) -> Result<(), String> {
    let data_len = u32::try_from(pcm.len().saturating_mul(2))
        .map_err(|_| "rendered WAV exceeds the RIFF size limit")?;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&MUSIC_SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(MUSIC_SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in pcm {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let temp = path.with_extension(format!("wav.tmp-{}", std::process::id()));
    fs::write(&temp, bytes).map_err(|error| format!("write generated music cache: {error}"))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("replace invalid generated music cache: {error}"))?;
    }
    fs::rename(&temp, path).map_err(|error| format!("install generated music cache: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffe_core::hmp::{HmpEvent, HmpTrack, HMP_DIVISION, HMP_TEMPO_US_PER_QUARTER};

    #[test]
    fn out_of_range_hmp_controller_data_clamps_to_midi_maximum() {
        assert!(matches!(
            to_midi_event(0xb9, 7, 0x80),
            Some(MidiEvent::ControlChange {
                channel: 9,
                ctrl: 7,
                value: 127,
            })
        ));
    }

    #[test]
    fn bundled_soundfont_renders_a_midi_channel_to_cached_wav() {
        let sf2 = GENERALUSER_GS_SF2;
        assert_eq!(&sf2[..4], b"RIFF");
        assert_eq!(&sf2[8..12], b"sfbk");
        let font = SoundFont::load(&mut Cursor::new(sf2)).expect("parse SoundFont");
        let mut synth = Synth::new(SynthDescriptor {
            sample_rate: MUSIC_SAMPLE_RATE as f32,
            audio_channels: 2,
            audio_groups: 1,
            ..SynthDescriptor::default()
        })
        .expect("create synth");
        synth.add_font(font, true);

        let song = HmpSong {
            header_track_count: 2,
            advertised_duration_seconds: 2,
            division: HMP_DIVISION,
            tempo_us_per_quarter: HMP_TEMPO_US_PER_QUARTER,
            tracks: vec![HmpTrack {
                end_tick: 192,
                events: vec![
                    HmpEvent {
                        tick: 0,
                        kind: HmpEventKind::Midi {
                            status: 0xc0,
                            data1: 0,
                            data2: None,
                        },
                    },
                    HmpEvent {
                        tick: 0,
                        kind: HmpEventKind::Midi {
                            status: 0x90,
                            data1: 60,
                            data2: Some(100),
                        },
                    },
                    HmpEvent {
                        tick: 96,
                        kind: HmpEventKind::Midi {
                            status: 0x80,
                            data1: 60,
                            data2: Some(0),
                        },
                    },
                    HmpEvent {
                        tick: 192,
                        kind: HmpEventKind::Meta {
                            kind: 0x2f,
                            data: Vec::new(),
                        },
                    },
                ],
            }],
        };
        let rendered = render_song(&song, &mut synth).expect("render note");
        assert_eq!(rendered.sample_rate, MUSIC_SAMPLE_RATE);
        assert!(rendered.pcm.iter().any(|sample| sample.unsigned_abs() > 32));

        let temp = tempfile::tempdir().expect("temp cache");
        let path = temp.path().join("song.wav");
        write_wav_atomic(&path, &rendered.pcm).expect("write generated WAV");
        let decoded =
            decode_wav(&fs::read(path).expect("read generated WAV")).expect("decode generated WAV");
        assert_eq!(decoded.sample_rate, MUSIC_SAMPLE_RATE);
        assert_eq!(decoded.pcm, rendered.pcm);
    }

    #[test]
    fn music_bin_round_trips_tracks_and_never_replaces_an_existing_file() {
        let mut songs = BTreeMap::new();
        for index in 0..SONG_COUNT {
            songs.insert(index, Arc::new(vec![index as i16, -(index as i16), 123]));
        }
        let rendered = RenderedMusic { songs };
        let temp = tempfile::tempdir().expect("music bundle directory");
        let path = temp.path().join("music.bin");
        assert!(rendered
            .write_music_bin_if_missing(&path)
            .expect("write bundle"));
        let loaded = RenderedMusic::load_music_bin(&path).expect("read bundle");
        assert_eq!(loaded.len(), SONG_COUNT);
        for index in 0..SONG_COUNT {
            assert_eq!(
                loaded.song(index).unwrap().as_slice(),
                [index as i16, -(index as i16), 123]
            );
        }
        assert!(!rendered
            .write_music_bin_if_missing(&path)
            .expect("preserve existing bundle"));
        fs::write(&path, b"existing").expect("install sentinel");
        assert!(!rendered
            .write_music_bin_if_missing(&path)
            .expect("do not replace existing file"));
        assert_eq!(fs::read(path).unwrap(), b"existing");
    }

    #[test]
    fn existing_music_bin_skips_rendering_even_without_source_songs() {
        let temp = tempfile::tempdir().expect("music bundle directory");
        let path = temp.path().join("music.bin");
        fs::write(&path, b"existing").expect("install sentinel");
        let library = MusicLibrary::new(temp.path().join("missing-music"));
        assert!(!RenderedMusic::ensure_music_bin(&library, &path).unwrap());
        assert_eq!(fs::read(path).unwrap(), b"existing");
    }
}
