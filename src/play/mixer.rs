use crate::{
    app::App,
    play::track_volume::{MAX_DB, MIN_DB, STEP_DB},
};
use eframe::egui;

const FADER_HEIGHT: f32 = 252.0;
pub(crate) mod volume;
const COLUMN_WIDTH: f32 = 140.0;
/// Chained effects stand apart from the gray instrument and track labels.
pub const EFFECT_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 190, 255);

pub fn effect_name(effect: &cat_smf_editor::chain_settings::PluginState) -> &str {
    effect.preset_name.as_deref().unwrap_or(&effect.plugin.name)
}

/// The chain of the master, or of the mixer track (the current track outside the
/// mixer), with its last effect selected.
pub fn open_effects(app: &mut App, master: bool) {
    let track = if app.playback.mixer_focused() {
        app.playback.mixer_track
    } else {
        app.track
    };
    let target = if master {
        crate::view::effects::Target::Master
    } else {
        crate::view::effects::Target::Track(track)
    };
    crate::view::effects::open(app, target, usize::MAX);
}

pub fn open(app: &mut App) {
    app.playback.mixer_track = app.track;
    app.playback.mixer_master = true;
    app.playback.mixer_open = true;
    app.playback.mixer_focus = true;
}

/// Hands keys back to the views; a focused fader would otherwise keep them.
pub fn unfocus(app: &mut App, ctx: &egui::Context) {
    app.playback.mixer_focus = false;
    ctx.memory_mut(|memory| {
        if let Some(id) = memory.focused() {
            memory.surrender_focus(id);
        }
    });
}

pub fn handle_key(app: &mut App, key: egui::Key) -> bool {
    if key == egui::Key::F {
        crate::view::analysis::toggle(app);
        return true;
    }
    let count = app.loaded.as_ref().map_or(0, |l| l.track_titles.len());
    if count == 0 {
        return false;
    }
    app.playback.mixer_track = app.playback.mixer_track.min(count - 1);
    match key {
        egui::Key::H | egui::Key::ArrowLeft => {
            if app.playback.mixer_master {
                return true;
            }
            if app.playback.mixer_track == 0 {
                app.playback.mixer_master = true;
            }
            app.playback.mixer_track = app.playback.mixer_track.saturating_sub(1);
        }
        egui::Key::L | egui::Key::ArrowRight => {
            if app.playback.mixer_master {
                app.playback.mixer_master = false;
                app.playback.mixer_track = 0;
            } else {
                app.playback.mixer_track = (app.playback.mixer_track + 1).min(count - 1);
            }
        }
        egui::Key::J | egui::Key::ArrowDown | egui::Key::K | egui::Key::ArrowUp => {
            if app.playback.mixer_master {
                return true;
            }
            let track = app.playback.mixer_track;
            let delta = if matches!(key, egui::Key::J | egui::Key::ArrowDown) {
                -STEP_DB
            } else {
                STEP_DB
            };
            let db = (gain_to_db(app.audition.gain(track)) + delta).clamp(MIN_DB, MAX_DB);
            if let Err(error) = volume::set(app, track, crate::play::track_volume::db_to_gain(db)) {
                app.load_error = Some(error);
            }
        }
        egui::Key::X | egui::Key::Enter | egui::Key::Space => {
            open_effects(app, app.playback.mixer_master);
        }
        _ => return false,
    }
    true
}

fn gain_to_db(gain: f32) -> i32 {
    (20.0 * gain.log10())
        .round()
        .clamp(MIN_DB as f32, MAX_DB as f32) as i32
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.playback.mixer_open {
        app.undo.end_volume_gesture();
        return;
    }
    let mut open = true;
    let mut chain = None;
    let mut volume_change = None;
    let mut frame = egui::Frame::window(&ctx.style());
    if app.playback.mixer_focus {
        frame.stroke = ctx.style().visuals.selection.stroke;
    }
    let window = egui::Window::new("Mixer (m)")
        .open(&mut open)
        .frame(frame)
        .default_width(760.0)
        .show(ctx, |ui| {
            let mut auto_gain = app.audition.auto_gain_enabled();
            if ui.checkbox(&mut auto_gain, "Master auto gain").changed() {
                app.audition.set_auto_gain_enabled(auto_gain);
            }
            ui.separator();
            ui.horizontal(|ui| {
                let enabled = app.loaded.as_ref().is_some_and(|l| l.music.format == 1);
                if let Some(slot) = master_row(ui, &app.audition.master().effects, app.playback.mixer_master, enabled) {
                    app.playback.mixer_master = true;
                    chain = Some((crate::view::effects::Target::Master, slot));
                }
            });
            let Some(loaded) = &app.loaded else {
                ui.label("Open an SMF first");
                return;
            };
            let count = loaded.track_titles.len();
            app.playback.mixer_track = app.playback.mixer_track.min(count.saturating_sub(1));
            let selection = (app.generation, app.playback.mixer_track, app.playback.mixer_master);
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
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(55.0, FADER_HEIGHT),
                            egui::Sense::hover(),
                        );
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
                                        chain = Some((crate::view::effects::Target::Track(track), slot));
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
                                        volume_change = Some((track, crate::play::track_volume::db_to_gain(db), response.is_pointer_button_down_on()));
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
            ui.separator();
            ui.label("h/l / ←/→: master / track · j/k / ↓/↑: track −/+3 dB · m: close · Esc: views · x: selected effects");
        });
    if let Some(window) = window
        && let Some(press) =
            ctx.input(|i| i.pointer.press_origin().filter(|_| i.pointer.any_pressed()))
    {
        if window.response.rect.contains(press) {
            app.playback.mixer_focus = true;
        } else if ctx
            .layer_id_at(press)
            .is_some_and(|layer| layer.order == egui::Order::Background)
        {
            unfocus(app, ctx);
        }
    }
    if let Some((track, gain, dragging)) = volume_change
        && let Err(error) = volume::change(app, track, gain, dragging)
    {
        app.load_error = Some(error);
    }
    if !open || !ctx.input(|i| i.pointer.primary_down()) {
        app.undo.end_volume_gesture();
    }
    app.playback.mixer_open = open;
    if let Some((target, slot)) = chain {
        crate::view::effects::open(app, target, slot);
    }
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
