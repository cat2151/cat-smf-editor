//! One measure of approximate MML, using the sibling mmlabc-to-smf-rust dialect.
use super::{
    score::{self, Note},
    Palette,
};
use crate::app::Loaded;
use cat_smf_editor::score::musical_time::Position;
use eframe::egui::{self, Color32};

const FONT: f32 = 14.0;
const LINE: f32 = 20.0;
const PAD: f32 = 8.0;

#[derive(Clone, Copy)]
struct Tone {
    color: Color32,
    start: u64,
    end: u64,
}
struct Token {
    text: String,
    tone: Option<Tone>,
    tick: u64,
}
type Tokens = Vec<Token>;
pub(super) struct Row {
    pub track: usize,
    tokens: Tokens,
    beats: Vec<u64>,
}

/// Use musical-time boundaries, including meter changes, rather than a 4/4 guess.
pub(super) fn measure(loaded: &Loaded, track: usize, tick: u64) -> Option<(u64, u64)> {
    let map = loaded.time.map(track)?;
    let mut position = Position {
        measure: map.position(tick).measure,
        beat: 1,
        tick_numerator: 0,
        tick_denominator: 1,
    };
    let start = map.tick_at(&position)?;
    position.measure += 1;
    let end = map.tick_at(&position)?;
    (end > start).then_some((start, end))
}

/// Actual beat boundaries in the selected measure (four columns for 4/4).
pub(super) fn beats(loaded: &Loaded, track: usize, span: Option<(u64, u64)>) -> Vec<u64> {
    let Some((start, end)) = span else {
        return Vec::new();
    };
    let Some(map) = loaded.time.map(track) else {
        return vec![start];
    };
    let measure = map.position(start).measure;
    (1..=map.position(end - 1).beat)
        .filter_map(|beat| {
            map.tick_at(&Position {
                measure,
                beat,
                tick_numerator: 0,
                tick_denominator: 1,
            })
        })
        .filter(|&tick| tick >= start && tick < end)
        .collect()
}

fn pitch(note: &Note) -> i32 {
    note.spelled.step.div_euclid(7) * 12
        + [0, 2, 4, 5, 7, 9, 11][note.spelled.step.rem_euclid(7) as usize]
        + i32::from(note.spelled.alter)
}

/// Group exact timing matches before assigning overlapping groups to voices.
fn split<'a>(notes: impl Iterator<Item = &'a Note>, ppqn: u16) -> Vec<Vec<&'a Note>> {
    let mut groups = std::collections::BTreeMap::<_, Vec<_>>::new();
    for note in notes.filter(|n| n.end > n.tick) {
        groups.entry((note.tick, note.end)).or_default().push(note);
    }
    let mut ordered = Vec::new();
    for mut group in groups.into_values() {
        group.sort_by_key(|n| (pitch(n), n.spelled.step));
        let mut names = std::collections::BTreeMap::<_, Vec<_>>::new();
        for &note in &group {
            names
                .entry((note.spelled.step.rem_euclid(7), note.spelled.alter))
                .or_default()
                .push(note);
        }
        // Eligibility uses original timing, before measure clipping or rounding.
        if names.len() >= 3 && group[0].end - group[0].tick >= u64::from(ppqn.max(1)).div_ceil(2) {
            ordered.push(group);
        } else {
            ordered.extend(names.into_values());
        }
    }
    ordered.sort_by_key(|g| {
        (
            g[0].tick,
            std::cmp::Reverse(pitch(g.last().unwrap())),
            g[0].end,
        )
    });
    let mut voices: Vec<Vec<&Note>> = Vec::new();
    for group in ordered {
        let high = group.last().unwrap();
        let free = voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.last().unwrap().end <= high.tick)
            .min_by_key(|(_, v)| (pitch(v.last().unwrap()) - pitch(high)).abs())
            .map(|(i, _)| i);
        if let Some(i) = free {
            voices[i].extend(group);
        } else {
            voices.push(group);
        }
    }
    voices
}

mod syntax;
use syntax::cell;
#[cfg(test)]
use syntax::{length, Length};

/// Rebuild this measure only. Keep each track contiguous, order its voices high-first,
/// and order active track groups by highest pitch, with silent groups at the bottom.
pub(super) fn rows(
    tracks: &[Vec<Note>],
    measure: Option<(u64, u64)>,
    ppqn: u16,
    beats: &[u64],
) -> Vec<Row> {
    let Some((start, end)) = measure else {
        return Vec::new();
    };
    let mut groups = Vec::new();
    for (track, notes) in tracks.iter().enumerate() {
        if notes.is_empty() {
            continue;
        }
        let mut voices = split(notes.iter().filter(|n| n.tick < end && n.end > start), ppqn);
        if voices.is_empty() {
            voices.push(Vec::new());
        }
        let mut rows = Vec::new();
        for voice in voices {
            let high = voice.iter().map(|n| pitch(n)).max();
            rows.push((
                high,
                Row {
                    track,
                    tokens: cell(&voice, start, end, ppqn),
                    beats: if beats.is_empty() {
                        vec![start]
                    } else {
                        beats.to_vec()
                    },
                },
            ));
        }
        rows.sort_by_key(|(high, _)| std::cmp::Reverse(*high));
        let high = rows[0].0;
        groups.push((
            high,
            track,
            rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>(),
        ));
    }
    groups.sort_by_key(|(high, track, _)| (std::cmp::Reverse(*high), *track));
    groups.into_iter().flat_map(|(_, _, rows)| rows).collect()
}

