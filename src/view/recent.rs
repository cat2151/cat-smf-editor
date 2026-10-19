//! Recent files own input and preview audio while leaving the editing App intact.
use crate::{
    app::App,
    store::recent::{now, Entry, History},
};
use eframe::egui::{self, Color32, Key, Modifiers};

pub(crate) mod commands;
mod prefetch;
pub(crate) use prefetch::update as prefetch;

/// The player is None for test auditions, which open no device.
struct Preview {
    _player: Option<crate::play::wav::Player>,
}

#[derive(Default)]
pub struct State {
    pub history: History,
    pub open: bool,
    pub cursor: usize,
    preview: Option<Preview>,
    /// The file the cursor moved to, sounding once its WAV is rendered.
    listen: Option<std::path::PathBuf>,
    prefetch: prefetch::State,
    reveal: bool,
    error: Option<String>,
}

pub(super) fn available(app: &App, ctx: &egui::Context) -> bool {
    !ctx.wants_keyboard_input()
        && app.config.navigation.departure.is_none()
        && !app.player_selection.open
        && !app.layer_editor.open
        && app.note_edit.is_none()
        && app.trill_edit.is_none()
        && !app.piano_roll.open
        && !app.tempo_curve.open
        && !app.comma_menu.open
        && !app.edit_menu.open
        && !app.help_open
        && !app.playback.open
        && !app.playback.mixer_focused()
        && !app.track_view.go_open
        && !app.track_view.split_open
}

