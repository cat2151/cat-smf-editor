//! Grand staff of the current track right of the single-column rows, scrolling with time.
use crate::app::App;
use crate::view::piano_roll::{self, minimap, viewport::BUTTON};
use crate::view::timeline_repeat as repeat;
use eframe::egui::{self, Color32};
use score::Staff;

mod beam;
mod glyphs;
mod notation;
mod score;
mod spacing;
#[cfg(test)]
mod tests;

/// Distance between two staff lines at vertical zoom 1.
const GAP: f32 = 9.0;
/// Room above the staff for text comments.
const COMMENTS: f32 = 34.0;
/// Share of the pitch color in notes that are not sounding on a dark background.
const TINT: f32 = 0.25;
const PAPER: Color32 = Color32::from_rgb(0xfb, 0xf9, 0xf2);
const INK: Color32 = Color32::from_gray(24);

#[derive(Default)]
pub struct State {
    viewport: piano_roll::viewport::Viewport,
    repeat_drag: repeat::State,
    light: bool,
    /// Cursor tick last scrolled into view while stopped.
    revealed: Option<u64>,
}

struct Palette {
    paper: Color32,
    ink: Color32,
    /// Staff lines and barlines.
    lines: Color32,
    others: Color32,
    comments: Color32,
}

impl Palette {
    fn new(light: bool, ui: &egui::Ui) -> Self {
        if light {
            Self {
                paper: PAPER,
                ink: INK,
                lines: INK,
                others: Color32::from_gray(200),
                comments: INK,
            }
        } else {
            Self {
                paper: ui.visuals().panel_fill,
                ink: Color32::WHITE,
                lines: Color32::from_gray(150),
                others: Color32::from_gray(70),
                comments: Color32::from_rgb(0xe6, 0xdb, 0x74),
            }
        }
    }

    /// Sounding notes blink in their pitch color; others are ink on paper, or a light gray
    /// faintly tinted with their pitch color.
    fn note(&self, light: bool, color: Color32, lit: bool) -> Color32 {
        match (lit, light) {
            (true, _) => color,
            (false, true) => self.ink,
            (false, false) => {
                let [r, g, b, _] = color.to_array();
                let tint = |c: u8| (220.0 * (1.0 - TINT) + f32::from(c) * TINT) as u8;
                Color32::from_rgb(tint(r), tint(g), tint(b))
            }
        }
    }
}

