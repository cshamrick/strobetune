//! Optional automatic string detection.
//!
//! This selects *which string is being played* using a YIN pitch estimate
//! (de Cheveigné & Kawahara, 2002) — an autocorrelation-style method that finds
//! the signal's period. Unlike an energy bank, it separates a fundamental from
//! its harmonics by looking at periodicity over time, which is exactly the
//! ambiguity that defeats per-band energy in standard tuning (a low E's
//! overtones land on the B and high-E bands; high E sympathetically rings the
//! low E string).
//!
//! Crucially, this runs ONLY to pick the selected string. The strobe display
//! itself never sees a pitch estimate — it stays pure phase folding against the
//! selected reference. Detection is a convenience for choosing the note; the
//! tuning feedback is still the stroboscopic drift.

use crate::config;

pub struct StringDetector {
    sample_rate: f32,
    /// Active string frequencies (with transpose applied).
    freqs: Vec<f32>,
    /// Rolling window of recent mono samples.
    buffer: Vec<f32>,
    buffer_cap: usize,
    tau_min: usize,
    tau_max: usize,
    /// Committed selection, once hysteresis is satisfied.
    current: Option<usize>,
    /// Candidate awaiting confirmation, and how many frames it has held.
    pending: Option<usize>,
    hold: u32,
}

impl StringDetector {
    pub fn new(freqs: &[f32], sample_rate: f32) -> Self {
        let tau_min = (sample_rate / config::DETECT_MAX_HZ).floor().max(1.0) as usize;
        let tau_max = (sample_rate / config::DETECT_MIN_HZ).ceil() as usize;
        // ~3 lags of headroom gives the difference function a couple of periods
        // of the lowest note to work with.
        let buffer_cap = tau_max * 3;
        StringDetector {
            sample_rate,
            freqs: freqs.to_vec(),
            buffer: Vec::with_capacity(buffer_cap),
            buffer_cap,
            tau_min,
            tau_max,
            current: None,
            pending: None,
            hold: 0,
        }
    }

    /// Update the candidate string set when tuning or transpose changes.
    pub fn set_frequencies(&mut self, freqs: &[f32]) {
        self.freqs = freqs.to_vec();
    }

    /// The currently committed string index, if any.
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    /// Feed new samples and re-evaluate the selected string.
    pub fn process(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);
        if self.buffer.len() > self.buffer_cap {
            let excess = self.buffer.len() - self.buffer_cap;
            self.buffer.drain(0..excess);
        }
        if self.buffer.len() < self.buffer_cap {
            return; // not enough history yet
        }

        // Gate on level so silence/noise holds the current selection.
        let rms = (self.buffer.iter().map(|s| s * s).sum::<f32>() / self.buffer.len() as f32).sqrt();
        if rms < config::NOISE_GATE {
            self.pending = None;
            self.hold = 0;
            return;
        }

        let candidate = self
            .estimate_pitch()
            .and_then(|f0| self.nearest_string(f0));

        let Some(candidate) = candidate else {
            self.pending = None;
            self.hold = 0;
            return;
        };

