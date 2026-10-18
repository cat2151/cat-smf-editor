//! `r` overlay: the current track's notes as rectangles over a staff and beat grid.
use crate::{
    app::{App, Loaded},
    view::{
        comma_menu::{pressed, swallow},
        row::beat_color,
    },
};
use cat_smf_editor::score::{musical_time::Position, note_view::note_on};
use eframe::egui::{self, Color32};

/// Width of one quarter note.
pub(crate) const PX_PER_QUARTER: f32 = 96.0;
/// Height of one semitone row.
pub(crate) const ROW_HEIGHT: f32 = 10.0;
/// Semitones shown above the highest and below the lowest note.
const MARGIN: u8 = 2;
/// Treble staff E4 G4 B4 D5 F5 and bass staff G2 B2 D3 F3 A3.
const STAFF: [u8; 10] = [64, 67, 71, 74, 77, 43, 47, 50, 53, 57];
pub(crate) const STAFF_COLOR: Color32 = Color32::WHITE;

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// Scrolls to the cursor's measure on the next frame.
    reveal: bool,
    drag: Option<drag::Drag>,
    viewport: viewport::Viewport,
    drag_pane: Option<usize>,
    range_drag: Option<(usize, f32)>,
    edge_drag: Option<navigation::Edge>,
    repeat_drag: crate::view::timeline_repeat::State,
    repeat_rect: Option<egui::Rect>,
    keyboard: keyboard::State,
    /// Why the last drag was not applied.
    pub(crate) error: Option<String>,
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if app.comma_menu.open
        || app.edit_menu.open
        || app.track_view.go_open
        || app.track_view.split_open
        || app.help_open
        || app.playback.open
        || app.playback.mixer_focused()
        || app.player_selection.open
        || app.loaded.is_none()
        || ctx.wants_keyboard_input()
    {
        return;
    }
    if !app.piano_roll.open && !app.tempo_curve.open {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
            // The conductor has no notes, so its `r` opens the tempo curve instead.
            if app.track == 0 && app.loaded.as_ref().is_some_and(|l| l.music.format == 1) {
                app.tempo_curve = crate::view::tempo_curve::State::opened();
            } else {
                app.piano_roll = State {
                    open: true,
                    reveal: true,
                    ..State::default()
                };
            }
            swallow(ctx);
        }
        return;
    }
    crate::view::help::shortcuts(app, ctx);
    if app.help_open {
        return;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Comma)) {
        crate::view::comma_menu::open(app);
        swallow(ctx);
        return;
    }
    crate::play::playback::shortcuts(app, ctx);
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F)) {
        app.playback.follow = !app.playback.follow;
    }
    let keys = pressed(ctx);
    if keys
        .iter()
        .any(|key| matches!(key, egui::Key::R | egui::Key::Escape))
    {
        app.piano_roll.open = false;
        app.tempo_curve.open = false;
    }
    // The overlay owns the keyboard while open, so its keys never reach other shortcuts.
    swallow(ctx);
}

/// Positions inside the roll, with the origin at tick 0 and the highest shown pitch.
pub(crate) struct Layout {
    pub(crate) low: u8,
    pub(crate) high: u8,
    pub(crate) size: egui::Vec2,
    /// x of each beat boundary and its beat number in the measure.
    pub(crate) beats: Vec<(f32, u128)>,
    /// y of each staff line within the shown pitches.
    pub(crate) staff: Vec<f32>,
    /// Note On index within its track and its rectangle from Note On to Note Off.
    pub(crate) notes: Vec<(usize, egui::Rect)>,
    pub(crate) colors: Vec<Color32>,
    /// x where the measure holding `cursor_tick` begins.
    pub(crate) start: f32,
    /// Width of one tick.
    pub(crate) scale: f32,
}

pub(crate) fn layout(loaded: &Loaded, track: usize, cursor_tick: u64) -> Layout {
    layout_of(loaded, &[track], track, cursor_tick)
}

