//! Regression tests for the distinct upper/lower panel gestures.
use super::edit::{click, hold, release};
use super::{circles, music, opened_in, with_curve};
use crate::view::{
    piano_roll::tests::at,
    tempo_curve::{POINT, TEMPO},
    tests::app,
};
use cat_smf_editor::edit::tempo_curve;
use eframe::egui::{self, PointerButton as Button};

#[test]
fn blank_drag_creates_a_point_with_feedback_then_commits_once() {
    for has_curve in [false, true] {
        let mut app = app(if has_curve { with_curve() } else { music() });
        let (ctx, _) = opened_in(&mut app, 0);
        let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
        let from = egui::pos2(panel.left() + 130.0, panel.center().y + 25.0);
        let to = from + egui::vec2(20.0, -15.0);
        let before = app.generation;
        let output = hold(&mut app, &ctx, 11.0, from, to);
        assert_eq!(app.generation, before);
        let count = if has_curve { 4 } else { 1 };
        assert_eq!(circles(&output, POINT).len(), count);
        assert!(circles(&output, POINT)
            .iter()
            .any(|p| p.0.distance(to) < 1.0));
        release(&mut app, &ctx, 12.0, to);
        assert_eq!(app.generation, before + 1);
        let curve = tempo_curve::read(&app.loaded.as_ref().unwrap().music)
            .unwrap()
            .unwrap();
        assert_eq!(curve.points.len(), count);
        assert!(curve.points.iter().any(|p| p.tick == 750));
    }
}

#[test]
fn piano_selection_fills_ninety_percent_and_keeps_window_in_place() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let notes = super::note_rects(&output);
    let from = notes[0].0.center();
    let to = from + egui::vec2(200.0, 10.0);
    let width = app.tempo_curve.viewport.width;
    let old_panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    let generation = app.generation;
    hold(&mut app, &ctx, 11.0, from, to);
    release(&mut app, &ctx, 12.0, to);
    let result = at(&mut app, &ctx, 13.0, vec![]);
    let points = circles(&result, POINT);
    let scale = (points[1].0.x - points[0].0.x) / 960.0;
    assert!((1000.0 * scale - width * 0.9).abs() < 0.1);
    super::wheel::wheel(&mut app, &ctx, 14.0);
    let wheeled = at(&mut app, &ctx, 15.0, vec![]);
    let wheeled_points = circles(&wheeled, POINT);
    assert!(
        (wheeled_points[1].0.x - wheeled_points[0].0.x - (points[1].0.x - points[0].0.x)).abs()
            < 0.01
    );
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    assert_eq!(panel.y_range(), old_panel.y_range());
    assert_eq!(app.generation, generation);
    assert!(app.playback.pending.is_none() && app.playback.active.is_none());
}

#[test]
fn piano_click_plays_only_the_clicked_beat_across_all_tracks() {
    let mut app = app(music());
    app.playback.last = crate::play::playback::Mode::BeatOnly;
    let (ctx, output) = opened_in(&mut app, 0);
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    let y = super::note_rects(&output)[0].0.center().y;
    // The last beat of a 4/4 bar, after the tempo change at tick 960.
    click(
        &mut app,
        &ctx,
        11.0,
        egui::pos2(panel.left() + 310.0, y),
        Button::Primary,
    );
    let active = app.playback.active.as_ref().unwrap();
    assert!((active.start_seconds - 1.4).abs() < 1e-9);
    assert!((active.end - 0.4).abs() < 1e-9);
    assert_eq!(active.sources.len(), 3);
    assert!(active.sources[2]
        .1
        .notes
        .iter()
        .any(|n| n.pitch == 72 && n.start == 0.0));
    app.audition.set_test_seconds(0.41);
    at(&mut app, &ctx, 12.0, vec![]);
    assert!(app.playback.active.is_none());
    assert!(app.playback.pending.is_none());
    assert_eq!(crate::play::playback::location(&app).0, 1440);
    // Current-position playback keeps the clicked beat after the audition ends.
    super::super::super::piano_roll::tests::press(&mut app, &ctx, egui::Key::P);
    super::super::super::piano_roll::tests::press(&mut app, &ctx, egui::Key::P);
    let current = app.playback.pending.as_ref().unwrap();
    assert_eq!(app.playback.last, crate::play::playback::Mode::Position);
    assert!((current.active.start_seconds - 1.4).abs() < 1e-9);
    assert!(current.active.end > 0.4);
}

