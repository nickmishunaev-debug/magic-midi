//! Music theory tables: keys and scales.

use nih_plug::prelude::Enum;

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
pub enum Key {
    #[id = "c"]
    #[name = "C"]
    C,
    #[id = "cs"]
    #[name = "C#"]
    Cs,
    #[id = "d"]
    #[name = "D"]
    D,
    #[id = "ds"]
    #[name = "D#"]
    Ds,
    #[id = "e"]
    #[name = "E"]
    E,
    #[id = "f"]
    #[name = "F"]
    F,
    #[id = "fs"]
    #[name = "F#"]
    Fs,
    #[id = "g"]
    #[name = "G"]
    G,
    #[id = "gs"]
    #[name = "G#"]
    Gs,
    #[id = "a"]
    #[name = "A"]
    A,
    #[id = "as"]
    #[name = "A#"]
    As,
    #[id = "b"]
    #[name = "B"]
    B,
}

impl Key {
    pub const ALL: [Key; 12] = [
        Key::C,
        Key::Cs,
        Key::D,
        Key::Ds,
        Key::E,
        Key::F,
        Key::Fs,
        Key::G,
        Key::Gs,
        Key::A,
        Key::As,
        Key::B,
    ];

    /// Semitones above C (C = 0 .. B = 11).
    pub fn semitone(self) -> i32 {
        self as i32
    }

    pub fn label(self) -> &'static str {
        ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][self as usize]
    }
}

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
pub enum ScaleKind {
    #[id = "major"]
    #[name = "Major"]
    Major,
    #[id = "minor"]
    #[name = "Minor"]
    Minor,
    #[id = "dorian"]
    #[name = "Dorian"]
    Dorian,
    #[id = "phrygian"]
    #[name = "Phrygian"]
    Phrygian,
    #[id = "mixolydian"]
    #[name = "Mixolydian"]
    Mixolydian,
    #[id = "harmonic"]
    #[name = "Harmonic Minor"]
    HarmonicMinor,
    #[id = "minpent"]
    #[name = "Minor Pentatonic"]
    MinorPentatonic,
}

impl ScaleKind {
    pub const ALL: [ScaleKind; 7] = [
        ScaleKind::Major,
        ScaleKind::Minor,
        ScaleKind::Dorian,
        ScaleKind::Phrygian,
        ScaleKind::Mixolydian,
        ScaleKind::HarmonicMinor,
        ScaleKind::MinorPentatonic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ScaleKind::Major => "Major",
            ScaleKind::Minor => "Minor",
            ScaleKind::Dorian => "Dorian",
            ScaleKind::Phrygian => "Phrygian",
            ScaleKind::Mixolydian => "Mixolydian",
            ScaleKind::HarmonicMinor => "Harmonic Minor",
            ScaleKind::MinorPentatonic => "Minor Pentatonic",
        }
    }

    /// Semitone offsets of each scale degree from the tonic.
    pub fn intervals(self) -> &'static [i32] {
        match self {
            ScaleKind::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleKind::Minor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleKind::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            ScaleKind::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            ScaleKind::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            ScaleKind::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            ScaleKind::MinorPentatonic => &[0, 3, 5, 7, 10],
        }
    }

    /// Scale-degree indices that sound "at rest" (the tonic triad).
    pub fn stable(self) -> &'static [usize] {
        match self {
            ScaleKind::MinorPentatonic => &[0, 1, 3],
            _ => &[0, 2, 4],
        }
    }

    /// Degrees that sound unfinished, used to end a "question" phrase.
    pub fn open(self) -> &'static [usize] {
        match self {
            ScaleKind::MinorPentatonic => &[2, 4],
            _ => &[1, 4, 6],
        }
    }

    /// The seven-note scale used to build chords. Five-note scales have no natural stacks of
    /// thirds, so they borrow their parent scale (minor pentatonic -> natural minor).
    pub fn chord_scale(self) -> ScaleKind {
        match self {
            ScaleKind::MinorPentatonic => ScaleKind::Minor,
            other => other,
        }
    }

    /// Converts a scale position to a MIDI note.
    /// `pos` 0 = tonic, 1 = next scale degree, `len` = the tonic one octave up.
    pub fn midi(self, tonic: i32, pos: i32) -> i32 {
        let iv = self.intervals();
        let len = iv.len() as i32;
        tonic + 12 * pos.div_euclid(len) + iv[pos.rem_euclid(len) as usize]
    }
}
