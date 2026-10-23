use crate::app::App;
use cat_smf_editor::score::display_rows::DisplayRow;
use eframe::egui;

#[derive(Clone, Default)]
pub struct State {
    pub split: Option<[usize; 2]>,
    /// Slot of `split` that `App::track` edits; both slots may show the same track.
    pub active: usize,
    pub go_open: bool,
    pub split_open: bool,
    pub track_list: bool,
    pub origin_tick: u64,
    pub cursor_tick: Option<u64>,
    pub scroll_tick: u64,
    pub scroll_fraction: f32,
    pub scroll_source: Option<(usize, usize)>,
    pub horizontal_offset: f32,
    pub apply_scroll: bool,
    /// Ticks shown by the active pane, outlined on the track map.
    pub visible_ticks: Option<(u64, u64)>,
}

pub fn row_count(app: &App, track: usize) -> usize {
    let Some(loaded) = &app.loaded else { return 0 };
    if app.note_mode {
        loaded.display_rows[track].len()
    } else {
        loaded.music.tracks[track].events.len()
    }
}

pub fn row_tick(app: &App, track: usize, row: usize) -> u64 {
    row_tick_loaded(app.loaded.as_ref().unwrap(), app.note_mode, track, row)
}

pub fn row_tick_loaded(
    loaded: &crate::app::Loaded,
    note_mode: bool,
    track: usize,
    row: usize,
) -> u64 {
    let events = &loaded.music.tracks[track].events;
    if note_mode {
        match loaded.display_rows[track].get(row) {
            DisplayRow::Event(i) => events[i].tick,
            DisplayRow::Beat(p) => loaded
                .time
                .map(track)
                .and_then(|m| m.tick_at(&p))
                .unwrap_or(0),
        }
    } else {
        events[row].tick
    }
}

// First row at or after the requested tick; shorter tracks stop at their last row.
pub fn row_at(app: &App, track: usize, tick: u64) -> usize {
    row_at_loaded(app.loaded.as_ref().unwrap(), app.note_mode, track, tick)
}

