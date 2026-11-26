use crate::mixer::{EffectLibrary, Mixer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// A destination for short MIDI messages (up to three bytes).
///
/// The player never needs long messages: the original HMP files contain only
/// channel and meta events. A backend that cannot open a device may be replaced
/// by [`NullMidiSink`].
pub trait MidiSink: Send {
    fn send(&mut self, message: &[u8]);
    /// Stop all notes and reset the device (called when a song stops).
    fn reset(&mut self);
}

/// A destination for digital sound effects.
pub trait SampleSink: Send {
    fn play_sample(&mut self, code: u8, volume: u8, pitch: i32);
    fn stop_all(&mut self);
    /// Swap the decoded effect set. The device/mixer sinks implement this; the
    /// recording and null sinks can ignore it.
    fn set_effects(&mut self, _effects: Arc<EffectLibrary>) {}
    /// Play a mono PCM stream at the mixing rate (a CD video's sound),
    /// replacing any stream playing. Sinks without a mixer ignore it.
    fn play_stream(&mut self, _pcm: Arc<Vec<i16>>, _volume: u8) {}
    fn stop_stream(&mut self) {}
    /// Play first-run rendered music PCM, independent of the video stream.
    fn play_music_stream(&mut self, _pcm: Arc<Vec<i16>>, _volume: u8) {}
    fn stop_music_stream(&mut self) {}
    fn set_music_stream_volume(&mut self, _volume: u8) {}
    /// Layer streams the game fades itself (see `Mixer::play_layer`); `gain` is linear.
    fn play_layer(&mut self, _slot: usize, _pcm: Arc<Vec<i16>>, _gain: f64, _looping: bool) {}
    fn set_layer_gain(&mut self, _slot: usize, _gain: f64) {}
    fn stop_layer(&mut self, _slot: usize) {}
}

/// A recorded effect call, for tests and for the diagnostics pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleCall {
    pub code: u8,
    pub volume: u8,
    pub pitch: i32,
}

/// Discards everything. This is the CI/headless backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullMidiSink;

impl MidiSink for NullMidiSink {
    fn send(&mut self, _message: &[u8]) {}
    fn reset(&mut self) {}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NullSampleSink;

impl SampleSink for NullSampleSink {
    fn play_sample(&mut self, _code: u8, _volume: u8, _pitch: i32) {}
    fn stop_all(&mut self) {}
}

/// Keeps every MIDI message in order. Cheap to clone: clones share the log.
#[derive(Debug, Default, Clone)]
pub struct RecordingMidiSink {
    messages: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl RecordingMidiSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn messages(&self) -> Vec<Vec<u8>> {
        lock(&self.messages).clone()
    }

    pub fn clear(&self) {
        lock(&self.messages).clear();
    }

    /// True when a three-byte channel message with this status and data was sent.
    pub fn contains(&self, message: &[u8]) -> bool {
        lock(&self.messages)
            .iter()
            .any(|recorded| recorded.as_slice() == message)
    }

    pub fn count(&self, message: &[u8]) -> usize {
        lock(&self.messages)
            .iter()
            .filter(|recorded| recorded.as_slice() == message)
            .count()
    }
}

impl MidiSink for RecordingMidiSink {
    fn send(&mut self, message: &[u8]) {
        lock(&self.messages).push(message.to_vec());
    }

    fn reset(&mut self) {
        lock(&self.messages).push(vec![0xff, 0xff]);
    }
}

/// Keeps every effect call in order. Cheap to clone: clones share the log.
#[derive(Debug, Default, Clone)]
pub struct RecordingSampleSink {
    calls: Arc<Mutex<Vec<SampleCall>>>,
}

impl RecordingSampleSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calls(&self) -> Vec<SampleCall> {
        lock(&self.calls).clone()
    }

    pub fn clear(&self) {
        lock(&self.calls).clear();
    }
}

impl SampleSink for RecordingSampleSink {
    fn play_sample(&mut self, code: u8, volume: u8, pitch: i32) {
        lock(&self.calls).push(SampleCall {
            code,
            volume,
            pitch,
        });
    }

    fn stop_all(&mut self) {}
}

/// The real effect backend: a [`Mixer`] plus, on Windows, a WinMM `waveOut`
/// thread. Without a device (`new`) it stays a headless mixer, which is also how
/// the tests read back rendered PCM. The mixer keeps decoded samples and voice
/// state; this type only serialises access to it.
pub struct MixerSampleSink {
    mixer: Arc<Mutex<Mixer>>,
    stop: Option<Arc<AtomicBool>>,
    thread: Option<JoinHandle<()>>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    output: Option<cpal::Stream>,
}

