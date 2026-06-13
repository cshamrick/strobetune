//! CPAL input capture. The audio callback only converts to mono and pushes
//! samples into a shared queue; all DSP happens on the UI thread.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::config;

/// Owns the live input stream and the queue it feeds. Dropping this stops audio.
pub struct AudioHandle {
    /// Kept alive for the lifetime of the app; `None` if no device was opened.
    _stream: Option<cpal::Stream>,
    queue: Arc<Mutex<VecDeque<f32>>>,
    pub sample_rate: f32,
    /// Human-readable status, e.g. the device name or why capture is unavailable.
    pub status: String,
}

impl AudioHandle {
    /// Try to open the default input device. On any failure this returns a
    /// handle with no stream and an explanatory status, so the UI still runs.
    pub fn start() -> Self {
        let queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));

        match Self::try_open(Arc::clone(&queue)) {
            Ok((stream, sample_rate, name)) => AudioHandle {
                _stream: Some(stream),
                queue,
                sample_rate,
                status: format!("input: {name}"),
            },
            Err(msg) => AudioHandle {
                _stream: None,
                queue,
                sample_rate: config::DEFAULT_SAMPLE_RATE,
                status: format!("no audio input ({msg})"),
            },
        }
    }

    fn try_open(
        queue: Arc<Mutex<VecDeque<f32>>>,
    ) -> Result<(cpal::Stream, f32, String), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())?;
        let name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let supported = device
            .default_input_config()
            .map_err(|e| format!("config error: {e}"))?;
        let sample_format = supported.sample_format();
        let stream_config: cpal::StreamConfig = supported.config();
        let sample_rate = stream_config.sample_rate as f32;
        let channels = stream_config.channels as usize;

        let err_fn = |err| eprintln!("audio stream error: {err}");

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream::<f32>(
                &device, stream_config, channels, queue, err_fn, |s| s,
            ),
            cpal::SampleFormat::I16 => build_stream::<i16>(
                &device, stream_config, channels, queue, err_fn,
                |s| s as f32 / i16::MAX as f32,
            ),
            cpal::SampleFormat::U16 => build_stream::<u16>(
                &device, stream_config, channels, queue, err_fn,
                |s| (s as f32 - 32768.0) / 32768.0,
            ),
            other => return Err(format!("unsupported sample format {other:?}")),
        }
        .map_err(|e| format!("build stream: {e}"))?;

        stream.play().map_err(|e| format!("play: {e}"))?;
        Ok((stream, sample_rate, name))
    }

    /// Drain all queued samples for processing this frame.
    pub fn drain(&self) -> Vec<f32> {
        match self.queue.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    queue: Arc<Mutex<VecDeque<f32>>>,
    err_fn: impl FnMut(cpal::Error) + Send + 'static,
    convert: fn(T) -> f32,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::SizedSample + Send + 'static,
{
    device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            // Downmix interleaved channels to mono and enqueue. try_lock keeps
            // the callback non-blocking: if the UI thread holds the lock we
            // simply skip this buffer rather than stall the audio thread.
            if let Ok(mut q) = queue.try_lock() {
                for frame in data.chunks(channels.max(1)) {
                    let sum: f32 = frame.iter().map(|&s| convert(s)).sum();
                    q.push_back(sum / channels.max(1) as f32);
                }
                let overflow = q.len().saturating_sub(config::MAX_QUEUED_SAMPLES);
                if overflow > 0 {
                    q.drain(..overflow);
                }
            }
        },
        err_fn,
        None,
    )
}