pub(super) fn height(rows: &[Row]) -> f32 {
    rows.len() as f32 * LINE + PAD
}

fn job<'a>(tokens: impl Iterator<Item = &'a Token>, playing: Option<u64>) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for token in tokens {
        let color = token.tone.map_or(Color32::from_gray(220), |tone| {
            Palette::dark_note(tone.color, score::lit(tone.start, tone.end, playing))
        });
        job.append(
            &token.text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(FONT),
                color,
                ..Default::default()
            },
        );
    }
    job
}

/// Fixed below the staff. Scrolling the score must not move or change the MML measure.
/// Returns the track of a clicked row.
pub(super) fn show(
    ui: &mut egui::Ui,
    rows: &[Row],
    current: usize,
    playing: Option<u64>,
    reveal: bool,
    reset: bool,
) -> Option<usize> {
    // Rows retain tracks that have notes elsewhere, even in a silent measure.
    let all_colored = !rows.iter().any(|row| row.track == current);
    let colored = |row: &Row| all_colored || row.track == current;
    let gray = Color32::from_gray(140);
    let font = egui::FontId::monospace(FONT);
    let labels: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let first = i == 0 || rows[i - 1].track != row.track;
            ui.painter().layout_no_wrap(
                if first {
                    format!("track{}", row.track + 1)
                } else {
                    String::new()
                },
                font.clone(),
                if colored(row) {
                    crate::view::track_color(row.track)
                } else {
                    gray
                },
            )
        })
        .collect();
    let label_width = labels
        .iter()
        .map(|g| g.size().x + PAD * 2.0)
        .fold(0.0_f32, f32::max);
    let galleys: Vec<Vec<_>> = rows
        .iter()
        .map(|row| {
            row.beats
                .iter()
                .enumerate()
                .map(|(i, &start)| {
                    let end = row.beats.get(i + 1).copied().unwrap_or(u64::MAX);
                    let mut job = job(
                        row.tokens
                            .iter()
                            .filter(|t| t.tick >= start && t.tick < end),
                        playing,
                    );
                    if !colored(row) {
                        for section in &mut job.sections {
                            section.format.color = gray;
                        }
                    }
                    ui.painter().layout_job(job)
                })
                .collect()
        })
        .collect();
    let count = rows.first().map_or(1, |r| r.beats.len()).max(1);
    let beat_width = galleys
        .iter()
        .flatten()
        .map(|g| g.size().x + PAD * 2.0)
        .fold(
            ((ui.available_width() - label_width) / count as f32).max(0.0),
            f32::max,
        );
    let width = label_width + count as f32 * beat_width;
    let mut scroll = egui::ScrollArea::both()
        .id_salt("staff_mml")
        .auto_shrink([false, false]);
    if reset {
        scroll = scroll.vertical_scroll_offset(0.0);
    }
    scroll
        .show(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, height(rows)), egui::Sense::click());
            let clicked = response
                .clicked()
                .then(|| response.interact_pointer_pos())
                .flatten()
                .and_then(|pos| rows.get(((pos.y - rect.top()) / LINE) as usize))
                .map(|row| row.track);
            ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
            for beat in 0..=count {
                ui.painter().vline(
                    rect.left() + label_width + beat as f32 * beat_width,
                    rect.y_range(),
                    egui::Stroke::new(
                        if beat == 0 { 1.5_f32 } else { 1.0_f32 },
                        crate::view::beat_color((beat % count + 1) as u128),
                    ),
                );
            }
            let first = rows.iter().position(|row| row.track == current);
            for (i, (label, cells)) in labels.into_iter().zip(galleys).enumerate() {
                let pos = rect.min + egui::vec2(PAD, i as f32 * LINE);
                if reveal && !reset && Some(i) == first {
                    ui.scroll_to_rect(
                        egui::Rect::from_min_size(pos, egui::vec2(1.0, LINE)),
                        Some(egui::Align::Center),
                    );
                }
                if !label.job.text.is_empty() {
                    ui.painter().galley(pos, label, Color32::WHITE);
                }
                for (beat, galley) in cells.into_iter().enumerate() {
                    if !galley.job.text.is_empty() {
                        ui.painter().galley(
                            pos + egui::vec2(label_width + beat as f32 * beat_width, 0.0),
                            galley,
                            Color32::WHITE,
                        );
                    }
                }
            }
            clicked
        })
        .inner
}

#[cfg(test)]
mod tests;
