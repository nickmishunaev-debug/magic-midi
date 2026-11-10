//! MIDI Gen: a VST3 plugin that generates melodies as MIDI.
//!
//! Module map:
//!   theory.rs  keys and scales
//!   mood.rs    mood presets (all the technical melody settings)
//!   rng.rs     tiny random number generator
//!   melody.rs  melody generation (rhythm, hook, call and response)
//!   engine.rs  playback: transport position -> timed MIDI events
//!   params.rs  host-visible parameters
//!   editor.rs  the egui window
//!   lib.rs     this file: glue between nih-plug and the engine

mod editor;
mod engine;
mod melody;
mod mood;
mod params;
mod rng;
mod theory;

use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use engine::{BlockInput, Engine, MIDI_CHANNEL};
use params::MidiGenParams;

struct MidiGen {
    params: Arc<MidiGenParams>,
    engine: Engine,
    /// Set by the Regenerate button (GUI thread), consumed by the audio thread
    regen: Arc<AtomicBool>,
}

impl Default for MidiGen {
    fn default() -> Self {
        Self {
            params: Arc::new(MidiGenParams::default()),
            engine: Engine::new(),
            regen: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Plugin for MidiGen {
    const NAME: &'static str = "MIDI Gen";
    const VENDOR: &'static str = "Nik";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    // Live requires an event input bus on instrument-category plugins
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.regen.clone())
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // We only emit MIDI, so keep the audio output silent
        for channel_samples in buffer.iter_samples() {
            for sample in channel_samples {
                *sample = 0.0;
            }
        }

        let num_samples = buffer.samples();

        let input = {
            let t = context.transport();
            let playing = t.playing;
            BlockInput {
                playing,
                tempo: t.tempo.unwrap_or(120.0),
                sample_rate: t.sample_rate as f64,
                host_pos: t.pos_beats(),
                num_samples,
                key: self.params.key.value(),
                scale: self.params.scale.value(),
                mood: self.params.mood.value(),
                // Only consume the button press while playing, so pressing Regenerate
                // while stopped takes effect when playback starts
                regen: playing && self.regen.swap(false, Ordering::Relaxed),
            }
        };

        for e in self.engine.process(&input) {
            if e.on {
                context.send_event(NoteEvent::NoteOn {
                    timing: e.timing,
                    voice_id: None,
                    channel: MIDI_CHANNEL,
                    note: e.note,
                    velocity: e.velocity,
                });
            } else {
                context.send_event(NoteEvent::NoteOff {
                    timing: e.timing,
                    voice_id: None,
                    channel: MIDI_CHANNEL,
                    note: e.note,
                    velocity: 0.0,
                });
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for MidiGen {
    const VST3_CLASS_ID: [u8; 16] = *b"NikMidiGenPlugin";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument];
}

nih_export_vst3!(MidiGen);