fn shortcut(index: usize) -> String {
    match index {
        0..=8 => (index + 1).to_string(),
        9..=34 => ((b'a' + (index - 9) as u8) as char).to_string(),
        35..=60 => ((b'A' + (index - 35) as u8) as char).to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
fn direct(key: Key, modifiers: Modifiers) -> Option<usize> {
    let name = key.name();
    let ch = name.chars().next()?;
    if name.len() != 1 {
        return None;
    }
    if modifiers == Modifiers::NONE && ('1'..='9').contains(&ch) {
        return Some((ch as u8 - b'1') as usize);
    }
    if !ch.is_ascii_alphabetic() {
        return None;
    }
    let lower = ch.to_ascii_lowercase();
    if modifiers == Modifiers::ALT && matches!(lower, 'j' | 'k') {
        return Some(9 + (lower as u8 - b'a') as usize);
    }
    if modifiers == Modifiers::SHIFT {
        return Some(35 + (lower as u8 - b'a') as usize);
    }
    if modifiers == Modifiers::NONE && !matches!(lower, 'j' | 'k') {
        return Some(9 + (lower as u8 - b'a') as usize);
    }
    None
}

fn age(seconds: u64) -> (String, Color32) {
    let (count, unit, rgb) = match seconds {
        0..=3599 => ((seconds / 60).max(1), "m", [249, 38, 114]),
        3600..=86399 => (seconds / 3600, "h", [253, 151, 31]),
        86400..=604799 => (seconds / 86400, "d", [230, 219, 116]),
        604800..=2591999 => (seconds / 604800, "w", [166, 226, 46]),
        2592000..=31535999 => (seconds / 2592000, "mon", [102, 217, 239]),
        _ => (seconds / 31536000, "y", [174, 129, 255]),
    };
    (
        format!("{count}{unit}"),
        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
    )
}

fn cells(entry: &Entry, index: usize, now: u64) -> [String; 7] {
    let seconds = entry.seconds.round() as u64;
    let key = match shortcut(index).as_str() {
        "j" => "Alt+j".into(),
        "k" => "Alt+k".into(),
        key => key.into(),
    };
    [
        key,
        age(now.saturating_sub(entry.opened)).0,
        entry
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        entry.title.clone(),
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        ),
        format!("{}tracks", entry.tracks),
        entry.path.display().to_string(),
    ]
}

fn silence(app: &mut App) {
    app.recent.preview = None;
    app.recent.listen = None;
}

fn preview(app: &mut App) {
    silence(app);
    app.recent.error = None;
    app.recent.listen = app
        .recent
        .history
        .entries
        .get(app.recent.cursor)
        .map(|e| e.path.clone());
    listen(app);
}

/// Plays the listened file if the cursor is still on it and its WAV exists.
fn listen(app: &mut App) {
    let Some(path) = app.recent.listen.clone() else {
        return;
    };
    if app
        .recent
        .history
        .entries
        .get(app.recent.cursor)
        .is_none_or(|e| e.path != path)
    {
        app.recent.listen = None;
        return;
    }
    let wav = match prefetch::wav(app, &path) {
        Ok(None) => return,
        Ok(Some(wav)) => Ok(wav),
        Err(error) => Err(error),
    };
    app.recent.listen = None;
    let player = wav.and_then(|wav| {
        if app.audition.is_disabled() {
            return Ok(None);
        }
        let (rate, samples) = crate::play::wav::read(&wav)?;
        crate::play::wav::Player::play(rate, samples).map(Some)
    });
    match player {
        Ok(player) => app.recent.preview = Some(Preview { _player: player }),
        Err(error) => app.recent.error = Some(error),
    }
}

/// Returns true for every frame owned by this overlay, including its close frame.
pub fn show(app: &mut App, ctx: &egui::Context) -> bool {
    let owned = app.recent.open;
    crate::view::commands::route(app, ctx);
    if !app.recent.open {
        return owned && !super::opening::pending(app);
    }
    draw(app, ctx);
    super::comma_menu::swallow(ctx);
    true
}

fn draw(app: &mut App, ctx: &egui::Context) {
    let now = now();
    let rows: Vec<_> = app
        .recent
        .history
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| cells(e, i, now))
        .collect();
    let font = egui::FontId::proportional(14.0);
    let mut widths = [0.0_f32; 7];
    ctx.fonts(|fonts| {
        for row in &rows {
            for (i, text) in row.iter().enumerate() {
                widths[i] = widths[i].max(
                    fonts
                        .layout_no_wrap(text.clone(), font.clone(), Color32::WHITE)
                        .size()
                        .x,
                );
            }
        }
    });
    let width = widths.iter().sum::<f32>() + 12.0 * 6.0 + 8.0 + 92.0;
    let screen = ctx.screen_rect().shrink(16.0);
    let row_height = 26.0;
    let height = (rows.len() as f32 * row_height).max(row_height);
    let mut select = None;
    let frame = egui::Frame::popup(&ctx.style());
    let margin = frame.total_margin();
    let max_width = (screen.width() - margin.left - margin.right).max(1.0);
    egui::Modal::new(egui::Id::new("recent-files"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(max_width);
            ui.heading("Recent files");
            if let Some(error) = app
                .recent
                .error
                .as_ref()
                .or(app.recent.history.error.as_ref())
            {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            let max_height = (screen.height()
                - margin.top
                - margin.bottom
                - ui.min_size().y
                - ui.spacing().item_spacing.y)
                .max(1.0);
            egui::ScrollArea::both()
                .max_width(max_width)
                .max_height(max_height)
                .auto_shrink([true, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.set_min_size(egui::vec2(width.min(max_width), height.min(max_height)));
                    for (i, row) in rows.iter().enumerate() {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(width, row_height),
                            egui::Sense::click(),
                        );
                        if i == app.recent.cursor {
                            ui.painter()
                                .rect_filled(rect, 2.0, ui.visuals().selection.bg_fill);
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0_f32, super::ACTIVE_PANE),
                                egui::StrokeKind::Inside,
                            );
                            if app.recent.reveal {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                        }
                        let mut x = rect.left() + 4.0;
                        for (column, text) in row.iter().enumerate() {
                            let color = if column == 1 {
                                age(now.saturating_sub(app.recent.history.entries[i].opened)).1
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.painter().text(
                                egui::pos2(x, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                text,
                                font.clone(),
                                color,
                            );
                            x += widths[column] + 12.0;
                        }
                        let status = prefetch::status(app, &app.recent.history.entries[i].path);
                        ui.painter().text(
                            egui::pos2(x, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            status,
                            font.clone(),
                            super::ACTIVE_PANE,
                        );
                        if response.clicked() {
                            select = Some(i);
                        }
                    }
                });
        });
    app.recent.reveal = false;
    if let Some(i) = select {
        let path = app.recent.history.entries[i].path.clone();
        app.recent.open = false;
        silence(app);
        app.open_mid_and_play(path);
    }
}

#[cfg(test)]
mod tests;
