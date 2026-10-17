use super::{
    hover::{hovered, lit, part_at, part_rect, Part},
    ROW_HEIGHT, STAFF_COLOR,
};
use crate::view::{note_edit::tests::render, row::beat_color, tests::app};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui::{self, epaint::Shape};

fn event(tick: u64, status: u8, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: if status == 0xff { 0x2f } else { 0 },
        data,
    }
}

/// An arpeggio C4 E4 G4 C5 in measure 1 and one more C4 in measure 3, at 480 PPQN.
/// End of Track is at measure 9, so the roll can scroll measure 3 to its left edge.
const NOTES: [(u64, u64, u8); 5] = [
    (0, 240, 60),
    (240, 240, 64),
    (480, 480, 67),
    (960, 120, 72),
    (3840, 960, 60),
];

fn arpeggio() -> crate::app::App {
    with_notes(&NOTES)
}

fn with_notes(notes: &[(u64, u64, u8)]) -> crate::app::App {
    let mut events: Vec<Event> = notes
        .iter()
        .flat_map(|&(tick, duration, key)| {
            [
                event(tick, 0x90, vec![key, 80]),
                event(tick + duration, 0x80, vec![key, 0]),
            ]
        })
        .collect();
    events.sort_by_key(|e| e.tick);
    events.push(event(15360, 0xff, vec![]));
    let mut track = Track { events };
    cat_smf_editor::edit::refresh_deltas(&mut track).unwrap();
    // Format 0: `r` on track 0 of format 1 opens the tempo curve instead.
    app(Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    })
}

pub(crate) fn press(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    key: egui::Key,
) -> egui::FullOutput {
    render(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect(),
    )
}

pub(crate) fn at(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

/// Opens the roll and returns its context and a frame drawn at time 10 s,
/// after the window's fade-in has finished.
fn opened_in(app: &mut crate::app::App) -> (egui::Context, egui::FullOutput) {
    let ctx = egui::Context::default();
    render(app, &ctx, vec![]);
    press(app, &ctx, egui::Key::R);
    assert!(app.piano_roll.open);
    at(app, &ctx, 0.0, vec![]);
    let output = at(app, &ctx, 10.0, vec![]);
    (ctx, output)
}

fn opened(app: &mut crate::app::App) -> egui::FullOutput {
    opened_in(app).1
}

/// Vertical lines in a beat color: (x, color, width).
fn beat_lines(output: &egui::FullOutput) -> Vec<(f32, egui::Color32, f32)> {
    let colors: Vec<_> = (1..=4).map(beat_color).collect();
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::LineSegment { points, stroke }
                if points[0].x == points[1].x && colors.contains(&stroke.color) =>
            {
                Some((points[0].x, stroke.color, stroke.width))
            }
            _ => None,
        })
        .collect()
}

fn all_notes(output: &egui::FullOutput) -> Vec<egui::Rect> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.stroke.width == 0.5 => Some(rect.rect),
            _ => None,
        })
        .collect()
}

fn note_rects(output: &egui::FullOutput) -> Vec<egui::Rect> {
    all_notes(output)
        .into_iter()
        .filter(|r| r.height() == ROW_HEIGHT)
        .collect()
}

#[test]
fn r_opens_and_r_or_escape_close_while_other_keys_stay_inside() {
    let mut app = arpeggio();
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::V);
    assert!(
        app.note_mode,
        "V must not reach the view while the roll is open"
    );
    press(&mut app, &ctx, egui::Key::R);
    assert!(!app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::V);
    assert!(!app.note_mode);
}

