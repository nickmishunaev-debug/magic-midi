//! The top-level mode of the plugin: what kind of MIDI it generates.

use nih_plug::prelude::Enum;

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    #[id = "melody"]
    #[name = "Melody"]
    Melody,
    #[id = "chords"]
    #[name = "Chords"]
    Chords,
}

impl Mode {
    pub const ALL: [Mode; 2] = [Mode::Melody, Mode::Chords];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Melody => "Melody",
            Mode::Chords => "Chords",
        }
    }
}