pub fn row_at_loaded(
    loaded: &crate::app::Loaded,
    note_mode: bool,
    track: usize,
    tick: u64,
) -> usize {
    let count = if note_mode {
        loaded.display_rows[track].len()
    } else {
        loaded.music.tracks[track].events.len()
    };
    let (mut low, mut high) = (0, count);
    while low < high {
        let mid = low + (high - low) / 2;
        if row_tick_loaded(loaded, note_mode, track, mid) < tick {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low.min(count.saturating_sub(1))
}

/// The row sounding at `tick`: its first row, or the last row before it.
pub fn row_sounding_loaded(
    loaded: &crate::app::Loaded,
    note_mode: bool,
    track: usize,
    tick: u64,
) -> usize {
    let row = row_at_loaded(loaded, note_mode, track, tick);
    let empty = if note_mode {
        loaded.display_rows[track].is_empty()
    } else {
        loaded.music.tracks[track].events.is_empty()
    };
    if empty {
        row
    } else if row_tick_loaded(loaded, note_mode, track, row) > tick {
        row.saturating_sub(1)
    } else {
        row
    }
}

pub fn go(app: &mut App, track: usize, tick: u64) {
    if app
        .loaded
        .as_ref()
        .is_none_or(|l| track >= l.music.tracks.len())
    {
        return;
    }
    let scroll = (app.track_view.scroll_tick, app.track_view.scroll_fraction);
    app.select_track(track);
    app.track_view.scroll_tick = scroll.0;
    app.track_view.scroll_fraction = scroll.1;
    app.track_view.cursor_tick = Some(tick);
    app.cursor_position = app
        .loaded
        .as_ref()
        .unwrap()
        .time
        .map(track)
        .map(|m| m.position(tick));
    if row_count(app, track) > 0 {
        let row = row_at(app, track, tick);
        app.cursor_row = Some(row);
        if row_tick(app, track, row) == tick {
            app.selected_event = if app.note_mode {
                match app.loaded.as_ref().unwrap().display_rows[track].get(row) {
                    DisplayRow::Event(i) => Some(i),
                    DisplayRow::Beat(_) => None,
                }
            } else {
                Some(row)
            };
        }
    }
    app.reset_scroll = false;
    app.reveal_cursor = true;
    app.track_view.go_open = false;
}

pub(crate) fn go_edge(app: &mut App, end: bool) {
    let count = row_count(app, app.track);
    if count > 0 {
        crate::cursor::moves::place(app, if end { count - 1 } else { 0 });
        app.reveal_cursor = true;
    }
    app.track_view.go_open = false;
}

pub(crate) fn step_track(app: &mut App, forward: bool, tick: u64) {
    let Some(count) = app.loaded.as_ref().map(|l| l.music.tracks.len()) else {
        return;
    };
    if count > 0 {
        let offset = if forward { 1 } else { count - 1 };
        go(app, (app.track + offset) % count, tick);
    }
}

pub(crate) fn split_on(app: &mut App) {
    app.track_view.split_open = false;
    let Some(loaded) = &app.loaded else { return };
    let count = loaded.music.tracks.len();
    if app.track_view.split.is_none() && count > 0 {
        app.track_view.split = Some([app.track, (app.track + 1) % count]);
        app.track_view.active = 0;
        app.reset_scroll = false;
        app.reveal_cursor = false;
    }
}

pub(crate) fn split_off(app: &mut App) {
    app.track_view.split_open = false;
    app.track_view.split = None;
    app.track_view.apply_scroll = true;
}

/// Replaces the focused column's track with the next or previous track.
pub(crate) fn step_column(app: &mut App, forward: bool) {
    split_on(app);
    step_track(app, forward, crate::play::playback::location(app).0);
}

/// Focuses the left (`slot` 0) or right (`slot` 1) pane.
pub fn focus(app: &mut App, slot: usize) {
    split_on(app);
    if let Some(tracks) = app.track_view.split
        && slot != app.track_view.active
    {
        app.track_view.active = slot;
        go(app, tracks[slot], crate::play::playback::location(app).0);
        app.reveal_cursor = false;
    }
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

// n and p step to the next / previous track instead.
const KEYS: &[u8] = b"123456789abcdefghijklmoqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Track index for a key in the track list: 1–9, then a–z without n / p, then Shift+a–z.
pub fn key_label(track: usize) -> Option<char> {
    KEYS.get(track).map(|&k| char::from(k))
}

fn show_split(app: &mut App, ctx: &egui::Context) {
    if !app.track_view.split_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Two columns")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            if ui.button("w: Off").clicked() {
                split_off(app);
            }
            if ui.button("l: Right").clicked() {
                focus(app, 1);
            }
            if ui.button("h: Left").clicked() {
                focus(app, 0);
            }
            if ui.button("j: Next track").clicked() {
                step_column(app, true);
            }
            if ui.button("k: Previous track").clicked() {
                step_column(app, false);
            }
        });
    if !open {
        app.track_view.split_open = false;
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    show_split(app, ctx);
    if !app.track_view.go_open {
        return;
    }
    let mut open = true;
    let mut target = None;
    let mut step = None;
    egui::Window::new(if app.track_view.track_list {
        "Go track list"
    } else {
        "Go"
    })
    .collapsible(false)
    .resizable(false)
    .open(&mut open)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(ctx, |ui| {
        ui.label("Esc: cancel · destination keeps the time before g");
        if !app.track_view.track_list {
            if ui.button("t: Go track list").clicked() {
                app.track_view.track_list = true;
            }
            if ui.button("h: Track start").clicked() {
                go_edge(app, false);
            }
            if ui.button("e: Track end").clicked() {
                go_edge(app, true);
            }
        } else if let Some(loaded) = &app.loaded {
            if ui.button("n: Next track").clicked() {
                step = Some(true);
            }
            if ui.button("p: Previous track").clicked() {
                step = Some(false);
            }
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    for (index, title) in loaded.track_titles.iter().enumerate() {
                        let label = key_label(index)
                            .map_or_else(|| format!("   {title}"), |k| format!("{k}: {title}"));
                        if ui.selectable_label(index == app.track, label).clicked() {
                            target = Some(index);
                        }
                    }
                });
            ui.label("1–9, a–z, Shift+a–z: go to that track · click for any track");
        }
    });
    if let Some(track) = target {
        go(app, track, app.track_view.origin_tick);
    }
    if let Some(forward) = step {
        step_track(app, forward, app.track_view.origin_tick);
    }
    if !open {
        app.track_view.go_open = false;
    }
}

#[cfg(test)]
mod center_tests;
#[cfg(test)]
mod column_tests;
#[cfg(test)]
mod step_tests;
#[cfg(test)]
pub(crate) mod tests;
