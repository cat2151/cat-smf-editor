//! 画面描画: パネル・行・鍵盤・help。
pub(crate) mod analysis;
pub(crate) mod beat_audition;
pub(crate) mod busy;
pub(crate) mod comma_menu;
pub(crate) mod commands;
pub(crate) mod edit_menu;
pub(crate) mod effects;
mod event_repeat;
pub(crate) mod event_selection;
mod help;
pub(crate) mod layers;
mod meta_text;
pub(crate) mod mml_input;
pub(crate) mod note_edit;
pub(crate) mod opening;
pub(crate) mod palette;
pub(crate) mod piano_roll;
mod pitch_keyboard;
mod row;
pub(crate) use row::beat_color;
pub(crate) mod files;
pub(crate) mod recent;
pub(crate) mod staff;
pub(crate) mod tempo_curve;
#[cfg(test)]
pub(crate) mod tests;
pub(crate) mod time_columns;
mod timeline_follow;
mod timeline_minimap;
mod timeline_repeat;
mod timeline_zoom;
mod title;
mod track_map;
mod track_pane;
pub(crate) mod trill_edit;
mod undo;

use crate::app::App;
use eframe::egui::{self, Color32, ScrollArea};

/// Outline of the focused pane in two-column view (Monokai orange).
pub const ACTIVE_PANE: Color32 = Color32::from_rgb(0xfd, 0x97, 0x1f);

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.startup_pending(ctx) {
        return;
    }
    opening::advance(app, ctx);
    let opening_input = opening::suspend_input(app, ctx);
    let palette_input = palette::begin(app, ctx);
    if crate::player::setup::show(app, ctx) {
        crate::play::transport::advance(app);
        app.audition.pump();
        palette::end(app, ctx, palette_input);
        return;
    }
    busy::run(app, ctx);
    crate::play::transport::stop_shortcut(app, ctx);
    if !opening::pending(app)
        && (app.palette.is_open()
            || (!app.recent.open
                && app.config.navigation.departure.is_none()
                && app.invalid_smf_name.is_none()))
    {
        crate::play::transport::advance(app);
    }
    show_contents(app, ctx);
    if app.palette.is_open() {
        app.audition.pump();
    }
    // Outside show_contents: a viewport skipped for one frame closes.
    analysis::show(app, ctx);
    if !opening::pending(app) {
        app.auto_save(ctx);
    }
    title::update(app, ctx);
    palette::end(app, ctx, palette_input);
    opening::show(app, ctx, opening_input);
}

