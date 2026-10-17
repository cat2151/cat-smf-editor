//! Clicking, dragging and `c` on the tempo curve.
use super::{circles, music, opened_in, squares, with_curve};
use crate::view::{
    piano_roll::tests::{at, press},
    tempo_curve::{CONTROL, POINT, TEMPO},
    tests::app,
};
use cat_smf_editor::{
    edit::tempo_curve::{self, Curve, Point},
    model::Music,
};
use eframe::egui::{self, epaint::Shape};

/// 480 ticks at 96px per 480 ticks.
const QUARTER: f32 = 96.0;

fn button(pos: egui::Pos2, which: egui::PointerButton, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: which,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// Presses and releases `which` at `pos` without moving, one frame per step from `time`.
pub(super) fn click(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    pos: egui::Pos2,
    which: egui::PointerButton,
) -> egui::FullOutput {
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(app, ctx, time + 0.01, vec![button(pos, which, true)]);
    at(app, ctx, time + 0.02, vec![button(pos, which, false)]);
    at(app, ctx, time + 0.03, vec![])
}

fn left(app: &mut crate::app::App, ctx: &egui::Context, time: f64, pos: egui::Pos2) {
    click(app, ctx, time, pos, egui::PointerButton::Primary);
}

fn right(app: &mut crate::app::App, ctx: &egui::Context, time: f64, pos: egui::Pos2) {
    click(app, ctx, time, pos, egui::PointerButton::Secondary);
}

/// Presses at `from` and moves to `to`, one frame per step from `time`.
/// Returns a frame drawn while the button is still down at `to`.
pub(super) fn hold(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    from: egui::Pos2,
    to: egui::Pos2,
) -> egui::FullOutput {
    let primary = egui::PointerButton::Primary;
    let steps = [
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, primary, true)],
        vec![egui::Event::PointerMoved(from + (to - from) * 0.5)],
        vec![egui::Event::PointerMoved(to)],
    ];
    for (i, events) in steps.into_iter().enumerate() {
        at(app, ctx, time + i as f64 * 0.01, events);
    }
    at(app, ctx, time + 0.04, vec![])
}

pub(super) fn release(app: &mut crate::app::App, ctx: &egui::Context, time: f64, pos: egui::Pos2) {
    let primary = egui::PointerButton::Primary;
    at(app, ctx, time, vec![button(pos, primary, false)]);
    at(app, ctx, time + 0.01, vec![]);
}

fn drag(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    from: egui::Pos2,
    to: egui::Pos2,
) {
    hold(app, ctx, time, from, to);
    release(app, ctx, time + 0.05, to);
}

fn music_of(app: &crate::app::App) -> &Music {
    &app.loaded.as_ref().unwrap().music
}

fn curve_of(app: &crate::app::App) -> Option<Curve> {
    tempo_curve::read(music_of(app)).unwrap()
}

/// Every event of every track, to compare songs.
fn fingerprint(music: &Music) -> String {
    format!("{:?}", music.tracks)
}

/// (tick, µs) of every conductor tempo event.
fn tempos_in(music: &Music) -> Vec<(u64, u32)> {
    music.tracks[0]
        .events
        .iter()
        .filter(|e| e.status == 0xff && e.meta_type == 0x51)
        .map(|e| {
            (
                e.tick,
                u32::from_be_bytes([0, e.data[0], e.data[1], e.data[2]]),
            )
        })
        .collect()
}

fn straight(a: Point, b: Point) -> Point {
    Point {
        tick: a.tick + (b.tick - a.tick) / 2,
        bpm: (a.bpm + b.bpm) / 2.0,
    }
}

/// The controls of `curve` are the straight ones of its segments; BPM within the last bits
/// that the stored JSON keeps.
fn assert_straight(curve: &Curve) {
    assert_eq!(curve.controls.len(), curve.points.len() - 1, "{curve:?}");
    for (w, c) in curve.points.windows(2).zip(&curve.controls) {
        let want = straight(w[0], w[1]);
        assert_eq!(c.tick, want.tick, "{curve:?}");
        assert!((c.bpm - want.bpm).abs() < 1e-9, "{curve:?}");
    }
}

fn centers(output: &egui::FullOutput, color: egui::Color32) -> Vec<egui::Pos2> {
    circles(output, color).iter().map(|c| c.0).collect()
}

#[test]
fn a_left_click_on_a_song_without_a_curve_stores_one_point_and_rewrites_the_tempos() {
    let mut app = app(music());
    let (ctx, output) = opened_in(&mut app, 0);
    assert!(curve_of(&app).is_none());
    // The 120 BPM tempo at tick 0 marks the roll's left edge and the 120 BPM height.
    let first = centers(&output, TEMPO)[0];
    let generation = app.generation;
    left(&mut app, &ctx, 11.0, first + egui::vec2(QUARTER, 0.0));
    assert_eq!(app.generation, generation + 1);
    let curve = curve_of(&app).expect("a curve is stored");
    assert_eq!(curve.points.len(), 1);
    assert!(curve.controls.is_empty());
    assert!(curve.points[0].tick.abs_diff(480) <= 1, "{curve:?}");
    assert!((curve.points[0].bpm - 120.0).abs() < 0.5, "{curve:?}");
    // One point is a flat curve: a single tempo at tick 0 replaces 120 then 150 BPM.
    let micros = (60_000_000.0 / curve.points[0].bpm).round() as u32;
    assert_eq!(tempos_in(music_of(&app)), [(0, micros)]);
    assert!(app.tempo_curve.error.is_none());
}