pub(crate) fn show(app: &mut App, ui: &mut egui::Ui) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    let track = app.track;
    let cursor = crate::play::playback::location(app).0;
    let playing = crate::play::transport::tick_for(app, track);
    let mut following = playing.filter(|_| {
        app.playback.follow && !ui.input(|i| i.pointer.any_down() || i.pointer.any_released())
    });
    // One list per track, since beams join notes of one track only.
    let mut voices = vec![score::notes(loaded, track)];
    let mut others: Vec<_> = (0..loaded.music.tracks.len())
        .filter(|&t| t != track)
        .map(|t| score::notes(loaded, t))
        .collect();
    let mut voiced = vec![track];
    // A track without notes shows the other tracks' notes in pitch colors instead of gray.
    if voices[0].is_empty() {
        voices = std::mem::take(&mut others);
        voiced = (0..loaded.music.tracks.len())
            .filter(|&t| t != track)
            .collect();
    }
    let mut overview = piano_roll::layout_of(loaded, &voiced, track, cursor);
    let mut layout = piano_roll::layout(loaded, track, cursor);
    let tempos = minimap::Data::read(loaded, track);
    tempos.extend_timeline(loaded, track, &mut overview);
    tempos.extend_timeline(loaded, track, &mut layout);
    let range = app.playback.repeat;
    let state = &mut app.staff;
    state.viewport.panes = 1;
    // Stopping also reveals the cursor, since playback scrolled away from it.
    let reveal = playing.is_none() && state.revealed != Some(cursor);
    state.revealed = playing.is_none().then_some(cursor);
    let palette = Palette::new(state.light, ui);
    let reference = playing.unwrap_or(cursor);
    ui.horizontal(|ui| {
        if let Some(title) = loaded.music.tracks.first().and_then(|t| t.name()) {
            ui.strong(title);
        }
        if let Some(bpm) = tempos.tempo_at(reference) {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 18.0), egui::Sense::hover());
            glyphs::Pen {
                painter: ui.painter(),
                origin: egui::pos2(rect.left() + 1.0, rect.bottom() - 4.0),
                gap: 5.0,
                color: ui.visuals().text_color(),
            }
            .note(0, true);
            ui.label(format!("= {bpm:.0}"));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.toggle_value(&mut state.light, "☀");
        });
    });
    let area = state.viewport.bars(ui, state.repeat_drag.is_none());
    let (area, navigated) = state
        .viewport
        .minimap(ui, area, &overview, tempos.layers(), playing);
    if navigated {
        app.playback.follow = false;
        following = None;
    }
    state.viewport.scale_layout(&mut layout);
    let gap = GAP * state.viewport.zoom().y;
    let ppqn = score::ppqn(loaded);
    let mut onsets: Vec<u64> = voices
        .iter()
        .chain(&others)
        .flatten()
        .map(|n| n.tick)
        .collect();
    onsets.sort_unstable();
    onsets.dedup();
    let end = (layout.size.x / layout.scale) as u64;
    let spacing = spacing::Spacing::new(
        spacing::bars(loaded, track, end, ppqn),
        &onsets,
        layout.scale,
        gap,
    );
    let left_tick = (state.viewport.scroll.x / layout.scale).max(0.0) as u64;
    let sharps = loaded
        .keys
        .map(track)
        .and_then(|k| k.at(left_tick))
        .map_or(0, |k| k.sharps());
    let strip = egui::Rect::from_min_max(
        area.min,
        egui::pos2(
            area.left() + gap * (4.6 + 1.1 * f32::from(sharps.unsigned_abs())),
            area.bottom(),
        ),
    );
    let area = area.with_min_x(strip.right());
    let all = || voices.iter().chain(&others).flatten();
    let steps = all().map(|n| n.spelled.step);
    let mut staff = Staff {
        top: 0.0,
        high: steps
            .clone()
            .max()
            .unwrap_or(0)
            .max(notation::TREBLE_TOP + 2)
            + 3,
        low: steps.min().unwrap_or(0).min(notation::BASS_BOTTOM - 2) - 3,
        gap,
    };
    let mut repeat_action = None;
    let mut clicked = None;
    let mut panel = area;
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        ui.spacing_mut().scroll.floating = false;
        let bar = ui.spacing().scroll.allocated_width();
        let mut scrollbar = egui::Rect::from_min_max(area.min, area.max - egui::Vec2::splat(bar))
            .shrink2(egui::Vec2::splat(BUTTON));
        scrollbar.min.y += repeat::HEIGHT;
        let mut scroll = egui::ScrollArea::both()
            .id_salt("staff")
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .scroll_bar_rect(scrollbar)
            .drag_to_scroll(false);
        let pending = state.viewport.pending.take();
        let center = |tick: u64| (spacing.x(tick) - (area.width() - bar) / 2.0).max(0.0);
        if let Some(tick) = following {
            scroll = scroll.scroll_offset(egui::vec2(
                center(tick),
                pending.unwrap_or(state.viewport.scroll).y,
            ));
        } else if reveal {
            scroll = scroll.scroll_offset(egui::vec2(center(cursor), state.viewport.scroll.y));
        } else if let Some(offset) = pending {
            scroll = scroll.scroll_offset(offset);
        }
        let output = scroll.show_viewport(ui, |ui, visible| {
            let (whole, response) = ui.allocate_exact_size(
                egui::vec2(
                    layout.size.x.max(visible.width()),
                    (COMMENTS + staff.height() + gap).max(visible.height() - repeat::HEIGHT)
                        + repeat::HEIGHT,
                ),
                egui::Sense::click(),
            );
            let screen = egui::Rect::from_min_max(
                whole.min + visible.min.to_vec2(),
                whole.min + visible.max.to_vec2(),
            );
            ui.painter().rect_filled(screen, 0.0, palette.paper);
            let row = egui::Rect::from_min_size(
                egui::pos2(whole.left(), screen.top()),
                egui::vec2(whole.width(), repeat::HEIGHT),
            );
            repeat_action = repeat::row(ui, row, layout.scale, range, &mut state.repeat_drag);
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                clicked = Some(spacing.tick(pos.x - whole.left()));
            }
            let painter = ui
                .painter()
                .with_clip_rect(screen.with_min_y(row.bottom()).intersect(ui.clip_rect()));
            staff.top = whole.top() + repeat::HEIGHT + COMMENTS;
            let x = |tick: u64| whole.left() + spacing.x(tick);
            staff.lines(&painter, screen.x_range(), palette.lines);
            for &(beat_x, beat) in &layout.beats {
                if beat == 1 {
                    staff.system_line(&painter, whole.left() + beat_x, 1.0, palette.lines);
                }
            }
            score::comments(
                &painter,
                loaded,
                &[0, track],
                x,
                whole.top() + repeat::HEIGHT + 2.0,
                palette.comments,
            );
            let shown = egui::Rangef::new(screen.left() - 4.0 * gap, screen.right() + 2.0 * gap);
            let beat = |tick| spacing.beat(tick, ppqn);
            for notes in &others {
                let gray = |_: &score::Note| palette.others;
                voice(&staff, &painter, notes, x, shown, beat, ppqn, gray);
            }
            // Every note starts lit, then alternates each eighth note while it sounds.
            let blink = u64::from(ppqn / 2).max(1);
            let color = |note: &score::Note| {
                let lit = playing.is_some_and(|tick| {
                    (note.tick..note.end).contains(&tick)
                        && ((tick - note.tick) / blink).is_multiple_of(2)
                });
                palette.note(state.light, note.color, lit)
            };
            for notes in &voices {
                voice(&staff, &painter, notes, x, shown, beat, ppqn, color);
            }
            if let Some(tick) = playing {
                painter.vline(
                    x(tick),
                    screen.y_range(),
                    egui::Stroke::new(2.0_f32, repeat::PLAYHEAD),
                );
            }
        });
        panel = output
            .inner_rect
            .with_min_y(output.inner_rect.top() + repeat::HEIGHT);
        state.viewport.finish(ui, area, panel, output.state.offset);
    });
    clefs(
        &ui.painter()
            .with_clip_rect(strip.with_min_y(panel.top()).with_max_y(panel.bottom())),
        strip,
        &staff,
        sharps,
        &palette,
    );
    if let Some(tick) = app.staff.viewport.seek.take() {
        crate::play::transport::timeline::seek(app, app.track, tick);
    }
    if let Some(tick) = clicked {
        crate::view::beat_audition::start(app, app.track, tick);
    }
    if let Some(action) = repeat_action {
        repeat::apply(app, action);
    }
}

