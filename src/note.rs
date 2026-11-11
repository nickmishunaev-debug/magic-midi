//! The note type shared by every generator, so the engine can play any of them the same way.

/// One note of a generated loop.
#[derive(Clone, Copy)]
pub struct Note {
    /// Start, in 16th-note steps from the start of the loop
    pub step: u32,
    /// Length in 16th-note steps (before the mood's gate is applied)
    pub len: u32,
    /// Pitch in semitones relative to the tonic (the key's root)
    pub semi: i32,
    /// Accent relative to the mood's base velocity
    pub vel_off: i32,
    /// Small delay in beats after the step, used to strum chords
    pub offset: f32,
}