#[test]
fn added_points_get_straight_control_points() {
    let mut app = app(music());
    let (ctx, output) = opened_in(&mut app, 0);
    let first = centers(&output, TEMPO)[0];
    left(&mut app, &ctx, 11.0, first);
    // The axis follows the stored curve, so later clicks are placed from the drawn point.
    let first = centers(&at(&mut app, &ctx, 11.5, vec![]), POINT)[0];
    left(
        &mut app,
        &ctx,
        12.0,
        first + egui::vec2(2.0 * QUARTER, -30.0),
    );
    let curve = curve_of(&app).unwrap();
    assert_eq!(curve.points.len(), 2, "{curve:?}");
    assert!(curve.points[1].bpm > curve.points[0].bpm);
    assert_straight(&curve);
    // A third point between them splits the segment into two straight ones.
    let first = centers(&at(&mut app, &ctx, 12.5, vec![]), POINT)[0];
    left(&mut app, &ctx, 13.0, first + egui::vec2(QUARTER, -40.0));
    let curve = curve_of(&app).unwrap();
    assert_eq!(curve.points.len(), 3, "{curve:?}");
    assert_straight(&curve);
}

#[test]
fn dragging_a_tempo_point_up_previews_until_release_then_stores_a_higher_bpm() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let before = music_of(&app).clone();
    let generation = app.generation;
    let second = centers(&output, POINT)[1];
    let to = second + egui::vec2(0.0, -40.0);
    let held = hold(&mut app, &ctx, 11.0, second, to);
    // While held nothing is stored, and the preview's tempo events differ from the stored ones.
    assert_eq!(fingerprint(music_of(&app)), fingerprint(&before));
    assert_eq!(app.generation, generation);
    assert_ne!(centers(&held, TEMPO), centers(&output, TEMPO));
    // The dragged point is drawn larger and pinned when the pointer leaves the BPM range.
    let dragged: Vec<_> = circles(&held, POINT)
        .into_iter()
        .filter(|c| c.1 > 5.0)
        .collect();
    assert_eq!(dragged.len(), 1);
    let pinned = egui::pos2(to.x, app.tempo_curve.viewport.tempo_rect.unwrap().top());
    assert!((dragged[0].0 - pinned).length() < 1.0);
    release(&mut app, &ctx, 11.05, to);
    assert_eq!(app.generation, generation + 1);
    let old = tempo_curve::read(&before).unwrap().unwrap();
    let curve = curve_of(&app).unwrap();
    assert!(curve.points[1].bpm > old.points[1].bpm, "{curve:?}");
    assert_eq!(curve.points[1].tick, old.points[1].tick);
    assert_eq!(curve.points[0], old.points[0]);
    assert_eq!(curve.points[2], old.points[2]);
    assert_eq!(curve.controls, old.controls);
    assert_ne!(tempos_in(music_of(&app)), tempos_in(&before));
    // Notes stay where they were.
    assert_eq!(
        fingerprint(&Music {
            tracks: music_of(&app).tracks[1..].to_vec(),
            ..before.clone()
        }),
        fingerprint(&Music {
            tracks: before.tracks[1..].to_vec(),
            ..before.clone()
        })
    );
}

#[test]
fn a_dragged_tempo_point_stays_between_its_neighbors() {
    for (dx, tick) in [(10_000.0, 1919), (-10_000.0, 1)] {
        let mut app = app(with_curve());
        let (ctx, output) = opened_in(&mut app, 0);
        let second = centers(&output, POINT)[1];
        drag(&mut app, &ctx, 11.0, second, second + egui::vec2(dx, 0.0));
        let curve = curve_of(&app).unwrap();
        assert_eq!(curve.points[1].tick, tick, "dx {dx}: {curve:?}");
        assert!((curve.points[1].bpm - 140.0).abs() < 1e-3, "{curve:?}");
        // The controls beside it are pulled into their shortened segments.
        assert!(curve.validate().is_ok());
    }
}