impl MixerSampleSink {
    /// Headless mixer: renders on demand, opens no device. Used by tests and as
    /// the fallback when `waveOutOpen` fails.
    pub fn new(effects: EffectLibrary) -> Self {
        Self {
            mixer: Arc::new(Mutex::new(Mixer::new(effects))),
            stop: None,
            thread: None,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            output: None,
        }
    }

    /// Open the platform audio device and start the mixer output.
    pub fn open(effects: EffectLibrary) -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        {
            let mixer = Arc::new(Mutex::new(Mixer::new(effects)));
            let (stop, thread) = crate::waveout::spawn(Arc::clone(&mixer))?;
            Ok(Self {
                mixer,
                stop: Some(stop),
                thread: Some(thread),
            })
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let mixer = Arc::new(Mutex::new(Mixer::new(effects)));
            let output = open_cpal_output(Arc::clone(&mixer))?;
            Ok(Self {
                mixer,
                stop: None,
                thread: None,
                output: Some(output),
            })
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            let _ = effects;
            Err("no wave output backend on this platform".to_owned())
        }
    }

    /// Shared mixer handle, for tests and the diagnostics pane.
    pub fn mixer(&self) -> Arc<Mutex<Mixer>> {
        Arc::clone(&self.mixer)
    }

    /// Render the next `output.len()` mono samples. This advances the voice
    /// positions, exactly as the device thread does, and is how a headless test
    /// proves that real sample data (not only cue records) is mixed.
    pub fn mix_into(&self, output: &mut [i16]) {
        lock(&self.mixer).mix_into(output);
    }

    pub fn render(&self, frames: usize) -> Vec<i16> {
        let mut output = vec![0i16; frames];
        self.mix_into(&mut output);
        output
    }
}

impl SampleSink for MixerSampleSink {
    fn play_sample(&mut self, code: u8, volume: u8, pitch: i32) {
        lock(&self.mixer).play(code, volume, pitch);
    }

    fn stop_all(&mut self) {
        lock(&self.mixer).stop_all();
    }

    fn set_effects(&mut self, effects: Arc<EffectLibrary>) {
        lock(&self.mixer).set_effects(effects);
    }

    fn play_stream(&mut self, pcm: Arc<Vec<i16>>, volume: u8) {
        lock(&self.mixer).play_stream(pcm, volume);
    }

    fn stop_stream(&mut self) {
        lock(&self.mixer).stop_stream();
    }

    fn play_music_stream(&mut self, pcm: Arc<Vec<i16>>, volume: u8) {
        lock(&self.mixer).play_music_stream(pcm, volume);
    }

    fn stop_music_stream(&mut self) {
        lock(&self.mixer).stop_music_stream();
    }

    fn set_music_stream_volume(&mut self, volume: u8) {
        lock(&self.mixer).set_music_stream_volume(volume);
    }

    fn play_layer(&mut self, slot: usize, pcm: Arc<Vec<i16>>, gain: f64, looping: bool) {
        lock(&self.mixer).play_layer(slot, pcm, gain, looping);
    }

    fn set_layer_gain(&mut self, slot: usize, gain: f64) {
        lock(&self.mixer).set_layer_gain(slot, gain);
    }

    fn stop_layer(&mut self, slot: usize) {
        lock(&self.mixer).stop_layer(slot);
    }
}

