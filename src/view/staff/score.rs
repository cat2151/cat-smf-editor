//! Notes, ledger lines and text comments on the grand staff.
use super::{
    glyphs::Pen,
    notation::{self, Spelled},
};
use crate::app::Loaded;
use crate::play::playback::Sounding;
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

/// Shared staff/MML highlight while the note sounds.
pub(super) fn lit(start: u64, end: u64, playing: Option<u64>) -> bool {
    playing.is_some_and(|tick| (start..end).contains(&tick))
}

/// SMPTE division has no quarter note; 480 only sizes the notes.
pub(super) fn ppqn(loaded: &Loaded) -> u16 {
    let division = loaded.visible().division;
    if division == 0 || division & 0x8000 != 0 {
        480
    } else {
        division
    }
}

/// Notes of `track` as `source` plays them, with accidentals only where the measure needs one.
pub(super) fn notes(loaded: &Loaded, track: usize, source: &Sounding) -> Vec<Note> {
    let ppqn = ppqn(loaded);
    let keys = loaded.keys.map(track);
    let meter = loaded.time.map(track);
    let durations = &source.durations;
    let mut accidentals = notation::Accidentals::default();
    source
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
    annotations(painter, loaded, tracks, x, top, color, true);
}

pub(super) fn comment_height(
    painter: &egui::Painter,
    loaded: &Loaded,
    tracks: &[usize],
    x: impl Fn(u64) -> f32,
) -> f32 {
    annotations(painter, loaded, tracks, x, 0.0, Color32::WHITE, false)
}

fn annotations(
    painter: &egui::Painter,
    loaded: &Loaded,
    tracks: &[usize],
    x: impl Fn(u64) -> f32,
    top: f32,
    color: Color32,
    draw: bool,
) -> f32 {
    let mut selected = tracks.to_vec();
    selected.sort_unstable();
    selected.dedup();
    let mut events: Vec<_> = selected
        .iter()
        .flat_map(|&t| loaded.visible().tracks[t].events.iter())
        .filter(|e| e.status == 0xff && e.meta_type == 0x01 && !e.data.is_empty())
        .collect();
    if crate::view::meta_text::guide_track(loaded, tracks).is_some() {
        events.extend(cat_smf_editor::score::chord_guide::chords(&loaded.music));
    }
    events.sort_by_key(|e| e.tick);
    let clip = painter.clip_rect();
    // Right edge of each lane's last label.
    let mut lanes: Vec<f32> = Vec::new();
    let mut height: f32 = 0.0;
    for event in events {
        let text = cat_smf_editor::score::chord_guide::chord(event).map_or_else(
            || String::from_utf8_lossy(&event.data),
            std::borrow::Cow::Borrowed,
        );
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
        let lane = lanes
            .iter()
            .position(|&right| right + 6.0 <= x)
            .unwrap_or_else(|| {
                lanes.push(f32::NEG_INFINITY);
                lanes.len() - 1
            });
        lanes[lane] = x + width;
        let y = top + lane as f32 * (galley.size().y + 1.0);
        height = height.max(y - top + galley.size().y + 2.0);
        if draw && x <= clip.right() && x + width >= clip.left() {
            painter.galley(egui::pos2(x, y), galley, color);
        }
    }
    height
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

    #[test]
    fn guide_chords_use_spacing_even_at_empty_onsets_and_with_own_key() {
        use cat_smf_editor::{edit, score::chord_guide};
        let mut music = edit::tracks::new_song();
        music.tracks[0].events = vec![edit::meta_input::Kind::KeySignature
            .event(0, "D minor")
            .unwrap()];
        chord_guide::add(&mut music).unwrap();
        for (tick, text) in [(0, "Isus4"), (1920, "I"), (2400, "V7")] {
            edit::insert(
                &mut music.tracks[1],
                chord_guide::event(tick, text).unwrap(),
            );
        }
        let source = cat_smf_editor::score::pattern::operations::create_source(&mut music).unwrap();
        let a = crate::view::tests::app(music);
        let loaded = a.loaded.as_ref().unwrap();
        let c = egui::Context::default();
        for (scale, offset) in [(0.1, 20.0), (0.2, -40.0)] {
            let axis = super::super::spacing::Spacing::new(
                vec![0.0, 1920.0, 3840.0],
                &[0, 1920, 2400],
                scale,
                8.0,
            );
            let out = c.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 300.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        comments(
                            ui.painter(),
                            loaded,
                            &[0],
                            |tick| offset + axis.x(tick),
                            20.0,
                            Color32::WHITE,
                        );
                    });
                },
            );
            for (tick, label) in [(1920, "I"), (2400, "V7")] {
                let texts: Vec<_> = out
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(t.pos),
                        _ => None,
                    })
                    .collect();
                assert_eq!(texts.len(), 1);
                assert!((texts[0].x - offset - axis.x(tick)).abs() < 0.01);
            }
            let raw = c.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    comments(
                        ui.painter(),
                        loaded,
                        &[0, source],
                        |tick| offset + axis.x(tick),
                        20.0,
                        Color32::WHITE,
                    );
                });
            });
            assert!(!raw.shapes.iter().any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if ["Isus4", "I", "V7"].contains(&t.galley.text()))));
        }
    }
}
