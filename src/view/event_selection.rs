//! Keyboard ranges over the event list's visible rows.
use crate::app::App;
use eframe::egui;
use std::ops::Range;

mod actions;
#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct State {
    anchor: Option<usize>,
    song: u64,
    generation: u64,
    track: usize,
    note_mode: bool,
    menu: bool,
}

pub(super) fn valid(app: &App) -> bool {
    let s = &app.event_selection;
    s.anchor.is_some()
        && s.song == app.song_id
        && s.generation == app.generation
        && s.track == app.track
        && s.note_mode == app.note_mode
}

pub(super) fn range(app: &App) -> Option<Range<usize>> {
    if !valid(app) {
        return None;
    }
    let a = app.event_selection.anchor?;
    let b = app.cursor_row?;
    Some(if b < a { b..a } else { a..b + 1 })
}

fn count(app: &App) -> usize {
    app.loaded.as_ref().map_or(0, |l| {
        if app.note_mode {
            l.display_rows[app.track].len()
        } else {
            l.music.tracks[app.track].events.len()
        }
    })
}

fn whole(app: &App) -> bool {
    range(app).is_some_and(|r| r == (0..count(app)))
}

/// Returns true while selection owns the keyboard, including its closing frame.
pub(super) fn shortcuts(app: &mut App, ctx: &egui::Context) -> bool {
    if !valid(app) {
        app.event_selection = State::default();
    }
    if app.mml_input.open
        || ctx.wants_keyboard_input()
        || app.help_open
        || app.comma_menu.open
        || app.edit_menu.open
        || app.note_edit.is_some()
        || app.trill_edit.is_some()
        || app.layer_editor.open
        || app.effect_editor.is_open()
        || app.track_view.go_open
        || app.track_view.split_open
        || app.playback.open
        || app.playback.mixer_focused()
        || app.player_selection.open
        || app.piano_roll.open
        || app.tempo_curve.open
        || app.recent.open
        || super::busy::pending(app)
    {
        return false;
    }
    if app.event_selection.menu {
        for key in super::comma_menu::pressed(ctx) {
            match key {
                egui::Key::Escape => app.event_selection.menu = false,
                egui::Key::A => actions::apply(app, actions::Action::Repeat),
                egui::Key::D => actions::apply(app, actions::Action::DeleteEvents),
                egui::Key::I if whole(app) => actions::apply(app, actions::Action::Init),
                egui::Key::T if whole(app) => actions::apply(app, actions::Action::DeleteTrack),
                _ => {}
            }
        }
        super::comma_menu::swallow(ctx);
        return true;
    }
    if super::consume_plain(ctx, egui::Key::S) && count(app) > 0 {
        if valid(app) {
            app.event_selection.menu = true;
        } else {
            let row = app
                .cursor_row
                .or(app.selected_event.and_then(|i| {
                    if app.note_mode {
                        app.loaded.as_ref()?.display_rows[app.track].event_row(i)
                    } else {
                        Some(i)
                    }
                }))
                .unwrap_or(0)
                .min(count(app) - 1);
            crate::cursor::moves::place(app, row);
            app.event_selection = State {
                anchor: Some(row),
                song: app.song_id,
                generation: app.generation,
                track: app.track,
                note_mode: app.note_mode,
                menu: false,
            };
            app.playback.follow = false;
            app.reveal_cursor = true;
        }
        super::comma_menu::swallow(ctx);
        return true;
    }
    if !valid(app) {
        return false;
    }
    if super::consume_plain(ctx, egui::Key::Escape) {
        app.event_selection = State::default();
    } else {
        crate::cursor::moves::shortcuts(app, ctx);
    }
    super::comma_menu::swallow(ctx);
    true
}

pub(super) fn show(app: &mut App, ctx: &egui::Context) {
    if !valid(app) || !app.event_selection.menu {
        return;
    }
    let mut open = true;
    egui::Window::new("Selection")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            let mut choices = vec![
                (actions::Action::Repeat, "a: A-B repeat"),
                (actions::Action::DeleteEvents, "d: Delete events"),
            ];
            if whole(app) {
                choices.extend([
                    (actions::Action::Init, "i: Init track"),
                    (actions::Action::DeleteTrack, "t: Delete track"),
                ]);
            }
            for (action, label) in choices {
                if ui.button(label).clicked() {
                    actions::apply(app, action);
                }
            }
        });
    if !open {
        app.event_selection.menu = false;
    }
}

/// Paint only the visible slice; long empty measures stay virtualized.
pub(super) fn paint(
    range: Option<Range<usize>>,
    ui: &egui::Ui,
    output: &egui::scroll_area::ScrollAreaOutput<()>,
    stride: f32,
) {
    let Some(range) = range else {
        return;
    };
    let top = output.inner_rect.top() - output.state.offset.y;
    let rect = egui::Rect::from_x_y_ranges(
        output.inner_rect.x_range(),
        top + range.start as f32 * stride..=top + range.end as f32 * stride,
    );
    ui.painter_at(output.inner_rect).rect_filled(
        rect.intersect(output.inner_rect),
        0.0,
        egui::Color32::from_rgba_unmultiplied(102, 217, 239, 45),
    );
}
