//! Audio capture service using cpal (WASAPI on Windows).
//!
//! Supports two modes:
//! - Loopback: captures system audio output (what you hear).
//! - Mic: captures microphone input.
//!
//! Architecture:
//! - Recording runs on a dedicated thread ("audio-capture").
//! - Audio is downmixed to mono, resampled to 16kHz, and sent in chunks
//!   via a crossbeam channel to the main thread.
//! - The main thread accumulates chunks into a buffer; on stop, the entire
//!   buffer is fed to VOSK in one pass.
//!
//! Loopback tail capture:
//! - When recording stops, the capture thread waits TAIL_MS milliseconds
//!   before dropping the WASAPI stream. This gives the driver time to
//!   deliver the last audio buffers. Without this pause, the final
//!   ~100-200ms of audio is lost inside the driver.
//! - After the stream is dropped, any remaining samples in the internal
//!   accumulation buffer are flushed to the channel.
//!
//! Non-blocking stop:
//! - `stop()` only sets the stop flag and returns immediately.
//! - The capture thread finishes on its own (sleep + flush + cleanup).
//! - The main thread polls `is_active()` to know when the thread has exited
//!   and all tail audio has been delivered.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use crossbeam_channel::Sender;
use parking_lot::Mutex;

use crate::core::devices::{is_loopback_name, resolve_device, AudioDeviceInfo};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Target sample rate for VOSK recognition (16kHz mono).
pub const SAMPLE_RATE: u32 = 16000;

/// Milliseconds to wait after stop flag is set before dropping the WASAPI stream.
/// This allows the loopback driver to deliver the final audio buffers.
/// 6000ms was determined empirically — shorter values cause tail truncation.
const TAIL_MS: u64 = 6000;

// ---------------------------------------------------------------------------
// AudioMode
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMode {
    Loopback,
    Mic,
}

// ---------------------------------------------------------------------------
// AudioRecorder
// ---------------------------------------------------------------------------

/// Manages audio capture on a background thread.
///
/// # Non-blocking stop
///
/// `stop()` sets the stop flag and returns immediately. The capture thread
/// continues running for TAIL_MS to collect the loopback tail, then exits.
/// Call `is_active()` to check whether the thread has finished.
pub struct AudioRecorder {
    stop_flag: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Internal accumulation buffer shared with the capture callback.
    audio_buffer: Arc<Mutex<Vec<i16>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
            thread: None,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// True while the capture thread is running (including the tail wait).
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Start capturing audio from the given device.
    ///
    /// `chunk_ms` controls how often audio chunks are sent to `tx`.
    /// If a recording is already in progress, it is stopped first.
    pub fn start(
        &mut self,
        mode: AudioMode,
        device_name: &str,
        chunk_ms: u32,
        tx: Sender<Vec<i16>>,
    ) -> Result<()> {
        if self.is_active() {
            self.stop();
        }

        self.stop_flag.store(false, Ordering::SeqCst);
        let stop_flag = Arc::clone(&self.stop_flag);
        let active = Arc::clone(&self.active);
        let audio_buffer = Arc::clone(&self.audio_buffer);

        let device = pick_device(mode, device_name)?;

        active.store(true, Ordering::SeqCst);

        let handle = thread::Builder::new()
            .name("audio-capture".into())
            .spawn(move || {
                if let Err(err) =
                    capture_loop(device, mode, chunk_ms, stop_flag, tx, audio_buffer)
                {
                    eprintln!("audio capture error: {err}");
                }
                active.store(false, Ordering::SeqCst);
            })
            .context("spawn audio thread")?;

        self.thread = Some(handle);
        Ok(())
    }

