//! Notes, ledger lines and text comments on the grand staff.
use super::{
    glyphs::Pen,
    notation::{self, Spelled},
};
use crate::app::Loaded;
use cat_smf_editor::score::note_view::note_on;
use eframe::egui::{self, Color32};

pub(super) struct Note {
    pub tick: u64,
    pub end: u64,
    pub spelled: Spelled,
    pub value: i32,
    pub accidental: Option<i8>,
    pub color: Color32,
}

/// Shared staff/MML pulse: start bright, then alternate each eighth while sounding.
pub(super) fn lit(start: u64, end: u64, playing: Option<u64>, ppqn: u16) -> bool {
    let blink = u64::from(ppqn / 2).max(1);
    playing.is_some_and(|tick| {
        (start..end).contains(&tick) && ((tick - start) / blink).is_multiple_of(2)
    })
}

/// SMPTE division has no quarter note; 480 only sizes the notes.
pub(super) fn ppqn(loaded: &Loaded) -> u16 {
    let division = loaded.music.division;
    if division == 0 || division & 0x8000 != 0 {
        480
    } else {
        division
    }
}

/// Notes of `track`, with accidentals only where the measure needs one.
pub(super) fn notes(loaded: &Loaded, track: usize) -> Vec<Note> {
    let ppqn = ppqn(loaded);
    let keys = loaded.keys.map(track);
    let meter = loaded.time.map(track);
    let durations = &loaded.note_views[track].durations;
    let mut accidentals = notation::Accidentals::default();
    loaded.music.tracks[track]
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .filter_map(|(i, e)| {
            let duration = durations[i]?;
            let key = keys.and_then(|k| k.at(e.tick));
            let sharps = key.map_or(0, |k| k.sharps());
            let spelled = notation::spell(e.data[0], sharps);
            let measure = meter.map_or(0, |m| m.position(e.tick).measure);
            Some(Note {
                tick: e.tick,
                end: e.tick + duration,
                spelled,
                value: notation::value(duration, ppqn),
                accidental: accidentals.next(measure, spelled, sharps),
                color: crate::view::pitch_keyboard::roll_color(e.data[0], key),
            })
        })
        .collect()
}

/// Vertical placement: `high` is the step at `top`.
#[derive(Clone, Copy)]
pub(super) struct Staff {
    pub top: f32,
    pub high: i32,
    pub low: i32,
    pub gap: f32,
}

impl Staff {
    /// Natural vertical extent of the grand staff and the notes it draws.
    pub fn from_steps(steps: impl Iterator<Item = i32> + Clone, gap: f32) -> Self {
        Self {
            top: 0.0,
            high: steps
                .clone()
                .max()
                .unwrap_or(notation::TREBLE_TOP + 2)
                .max(notation::TREBLE_TOP + 2)
                + 3,
            low: steps
                .min()
                .unwrap_or(notation::BASS_BOTTOM - 2)
                .min(notation::BASS_BOTTOM - 2)
                - 3,
            gap,
        }
    }

    pub fn y(&self, step: i32) -> f32 {
        self.top + (self.high - step) as f32 * self.gap / 2.0
    }

    pub fn height(&self) -> f32 {
        (self.high - self.low) as f32 * self.gap / 2.0
    }

    /// Steps of both staves' ten lines.
    pub fn line_steps() -> impl Iterator<Item = i32> {
        (0..5).flat_map(|i| {
            [
                notation::TREBLE_BOTTOM + 2 * i,
                notation::BASS_BOTTOM + 2 * i,
            ]
        })
    }

    pub fn lines(&self, painter: &egui::Painter, x: egui::Rangef, color: Color32) {
        for step in Self::line_steps() {
            painter.hline(x, self.y(step), egui::Stroke::new(1.0_f32, color));
        }
    }

    /// Vertical line from the treble staff's top to the bass staff's bottom.
    pub fn system_line(&self, painter: &egui::Painter, x: f32, width: f32, color: Color32) {
        painter.vline(
            x,
            self.y(notation::TREBLE_TOP)..=self.y(notation::BASS_BOTTOM),
            egui::Stroke::new(width, color),
        );
    }

    /// Ledger lines, accidental and note with its head's left edge at `x`; without `stem`
    /// a beam draws the stem.
    pub fn note(&self, painter: &egui::Painter, x: f32, note: &Note, color: Color32, stem: bool) {
        let step = note.spelled.step;
        let pen = Pen {
            painter,
            origin: egui::pos2(x, self.y(step)),
            gap: self.gap,
            color,
        };
        let head = if stem {
            pen.note(note.value, stem_up(step))
        } else {
            pen.head(note.value)
        };
        let ledgers = (step..=notation::BASS_BOTTOM - 2)
            .chain(notation::TREBLE_TOP + 2..=step)
            .chain((step == notation::MIDDLE_C).then_some(step))
            .filter(|s| s % 2 == 1);
        for ledger in ledgers {
            painter.hline(
                x - self.gap * 0.35..=x + head + self.gap * 0.35,
                self.y(ledger),
                egui::Stroke::new(1.0_f32, color),
            );
        }
        if let Some(alter) = note.accidental {
            Pen {
                origin: egui::pos2(x - self.gap * 0.9, self.y(step)),
                ..pen
            }
            .accidental(alter);
        }
    }
}

/// Middle line of the note's staff; middle C belongs to the treble.
pub(super) fn middle(step: i32) -> i32 {
    if step >= notation::MIDDLE_C {
        notation::TREBLE_BOTTOM + 4
    } else {
        notation::BASS_BOTTOM + 4
    }
}

/// Stems point toward the middle line of the note's staff.
fn stem_up(step: i32) -> bool {
    step < middle(step)
}

/// Text events of `tracks` at their tick, stacked in lanes downward from `top`.
pub(super) fn comments(
    painter: &egui::Painter,
    loaded: &Loaded,
    tracks: &[usize],
    x: impl Fn(u64) -> f32,
    top: f32,
    color: Color32,
) {
    let mut events: Vec<_> = tracks
        .iter()
        .flat_map(|&t| loaded.music.tracks[t].events.iter())
        .filter(|e| e.status == 0xff && e.meta_type == 0x01 && !e.data.is_empty())
        .collect();
    events.sort_by_key(|e| e.tick);
    let clip = painter.clip_rect();
    // Right edge of each lane's last label.
    let mut lanes: Vec<f32> = Vec::new();
    for event in events {
        let text = String::from_utf8_lossy(&event.data);
        if text.trim().is_empty() {
            continue;
        }
        let galley = painter.layout_no_wrap(
            cat_smf_editor::edit::trill::label(event)
                .map_or_else(|| text.replace(['\r', '\n', '\t'], " "), str::to_owned),
            egui::FontId::proportional(12.0),
            color,
        );
        let x = x(event.tick);
        let width = galley.size().x;
        if x > clip.right() || x + width < clip.left() {
            continue;
        }
        let lane = lanes
            .iter()
            .position(|&right| right + 6.0 <= x)
            .unwrap_or_else(|| {
                lanes.push(f32::NEG_INFINITY);
                lanes.len() - 1
            });
        lanes[lane] = x + width;
        let y = top + lane as f32 * (galley.size().y + 1.0);
        painter.galley(egui::pos2(x, y), galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_point_to_the_middle_line() {
        assert!(stem_up(notation::MIDDLE_C));
        assert!(stem_up(notation::TREBLE_BOTTOM));
        assert!(!stem_up(notation::TREBLE_BOTTOM + 4));
        assert!(stem_up(notation::BASS_BOTTOM));
        assert!(!stem_up(notation::BASS_BOTTOM + 6));
    }
}
