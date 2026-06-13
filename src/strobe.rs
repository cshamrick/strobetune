//! The virtual strobe: a reference wheel, a lamp, and the accumulated image
//! formed by their interaction.
//!
//! Core idea (this is the whole trick, and it needs no pitch detection):
//!
//! * The wheel phase advances by `ref_freq / sample_rate` cycles per sample.
//! * Each sample's lamp brightness is added into the accumulator bin indexed by
//!   the *current wheel phase* — i.e. we fold the lamp signal over the
//!   reference period.
//! * If the input frequency equals the reference, the lamp's bright moments
//!   always land at the same wheel phase, so the bright band sits still.
//! * If the input is sharp or flat, the coincidence point precesses around the
//!   wheel at the beat frequency `f_in - f_ref`, so the band drifts — direction
//!   given by the sign of the detuning. We never compute that frequency; the
//!   drift emerges from the phase folding plus per-frame persistence/decay.

use std::f32::consts::PI;

use crate::config;
use crate::dsp::Conditioner;

pub struct Strobe {
    /// Accumulated lamp energy per wheel-phase bin. Length == `config::STROBE_BINS`.
    pub bins: Vec<f32>,
    /// Wheel phase in cycles, kept in [0, 1).
    wheel_phase: f32,
    ref_freq: f32,
    sample_rate: f32,
    /// Band position last frame, for measuring apparent rotation speed.
    prev_phase: Option<f32>,
    /// Smoothed apparent drift speed of the image, in cycles per frame. This is
    /// a property of the *displayed pattern's* motion (how fast the band slides),
    /// not a pitch or cents measurement: still ≈ in tune, fast = far off.
    pub drift: f32,
    /// Smoothed input level for the UI meter, in [0, 1].
    pub level: f32,
    /// Whether the lamp is currently active (signal above the noise gate).
    pub lamp_active: bool,
}

impl Strobe {
    pub fn new(ref_freq: f32, sample_rate: f32) -> Self {
        Strobe {
            bins: vec![0.0; config::STROBE_BINS],
            wheel_phase: 0.0,
            ref_freq,
            sample_rate,
            prev_phase: None,
            drift: 0.0,
            level: 0.0,
            lamp_active: false,
        }
    }

    pub fn set_ref_freq(&mut self, ref_freq: f32) {
        self.ref_freq = ref_freq;
    }

    /// Fade the accumulated image. Called once per UI frame to give the display
    /// visual persistence and to let a drifting band actually appear to move.
    pub fn decay(&mut self) {
        for b in &mut self.bins {
            *b *= config::ACCUM_DECAY;
        }
    }

    /// Advance the wheel and fold a batch of input samples into the accumulator.
    pub fn process(&mut self, samples: &[f32], cond: &mut Conditioner) {
        let phase_step = self.ref_freq / self.sample_rate;

        // Input level (RMS) over the batch, for the meter / gate.
        if !samples.is_empty() {
            let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
            let rms = (sum_sq / samples.len() as f32).sqrt();
            // Smooth so the meter doesn't flicker.
            self.level = 0.8 * self.level + 0.2 * (rms * 4.0).min(1.0);
            self.lamp_active = rms >= config::NOISE_GATE;
        } else {
            self.level *= 0.8;
            self.lamp_active = false;
        }

        let gate_open = self.lamp_active;
        let n_bins = self.bins.len();

        for &sample in samples {
            // The wheel keeps turning regardless of input.
            self.wheel_phase += phase_step;
            if self.wheel_phase >= 1.0 {
                self.wheel_phase -= self.wheel_phase.floor();
            }

            let lamp = cond.lamp(sample);
            if gate_open {
                let bin = (self.wheel_phase * n_bins as f32) as usize;
                let bin = bin.min(n_bins - 1);
                self.bins[bin] += lamp;
            }
        }

        self.update_drift();
    }

    /// Sample the accumulated image at a fractional position around the wheel
    /// (`pos` in [0, 1)), normalized to [0, 1] against the current peak. Used by
    /// the renderer to map the wheel onto however many columns are available.
    pub fn brightness_at(&self, pos: f32, peak: f32) -> f32 {
        if peak <= f32::EPSILON {
            return 0.0;
        }
        let n = self.bins.len();
        let idx = ((pos.rem_euclid(1.0)) * n as f32) as usize;
        (self.bins[idx.min(n - 1)] / peak).clamp(0.0, 1.0)
    }