        if self.pending == Some(candidate) {
            self.hold += 1;
        } else {
            self.pending = Some(candidate);
            self.hold = 0;
        }
        if self.hold >= config::DETECT_HOLD_FRAMES {
            self.current = Some(candidate);
        }
    }

    /// YIN fundamental estimate over the current buffer, in Hz.
    fn estimate_pitch(&self) -> Option<f32> {
        let n = self.buffer.len();
        let tau_max = self.tau_max.min(n / 2);
        if tau_max <= self.tau_min {
            return None;
        }
        let w = n - tau_max; // comparison window

        // 1. Difference function.
        let mut diff = vec![0.0f32; tau_max + 1];
        for (tau, slot) in diff.iter_mut().enumerate().take(tau_max + 1).skip(1) {
            let mut sum = 0.0;
            for j in 0..w {
                let d = self.buffer[j] - self.buffer[j + tau];
                sum += d * d;
            }
            *slot = sum;
        }

        // 2. Cumulative mean normalized difference.
        let mut cmnd = vec![1.0f32; tau_max + 1];
        let mut running = 0.0;
        for tau in 1..=tau_max {
            running += diff[tau];
            if running > 0.0 {
                cmnd[tau] = diff[tau] * tau as f32 / running;
            }
        }

        // 3. Absolute threshold: first dip below the threshold, descended to its
        //    local minimum. Taking the *first* dip (smallest lag) rejects the
        //    octave-below error; descending to the local min sharpens it.
        let mut tau = self.tau_min.max(1);
        let mut best = None;
        while tau <= tau_max {
            if cmnd[tau] < config::DETECT_YIN_THRESHOLD {
                while tau < tau_max && cmnd[tau + 1] < cmnd[tau] {
                    tau += 1;
                }
                best = Some(tau);
                break;
            }
            tau += 1;
        }
        let tau = best?;

        // 4. Parabolic interpolation around the minimum for sub-sample accuracy.
        let refined = if tau > self.tau_min && tau < tau_max {
            let x0 = cmnd[tau - 1];
            let x1 = cmnd[tau];
            let x2 = cmnd[tau + 1];
            let denom = 2.0 * (x0 - 2.0 * x1 + x2);
            if denom.abs() > 1e-9 {
                tau as f32 + (x0 - x2) / denom
            } else {
                tau as f32
            }
        } else {
            tau as f32
        };

        Some(self.sample_rate / refined)
    }

    /// Nearest string to a detected pitch, if within tolerance.
    fn nearest_string(&self, f0: f32) -> Option<usize> {
        let mut best: Option<usize> = None;
        let mut best_dist = f32::MAX;
        for (i, &f) in self.freqs.iter().enumerate() {
            let dist = (f0 / f).log2().abs() * 12.0; // semitones
            if dist < best_dist {
                best_dist = dist;
                best = Some(i);
            }
        }
        match best {
            Some(i) if best_dist <= config::DETECT_TOLERANCE_SEMITONES => Some(i),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::{active_frequency, TUNINGS};
    use std::f32::consts::PI;

    fn freqs_of(tuning_idx: usize) -> Vec<f32> {
        TUNINGS[tuning_idx]
            .strings
            .iter()
            .map(|s| active_frequency(s.freq, 0))
            .collect()
    }

    /// Feed a set of (frequency, amplitude) partials and return the selection.
    fn detect(freqs: &[f32], partials: &[(f32, f32)]) -> Option<usize> {
        let fs = 48_000.0;
        let mut det = StringDetector::new(freqs, fs);
        let mut n: u64 = 0;
        for _ in 0..40 {
            let mut samples = Vec::with_capacity(1024);
            for _ in 0..1024 {
                let t = n as f32 / fs;
                let v: f32 = partials
                    .iter()
                    .map(|&(f, a)| a * (2.0 * PI * f * t).sin())
                    .sum();
                samples.push(v);
                n += 1;
            }
            det.process(&samples);
        }
        det.current()
    }

    #[test]
    fn low_e_with_weak_fundamental_and_strong_harmonics() {
        // The case that defeated the energy bank: a low E whose 3rd/4th harmonics
        // land on the B3 and E4 bands and are louder than the 82 Hz fundamental.
        let freqs = freqs_of(0);
        let f0 = freqs[0];
        let partials = [
            (f0, 0.10),
            (2.0 * f0, 0.30),
            (3.0 * f0, 0.50), // ~B3 band
            (4.0 * f0, 0.30), // = E4 band
        ];
        assert_eq!(detect(&freqs, &partials), Some(0));
    }

    #[test]
    fn high_e_with_sympathetic_low_e() {
        // High E dominant, with a weaker low-E series ringing sympathetically.
        // YIN's "first dip" should lock to the high-E period, not the low E.
        let freqs = freqs_of(0);
        let e4 = freqs[5];
        let e2 = freqs[0];
        let partials = [
            (e4, 0.50),
            (2.0 * e4, 0.20),
            (e2, 0.08),
            (2.0 * e2, 0.06),
            (4.0 * e2, 0.06),
        ];
        assert_eq!(detect(&freqs, &partials), Some(5));
    }

    #[test]
    fn a_string_with_third_harmonic_on_high_e() {
        let freqs = freqs_of(0);
        let a2 = freqs[1];
        let partials = [(a2, 0.25), (2.0 * a2, 0.20), (3.0 * a2, 0.40)]; // 3rd ~ E4
        assert_eq!(detect(&freqs, &partials), Some(1));
    }

    #[test]
    fn drop_d_low_d_not_its_octave() {
        let freqs = freqs_of(1); // Drop D
        let d2 = freqs[0];
        let partials = [(d2, 0.15), (2.0 * d2, 0.50), (3.0 * d2, 0.20)];
        assert_eq!(detect(&freqs, &partials), Some(0));
    }

    #[test]
    fn middle_string_clean() {
        let freqs = freqs_of(0);
        let g3 = freqs[3];
        assert_eq!(detect(&freqs, &[(g3, 0.4), (2.0 * g3, 0.2)]), Some(3));
    }

    #[test]
    fn silence_yields_no_detection() {
        let freqs = freqs_of(0);
        assert_eq!(detect(&freqs, &[]), None);
    }
}