#[test]
fn right_click_and_drag_adjust_the_nearest_repeat_end_continuously() {
    let mut app = app(music());
    let (ctx, output) = opened_in(&mut app, 0);
    let origin = circles(&output, TEMPO)[0].0.x;
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    let y = panel.top() - panel.height() - 28.0;
    app.playback.repeat = Some((480, 1440));
    click(
        &mut app,
        &ctx,
        11.0,
        egui::pos2(origin + 120.0, y),
        Button::Secondary,
    );
    assert_eq!(app.playback.repeat, Some((600, 1440)));
    let from = egui::pos2(origin + 270.0, y);
    at(&mut app, &ctx, 12.0, vec![egui::Event::PointerMoved(from)]);
    at(
        &mut app,
        &ctx,
        12.1,
        vec![egui::Event::PointerButton {
            pos: from,
            button: Button::Secondary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for (i, x) in [300.0, 320.0, 350.0].into_iter().enumerate() {
        at(
            &mut app,
            &ctx,
            12.2 + i as f64 * 0.1,
            vec![egui::Event::PointerMoved(egui::pos2(origin + x, y))],
        );
        assert_eq!(app.playback.repeat, Some((600, (x / 0.2).round() as u64)));
    }
}

#[test]
fn bpm_scrollbar_moves_only_the_lower_axis() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let notes = super::note_rects(&output);
    let generation = app.generation;
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    let pos = egui::pos2(panel.right() + 20.0, panel.bottom() - 10.0);
    click(&mut app, &ctx, 11.0, pos, Button::Primary);
    assert!(
        app.tempo_curve.viewport.axis.is_some(),
        "vertical scrollbar must change the BPM viewport"
    );
    let output = at(&mut app, &ctx, 12.0, vec![]);
    assert_eq!(super::note_rects(&output), notes);
    assert_eq!(app.generation, generation);
}

#[test]
fn zooming_out_keeps_the_entire_visible_lower_panel_editable() {
    let mut app = app(music());
    let (ctx, _) = opened_in(&mut app, 0);
    for _ in 0..6 {
        app.tempo_curve.viewport.zoom_edge(
            0,
            false,
            super::super::Axis {
                low: 120.0,
                high: 150.0,
            },
            200.0,
        );
    }
    at(&mut app, &ctx, 11.0, vec![]);
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    let pos = egui::pos2(panel.right() - 100.0, panel.center().y);
    click(&mut app, &ctx, 12.0, pos, Button::Primary);
    let curve = tempo_curve::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .unwrap();
    assert_eq!(curve.points.len(), 1);
    assert!(curve.points[0].tick > 3840);
}

#[test]
fn upper_right_click_and_drag_adjust_nearest_view_end_without_playing() {
    for (right, dragging) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut song = music();
        for track in &mut song.tracks {
            track.events.last_mut().unwrap().tick = 192_000;
            cat_smf_editor::edit::refresh_deltas(track).unwrap();
        }
        let mut app = app(song);
        let (ctx, _) = opened_in(&mut app, 0);
        app.tempo_curve.viewport.pending_scroll = Some(500.0);
        let output = at(&mut app, &ctx, 11.0, vec![]);
        let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
        let width = app.tempo_curve.viewport.width;
        let y = panel.top() - 30.0;
        let fraction = if right { 0.8 } else { 0.2 };
        let pos = egui::pos2(panel.left() + width * fraction, y);
        let old_origin = circles(&output, TEMPO)[0].0.x;
        let generation = app.generation;
        if dragging {
            let from = pos + egui::vec2(if right { -20.0 } else { 20.0 }, 0.0);
            at(&mut app, &ctx, 12.0, vec![egui::Event::PointerMoved(from)]);
            at(
                &mut app,
                &ctx,
                12.1,
                vec![egui::Event::PointerButton {
                    pos: from,
                    button: Button::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            let preview = at(&mut app, &ctx, 12.2, vec![egui::Event::PointerMoved(pos)]);
            assert_eq!(circles(&preview, TEMPO)[0].0.x, old_origin);
            at(
                &mut app,
                &ctx,
                12.3,
                vec![egui::Event::PointerButton {
                    pos,
                    button: Button::Secondary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        } else {
            click(&mut app, &ctx, 12.0, pos, Button::Secondary);
        }
        let result = at(&mut app, &ctx, 13.0, vec![]);
        let dots = circles(&result, TEMPO);
        let scale = (dots[1].0.x - dots[0].0.x) / 960.0;
        let left = (500.0 + if right { 0.0 } else { width * fraction }) / 0.2;
        let end = (500.0 + if right { width * fraction } else { width }) / 0.2;
        assert!(((end - left) * scale - width * 0.9).abs() < 0.1);
        assert!((dots[0].0.x + left * scale - (panel.left() + width * 0.05)).abs() < 0.1);
        assert_eq!(app.generation, generation);
        assert!(app.playback.active.is_none() && app.playback.pending.is_none());
        super::wheel::wheel(&mut app, &ctx, 14.0);
        let wheeled = at(&mut app, &ctx, 15.0, vec![]);
        let next = circles(&wheeled, TEMPO);
        assert!((next[1].0.x - next[0].0.x - (dots[1].0.x - dots[0].0.x)).abs() < 0.01);
    }
}