/// One track's notes whose head lies in `shown`, with beams where they join.
#[allow(clippy::too_many_arguments)]
fn voice(
    staff: &Staff,
    painter: &egui::Painter,
    notes: &[score::Note],
    x: impl Fn(u64) -> f32 + Copy,
    shown: egui::Rangef,
    beat: impl Fn(u64) -> (usize, u64),
    ppqn: u16,
    color: impl Fn(&score::Note) -> Color32 + Copy,
) {
    let groups = beam::groups(notes, ppqn, beat);
    let mut beamed = vec![false; notes.len()];
    for group in &groups {
        beamed[group.clone()].fill(true);
    }
    for (note, &beamed) in notes.iter().zip(&beamed) {
        if shown.contains(x(note.tick)) {
            staff.note(painter, x(note.tick), note, color(note), !beamed);
        }
    }
    for group in groups {
        let (first, last) = (x(notes[group.start].tick), x(notes[group.end - 1].tick));
        if last >= shown.min && first <= shown.max {
            beam::draw(staff, painter, &notes[group], x, color);
        }
    }
}

/// The part that does not scroll: clefs and key signature on both staves.
fn clefs(painter: &egui::Painter, strip: egui::Rect, staff: &Staff, sharps: i8, palette: &Palette) {
    let gap = staff.gap;
    painter.rect_filled(strip, 0.0, palette.paper);
    staff.lines(
        painter,
        egui::Rangef::new(strip.left() + gap * 0.5, strip.right()),
        palette.lines,
    );
    staff.system_line(painter, strip.left() + gap * 0.5, 1.5, palette.lines);
    let pen = |step: i32, x: f32| glyphs::Pen {
        painter,
        origin: egui::pos2(strip.left() + x * gap, staff.y(step)),
        gap,
        color: palette.ink,
    };
    pen(notation::TREBLE_BOTTOM + 2, 1.6).treble_clef();
    pen(notation::BASS_BOTTOM + 6, 1.1).bass_clef();
    let alter = if sharps > 0 { 1 } else { -1 };
    for (i, step) in notation::key_steps(sharps).into_iter().enumerate() {
        let x = 3.9 + 1.1 * i as f32;
        pen(step, x).accidental(alter);
        pen(step - 14, x).accidental(alter);
    }
}
