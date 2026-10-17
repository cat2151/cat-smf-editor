//! Recent files own input and preview audio while leaving the editing App intact.
use crate::{
    app::App,
    store::recent::{now, Entry, History},
};
use eframe::egui::{self, Color32, Key, Modifiers};

#[derive(Default)]
pub struct State {
    pub history: History,
    pub open: bool,
    pub cursor: usize,
    preview: Option<Box<App>>,
    pub cache: std::collections::HashMap<std::path::PathBuf, crate::play::audition::Audition>,
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

fn park_preview(app: &mut App) {
    if let Some(mut preview) = app.recent.preview.take()
        && let Some(loaded) = &preview.loaded
        && let Some(audio) = preview.audition.take_prepared()
    {
        app.recent.cache.insert(loaded.original.clone(), audio);
    }
}

fn preview(app: &mut App) {
    park_preview(app);
    app.recent.error = None;
    let Some(entry) = app.recent.history.entries.get(app.recent.cursor) else {
        return;
    };
    if app
        .loaded
        .as_ref()
        .is_some_and(|l| l.original == entry.path)
        && let Some(audio) = app.audition.take_prepared()
    {
        app.recent.cache.insert(entry.path.clone(), audio);
    }
    let mut preview = App::empty();
    preview.player_selection.executable = app.player_selection.executable.clone();
    #[cfg(test)]
    {
        preview.audition = crate::play::audition::Audition::disabled();
    }
    preview.load_mid_unchecked(entry.path.clone());
    if let Some(error) = preview.load_error.take() {
        app.recent.error = Some(error);
    } else {
        if let Some(audio) = app.recent.cache.remove(&entry.path) {
            preview.audition.reuse_prepared(audio);
        }
        crate::play::playback::start(&mut preview, crate::play::playback::Mode::AllTracks);
        app.recent.preview = Some(Box::new(preview));
    }
}

/// Returns true for every frame owned by this overlay, including its close frame.
pub fn show(app: &mut App, ctx: &egui::Context) -> bool {
    let mut opened = false;
    if !app.recent.open
        && available(app, ctx)
        && ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::P))
    {
        crate::play::transport::stop(app);
        app.recent.open = true;
        app.recent.cursor = 0;
        app.recent.reveal = true;
        app.recent.error = None;
        opened = true;
    }
    if !app.recent.open {
        return false;
    }
    let mut cancel = false;
    let mut chosen = None;
    let mut moved = false;
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        if let egui::Event::Key {
            key,
            modifiers,
            pressed: true,
            repeat,
            ..
        } = event
        {
            if opened {
                continue;
            }
            let delta = match (key, modifiers) {
                (Key::J | Key::ArrowDown, Modifiers::NONE) => 1,
                (Key::K | Key::ArrowUp, Modifiers::NONE) => -1,
                (Key::PageDown, Modifiers::NONE) => 10,
                (Key::PageUp, Modifiers::NONE) => -10,
                _ => 0,
            };
            if key == Key::Escape {
                cancel = true;
                break;
            }
            if delta != 0 && !app.recent.history.entries.is_empty() {
                app.recent.cursor = app
                    .recent
                    .cursor
                    .saturating_add_signed(delta)
                    .min(app.recent.history.entries.len() - 1);
                moved = true;
            } else if !repeat && key == Key::Enter && modifiers == Modifiers::NONE {
                chosen = Some(app.recent.cursor);
                break;
            } else if !repeat
                && let Some(index) = direct(key, modifiers)
                && index < app.recent.history.entries.len()
            {
                chosen = Some(index);
                break;
            }
        }
    }
    if cancel || chosen.is_some() {
        app.recent.open = false;
        park_preview(app);
        if let Some(path) = chosen
            .and_then(|i| app.recent.history.entries.get(i))
            .map(|e| e.path.clone())
        {
            app.open_mid_and_play(path);
        }
        super::comma_menu::swallow(ctx);
        return true;
    }
    if moved {
        app.recent.reveal = true;
        preview(app);
        ctx.request_repaint();
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Space))
        && let Some(preview) = &mut app.recent.preview
    {
        crate::play::transport::stop(preview);
    }
    // Pump the independent preview voice while the document stays untouched.
    if !moved && let Some(preview) = &mut app.recent.preview {
        crate::play::transport::advance(preview);
        preview.audition.pump();
        if preview.audition.needs_pump() || preview.playback.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(32));
        }
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
            if let Some(error) = app.recent.preview.as_ref().and_then(|p| p.audition.error()) {
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
                        let path = &app.recent.history.entries[i].path;
                        let status = if let Some(preview) = app
                            .recent
                            .preview
                            .as_ref()
                            .filter(|p| p.loaded.as_ref().is_some_and(|l| &l.original == path))
                        {
                            if preview.audition.error().is_some() {
                                "準備失敗"
                            } else if crate::play::transport::preparing(preview) {
                                "準備中…"
                            } else if preview.audition.is_prepared() {
                                "準備済み"
                            } else {
                                ""
                            }
                        } else if app.recent.cache.get(path).is_some_and(|a| a.is_prepared())
                            || app.loaded.as_ref().is_some_and(|l| &l.original == path)
                                && app.audition.is_prepared()
                        {
                            "準備済み"
                        } else {
                            ""
                        };
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
        park_preview(app);
        app.open_mid_and_play(path);
    }
}

#[cfg(test)]
mod tests;