fn show_contents(app: &mut App, ctx: &egui::Context) {
    let mut invalid_smf = files::invalid_smf_dialog(app, ctx);
    if !invalid_smf && !app.mml_input.open && recent::show(app, ctx) {
        return;
    }
    if !invalid_smf && !app.mml_input.open && !event_selection::valid(app) {
        crate::view::files::inputs(app, ctx);
    }
    let departure = app.config.navigation.departure.is_some();
    if !invalid_smf && departure {
        crate::view::files::confirmation(app, ctx);
    }
    if !invalid_smf {
        invalid_smf = files::invalid_smf_dialog(app, ctx);
    }
    let confirming = departure || invalid_smf;
    if !confirming {
        #[cfg(windows)]
        crate::player::resident_link::refresh_browser_activity(app);
        #[cfg(windows)]
        let started = std::time::Instant::now();
        let selection_owns_input = event_selection::shortcuts(app, ctx);
        if !selection_owns_input {
            mml_input::shortcuts(app, ctx);
        }
        let mml_owns_input = app.mml_input.open;
        if mml_owns_input {
            mml_input::show(app, ctx);
        }
        if !mml_owns_input && !selection_owns_input {
            crate::play::transport::stop_shortcut(app, ctx);
            undo::shortcuts(app, ctx);
            crate::view::layers::show(app, ctx);
            crate::view::effects::show(app, ctx);
            #[cfg(windows)]
            crate::player::resident_link::trace_slow(app, "advance", started);
            if app.player_selection.no_notes_error {
                crate::player::bulk::show_note_error(app, ctx);
                return;
            }
            if app.trill_edit.is_none() {
                crate::view::piano_roll::shortcuts(app, ctx);
                crate::view::comma_menu::shortcuts(app, ctx);
                crate::view::edit_menu::shortcuts(app, ctx);
                crate::cursor::split::shortcuts(app, ctx);
                crate::view::layers::shortcuts(app, ctx);
                if !app.track_view.go_open && !app.track_view.split_open {
                    crate::play::playback::shortcuts(app, ctx);
                    crate::play::transport::shortcuts(app, ctx);
                    crate::view::help::shortcuts(app, ctx);
                    crate::cursor::moves::shortcuts(app, ctx);
                }
            }
        }
    }
    app.audition.pump();
    if app.loaded.is_some() {
        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
            crate::play::transport::status(app, ui);
        });

        let mut track = app.track;
        let mut add_track = false;
        egui::SidePanel::left("tracks")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Tracks");
                    if let Some(loaded) = &app.loaded {
                        ui.label(loaded.music.tracks.len().to_string());
                    }
                });
                let Some(loaded) = &app.loaded else {
                    ui.label("—");
                    return;
                };
                ScrollArea::both()
                    .id_salt(("track-list", app.generation))
                    .show(ui, |ui| {
                        for (index, title) in loaded.track_titles.iter().enumerate() {
                            ui.selectable_value(&mut track, index, title);
                        }
                        add_track = ui
                            .add_enabled(
                                !confirming
                                    && !app.mml_input.open
                                    && !app.recent.open
                                    && recent::available(app, ctx),
                                egui::Button::new("+"),
                            )
                            .on_hover_text("Add track")
                            .clicked();
                    });
            });
        if add_track {
            busy::request(app, ctx, busy::Task::AddTrack);
        } else if track != app.track {
            app.select_track(track);
        }
    }

    let mut browse_patch = false;
    let mut replace_effect = None;
    let mut map_click = None;
    egui::CentralPanel::default().show(ctx, |ui| {
        if !invalid_smf && let Some(error) = &app.load_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        if let Some(error) = &app.saved.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        // Raw rows are too wide for two columns.
        if let Some(tracks) = app.track_view.split.filter(|_| app.note_mode) {
            ui.columns(2, |panes| {
                let active = app.track_view.active;
                for slot in [active, 1 - active] {
                    panes[slot].push_id(slot, |ui| {
                        if let Some(tick) = pane(
                            app,
                            ui,
                            tracks[slot],
                            slot == active,
                            &mut browse_patch,
                            &mut replace_effect,
                        ) {
                            map_click = Some((slot, tracks[slot], tick));
                        }
                    });
                    if slot == active {
                        let rect = panes[slot].min_rect().expand(2.0);
                        panes[slot].painter().with_clip_rect(rect).rect_stroke(
                            rect,
                            2.0,
                            egui::Stroke::new(2.0_f32, ACTIVE_PANE),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });
        } else if app.note_mode {
            // The single column leaves the second column to the score.
            ui.columns(2, |panes| {
                if let Some(tick) = pane(
                    app,
                    &mut panes[0],
                    app.track,
                    true,
                    &mut browse_patch,
                    &mut replace_effect,
                ) {
                    map_click = Some((app.track_view.active, app.track, tick));
                }
                // The tick overlays show the same time axis and own the wheel while open.
                if !app.piano_roll.open && !app.tempo_curve.open {
                    staff::show(app, &mut panes[1]);
                }
            });
        } else if let Some(tick) = pane(
            app,
            ui,
            app.track,
            true,
            &mut browse_patch,
            &mut replace_effect,
        ) {
            map_click = Some((app.track_view.active, app.track, tick));
        }
        app.track_view.apply_scroll = false;
        app.reset_scroll = false;
        app.reveal_cursor = false;
        app.note_edit_request = None;
        if let Some(error) = app.audition.error() {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    });
    if let Some((slot, track, tick)) = map_click {
        crate::view::track_map::go(app, slot, track, tick);
    }
    if browse_patch {
        crate::player::bulk::request(app);
    }
    if let Some(slot) = replace_effect {
        crate::view::effects::open(app, crate::view::effects::Target::Track(app.track), slot);
    }
    if confirming {
        return;
    }
    crate::player::selection::show(app, ctx);
    crate::view::help::show(app, ctx);
    crate::play::mixer::show(app, ctx);
    crate::view::event_selection::show(app, ctx);
    crate::view::edit_menu::show(app, ctx);
    crate::view::note_edit::show(app, ctx);
    crate::view::piano_roll::show(app, ctx);
    crate::view::tempo_curve::show(app, ctx);
    crate::view::comma_menu::show(app, ctx);
    crate::play::playback::show(app, ctx);
    crate::cursor::split::show(app, ctx);
    crate::view::trill_edit::show(app, ctx);
    #[cfg(windows)]
    if crate::player::resident_link::loading(app) {
        egui::Modal::new(egui::Id::new("browser-loading")).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("now loading...");
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    if app.playback.pending.is_some() {
        ctx.request_repaint();
    }
    if app.audition.needs_pump() {
        ctx.request_repaint_after(std::time::Duration::from_millis(32));
    }
}

/// The track's rows with its map on the right; the map follows the rows' visible ticks.
fn pane(
    app: &mut App,
    ui: &mut egui::Ui,
    track: usize,
    active: bool,
    browse_patch: &mut bool,
    replace_effect: &mut Option<usize>,
) -> Option<u64> {
    let rect = ui.available_rect_before_wrap();
    let split = rect.right() - crate::view::track_map::WIDTH;
    let rows = egui::Rect::from_min_max(rect.min, egui::pos2(split, rect.bottom()));
    let map = egui::Rect::from_min_max(egui::pos2(split, rect.top()), rect.max);
    let action = crate::view::track_map::show(
        app,
        &mut ui.new_child(egui::UiBuilder::new().max_rect(map)),
        track,
    );
    let mut tick = None;
    match action {
        Some(track_map::Action::Selecting) => app.playback.follow = false,
        Some(track_map::Action::Repeat(range)) => {
            crate::play::transport::repeat::start(app, range);
        }
        Some(track_map::Action::Go(target)) => tick = Some(target),
        None => {}
    }
    crate::view::track_pane::show(
        app,
        &mut ui.new_child(egui::UiBuilder::new().max_rect(rows).layout(*ui.layout())),
        track,
        active,
        browse_patch,
        replace_effect,
    );
    ui.advance_cursor_after_rect(rect);
    tick
}
