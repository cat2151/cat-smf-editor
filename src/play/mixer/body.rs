//! The mixer's faders and effect rows, drawn into a floating window or a pane.
use super::{effect_name, gain_to_db, mixer_effect_name, EFFECT_COLOR};
use crate::app::App;
use cat_smf_editor::score::layer_track::Patch;
use eframe::egui;
use std::borrow::Cow;

const COLUMN_WIDTH: f32 = 140.0;
const ROW_HEIGHT: f32 = 20.0;

/// What the user did in one frame; applied once after the mixer is drawn.
#[derive(Default)]
pub(crate) struct Outcome {
    pub chain: Option<(crate::view::effects::Target, usize)>,
    /// Track, gain and whether the fader is still held.
    pub volume_change: Option<(usize, f32, bool)>,
}

pub(super) fn show(app: &mut App, ui: &mut egui::Ui) -> Outcome {
    let mut outcome = Outcome::default();
    let chain = &mut outcome.chain;
    let volume_change = &mut outcome.volume_change;
    let mut auto_gain = app.audition.auto_gain_enabled();
    if ui.checkbox(&mut auto_gain, "Master auto gain").changed() {
        app.audition.set_auto_gain_enabled(auto_gain);
    }
    ui.separator();
    ui.horizontal(|ui| {
        let enabled = app.loaded.as_ref().is_some_and(|l| l.music.format == 1);
        if let Some(slot) = master_row(
            ui,
            &app.audition.master().effects,
            app.playback.mixer_master,
            enabled,
        ) {
            app.playback.mixer_master = true;
            *chain = Some((crate::view::effects::Target::Master, slot));
        }
    });
    let Some(loaded) = &app.loaded else {
        ui.label("Open an SMF first");
        return outcome;
    };
    let count = loaded.track_titles.len();
    let effect_rows = loaded
        .patches
        .iter()
        .map(|p| p.effects.len())
        .max()
        .unwrap_or(0);
    app.playback.mixer_track = app.playback.mixer_track.min(count.saturating_sub(1));
    let selection = (
        app.generation,
        app.playback.mixer_track,
        app.playback.mixer_master,
    );
    let selection_id = ui.id().with("mixer-selection");
    let reveal = ui.data_mut(|data| {
        let changed = data.get_temp::<(u64, usize, bool)>(selection_id) != Some(selection);
        data.insert_temp(selection_id, selection);
        changed
    });
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal_top(|ui| {
            for (track, title) in loaded.track_titles.iter().enumerate() {
                ui.push_id(track, |ui| {
                    let selected = !app.playback.mixer_master && track == app.playback.mixer_track;
                    let fill = if selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let frame = egui::Frame::new().fill(fill).show(ui, |ui| {
                        ui.set_width(COLUMN_WIDTH);
                        ui.vertical(|ui| {
                            let header = ui.add_sized(
                                [COLUMN_WIDTH, ROW_HEIGHT],
                                egui::Button::new(title).selected(selected).truncate(),
                            );
                            if header.clicked() {
                                app.playback.mixer_master = false;
                                app.playback.mixer_track = track;
                            }
                            let patch = &loaded.patches[track];
                            name_label(ui, patch.label().into(), None, &instrument_tooltip(patch));
                            let effects = &patch.effects;
                            if let Some(slot) = effect_labels(ui, effects) {
                                *chain = Some((crate::view::effects::Target::Track(track), slot));
                            }
                            // Every strip reserves the same number of effect rows, including
                            // their trailing spacing, so gains share one vertical coordinate.
                            for _ in effects.len()..effect_rows {
                                ui.allocate_exact_size(
                                    egui::vec2(COLUMN_WIDTH, ROW_HEIGHT),
                                    egui::Sense::hover(),
                                );
                            }
                            let mut db = gain_to_db(app.audition.gain(track));
                            let response = super::fader::show(ui, &mut db);
                            if response.dragged() || response.clicked() {
                                app.playback.mixer_master = false;
                                app.playback.mixer_track = track;
                            }
                            if response.changed() {
                                app.playback.mixer_master = false;
                                app.playback.mixer_track = track;
                                *volume_change = Some((
                                    track,
                                    crate::play::track_volume::db_to_gain(db),
                                    response.is_pointer_button_down_on(),
                                ));
                            }
                            ui.label(format!("{db:+} dB"));
                        });
                    });
                    if selected && reveal {
                        frame.response.scroll_to_me(Some(egui::Align::Center));
                    }
                });
            }
        });
    });
    outcome
}

