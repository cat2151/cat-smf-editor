//! The mixer's faders and effect rows, drawn into a floating window or a pane.
use super::{effect_name, gain_to_db, EFFECT_COLOR};
use crate::app::App;
use eframe::egui;

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
                            ui.add_sized(
                                [COLUMN_WIDTH, ROW_HEIGHT],
                                egui::Label::new(loaded.patches[track].label()).truncate(),
                            )
                            .on_hover_text(loaded.patches[track].label());
                            let effects = &loaded.patches[track].effects;
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

/// One label per chained effect; returns the clicked slot.
fn effect_labels(
    ui: &mut egui::Ui,
    effects: &[cat_smf_editor::chain_settings::PluginState],
) -> Option<usize> {
    let mut clicked = None;
    for (slot, effect) in effects.iter().enumerate() {
        let name = effect_name(effect);
        let color = if effect.bypass {
            EFFECT_COLOR.gamma_multiply(0.35)
        } else {
            EFFECT_COLOR
        };
        let label =
            egui::Label::new(egui::RichText::new(format!("{} {name}", slot + 1)).color(color))
                .truncate()
                .sense(egui::Sense::click());
        if ui
            .push_id(("effect", slot), |ui| {
                ui.add_sized([COLUMN_WIDTH, ROW_HEIGHT], label)
            })
            .inner
            .on_hover_text(name)
            .clicked()
        {
            clicked = Some(slot);
        }
    }
    clicked
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
        let name = effect_name(effect);
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
            .on_hover_text(name);
        if replace.clicked() {
            clicked = Some(slot);
        }
    }
    clicked
}
