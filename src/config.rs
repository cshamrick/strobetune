//! Runtime constants and defaults.

/// Number of phase bins around the virtual wheel. This is the angular
/// resolution of the accumulated strobe image, independent of terminal width.
pub const STROBE_BINS: usize = 240;

/// Per-frame multiplicative decay applied to the strobe accumulator. This is
/// the "visual persistence" of the display: lower = shorter trails / snappier,
/// higher = longer trails / smoother but laggier motion.
pub const ACCUM_DECAY: f32 = 0.86;

/// Q of the bandpass centered on the active reference frequency. Higher Q
/// isolates the fundamental more tightly (tames guitar harmonics) but rings
/// longer. This is conditioning only — it never estimates pitch.
pub const BANDPASS_Q: f32 = 6.0;

/// RMS input level below which we treat the signal as silence: the lamp stays
/// dark and nothing is accumulated. Prevents background noise from drifting.
pub const NOISE_GATE: f32 = 0.0030;

/// Upper bound on samples buffered between the audio callback and the UI loop.
/// If the consumer falls behind, the oldest samples are dropped.
pub const MAX_QUEUED_SAMPLES: usize = 19_200;

/// Event-poll / frame budget in milliseconds (~60 fps).
pub const FRAME_POLL_MS: u64 = 16;

/// Used only if the input device's sample rate can't be queried.
pub const DEFAULT_SAMPLE_RATE: f32 = 48_000.0;

/// Clamp for the transpose control, in semitones.
pub const TRANSPOSE_LIMIT: i32 = 12;

// --- Auto string detection (opt-in) ---------------------------------------
// Detection uses a YIN (autocorrelation) pitch estimate ONLY to choose which
// string is selected; it is kept entirely out of the strobe display path,
// which remains pure phase folding. Energy banks can't separate a fundamental
// from a coincident harmonic, but periodicity-based pitch detection can.

/// Lowest / highest fundamentals the detector will look for, in Hz. These set
/// the autocorrelation lag range (and so the analysis buffer length).
pub const DETECT_MIN_HZ: f32 = 60.0;
pub const DETECT_MAX_HZ: f32 = 1400.0;

/// YIN absolute threshold: the first cumulative-mean-normalized dip below this
/// is taken as the period. Lower biases toward the true fundamental (rejects
/// octave errors); too low and weak/noisy input won't register.
pub const DETECT_YIN_THRESHOLD: f32 = 0.15;

/// Subharmonic correction. After YIN picks a period, a shorter period that is an
/// integer fraction of it and whose dip is within this margin of the found dip
/// is treated as the true fundamental. This unwinds octave/harmonic-down errors
/// (e.g. high E sliding onto the A string it sympathetically rings) while
/// leaving genuinely-played low strings — whose own low dip is the deepest —
/// untouched.
pub const DETECT_SUBHARMONIC_MARGIN: f32 = 0.10;

/// How close (in semitones) the detected pitch must be to a string for it to be
/// selected. Generous enough for a badly out-of-tune string, tight enough to
/// reject stray sounds.
pub const DETECT_TOLERANCE_SEMITONES: f32 = 2.0;

/// Frames a candidate must stay the winner before the first lock (hysteresis,
/// so the selection doesn't flicker mid-pluck or during decay).
pub const DETECT_HOLD_FRAMES: u32 = 4;

/// Frames a *different* string must stay the winner before it replaces the
/// current selection. Larger than the initial lock so a brief, ambiguous reading
/// (e.g. the open A ringing sympathetically while a high E decays) can't yank the
/// selection away from the string you're actually tuning.
pub const DETECT_SWITCH_FRAMES: u32 = 16;

// --- Drift-speed colouring ------------------------------------------------

/// Smoothing for the measured drift speed (higher = steadier colour).
pub const DRIFT_SMOOTH: f32 = 0.85;

/// Apparent drift speed (cycles per frame) at which the colour reaches the
/// "fast" end of the ramp. Tune to taste against a real instrument.
pub const DRIFT_FULL_RED: f32 = 0.020;

/// Colour ramp for drift speed, in HSV. A calm, cool palette: a still (in-tune)
/// band reads teal and shifts toward blue as it drifts. Lower saturation keeps
/// it easy on the eyes.
pub const DRIFT_HUE_STILL: f32 = 175.0;
pub const DRIFT_HUE_FAST: f32 = 220.0;
pub const DRIFT_SATURATION: f32 = 0.6;
