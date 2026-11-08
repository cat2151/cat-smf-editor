//! The mixer's faders and effect rows, drawn into a floating window or a pane.
use super::{effect_name, gain_to_db, EFFECT_COLOR};
use crate::{
    app::App,
    play::track_volume::{MAX_DB, MIN_DB, STEP_DB},
};
use eframe::egui;

const FADER_HEIGHT: f32 = 252.0;
const COLUMN_WIDTH: f32 = 140.0;

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
            ui.vertical(|ui| {
                ui.add_space(40.0 + 2.0 * ui.spacing().item_spacing.y);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(55.0, FADER_HEIGHT), egui::Sense::hover());
                let radius = ui.spacing().interact_size.y / 2.5;
                let levels = (MAX_DB - MIN_DB) / STEP_DB;
                for step in 0..=levels {
                    let db = MAX_DB - step * STEP_DB;
                    let y = rect.top()
                        + radius
                        + (FADER_HEIGHT - 2.0 * radius) * step as f32 / levels as f32;
                    ui.painter().text(
                        egui::pos2(rect.right(), y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{db:+} dB"),
                        egui::FontId::proportional(12.0),
                        ui.visuals().text_color(),
                    );
                }
            });
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
                                [COLUMN_WIDTH, 20.0],
                                egui::Button::new(title).selected(selected).truncate(),
                            );
                            if header.clicked() {
                                app.playback.mixer_master = false;
                                app.playback.mixer_track = track;
                            }
                            ui.add_sized(
                                [COLUMN_WIDTH, 20.0],
                                egui::Label::new(loaded.patches[track].label()).truncate(),
                            )
                            .on_hover_text(loaded.patches[track].label());
                            if let Some(slot) = effect_rows(ui, &loaded.patches[track].effects) {
                                *chain = Some((crate::view::effects::Target::Track(track), slot));
                            }
                            let mut db = gain_to_db(app.audition.gain(track));
                            ui.spacing_mut().slider_width = FADER_HEIGHT;
                            let response = ui.add(
                                egui::Slider::new(&mut db, MIN_DB..=MAX_DB)
                                    .vertical()
                                    .handle_shape(egui::style::HandleShape::Circle)
                                    .trailing_fill(true)
                                    .step_by(STEP_DB as f64)
                                    .show_value(false),
                            );
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
fn effect_rows(
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
                ui.add_sized([COLUMN_WIDTH, 20.0], label)
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