/// The notes of all `tracks` on one roll, with beats from the meter of track `meter`.
pub(crate) fn layout_of(
    loaded: &Loaded,
    tracks: &[usize],
    meter: usize,
    cursor_tick: u64,
) -> Layout {
    let music = &loaded.music;
    let division = music.division;
    // SMPTE division has no quarter note; 480 only sizes the drawing.
    let ppqn = if division == 0 || division & 0x8000 != 0 {
        480
    } else {
        division
    };
    let scale = PX_PER_QUARTER / f32::from(ppqn);
    // (Note On index, tick, duration, pitch)
    let ons: Vec<(usize, u64, u64, u8)> = tracks
        .iter()
        .flat_map(|&t| {
            let durations = &loaded.note_views[t].durations;
            music.tracks[t]
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| note_on(e))
                .filter_map(move |(i, e)| durations[i].map(|d| (i, e.tick, d, e.data[0])))
        })
        .collect();
    let low = ons.iter().map(|n| n.3).min().unwrap_or(60);
    let high = ons.iter().map(|n| n.3).max().unwrap_or(60);
    let (low, high) = (
        low.saturating_sub(MARGIN),
        high.saturating_add(MARGIN).min(127),
    );
    let top = |p: u8| f32::from(high - p) * ROW_HEIGHT;
    let end = music
        .tracks
        .iter()
        .filter_map(|t| t.events.last())
        .map(|e| e.tick)
        .max()
        .unwrap_or(0);
    let map = loaded.time.map(meter);
    let beats = beats_of(loaded, meter, end, scale);
    let start = map
        .and_then(|m| {
            m.tick_at(&Position {
                measure: m.position(cursor_tick).measure,
                beat: 1,
                tick_numerator: 0,
                tick_denominator: 1,
            })
        })
        .unwrap_or(0);
    Layout {
        low,
        high,
        size: egui::vec2(
            end as f32 * scale + PX_PER_QUARTER,
            f32::from(high - low + 1) * ROW_HEIGHT,
        ),
        beats,
        staff: STAFF
            .into_iter()
            .filter(|p| (low..=high).contains(p))
            .map(|p| top(p) + ROW_HEIGHT * 0.5)
            .collect(),
        colors: tracks
            .iter()
            .flat_map(|&t| {
                music.tracks[t]
                    .events
                    .iter()
                    .enumerate()
                    .filter(move |(i, e)| {
                        note_on(e) && loaded.note_views[t].durations[*i].is_some()
                    })
                    .map(move |(_, e)| {
                        super::pitch_keyboard::roll_color(
                            e.data[0],
                            loaded.keys.map(t).and_then(|k| k.at(e.tick)),
                        )
                    })
            })
            .collect(),
        notes: ons
            .iter()
            .map(|&(i, tick, duration, pitch)| {
                let left = tick as f32 * scale;
                let rect = egui::Rect::from_min_max(
                    egui::pos2(left, top(pitch)),
                    egui::pos2(left + duration as f32 * scale, top(pitch) + ROW_HEIGHT),
                );
                (i, rect)
            })
            .collect(),
        start: start as f32 * scale,
        scale,
    }
}

fn beats_of(loaded: &Loaded, meter: usize, end: u64, scale: f32) -> Vec<(f32, u128)> {
    loaded.time.map(meter).map_or_else(Vec::new, |m| {
        m.beat_runs(end)
            .iter()
            .flat_map(|run| {
                (0..run.count).map(move |i| {
                    let tick = run.tick as f64 + (i * run.step) as f64 / run.denominator as f64;
                    (tick as f32 * scale, run.position(i).beat)
                })
            })
            .collect()
    })
}

pub(crate) use window::show;

/// Beat lines over the full height of `rect`; a measure's first beat is thicker.
pub(crate) fn draw_beats(painter: &egui::Painter, rect: egui::Rect, beats: &[(f32, u128)]) {
    for &(x, beat) in beats {
        let width = if beat == 1 { 1.5_f32 } else { 0.7_f32 };
        painter.vline(
            rect.left() + x,
            rect.y_range(),
            egui::Stroke::new(width, beat_color(beat)),
        );
    }
}

/// `moved` replaces one rectangle of `layout.notes` while it is dragged.
/// `hover` is the index into `layout.notes` and the part to paint in its hover color.
fn draw(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout: &Layout,
    moved: Option<(usize, egui::Rect)>,
    hover: Option<(usize, hover::Part)>,
) {
    let origin = rect.min.to_vec2();
    draw_beats(painter, rect, &layout.beats);
    for &y in &layout.staff {
        painter.hline(
            rect.x_range(),
            rect.top() + y,
            egui::Stroke::new(1.0_f32, STAFF_COLOR),
        );
    }
    let note = |i: usize| match moved {
        Some((m, rect)) if m == i => rect,
        _ => layout.notes[i].1,
    };
    for i in 0..layout.notes.len() {
        painter.rect(
            note(i).translate(origin),
            0.0,
            layout.colors[i],
            egui::Stroke::new(0.5_f32, layout.colors[i].gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
    }
    if let Some((i, part)) = hover {
        painter.rect(
            hover::part_rect(note(i), part).translate(origin),
            0.0,
            part.color(),
            egui::Stroke::new(1.0_f32, Color32::BLACK),
            egui::StrokeKind::Inside,
        );
    }
}

mod drag;
mod hover;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) mod viewport;
mod window;

mod canvas;
mod panes;

mod navigation;

mod keyboard;
pub(crate) mod minimap;
