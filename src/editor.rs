//! The plugin window (egui). To add a control: add a parameter in `params.rs`, then draw it here.

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::mood::Mood;
use crate::params::MidiGenParams;
use crate::theory::{Key, ScaleKind};

/// A dropdown for an enum parameter.
fn enum_combo<E>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    label: &str,
    param: &EnumParam<E>,
    all: &[E],
    name: fn(E) -> &'static str,
) where
    E: Enum + PartialEq + Copy + 'static,
{
    let cur = param.value();
    egui::ComboBox::from_label(label)
        .selected_text(name(cur))
        .show_ui(ui, |ui| {
            for &v in all {
                if ui.selectable_label(v == cur, name(v)).clicked() {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, v);
                    setter.end_set_parameter(param);
                }
            }
        });
}

/// Builds the editor. `regen` is set to true when the Regenerate button is clicked.
pub fn create(params: Arc<MidiGenParams>, regen: Arc<AtomicBool>) -> Option<Box<dyn Editor>> {
    let state = params.editor_state.clone();
    create_egui_editor(
        state,
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            egui::CentralPanel::default().show(egui_ctx, |ui| {
                ui.heading("MIDI Gen");
                ui.add_space(8.0);

                enum_combo(ui, setter, "Key", &params.key, &Key::ALL, Key::label);
                enum_combo(ui, setter, "Scale", &params.scale, &ScaleKind::ALL, ScaleKind::label);
                ui.add_space(10.0);

                ui.label("Mood");
                let cur = params.mood.value();
                ui.horizontal_wrapped(|ui| {
                    for &m in &Mood::ALL {
                        if ui.selectable_label(m == cur, m.label()).clicked() {
                            setter.begin_set_parameter(&params.mood);
                            setter.set_parameter(&params.mood, m);
                            setter.end_set_parameter(&params.mood);
                        }
                    }
                });
                ui.small(cur.tagline());
                ui.add_space(14.0);

                if ui
                    .add_sized([ui.available_width(), 40.0], egui::Button::new("Regenerate"))
                    .clicked()
                {
                    regen.store(true, Ordering::Relaxed);
                }
            });
        },
    )
}
