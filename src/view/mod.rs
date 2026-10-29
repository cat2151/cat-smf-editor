//! 画面描画: パネル・行・鍵盤・help。
pub(crate) mod analysis;
pub(crate) mod beat_audition;
pub(crate) mod busy;
pub(crate) mod comma_menu;
pub(crate) mod command_input;
mod command_row;
pub(crate) mod commands;
pub(crate) mod drums;
pub(crate) mod edit_menu;
pub(crate) mod edit_style;
pub(crate) mod effects;
mod event_repeat;
pub(crate) mod event_selection;
pub(crate) mod help;
pub(crate) mod layers;
mod meta_text;
pub(crate) mod mml_input;
pub(crate) mod modal;
pub(crate) mod note_edit;
mod note_lyrics;
pub(crate) mod opening;
pub(crate) mod palette;
pub(crate) mod patterns;
pub(crate) mod piano_roll;
mod pitch_keyboard;
mod row;
pub(crate) use row::beat_color;
pub(crate) mod files;
pub(crate) mod macro_editor;
pub(crate) mod parameter_editor;
pub(crate) mod recent;
pub(crate) mod recording;
mod score_panes;
pub(crate) mod selection_history;
pub(crate) mod staff;
pub(crate) mod tempo_curve;
#[cfg(test)]
pub(crate) mod tests;
mod text_input;
pub(crate) mod time_columns;
mod timeline_drag;
mod timeline_follow;
mod timeline_minimap;
mod timeline_repeat;
mod timeline_zoom;
mod title;
mod track_map;
pub(crate) mod track_overview;
mod track_pane;
pub(crate) mod track_rename;
pub(crate) mod trill_edit;
mod undo;
pub(crate) mod voicevox;

use crate::app::App;
use eframe::egui::{self, Color32, ScrollArea};

/// Outline of the focused pane in two-column view (Monokai orange).
pub const ACTIVE_PANE: Color32 = Color32::from_rgb(0xfd, 0x97, 0x1f);

/// One Monokai accent per track, independent of the displayed measure and row order.
pub(crate) fn track_color(track: usize) -> Color32 {
    const COLORS: [Color32; 6] = [
        Color32::from_rgb(0x66, 0xd9, 0xef),
        Color32::from_rgb(0xf9, 0x26, 0x72),
        Color32::from_rgb(0xfd, 0x97, 0x1f),
        Color32::from_rgb(0xa6, 0xe2, 0x2e),
        Color32::from_rgb(0xae, 0x81, 0xff),
        Color32::from_rgb(0xe6, 0xdb, 0x74),
    ];
    COLORS[track % COLORS.len()]
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    help::input(app, ctx);
    patterns::validate(app);
    if app.startup_pending(ctx) {
        app.lifecycle.interrupted();
        return;
    }
    opening::advance(app, ctx);
    crate::play::sung::prerender::poll(app, ctx);
    voicevox::singer::poll(app, ctx);
    let opening_input = opening::suspend_input(app, ctx);
    patterns::workspace::suspend_background(app, ctx);
    let palette_input = modal::begin(app, ctx);
    if crate::player::setup::show(app, ctx) {
        app.lifecycle.interrupted();
        crate::play::transport::advance(app);
        crate::play::mascot::update(app);
        app.audition.pump();
        modal::end(app, ctx, palette_input);
        help::observe(app, ctx);
        help::show(app, ctx);
        return;
    }
    busy::run(app, ctx);
    crate::play::transport::stop_shortcut(app, ctx);
    if !opening::pending(app)
        && (crate::view::modal::is_open(app)
            || (!app.recent.open
                && app.config.navigation.departure.is_none()
                && app.invalid_smf_name.is_none()))
    {
        crate::play::transport::advance(app);
    }
    crate::play::mascot::update(app);
    app.frame_started(ctx);
    let style_input = edit_style::suspend_background(app, ctx);
    let event_list_drawn = show_contents(app, ctx);
    patterns::show(app, ctx);
    patterns::workspace::root_dialog(app, ctx);
    track_rename::show(app, ctx);
    if crate::view::modal::is_open(app) {
        app.audition.pump();
    }
    // Outside show_contents: a viewport skipped for one frame closes.
    analysis::show(app, ctx);
    patterns::presentation::refocus_root(app, ctx);
    patterns::workspace::show(app, ctx);
    if !opening::pending(app) {
        app.auto_save(ctx);
    }
    title::update(app, ctx);
    app.frame_completed(ctx, event_list_drawn);
    edit_style::show(app, ctx, style_input);
    modal::end(app, ctx, palette_input);
    opening::show(app, ctx, opening_input);
    voicevox::progress::show(app, ctx);
    help::observe(app, ctx);
    help::show(app, ctx);
}

