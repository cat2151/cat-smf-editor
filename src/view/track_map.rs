//! Whole-track note map right of each pane: pitch across, time down, the visible ticks outlined.
use crate::app::App;
use crate::view::pitch_keyboard;
use cat_smf_editor::score::musical_time::Position;
use cat_smf_editor::score::note_view::note_on;
use eframe::egui::{self, Color32};

mod tempo;

pub const WIDTH: f32 = 120.0;

/// Other tracks' notes, meta event rows and the visible-range outline.
const DARK: Color32 = Color32::from_gray(64);

/// `slot` is the pane's side in two-column view; a click there focuses it and moves playback.
/// Runs after the frame's rows, so the cursor reveal reaches the next frame's rows.
pub fn go(app: &mut App, slot: usize, track: usize, tick: u64) {
    if app.track_view.split.is_some() {
        crate::cursor::split::focus(app, slot);
    }
    crate::cursor::split::go(app, track, tick);
    crate::play::transport::jump(app, tick);
}

/// Clicks navigate to a measure; drags select A–B repeat.
pub fn show(app: &App, ui: &mut egui::Ui, track: usize) -> Option<Action> {
    let loaded = app.loaded.as_ref()?;
    let rect = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let events = &loaded.music.tracks[track].events;
    let end = events.last().map_or(0, |e| e.tick).max(1) as f32;
    let y = |tick: u64| rect.top() + rect.height() * (tick as f32 / end);
    let row = |tick: u64| {
        let top = y(tick).floor();
        egui::Rect::from_x_y_ranges(rect.x_range(), top..=top + 1.0)
    };
    let curve = cat_smf_editor::edit::tempo_curve::read(&loaded.music)
        .ok()
        .flatten();
    if let Some(curve) = &curve {
        tempo::draw(&painter, rect, f64::from(end), curve);
    }
    // Curve-generated conductor tempos use a thin curve instead of full-width rows.
    // Other meta events stay visible above it; End of Track marks nothing.
    for (source, event) in loaded
        .music
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(source, t)| t.events.iter().map(move |e| (source, e)))
        .filter(|(_, e)| e.status == 0xff && e.meta_type != 0x2f)
    {
        if curve.is_some()
            && source == 0
            && (event.meta_type == 0x51 || cat_smf_editor::edit::tempo_curve::owned(event))
        {
            continue;
        }
        painter.rect_filled(row(event.tick), 0.0, DARK);
    }
    let range = loaded.time_columns[track].pitch_range;
    // A track without notes, such as the conductor, spans every track's notes.
    let others_range = range.or_else(|| {
        loaded
            .time_columns
            .iter()
            .filter_map(|c| c.pitch_range)
            .reduce(|(min, max), (low, high)| (min.min(low), max.max(high)))
    });
    let note_rect = |range: (u8, u8), pitch: u8, tick: u64, duration: Option<u64>| {
        let top = y(tick);
        let bottom = y(tick + duration.unwrap_or(0)).max(top + 1.0);
        egui::Rect::from_x_y_ranges(
            pitch_keyboard::key_rect(rect, range, pitch).x_range(),
            top..=bottom,
        )
    };
    if let Some(range) = others_range {
        for (other, other_track) in loaded.music.tracks.iter().enumerate() {
            if other == track {
                continue;
            }
            let durations = &loaded.note_views[other].durations;
            for (index, event) in other_track
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| note_on(e))
            {
                let pitch = event.data[0];
                if (range.0..=range.1).contains(&pitch) {
                    painter.rect_filled(
                        note_rect(range, pitch, event.tick, durations[index]),
                        0.0,
                        DARK,
                    );
                }
            }
        }
    }
    let playing = crate::play::transport::tick_for(app, track)
        .unwrap_or_else(|| crate::play::playback::location(app).0);
    painter.rect_filled(row(playing), 0.0, Color32::WHITE);
    if let Some(range) = range {
        let durations = &loaded.note_views[track].durations;
        let keys = loaded.keys.map(track);
        for (index, event) in events.iter().enumerate().filter(|(_, e)| note_on(e)) {
            let pitch = event.data[0];
            painter.rect_filled(
                note_rect(range, pitch, event.tick, durations[index]),
                0.0,
                pitch_keyboard::note_color(pitch, keys.and_then(|k| k.at(event.tick))),
            );
        }
    }
    let visible = ui
        .ctx()
        .data_mut(|d| d.get_temp::<(u64, u64)>(visible_id(app.generation, track, app.note_mode)))
        .or(app.track_view.visible_ticks);
    if let Some((first, last)) = visible {
        let top = y(first);
        painter.rect_stroke(
            egui::Rect::from_x_y_ranges(rect.x_range(), top..=y(last).max(top + 2.0)),
            0.0,
            egui::Stroke::new(1.5_f32, DARK),
            egui::StrokeKind::Inside,
        );
    }
    let local = |p: egui::Pos2| ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0) * end;
    let selection = super::event_repeat::gesture(
        ui,
        &response,
        |pos| local(pos).round() as u64,
        y,
        app.playback.repeat,
    );
    if let Some(range) = selection.range {
        return Some(Action::Repeat(range));
    }
    if selection.dragging {
        return Some(Action::Selecting);
    }
    let pointer = response
        .clicked()
        .then(|| response.interact_pointer_pos())??;
    let tick = local(pointer) as u64;
    let map = loaded.time.map(track)?;
    map.tick_at(&Position {
        measure: map.position(tick).measure,
        beat: 1,
        tick_numerator: 0,
        tick_denominator: 1,
    })
    .map(Action::Go)
}

pub(super) enum Action {
    Go(u64),
    Repeat((u64, u64)),
    Selecting,
}

pub(super) fn visible_id(generation: u64, track: usize, note_mode: bool) -> egui::Id {
    egui::Id::new(("track-map-visible", generation, track, note_mode))
}

#[cfg(test)]
mod tests;