#[test]
fn controls_drag_directly_and_c_has_no_effect() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let before = curve_of(&app).unwrap();
    let control = squares(&output, CONTROL)[0].0.center();
    drag(
        &mut app,
        &ctx,
        11.0,
        control,
        control + egui::vec2(10_000.0, -40.0),
    );
    let curve = curve_of(&app).unwrap();
    assert_eq!(curve.points, before.points);
    assert_eq!(curve.controls[1], before.controls[1]);
    assert_eq!(curve.controls[0].tick, before.points[1].tick);
    assert!(curve.controls[0].bpm > before.controls[0].bpm);
    press(&mut app, &ctx, egui::Key::C);
    let output = at(&mut app, &ctx, 12.0, vec![]);
    let point = centers(&output, POINT)[2];
    drag(&mut app, &ctx, 13.0, point, point + egui::vec2(0.0, -20.0));
    assert!(curve_of(&app).unwrap().points[2].bpm > before.points[2].bpm);
    let output = at(&mut app, &ctx, 14.0, vec![]);
    let point = centers(&output, POINT)[0];
    left(
        &mut app,
        &ctx,
        15.0,
        point + egui::vec2(QUARTER * 5.0, -30.0),
    );
    assert_eq!(curve_of(&app).unwrap().points.len(), 4);
}

#[test]
fn a_right_click_removes_a_point_and_the_last_one_removes_the_curve_but_keeps_the_tempos() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let points = centers(&output, POINT);
    right(&mut app, &ctx, 11.0, points[1]);
    let curve = curve_of(&app).unwrap();
    assert_eq!(curve.points.len(), 2, "{curve:?}");
    assert_eq!(curve.points[1].tick, 1920);
    assert_straight(&curve);
    // Right-clicking empty space removes nothing.
    right(&mut app, &ctx, 12.0, points[1] + egui::vec2(0.0, 60.0));
    assert_eq!(curve_of(&app).unwrap().points.len(), 2);
    let output = at(&mut app, &ctx, 13.0, vec![]);
    let points = centers(&output, POINT);
    right(&mut app, &ctx, 14.0, points[1]);
    assert_eq!(curve_of(&app).unwrap().points.len(), 1);
    let tempos = tempos_in(music_of(&app));
    let output = at(&mut app, &ctx, 15.0, vec![]);
    right(&mut app, &ctx, 16.0, centers(&output, POINT)[0]);
    assert!(curve_of(&app).is_none());
    assert!(!music_of(&app).tracks[0]
        .events
        .iter()
        .any(tempo_curve::owned));
    assert_eq!(tempos_in(music_of(&app)), tempos);
    assert!(app.tempo_curve.error.is_none());
}

#[test]
fn repeat_row_drags_both_ways_loops_and_click_resumes_normal_playback() {
    for reverse in [false, true] {
        let mut app = app(music());
        let (ctx, output) = opened_in(&mut app, 0);
        // The repeat row is immediately above the roll (tempo tick 0 is its left edge).
        let dot = circles(&output, TEMPO)[0].0;
        let top = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Rect(r)
                    if r.fill == egui::Color32::from_gray(40)
                        && (r.rect.height() - 24.0).abs() < 0.01 =>
                {
                    Some(r.rect.center().y)
                }
                _ => None,
            })
            .unwrap();
        let a = egui::pos2(dot.x + QUARTER, top);
        let b = egui::pos2(dot.x + QUARTER * 3.0, top);
        let (from, to) = if reverse { (b, a) } else { (a, b) };
        let before = fingerprint(music_of(&app));
        drag(&mut app, &ctx, 11.0, from, to);
        assert_eq!(app.playback.repeat, Some((480, 1440)));
        let active = app.playback.active.as_ref().unwrap();
        assert!((active.start_seconds - 0.5).abs() < 1e-9);
        // Crosses the tempo change at tick 960: 0.5 + 0.4 seconds.
        assert!((active.end - 0.9).abs() < 1e-9);
        assert_eq!(active.sources.len(), 3);
        assert!(active
            .sources
            .iter()
            .flat_map(|(_, s)| &s.notes)
            .all(|n| n.start + n.duration <= 0.9 + 1e-9));
        // The note held across A is present at the start.
        assert!(active.sources[2]
            .1
            .notes
            .iter()
            .any(|n| n.pitch == 67 && n.start == 0.0));
        app.audition.set_test_seconds(0.25);
        let output = at(&mut app, &ctx, 12.0, vec![]);
        let marker = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::LineSegment { points, stroke }
                    if stroke.color == super::super::repeat::PLAYHEAD
                        && (points[0].y - (top - 12.0)).abs() < 0.1 =>
                {
                    Some(points[0].x)
                }
                _ => None,
            })
            .unwrap();
        assert!((marker - (dot.x + QUARTER * 1.5)).abs() < 0.1);
        app.audition.set_test_seconds(0.9);
        at(&mut app, &ctx, 12.1, vec![]);
        assert_eq!(
            app.playback.pending.as_ref().unwrap().active.start_seconds,
            0.5
        );
        at(&mut app, &ctx, 12.2, vec![]);
        app.audition.set_test_seconds(0.25);
        left(&mut app, &ctx, 13.0, a);
        assert_eq!(app.playback.repeat, None);
        let active = app.playback.active.as_ref().unwrap();
        assert_eq!(active.start_seconds, 0.75);
        assert!(active.end > 0.9);
        assert_eq!(fingerprint(music_of(&app)), before);
    }
}