fn show_contents(app: &mut App, ctx: &egui::Context) -> bool {
    if files::missing_last_dialog(app, ctx) {
        return false;
    }
    let mut invalid_smf = files::invalid_smf_dialog(app, ctx);
    if !invalid_smf && app.edit_style.is_none() && !app.mml_input.open && recent::show(app, ctx) {
        return false;
    }
    if !invalid_smf
        && app.edit_style.is_none()
        && !app.mml_input.open
        && !event_selection::valid(app)
    {
        crate::view::files::inputs(app, ctx);
    }
    let departure = app.config.navigation.departure.is_some();
    if !invalid_smf && departure {
        crate::view::files::confirmation(app, ctx);
    }
    if !invalid_smf {
        invalid_smf = files::invalid_smf_dialog(app, ctx);
    }
    let notice = !invalid_smf && !departure && files::backup_notice_dialog(app, ctx);
    let confirming = departure || invalid_smf || notice || app.edit_style.is_some();
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
            crate::view::drums::show(app, ctx);
            crate::view::effects::show(app, ctx);
            #[cfg(windows)]
            crate::player::resident_link::trace_slow(app, "advance", started);
            if app.player_selection.no_notes_error {
                crate::player::bulk::show_note_error(app, ctx);
                return false;
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
        let mut track_clicked = false;
        let mut add_track = false;
        let mut rename_track = None;
        let can_rename = matches!(
            commands::owner(app, ctx),
            commands::Screen::Main | commands::Screen::Timeline
        );
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
                            let response = ui.selectable_value(&mut track, index, title);
                            track_clicked |= response.clicked();
                            #[cfg(test)]
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new(("actual-track", index)), response.rect)
                            });
                            if can_rename {
                                response.context_menu(|ui| {
                                    if ui.button("Rename…").clicked() {
                                        rename_track = Some(index);
                                        ui.close_menu();
                                    }
                                });
                            }
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
        } else if track_clicked {
            patterns::track_selected(app);
        }
        if let Some(track) = rename_track {
            track_rename::begin(app, track);
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
        map_click = score_panes::show(app, ui, &mut browse_patch, &mut replace_effect);
        app.track_view.apply_scroll = false;
        app.reset_scroll = false;
        app.reveal_cursor = false;
        app.note_edit_request = None;
        if let Some(error) = app.audition.error() {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        if let Some(error) = &app.voicevox.error {
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
        return app.loaded.is_some();
    }
    crate::player::selection::show(app, ctx);
    crate::play::mixer::show(app, ctx);
    crate::view::event_selection::show(app, ctx);
    crate::view::edit_menu::show(app, ctx);
    crate::view::note_edit::show(app, ctx);
    crate::view::voicevox::lyrics::show(app, ctx);
    crate::view::piano_roll::show(app, ctx);
    crate::view::track_overview::show(app, ctx);
    crate::view::tempo_curve::show(app, ctx);
    crate::view::parameter_editor::show(app, ctx);
    crate::view::macro_editor::show(app, ctx);
    crate::view::comma_menu::show(app, ctx);
    crate::view::voicevox::show(app, ctx);
    crate::view::voicevox::singer::show(app, ctx);
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
    app.loaded.is_some()
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

mod list_navigation;
