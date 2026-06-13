//! Input conditioning. This module filters and rectifies the signal so the
//! virtual lamp responds cleanly to a single string. It deliberately does NOT
//! estimate pitch, compute cents, or make any "in tune" decision.

/// A single biquad section, transposed direct form II.
///
/// Coefficients are the RBJ "cookbook" band-pass with constant 0 dB peak gain,
/// centered on a chosen frequency. Used both for input conditioning (centered
/// on the active reference) and for the detector's per-string energy bank.
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn new() -> Self {
        // Identity passthrough until tuned.
        Biquad { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 }
    }

    /// Recompute band-pass coefficients for a center frequency and Q.
    pub fn set_bandpass(&mut self, center_hz: f32, q: f32, sample_rate: f32) {
        // Guard against degenerate values that would NaN the filter.
        let center = center_hz.clamp(1.0, sample_rate * 0.45);
        let w0 = 2.0 * std::f32::consts::PI * center / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q.max(0.1));

        // RBJ band-pass (constant 0 dB peak gain).
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Turns raw mono samples into a virtual-lamp brightness value.
///
/// Path: band-pass around the reference -> half-wave rectify. Half-wave
/// rectification produces a single bright excursion per input cycle, which
/// reads as one clean band on the wheel.
pub struct Conditioner {
    bandpass: Biquad,
    q: f32,
    sample_rate: f32,
}

impl Conditioner {
    pub fn new(reference_hz: f32, q: f32, sample_rate: f32) -> Self {
        let mut bandpass = Biquad::new();
        bandpass.set_bandpass(reference_hz, q, sample_rate);
        Conditioner { bandpass, q, sample_rate }
    }

    /// Re-center the band-pass when the active reference changes.
    pub fn set_reference(&mut self, reference_hz: f32) {
        self.bandpass.set_bandpass(reference_hz, self.q, self.sample_rate);
    }

    /// Conditioned lamp brightness for one input sample (>= 0).
    #[inline]
    pub fn lamp(&mut self, sample: f32) -> f32 {
        self.bandpass.process(sample).max(0.0)
    }
}
