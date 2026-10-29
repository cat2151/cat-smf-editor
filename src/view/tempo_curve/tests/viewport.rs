use super::{circles, edit::click, music, opened_in, squares, with_curve};
use crate::view::{
    piano_roll::tests::at,
    tempo_curve::{Axis, Roll, Target, POINT, TEMPO},
    tests::app,
};
use cat_smf_editor::edit::tempo_curve::{Curve, Point};
use eframe::egui;

fn bars(output: &egui::FullOutput) -> Vec<egui::Rect> {
    squares(output, super::super::viewport::BAR_COLOR)
        .into_iter()
        .map(|s| s.0)
        .collect()
}

#[test]
fn nearest_mark_wins_including_nearby_points_and_exact_overlaps() {
    let roll = Roll {
        rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 200.0)),
        scale: 1.0,
        axis: Axis {
            low: 100.0,
            high: 200.0,
        },
    };
    let mut curve = Curve {
        version: 1,
        points: vec![
            Point {
                tick: 20,
                bpm: 150.0,
            },
            Point {
                tick: 100,
                bpm: 160.0,
            },
        ],
        controls: vec![Point {
            tick: 24,
            bpm: 150.0,
        }],
    };
    assert_eq!(
        roll.hit(&curve, egui::pos2(21.0, 100.0)),
        Some((Target::Point, 0))
    );
    assert_eq!(
        roll.hit(&curve, egui::pos2(23.0, 100.0)),
        Some((Target::Control, 0))
    );
    assert_eq!(roll.hit(&curve, egui::pos2(50.0, 50.0)), None);
    curve.controls[0] = curve.points[0];
    assert_eq!(
        roll.hit(&curve, egui::pos2(20.0, 100.0)),
        Some((Target::Point, 0))
    );
}

#[test]
fn edge_zoom_centers_the_clicked_time_or_bpm() {
    for edge in 0..4 {
        let mut view = super::super::viewport::Viewport::default();
        view.scroll = 500.0;
        view.width = 400.0;
        let base = Axis {
            low: 100.0,
            high: 200.0,
        };
        view.zoom_edge(edge, true, base, if edge < 2 { 100.0 } else { 0.25 });
        if edge < 2 {
            assert_eq!(view.pending_scroll, Some(700.0));
            view.scroll = view.pending_scroll.take().unwrap();
            view.zoom_edge(edge, false, base, 200.0);
            assert_eq!(view.pending_scroll, Some(400.0));
        } else {
            let axis = view.axis.unwrap();
            assert!((axis.low - (175.0 - 100.0 / 3.0)).abs() < 1e-10);
            assert!((axis.high - (175.0 + 100.0 / 3.0)).abs() < 1e-10);
            view.zoom_edge(edge, false, base, 0.5);
            let axis = view.axis.unwrap();
            assert!((axis.low - 125.0).abs() < 1e-10);
            assert!((axis.high - 225.0).abs() < 1e-10);
        }
    }
}

#[test]
fn bar_clicks_zoom_the_curve_but_leave_piano_pitch_positions_fixed() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let before = super::note_rects(&output);
    let points = circles(&output, POINT);
    let curve_top = points.iter().map(|p| p.0.y).fold(f32::INFINITY, f32::min);
    assert!(
        before.iter().all(|(note, _)| note.bottom() < curve_top),
        "notes must be above the tempo panel, never overlaid"
    );
    let edges = bars(&output);
    assert_eq!(edges.len(), 2);
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    assert!(edges[0].bottom() < panel.top());
    assert!(edges[1].right() < panel.left());
    let pos = egui::pos2(edges[1].center().x, (curve_top + points[0].0.y) / 2.0);
    click(&mut app, &ctx, 11.0, pos, egui::PointerButton::Primary);
    at(
        &mut app,
        &ctx,
        11.1,
        vec![egui::Event::PointerMoved(egui::Pos2::ZERO)],
    );
    let zoomed = at(&mut app, &ctx, 11.2, vec![]);
    assert_eq!(super::note_rects(&zoomed), before);
    assert_ne!(circles(&zoomed, POINT)[1].0.y, points[1].0.y);
    click(&mut app, &ctx, 12.0, pos, egui::PointerButton::Secondary);
    let restored = at(&mut app, &ctx, 12.1, vec![]);
    assert!((circles(&restored, POINT)[1].0.y - points[1].0.y).abs() < 0.01);
    let pos = edges[0].center();
    click(&mut app, &ctx, 13.0, pos, egui::PointerButton::Primary);
    let zoomed = at(&mut app, &ctx, 13.1, vec![]);
    let new_points = circles(&zoomed, POINT);
    let old_span = points[1].0.x - points[0].0.x;
    let new_span = new_points[1].0.x - new_points[0].0.x;
    assert!((new_span / old_span - 1.5).abs() < 0.01);

    for ((before, _), (after, _)) in before.iter().zip(super::note_rects(&zoomed)) {
        assert_eq!(before.y_range(), after.y_range());
    }
}

