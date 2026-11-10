//! Moods: friendly presets that decide all the technical melody settings.
//!
//! To add a mood: add a variant, list it in `ALL`, and fill in `label`, `tagline` and `settings`.

use nih_plug::prelude::Enum;

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
pub enum Mood {
    #[id = "dreamy"]
    #[name = "Dreamy"]
    Dreamy,
    #[id = "uplifting"]
    #[name = "Uplifting"]
    Uplifting,
    #[id = "dark"]
    #[name = "Dark"]
    Dark,
    #[id = "groovy"]
    #[name = "Groovy"]
    Groovy,
    #[id = "hypnotic"]
    #[name = "Hypnotic"]
    Hypnotic,
    #[id = "emotional"]
    #[name = "Emotional"]
    Emotional,
}

/// Everything technical that a mood decides, so the user never has to.
#[derive(Clone, Copy)]
pub struct MoodSettings {
    /// Melody length in bars before it loops
    pub bars: u32,
    /// 0..1, how busy the rhythm is
    pub density: f32,
    /// Fraction of each note's slot that is held
    pub gate: f64,
    /// Base loudness (1..127)
    pub velocity: i32,
    /// Register shift in octaves
    pub octave: i32,
    /// >0 leans upward, <0 leans downward
    pub up_bias: f32,
    /// How often the melody jumps instead of stepping (1.0 = normal)
    pub leapiness: f32,
    /// Repeat the hook exactly (riff-style)
    pub repeat: bool,
}

impl Mood {
    pub const ALL: [Mood; 6] = [
        Mood::Dreamy,
        Mood::Uplifting,
        Mood::Dark,
        Mood::Groovy,
        Mood::Hypnotic,
        Mood::Emotional,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mood::Dreamy => "Dreamy",
            Mood::Uplifting => "Uplifting",
            Mood::Dark => "Dark",
            Mood::Groovy => "Groovy",
            Mood::Hypnotic => "Hypnotic",
            Mood::Emotional => "Emotional",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Mood::Dreamy => "Sparse and floating, with long soft notes",
            Mood::Uplifting => "Bright and rising, with bigger melodic jumps",
            Mood::Dark => "Low, tense and sparse",
            Mood::Groovy => "Busy, syncopated and bouncy",
            Mood::Hypnotic => "A short riff that locks in and repeats",
            Mood::Emotional => "Slow, falling lines with long, aching notes",
        }
    }

    pub fn settings(self) -> MoodSettings {
        match self {
            Mood::Dreamy => MoodSettings {
                bars: 4,
                density: 0.1,
                gate: 0.95,
                velocity: 80,
                octave: 0,
                up_bias: 0.05,
                leapiness: 0.8,
                repeat: false,
            },
            Mood::Uplifting => MoodSettings {
                bars: 4,
                density: 0.6,
                gate: 0.75,
                velocity: 105,
                octave: 0,
                up_bias: 0.12,
                leapiness: 1.4,
                repeat: false,
            },
            Mood::Dark => MoodSettings {
                bars: 2,
                density: 0.3,
                gate: 0.6,
                velocity: 95,
                octave: -1,
                up_bias: -0.08,
                leapiness: 0.7,
                repeat: false,
            },
            Mood::Groovy => MoodSettings {
                bars: 2,
                density: 0.85,
                gate: 0.45,
                velocity: 108,
                octave: -1,
                up_bias: 0.0,
                leapiness: 1.0,
                repeat: false,
            },
            Mood::Hypnotic => MoodSettings {
                bars: 4,
                density: 0.4,
                gate: 0.7,
                velocity: 90,
                octave: -1,
                up_bias: 0.0,
                leapiness: 0.4,
                repeat: true,
            },
            Mood::Emotional => MoodSettings {
                bars: 4,
                density: 0.15,
                gate: 1.0,
                velocity: 85,
                octave: 0,
                up_bias: -0.1,
                leapiness: 0.6,
                repeat: false,
            },
        }
    }
}
