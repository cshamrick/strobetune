//! Application state and input handling.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::audio::AudioHandle;
use crate::config;
use crate::detector::StringDetector;
use crate::dsp::Conditioner;
use crate::notes::{self, TUNINGS};
use crate::strobe::Strobe;

pub struct App {
    pub tuning_idx: usize,
    /// Selected string, 0-based (UI shows it as 1..=6).
    pub string_idx: usize,
    pub transpose: i32,
    /// Whether automatic string detection is driving the selection.
    pub auto: bool,
    pub strobe: Strobe,
    pub cond: Conditioner,
    pub detector: StringDetector,
    pub audio: AudioHandle,
    pub should_quit: bool,
}

impl App {
    pub fn new(audio: AudioHandle) -> Self {
        let tuning_idx = 0;
        let string_idx = 0;
        let transpose = 0;
        let ref_freq =
            notes::active_frequency(TUNINGS[tuning_idx].strings[string_idx].freq, transpose);
        let strobe = Strobe::new(ref_freq, audio.sample_rate);
        let cond = Conditioner::new(ref_freq, config::BANDPASS_Q, audio.sample_rate);
        let detector =
            StringDetector::new(&active_freqs(tuning_idx, transpose), audio.sample_rate);
        App {
            tuning_idx,
            string_idx,
            transpose,
            auto: true,
            strobe,
            cond,
            detector,
            audio,
            should_quit: false,
        }
    }

    /// The currently selected open string before transpose.
    pub fn current_string(&self) -> &notes::GuitarString {
        &TUNINGS[self.tuning_idx].strings[self.string_idx]
    }

    /// Active reference frequency driving the wheel (string + transpose).
    pub fn active_freq(&self) -> f32 {
        notes::active_frequency(self.current_string().freq, self.transpose)
    }

    /// Displayed tuning name, e.g. "Standard E♭".
    pub fn tuning_name(&self) -> String {
        TUNINGS[self.tuning_idx].display_name(self.transpose)
    }

    /// Push the active reference into the wheel and the conditioning filter.
    /// Used whenever the selected string changes (string, tuning, or transpose).
    fn retune(&mut self) {
        let f = self.active_freq();
        self.strobe.set_ref_freq(f);
        self.cond.set_reference(f);
    }

    /// Re-center the detection bank. Only needed when the candidate set of
    /// frequencies changes — i.e. on tuning or transpose, not on string select.
    fn refresh_detector(&mut self) {
        self.detector
            .set_frequencies(&active_freqs(self.tuning_idx, self.transpose));
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            // String selection 1..=6. A manual choice leaves auto mode.
            KeyCode::Char(c @ '1'..='6') => {
                self.auto = false;
                self.string_idx = (c as u8 - b'1') as usize;
                self.retune();
            }

            // Toggle automatic string detection.
            KeyCode::Char('a') => self.auto = !self.auto,

            // Transpose reset (0 is not a valid string, so no conflict).
            KeyCode::Char('0') => {
                self.transpose = 0;
                self.retune();
                self.refresh_detector();
            }

            // Tuning preset cycling: 't' next, 'T' (Shift) previous.
            KeyCode::Char('t') => {
                self.tuning_idx = (self.tuning_idx + 1) % TUNINGS.len();
                self.retune();
                self.refresh_detector();
            }
            KeyCode::Char('T') => {
                self.tuning_idx = (self.tuning_idx + TUNINGS.len() - 1) % TUNINGS.len();
                self.retune();
                self.refresh_detector();
            }

            // Transpose by semitone.
            KeyCode::Char('[') => {
                self.transpose = (self.transpose - 1).max(-config::TRANSPOSE_LIMIT);
                self.retune();
                self.refresh_detector();
            }
            KeyCode::Char(']') => {
                self.transpose = (self.transpose + 1).min(config::TRANSPOSE_LIMIT);
                self.retune();
                self.refresh_detector();
            }

            _ => {}
        }
    }

    /// Advance the simulation one frame: update the detector, optionally let it
    /// pick the string, fade the image, then fold in any new audio.
    pub fn update(&mut self) {
        let samples = self.audio.drain();

        self.detector.process(&samples);
        if self.auto {
            if let Some(idx) = self.detector.current() {
                if idx != self.string_idx {
                    self.string_idx = idx;
                    self.retune();
                }
            }
        }

        self.strobe.decay();
        self.strobe.process(&samples, &mut self.cond);
    }
}

/// Active frequencies of all six strings for a tuning at a given transpose.
fn active_freqs(tuning_idx: usize, transpose: i32) -> [f32; 6] {
    let mut out = [0.0; 6];
    for (i, s) in TUNINGS[tuning_idx].strings.iter().enumerate() {
        out[i] = notes::active_frequency(s.freq, transpose);
    }
    out
}