fn instrument_tooltip(patch: &Patch) -> Cow<'_, str> {
    let label = patch.label();
    let Some(instrument) = patch
        .layers
        .first()
        .and_then(|layer| layer.instrument.as_ref())
    else {
        return label.into();
    };
    let plugin = instrument.plugin.name.as_str();
    match label {
        "" => plugin.into(),
        _ if plugin.is_empty() || plugin == label => label.into(),
        _ => format!("{plugin}\n{label}").into(),
    }
}

/// One label per chained effect; returns the clicked slot.
fn effect_labels(
    ui: &mut egui::Ui,
    effects: &[cat_smf_editor::chain_settings::PluginState],
) -> Option<usize> {
    let mut clicked = None;
    for (slot, effect) in effects.iter().enumerate() {
        let name = mixer_effect_name(effect);
        let color = if effect.bypass {
            EFFECT_COLOR.gamma_multiply(0.35)
        } else {
            EFFECT_COLOR
        };
        let label = egui::RichText::new(format!("{} {name}", slot + 1)).color(color);
        if ui
            .push_id(("effect", slot), |ui| {
                name_label(
                    ui,
                    label.into(),
                    Some(egui::Sense::click()),
                    effect_name(effect),
                )
            })
            .inner
            .clicked()
        {
            clicked = Some(slot);
        }
    }
    clicked
}

fn name_label(
    ui: &mut egui::Ui,
    text: egui::WidgetText,
    sense: Option<egui::Sense>,
    original: &str,
) -> egui::Response {
    ui.add_sized([COLUMN_WIDTH, ROW_HEIGHT], |ui: &mut egui::Ui| {
        let mut label = egui::Label::new(text).truncate();
        if let Some(sense) = sense {
            label = label.sense(sense);
        }
        // Use the label layout without its automatic tooltip for elided text.
        let (pos, galley, response) = label.layout_in_ui(ui);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), galley.text())
        });
        if ui.is_rect_visible(response.rect) {
            let color = if sense.is_some() {
                ui.style().interact(&response).text_color()
            } else {
                ui.visuals().text_color()
            };
            let underline = if response.has_focus() || response.highlighted() {
                egui::Stroke::new(1.0_f32, color)
            } else {
                egui::Stroke::NONE
            };
            if ui.style().interaction.selectable_labels {
                egui::text_selection::LabelSelectionState::label_text_selection(
                    ui, &response, pos, galley, color, underline,
                );
            } else {
                ui.painter().add(
                    egui::epaint::TextShape::new(pos, galley, color).with_underline(underline),
                );
            }
        }
        response.on_hover_text(original)
    })
}

/// The master row: the chain button carries the mixer cursor; an effect opens
/// the chain on itself. Returns the slot to select.
fn master_row(
    ui: &mut egui::Ui,
    effects: &[cat_smf_editor::chain_settings::PluginState],
    selected: bool,
    enabled: bool,
) -> Option<usize> {
    let mut clicked = None;
    if ui
        .add_enabled(
            enabled,
            egui::Button::new("Master effects").selected(selected),
        )
        .clicked()
    {
        clicked = Some(effects.len().saturating_sub(1));
    }
    for (slot, effect) in effects.iter().enumerate() {
        let name = mixer_effect_name(effect);
        let color = if effect.bypass {
            EFFECT_COLOR.gamma_multiply(0.35)
        } else {
            EFFECT_COLOR
        };
        let replace = ui
            .push_id(("master-effect", slot), |ui| {
                ui.add_enabled(
                    enabled,
                    egui::Button::new(
                        egui::RichText::new(format!("{} {name}", slot + 1)).color(color),
                    ),
                )
            })
            .inner
            .on_hover_text(effect_name(effect));
        if replace.clicked() {
            clicked = Some(slot);
        }
    }
    clicked
}