    /// Current peak bin value, for normalization.
    pub fn peak(&self) -> f32 {
        self.bins.iter().copied().fold(0.0_f32, f32::max)
    }

    /// Circular-mean position of the bright band, in cycles, or None if dark.
    fn band_phase(&self) -> Option<f32> {
        let (mut sx, mut sy) = (0.0f32, 0.0f32);
        let n = self.bins.len() as f32;
        for (i, &b) in self.bins.iter().enumerate() {
            let a = 2.0 * PI * i as f32 / n;
            sx += b * a.cos();
            sy += b * a.sin();
        }
        if sx.abs() < 1e-9 && sy.abs() < 1e-9 {
            return None;
        }
        let mut ph = sy.atan2(sx) / (2.0 * PI);
        if ph < 0.0 {
            ph += 1.0;
        }
        Some(ph)
    }

    /// Update the smoothed apparent drift speed from frame-to-frame band motion.
    fn update_drift(&mut self) {
        let phase = self.band_phase();
        match (self.prev_phase, phase) {
            (Some(p0), Some(p1)) => {
                let mut d = p1 - p0;
                if d > 0.5 {
                    d -= 1.0;
                } else if d <= -0.5 {
                    d += 1.0;
                }
                self.drift = config::DRIFT_SMOOTH * self.drift
                    + (1.0 - config::DRIFT_SMOOTH) * d.abs();
            }
            _ => self.drift *= config::DRIFT_SMOOTH,
        }
        self.prev_phase = phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::Conditioner;

    /// Circular mean of the accumulator, in cycles [0, 1). This tracks the
    /// angular position of the bright band.
    fn band_phase(bins: &[f32]) -> f32 {
        let n = bins.len() as f32;
        let (mut sx, mut sy) = (0.0f32, 0.0f32);
        for (i, &b) in bins.iter().enumerate() {
            let a = 2.0 * PI * i as f32 / n;
            sx += b * a.cos();
            sy += b * a.sin();
        }
        let mut ph = sy.atan2(sx) / (2.0 * PI);
        if ph < 0.0 {
            ph += 1.0;
        }
        ph
    }

    /// Total signed angular drift of the band over the run (after warmup),
    /// unwrapped across the 0/1 seam. Sign encodes drift direction.
    fn net_drift(ref_hz: f32, input_hz: f32) -> f32 {
        let fs = 48_000.0;
        let frame_len = 1024;
        let frames = 48;
        let warmup = 8;

        let mut strobe = Strobe::new(ref_hz, fs);
        let mut cond = Conditioner::new(ref_hz, crate::config::BANDPASS_Q, fs);

        let mut n: u64 = 0;
        let mut phases = Vec::new();
        for _ in 0..frames {
            let mut samples = Vec::with_capacity(frame_len);
            for _ in 0..frame_len {
                let t = n as f32 / fs;
                samples.push(0.3 * (2.0 * PI * input_hz * t).sin());
                n += 1;
            }
            strobe.decay();
            strobe.process(&samples, &mut cond);
            phases.push(band_phase(&strobe.bins));
        }

        let mut total = 0.0;
        for w in phases[warmup..].windows(2) {
            let mut d = w[1] - w[0];
            while d > 0.5 {
                d -= 1.0;
            }
            while d <= -0.5 {
                d += 1.0;
            }
            total += d;
        }
        total
    }

    #[test]
    fn matched_is_stationary_detuned_drifts_oppositely() {
        let matched = net_drift(220.0, 220.0).abs();
        let sharp = net_drift(220.0, 224.0);
        let flat = net_drift(220.0, 216.0);

        // A matched string barely moves compared to a detuned one.
        assert!(
            matched < sharp.abs() && matched < flat.abs(),
            "matched={matched}, sharp={sharp}, flat={flat}"
        );
        // Sharp and flat drift in opposite directions.
        assert!(
            sharp.signum() != flat.signum(),
            "sharp={sharp}, flat={flat} should have opposite signs"
        );
        // And the detuned drift is substantial, not noise.
        assert!(sharp.abs() > 0.25 && flat.abs() > 0.25, "sharp={sharp}, flat={flat}");
    }
}