#[test]
fn beat_lines_use_the_four_beat_colors_on_every_quarter() {
    let mut app = arpeggio();
    let output = opened(&mut app);
    let lines = beat_lines(&output);
    // 8 measures of 4/4 up to End of Track at 15360, which is itself a beat.
    assert_eq!(lines.len(), 33);
    let scale = 96.0 / 480.0;
    for (i, &(x, color, width)) in lines.iter().enumerate() {
        assert!((x - lines[0].0 - i as f32 * 480.0 * scale).abs() < 0.01);
        assert_eq!(color, beat_color(i as u128 % 4 + 1));
        assert_eq!(width, if i % 4 == 0 { 1.5 } else { 0.7 });
    }
    let staff = output
        .shapes
        .iter()
        .filter(|c| {
            matches!(&c.shape, Shape::LineSegment { points, stroke }
                if points[0].y == points[1].y && stroke.color == STAFF_COLOR)
        })
        .count();
    // Bb3..D5 (C4..C5 plus the margin) holds treble E4 G4 B4 D5 only.
    assert_eq!(staff, 4);
}

#[test]
fn note_rectangles_span_note_on_to_note_off_on_their_pitch_row() {
    let mut app = arpeggio();
    let output = opened(&mut app);
    let rects = note_rects(&output);
    assert_eq!(rects.len(), NOTES.len());
    let scale = 96.0 / 480.0;
    let origin = rects[0].left() - NOTES[0].0 as f32 * scale;
    let top = rects[0].top() + (f32::from(NOTES[0].2) - 72.0 - 2.0) * ROW_HEIGHT;
    for (rect, &(tick, duration, key)) in rects.iter().zip(&NOTES) {
        assert!((rect.left() - origin - tick as f32 * scale).abs() < 0.01);
        assert!((rect.right() - origin - (tick + duration) as f32 * scale).abs() < 0.01);
        assert!((rect.top() - top - (72.0 + 2.0 - f32::from(key)) * ROW_HEIGHT).abs() < 0.01);
    }
    // Beat lines share the origin: the first one is tick 0.
    assert!((beat_lines(&output)[0].0 - origin).abs() < 0.01);
}

#[test]
fn the_roll_starts_at_the_cursors_measure() {
    let mut unscrolled = arpeggio();
    let left = beat_lines(&opened(&mut unscrolled))[0].0;
    let mut app = arpeggio();
    // Beat 2 of measure 3; the roll starts at the measure, not the beat.
    crate::cursor::split::go(&mut app, 0, 4320);
    let lines = beat_lines(&opened(&mut app));
    assert!(
        (lines[8].0 - left).abs() < 0.01,
        "measure 3 at x {} should sit where measure 1 sits unscrolled, {left}",
        lines[8].0
    );
}

/// Filled rectangles in `color`.
fn rects_in(output: &egui::FullOutput, color: egui::Color32) -> Vec<egui::Rect> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect)
                if rect.fill == color
                    && rect.stroke == egui::Stroke::new(1.0_f32, egui::Color32::BLACK) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

fn near(a: egui::Rect, b: egui::Rect) -> bool {
    (a.min - b.min).length() < 0.01 && (a.max - b.max).length() < 0.01
}

#[test]
fn start_middle_and_end_parts_each_occupy_one_third_of_the_note() {
    let wide = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(96.0, ROW_HEIGHT));
    for (x, part) in [
        (99.9, None),
        (100.0, Some(Part::Start)),
        (131.9, Some(Part::Start)),
        (132.0, Some(Part::Middle)),
        (164.0, Some(Part::Middle)),
        (164.1, Some(Part::End)),
        (196.0, Some(Part::End)),
        (196.1, None),
    ] {
        assert_eq!(part_at(wide, x), part, "x {x} on a 96px note");
    }
    let narrow = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(16.0, ROW_HEIGHT));
    for (x, part) in [
        (105.3, Part::Start),
        (105.4, Part::Middle),
        (110.6, Part::Middle),
        (110.7, Part::End),
    ] {
        assert_eq!(part_at(narrow, x), Some(part), "x {x} on a 16px note");
    }
    // The three parts tile the note.
    for note in [wide, narrow] {
        let [start, middle, end] =
            [Part::Start, Part::Middle, Part::End].map(|p| part_rect(note, p));
        for part in [start, middle, end] {
            assert!((part.width() - note.width() / 3.0).abs() < 0.001);
        }
        assert_eq!(start.left(), note.left());
        assert_eq!(start.right(), middle.left());
        assert_eq!(middle.right(), end.left());
        assert_eq!(end.right(), note.right());
        assert!([start, middle, end]
            .iter()
            .all(|r| r.y_range() == note.y_range()));
    }
}