    /// Signal the capture thread to stop. Returns immediately.
    ///
    /// The thread will continue for TAIL_MS to collect the loopback tail,
    /// then flush remaining samples and exit. Use `is_active()` to know
    /// when it has fully stopped.
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        // Detach the thread handle — it will finish on its own.
        // We don't join here to avoid blocking the UI thread.
        if let Some(handle) = self.thread.take() {
            // Spawn a tiny reaper thread that just joins the old handle
            // so we don't leak OS resources.
            thread::Builder::new()
                .name("audio-reaper".into())
                .spawn(move || {
                    let _ = handle.join();
                })
                .ok();
        }
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Device selection
// ---------------------------------------------------------------------------

/// Pick the audio device: by name if provided, otherwise auto-detect based on mode.
fn pick_device(mode: AudioMode, device_name: &str) -> Result<AudioDeviceInfo> {
    if !device_name.trim().is_empty() {
        if let Some(dev) = resolve_device(device_name) {
            return Ok(dev);
        }
    }

    match mode {
        AudioMode::Loopback => crate::core::find_loopback_device()
            .ok_or_else(|| anyhow!("Loopback device not found")),
        AudioMode::Mic => {
            let host = cpal::default_host();
            host.default_input_device()
                .and_then(|d| d.name().ok())
                .and_then(|name| resolve_device(&name))
                .ok_or_else(|| anyhow!("Microphone not found"))
        }
    }
}

// ---------------------------------------------------------------------------
// Capture loop (runs on audio-capture thread)
// ---------------------------------------------------------------------------

/// Main capture loop. Opens the device, starts the stream, waits for the stop
/// flag, then collects the loopback tail and flushes remaining samples.
fn capture_loop(
    device_info: AudioDeviceInfo,
    mode: AudioMode,
    chunk_ms: u32,
    stop_flag: Arc<AtomicBool>,
    tx: Sender<Vec<i16>>,
    buffer: Arc<Mutex<Vec<i16>>>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = open_cpal_device(&host, &device_info, mode)?;
    let config = pick_stream_config(&device, mode)?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let block_size = ((sample_rate as u64 * chunk_ms as u64) / 1000).max(256) as usize;

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match config.sample_format() {
        SampleFormat::I16 => build_i16_stream(
            &device, &config, channels, sample_rate, block_size,
            tx.clone(), buffer.clone(), err_fn,
        )?,
        SampleFormat::F32 => build_f32_stream(
            &device, &config, channels, sample_rate, block_size,
            tx.clone(), buffer.clone(), err_fn,
        )?,
        SampleFormat::U16 => build_u16_stream(
            &device, &config, channels, sample_rate, block_size,
            tx.clone(), buffer.clone(), err_fn,
        )?,
        other => return Err(anyhow!("Unsupported sample format: {other:?}")),
    };

    stream.play()?;

    // Wait until the main thread signals us to stop.
    while !stop_flag.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(20));
    }

    // --- Loopback tail collection ---
    // Keep the stream alive for TAIL_MS so WASAPI delivers the final buffers.
    thread::sleep(std::time::Duration::from_millis(TAIL_MS));
    drop(stream);

    // Flush any samples still sitting in the accumulation buffer.
    {
        let mut buf = buffer.lock();
        if !buf.is_empty() {
            let _ = tx.send(buf.clone());
            buf.clear();
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// cpal device / stream helpers
// ---------------------------------------------------------------------------

fn open_cpal_device(
    host: &cpal::Host,
    info: &AudioDeviceInfo,
    mode: AudioMode,
) -> Result<cpal::Device> {
    // Loopback devices are opened as output devices on Windows.
    if info.is_output || matches!(mode, AudioMode::Loopback) || is_loopback_name(&info.name) {
        #[cfg(windows)]
        {
            if let Ok(outputs) = host.output_devices() {
                for device in outputs {
                    if let Ok(name) = device.name() {
                        if info.name.starts_with(&name) || name == info.name {
                            return Ok(device);
                        }
                    }
                }
            }
        }
    }

    // Fallback: search input devices.
    if let Ok(inputs) = host.input_devices() {
        for device in inputs {
            if device.name().ok().as_deref() == Some(info.name.as_str()) {
                return Ok(device);
            }
        }
    }

    host.default_input_device()
        .ok_or_else(|| anyhow!("Could not open device: {}", info.name))
}

fn pick_stream_config(
    device: &cpal::Device,
    mode: AudioMode,
) -> Result<cpal::SupportedStreamConfig> {
    if matches!(mode, AudioMode::Loopback) || device.default_output_config().is_ok() {
        if let Ok(out) = device.default_output_config() {
            return Ok(out);
        }
    }
    device
        .default_input_config()
        .map_err(|e| anyhow!("Stream config error: {e}"))
}

// ---------------------------------------------------------------------------
// Audio processing: downmix → resample → chunk
// ---------------------------------------------------------------------------

/// Downmix multi-channel interleaved i16 to mono by averaging channels.
fn downmix_to_mono_i16(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

/// Linear resampling from `from_rate` to `to_rate`.
fn resample_linear(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let out_len = ((input.len() as u64 * to_rate as u64) / from_rate as u64) as usize;
    let mut out = Vec::with_capacity(out_len.max(1));
    for i in 0..out_len.max(1) {
        let src_pos = (i as f64 * from_rate as f64) / to_rate as f64;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0) as f32;
        let b = input.get(idx + 1).copied().unwrap_or(a as i16) as f32;
        out.push((a + (b - a) * frac) as i16);
    }
    out
}

/// Process raw audio from the stream callback:
/// downmix → resample → accumulate → emit chunks when block_size is reached.
fn emit_chunk(
    raw: Vec<i16>,
    channels: usize,
    sample_rate: u32,
    block_size: usize,
    tx: &Sender<Vec<i16>>,
    buffer: &Mutex<Vec<i16>>,
) {
    let mono = downmix_to_mono_i16(&raw, channels);
    let at_target = resample_linear(&mono, sample_rate, SAMPLE_RATE);

    let mut buf = buffer.lock();
    buf.extend(at_target);
    while buf.len() >= block_size {
        let chunk: Vec<i16> = buf.drain(0..block_size).collect();
        let _ = tx.try_send(chunk);
    }
}

// ---------------------------------------------------------------------------
// Stream builders (one per sample format)
// ---------------------------------------------------------------------------

fn build_i16_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    sample_rate: u32,
    block_size: usize,
    tx: Sender<Vec<i16>>,
    buffer: Arc<Mutex<Vec<i16>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream> {
    let cfg = config.clone().into();
    device
        .build_input_stream(
            &cfg,
            move |data: &[i16], _| {
                emit_chunk(data.to_vec(), channels, sample_rate, block_size, &tx, &buffer);
            },
            err_fn,
            None,
        )
        .map_err(|e| anyhow!("build i16 stream: {e}"))
}

fn build_f32_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    sample_rate: u32,
    block_size: usize,
    tx: Sender<Vec<i16>>,
    buffer: Arc<Mutex<Vec<i16>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream> {
    let cfg = config.clone().into();
    device
        .build_input_stream(
            &cfg,
            move |data: &[f32], _| {
                let pcm: Vec<i16> = data
                    .iter()
                    .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();
                emit_chunk(pcm, channels, sample_rate, block_size, &tx, &buffer);
            },
            err_fn,
            None,
        )
        .map_err(|e| anyhow!("build f32 stream: {e}"))
}

fn build_u16_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    channels: usize,
    sample_rate: u32,
    block_size: usize,
    tx: Sender<Vec<i16>>,
    buffer: Arc<Mutex<Vec<i16>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream> {
    let cfg = config.clone().into();
    device
        .build_input_stream(
            &cfg,
            move |data: &[u16], _| {
                let pcm: Vec<i16> = data
                    .iter()
                    .map(|&s| (s as i32 - 32768) as i16)
                    .collect();
                emit_chunk(pcm, channels, sample_rate, block_size, &tx, &buffer);
            },
            err_fn,
            None,
        )
        .map_err(|e| anyhow!("build u16 stream: {e}"))
}