impl Drop for MixerSampleSink {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl std::fmt::Debug for MixerSampleSink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let device_open = self.thread.is_some() || self.output.is_some();
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        let device_open = self.thread.is_some();
        formatter
            .debug_struct("MixerSampleSink")
            .field("device", &device_open)
            .finish()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
struct Resampler {
    phase: f64,
    step: f64,
    current: i16,
    next: i16,
    primed: bool,
    scratch: Vec<i16>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl Resampler {
    fn new(output_rate: u32) -> Self {
        Self {
            phase: 0.0,
            step: f64::from(crate::mixer::SAMPLE_RATE) / f64::from(output_rate.max(1)),
            current: 0,
            next: 0,
            primed: false,
            scratch: Vec::new(),
        }
    }

    fn fill(&mut self, output: &mut [f32], channels: usize, mixer: &Arc<Mutex<Mixer>>) {
        let channels = channels.max(1);
        let frames = output.len() / channels;
        let needed = (self.phase + frames as f64 * self.step).floor() as usize
            + usize::from(!self.primed) * 2;
        self.scratch.resize(needed, 0);
        lock(mixer).mix_into(&mut self.scratch);
        let mut source = 0;
        if !self.primed {
            self.current = self.scratch[0];
            self.next = self.scratch[1];
            source = 2;
            self.primed = true;
        }
        for frame in 0..frames {
            let frac = self.phase as f32;
            let sample =
                f32::from(self.current) + (f32::from(self.next) - f32::from(self.current)) * frac;
            let sample = sample / 32768.0;
            for channel in 0..channels {
                output[frame * channels + channel] = sample;
            }
            self.phase += self.step;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
                self.current = self.next;
                self.next = self.scratch.get(source).copied().unwrap_or(0);
                source += 1;
            }
        }
        output[frames * channels..].fill(0.0);
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn open_cpal_output(mixer: Arc<Mutex<Mixer>>) -> Result<cpal::Stream, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default audio output device".to_owned())?;
    let supported = device
        .default_output_config()
        .map_err(|error| format!("query default audio format: {error}"))?;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let mut resampler = Resampler::new(config.sample_rate);
    let channels = usize::from(config.channels);
    let error_callback = |error| eprintln!("audio stream error: {error}");
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            config,
            move |output: &mut [f32], _| resampler.fill(output, channels, &mixer),
            error_callback,
            None,
        ),
        cpal::SampleFormat::I16 => {
            let mut floats = Vec::new();
            device.build_output_stream(
                config,
                move |output: &mut [i16], _| {
                    floats.resize(output.len(), 0.0);
                    resampler.fill(&mut floats, channels, &mixer);
                    for (to, from) in output.iter_mut().zip(&floats) {
                        *to = (from * 32767.0).round() as i16;
                    }
                },
                error_callback,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let mut floats = Vec::new();
            device.build_output_stream(
                config,
                move |output: &mut [u16], _| {
                    floats.resize(output.len(), 0.0);
                    resampler.fill(&mut floats, channels, &mixer);
                    for (to, from) in output.iter_mut().zip(&floats) {
                        *to = ((from.clamp(-1.0, 1.0) + 1.0) * 32767.5).round() as u16;
                    }
                },
                error_callback,
                None,
            )
        }
        format => return Err(format!("unsupported audio sample format: {format:?}")),
    }
    .map_err(|error| format!("open audio output: {error}"))?;
    stream
        .play()
        .map_err(|error| format!("start audio output: {error}"))?;
    Ok(stream)
}

/// The headless mixer under the name the tests use: every call is mixed into
/// inspectable 16-bit PCM rather than only recorded.
pub type RecordingMixSink = MixerSampleSink;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// WinMM system-MIDI output, the closest dependency-free equivalent of the
/// original's Win32 MIDI path. It sends short messages to the system synth (the
/// Microsoft GS Wavetable Synth on a default Windows install).
#[cfg(target_os = "windows")]
pub struct WindowsMidiSink {
    handle: *mut std::ffi::c_void,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsMidiSink {}

#[cfg(target_os = "windows")]
#[link(name = "winmm")]
extern "system" {
    fn midiOutOpen(
        handle: *mut *mut std::ffi::c_void,
        device: u32,
        callback: usize,
        instance: usize,
        flags: u32,
    ) -> u32;
    fn midiOutClose(handle: *mut std::ffi::c_void) -> u32;
    fn midiOutShortMsg(handle: *mut std::ffi::c_void, message: u32) -> u32;
    fn midiOutReset(handle: *mut std::ffi::c_void) -> u32;
}

#[cfg(target_os = "windows")]
impl WindowsMidiSink {
    /// `device = None` uses `MIDI_MAPPER`, as MCI/DirectMusic would by default.
    pub fn open(device: Option<u32>) -> Result<Self, String> {
        let mut handle = std::ptr::null_mut();
        let device = device.unwrap_or(u32::MAX);
        // SAFETY: WinMM writes one handle; the callback and instance are null.
        let result = unsafe { midiOutOpen(&mut handle, device, 0, 0, 0) };
        if result != 0 {
            return Err(format!("midiOutOpen failed with MMSYSERR {result}"));
        }
        Ok(Self { handle })
    }
}

#[cfg(target_os = "windows")]
impl MidiSink for WindowsMidiSink {
    fn send(&mut self, message: &[u8]) {
        if message.is_empty() || message.len() > 3 {
            return;
        }
        let mut packed = 0u32;
        for (index, byte) in message.iter().enumerate() {
            packed |= u32::from(*byte) << (8 * index);
        }
        // SAFETY: the handle is valid until Drop, and the message is a short one.
        unsafe {
            midiOutShortMsg(self.handle, packed);
        }
    }

    fn reset(&mut self) {
        // SAFETY: the handle is valid until Drop.
        unsafe {
            midiOutReset(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsMidiSink {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by WindowsMidiSink::open and is closed once.
        unsafe {
            midiOutClose(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
impl std::fmt::Debug for WindowsMidiSink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WindowsMidiSink")
            .field("handle", &self.handle)
            .finish()
    }
}
