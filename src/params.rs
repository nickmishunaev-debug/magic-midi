//! The plugin's host-visible parameters (saved with the DAW project, automatable).

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

use crate::mood::Mood;
use crate::theory::{Key, ScaleKind};

#[derive(Params)]
pub struct MidiGenParams {
    /// Remembers the editor window size
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "key"]
    pub key: EnumParam<Key>,
    #[id = "scale"]
    pub scale: EnumParam<ScaleKind>,
    #[id = "mood"]
    pub mood: EnumParam<Mood>,
}

impl Default for MidiGenParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(340, 340),
            key: EnumParam::new("Key", Key::A),
            scale: EnumParam::new("Scale", ScaleKind::Minor),
            mood: EnumParam::new("Mood", Mood::Groovy),
        }
    }
}
