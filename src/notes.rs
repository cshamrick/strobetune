//! Note names, tuning presets, and transpose helpers.
//!
//! Frequencies are hardcoded for the MVP. Transpose is applied as a pure
//! function of a base frequency and a semitone offset; preset definitions are
//! never mutated.

/// Chromatic scale spelled with flats, which matches the desired display names
/// (e.g. E down one semitone reads as "E♭", not "D♯").
pub const FLAT_NAMES: [&str; 12] = [
    "C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭", "B",
];

/// One open string within a tuning preset.
#[derive(Clone, Copy)]
pub struct GuitarString {
    /// Display label including octave, e.g. "E2".
    pub label: &'static str,
    /// Base frequency in Hz, before transpose.
    pub freq: f32,
}

/// A hardcoded tuning preset.
pub struct Tuning {
    /// Stable identifier, e.g. "standard". Reserved for future config/saving.
    #[allow(dead_code)]
    pub id: &'static str,
    /// Name prefix; the transposed root note is appended to form the label.
    /// "Standard" + "E" -> "Standard E"; "Drop" + "D" -> "Drop D".
    pub prefix: &'static str,
    /// Root/key pitch class as an index into [`FLAT_NAMES`].
    pub root_pc: i32,
    /// The six open strings. Index 0 == string "1" in the UI.
    pub strings: [GuitarString; 6],
}

impl Tuning {
    /// Displayed tuning name for a given transpose, e.g. "Standard E♭".
    pub fn display_name(&self, transpose: i32) -> String {
        format!("{} {}", self.prefix, transposed_pitch_class(self.root_pc, transpose))
    }
}

/// Name of a pitch class shifted by a number of semitones.
pub fn transposed_pitch_class(root_pc: i32, semitones: i32) -> &'static str {
    FLAT_NAMES[(root_pc + semitones).rem_euclid(12) as usize]
}

/// Apply transpose to a base frequency: f * 2^(semitones / 12).
pub fn active_frequency(base_freq: f32, semitones: i32) -> f32 {
    base_freq * 2.0_f32.powf(semitones as f32 / 12.0)
}

/// The hardcoded preset list, in cycle order.
pub const TUNINGS: [Tuning; 3] = [
    Tuning {
        id: "standard",
        prefix: "Standard",
        root_pc: 4, // E
        strings: [
            GuitarString { label: "E2", freq: 82.4069 },
            GuitarString { label: "A2", freq: 110.0000 },
            GuitarString { label: "D3", freq: 146.8324 },
            GuitarString { label: "G3", freq: 195.9977 },
            GuitarString { label: "B3", freq: 246.9417 },
            GuitarString { label: "E4", freq: 329.6276 },
        ],
    },
    Tuning {
        id: "drop_d",
        prefix: "Drop",
        root_pc: 2, // D
        strings: [
            GuitarString { label: "D2", freq: 73.4162 },
            GuitarString { label: "A2", freq: 110.0000 },
            GuitarString { label: "D3", freq: 146.8324 },
            GuitarString { label: "G3", freq: 195.9977 },
            GuitarString { label: "B3", freq: 246.9417 },
            GuitarString { label: "E4", freq: 329.6276 },
        ],
    },
    Tuning {
        id: "open_g",
        prefix: "Open",
        root_pc: 7, // G
        strings: [
            GuitarString { label: "D2", freq: 73.4162 },
            GuitarString { label: "G2", freq: 97.9989 },
            GuitarString { label: "D3", freq: 146.8324 },
            GuitarString { label: "G3", freq: 195.9977 },
            GuitarString { label: "B3", freq: 246.9417 },
            GuitarString { label: "D4", freq: 293.6648 },
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_match_spec() {
        let std = &TUNINGS[0];
        assert_eq!(std.display_name(0), "Standard E");
        assert_eq!(std.display_name(-1), "Standard E♭");
        assert_eq!(std.display_name(-2), "Standard D");
        assert_eq!(std.display_name(1), "Standard F");

        let drop_d = &TUNINGS[1];
        assert_eq!(drop_d.display_name(0), "Drop D");
        assert_eq!(drop_d.display_name(-1), "Drop D♭");

        let open_g = &TUNINGS[2];
        assert_eq!(open_g.display_name(0), "Open G");
        assert_eq!(open_g.display_name(2), "Open A");
    }

    #[test]
    fn transpose_is_a_pure_semitone_ratio() {
        // +12 semitones doubles the frequency; -12 halves it.
        let base = 110.0;
        assert!((active_frequency(base, 12) - 220.0).abs() < 1e-3);
        assert!((active_frequency(base, -12) - 55.0).abs() < 1e-3);
        assert!((active_frequency(base, 0) - base).abs() < 1e-6);
    }
}