#[test]
fn hover_needs_the_pointer_on_the_notes_row_and_prefers_the_note_drawn_last() {
    let row = |left: f32, width: f32| {
        egui::Rect::from_min_size(egui::pos2(left, 20.0), egui::vec2(width, ROW_HEIGHT))
    };
    let notes = [(3, row(0.0, 96.0)), (7, row(48.0, 96.0))];
    assert_eq!(hovered(&notes, egui::pos2(20.0, 19.9)), None);
    assert_eq!(
        hovered(&notes, egui::pos2(40.0, 25.0)),
        Some((0, Part::Middle))
    );
    assert_eq!(
        hovered(&notes, egui::pos2(50.0, 25.0)),
        Some((1, Part::Start))
    );
    assert_eq!(
        hovered(&notes, egui::pos2(140.0, 30.0)),
        Some((1, Part::End))
    );
}

#[test]
fn the_hover_color_blinks_every_half_second() {
    for (time, on) in [
        (0.0, true),
        (0.24, true),
        (0.25, false),
        (0.49, false),
        (0.5, true),
        (10.1, true),
        (10.3, false),
    ] {
        assert_eq!(lit(time), on, "time {time}");
    }
}

#[test]
fn hovering_a_part_paints_it_in_its_color_only_in_the_lit_half_of_the_blink() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    // G4, 480 ticks = 96px wide, so the start and end parts are 8px.
    let note = note_rects(&output)[2];
    // Time only moves forward: each part gets its own second, lit at the top of it.
    for (time, part, x) in [
        (11.0, Part::Start, note.left() + 4.0),
        (12.0, Part::Middle, note.center().x),
        (13.0, Part::End, note.right() - 4.0),
    ] {
        let pointer = egui::pos2(x, note.center().y);
        at(
            &mut app,
            &ctx,
            time,
            vec![egui::Event::PointerMoved(pointer)],
        );
        let output = at(&mut app, &ctx, time, vec![]);
        let colors = [Part::Start, Part::Middle, Part::End].map(|p| rects_in(&output, p.color()));
        for (other, rects) in [Part::Start, Part::Middle, Part::End].iter().zip(&colors) {
            if *other == part {
                assert_eq!(rects.len(), 1, "{part:?} hovered");
                assert!(
                    near(rects[0], part_rect(note, part)),
                    "{part:?}: {:?}",
                    rects[0]
                );
            } else {
                assert!(
                    rects.is_empty(),
                    "{other:?} painted while {part:?} is hovered"
                );
            }
        }
        let dark = at(&mut app, &ctx, time + 0.3, vec![]);
        assert!(
            rects_in(&dark, part.color()).is_empty(),
            "{part:?} in the dark half"
        );
        assert_eq!(note_rects(&dark).len(), NOTES.len());
    }
    at(&mut app, &ctx, 14.0, vec![egui::Event::PointerGone]);
    let gone = at(&mut app, &ctx, 14.0, vec![]);
    for part in [Part::Start, Part::Middle, Part::End] {
        assert!(rects_in(&gone, part.color()).is_empty());
    }
}

#[test]
fn hovering_works_when_the_roll_is_taller_than_the_window() {
    // C1 and C8 widen the pitch range so the roll scrolls vertically too.
    let mut notes = NOTES.to_vec();
    notes.extend([(1920, 240, 24), (1920, 240, 108)]);
    let mut app = with_notes(&notes);
    let (ctx, output) = opened_in(&mut app);
    let note = note_rects(&output)[2];
    at(
        &mut app,
        &ctx,
        11.0,
        vec![egui::Event::PointerMoved(note.center())],
    );
    let output = at(&mut app, &ctx, 11.0, vec![]);
    assert_eq!(rects_in(&output, Part::Middle.color()).len(), 1);
}

mod drag;

mod navigation;

mod overview_repeat;
mod split;

mod playback;
mod seek;

mod gestures;

mod colors;

mod keyboard;
mod tempo_layers;