#[test]
fn full_pitch_range_does_not_change_the_overlay_height_or_tempo_axis() {
    let mut normal = app(music());
    let (_, output) = opened_in(&mut normal, 0);
    let mut song = music();
    for track in song.tracks.iter_mut().skip(1) {
        for event in &mut track.events {
            if event.status == 0x90 || event.status == 0x80 {
                event.data[0] = if event.tick == 0 { 0 } else { 127 };
            }
        }
    }
    let mut wide = app(song);
    let (_, wide_output) = opened_in(&mut wide, 0);
    assert_eq!(bars(&wide_output), bars(&output));
    assert_eq!(circles(&wide_output, TEMPO), circles(&output, TEMPO));
    let bottom = wide.tempo_curve.viewport.tempo_rect.unwrap().top();
    assert!(super::note_rects(&wide_output)
        .iter()
        .all(|(rect, _)| rect.bottom() < bottom));
}

#[test]
fn question_opens_contextual_help_and_escape_closes_only_help() {
    let mut app = app(with_curve());
    let (ctx, _) = opened_in(&mut app, 0);
    at(&mut app, &ctx, 11.0, vec![egui::Event::Text("?".into())]);
    assert!(app.help_open);
    at(&mut app, &ctx, 12.0, vec![]);
    assert_eq!(crate::view::help::test_target(&app), "Tempo");
    super::super::super::piano_roll::tests::press(&mut app, &ctx, egui::Key::F1);
    assert!(!app.help_open);
    assert!(app.tempo_curve.open);
    at(&mut app, &ctx, 13.0, vec![egui::Event::Text("?".into())]);
    assert!(app.help_open);
    at(&mut app, &ctx, 14.0, vec![egui::Event::Text("?".into())]);
    assert!(!app.help_open);
    assert!(app.tempo_curve.open);
}

#[test]
fn piano_panel_clicks_do_not_edit_the_tempo_curve() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let generation = app.generation;
    let note = super::note_rects(&output)[0].0.center();
    click(&mut app, &ctx, 11.0, note, egui::PointerButton::Primary);
    click(&mut app, &ctx, 12.0, note, egui::PointerButton::Secondary);
    assert_eq!(app.generation, generation);
}

#[test]
fn zoom_zone_drags_fit_the_selected_range_but_content_wheel_scrolls_time() {
    for edge in [0, 2] {
        for reverse in [false, true] {
            let mut song = music();
            for track in &mut song.tracks {
                track.events.last_mut().unwrap().tick = 192_000;
                cat_smf_editor::edit::refresh_deltas(track).unwrap();
            }
            let mut app = app(song);
            let (ctx, output) = opened_in(&mut app, 0);
            let zone = bars(&output)[usize::from(edge == 2)];
            app.tempo_curve.viewport.pending_scroll = Some(500.0);
            let output = at(&mut app, &ctx, 10.5, vec![]);
            let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
            let width = app.tempo_curve.viewport.width;
            let scroll = app.tempo_curve.viewport.scroll;
            let initial = circles(&output, TEMPO);
            let generation = app.generation;
            let pos = |fraction: f32| {
                if edge < 2 {
                    egui::pos2(panel.left() + width * fraction, zone.center().y)
                } else {
                    egui::pos2(zone.center().x, panel.top() + panel.height() * fraction)
                }
            };
            let (from, to) = if reverse {
                (pos(0.6), pos(0.2))
            } else {
                (pos(0.2), pos(0.6))
            };
            let preview = super::edit::hold(&mut app, &ctx, 11.0, from, to);
            assert_eq!(circles(&preview, TEMPO), initial);
            assert!(!squares(&preview, TEMPO.gamma_multiply(0.5)).is_empty());
            super::edit::release(&mut app, &ctx, 12.0, to);
            let output = at(&mut app, &ctx, 13.0, vec![]);
            let next = circles(&output, TEMPO);
            let axis = app.tempo_curve.viewport.axis;
            if edge < 2 {
                let factor = (next[1].0.x - next[0].0.x) / (initial[1].0.x - initial[0].0.x);
                assert!((factor - 2.25).abs() < 0.001);
                let selected_start = (scroll + width * 0.2) / 0.2;
                let scale = (next[1].0.x - next[0].0.x) / 960.0;
                assert!(
                    (next[0].0.x + selected_start * scale - (panel.left() + width * 0.05)).abs()
                        < 0.1
                );
                assert_eq!(axis, None);
            } else {
                let axis = axis.unwrap();
                // 120..150 BPM, selected y=20%..60% => 132..144 BPM.
                assert!((12.0 / (axis.high - axis.low) - 0.9).abs() < 1e-6);
                assert!(((axis.low + axis.high) / 2.0 - 138.0).abs() < 1e-5);
                assert_eq!(app.tempo_curve.viewport.scroll, scroll);
                assert_eq!(next[0].0.x, initial[0].0.x);
            }
            super::wheel::wheel(&mut app, &ctx, 14.0);
            let wheeled = at(&mut app, &ctx, 15.0, vec![]);
            let points = circles(&wheeled, TEMPO);
            assert!((points[1].0.x - points[0].0.x - (next[1].0.x - next[0].0.x)).abs() < 0.01);
            assert_eq!(app.tempo_curve.viewport.axis, axis);
            assert_eq!(app.generation, generation);
            assert!(app.playback.active.is_none() && app.playback.pending.is_none());
        }
    }
}
