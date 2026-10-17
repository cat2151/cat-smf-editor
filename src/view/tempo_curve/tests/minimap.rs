use super::{
    circles,
    edit::{click, hold, release},
    music, opened_in, TEMPO,
};
use crate::{
    play::{playback, transport},
    view::{piano_roll::tests::at, tests::app},
};
use eframe::egui::{self, PointerButton};

fn long_song() -> crate::app::App {
    let mut song = music();
    for track in &mut song.tracks {
        track.events.last_mut().unwrap().tick = 192_000;
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    app(song)
}

#[test]
fn minimap_click_centers_time_and_seeks_running_playback_and_stops_follow() {
    let mut app = long_song();
    let (ctx, _) = opened_in(&mut app, 0);
    let generation = app.generation;
    let map = app.tempo_curve.viewport.minimap_rect.unwrap();
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    assert!(map.bottom() < panel.top() - panel.height());
    playback::start(&mut app, playback::Mode::AllTracks);
    transport::advance(&mut app);
    app.playback.follow = true;
    let pos = egui::pos2(map.left() + map.width() * 0.7, map.center().y);
    click(&mut app, &ctx, 11.0, pos, PointerButton::Primary);
    assert!(!app.playback.follow);
    let view = &app.tempo_curve.viewport;
    assert!((view.scroll + view.width / 2.0 - 38496.0 * 0.7).abs() < 0.1);
    assert_eq!(app.generation, generation);
    assert_eq!(app.playback.last, playback::Mode::AllTracks);
    assert!(app.playback.active.is_some());
    let clicked = crate::play::playback::location(&app).0;
    assert!(clicked > 100_000);
    let tempo = cat_smf_editor::score::audition_timing::TempoMap::new(
        &app.loaded.as_ref().unwrap().music,
        0,
    );
    assert!(
        (app.playback.active.as_ref().unwrap().start_seconds - tempo.duration(0, clicked)).abs()
            < 1e-8
    );
    let output = at(&mut app, &ctx, 12.0, vec![]);
    assert_eq!(app.tempo_curve.viewport.minimap_rect, Some(map));
    let dots = circles(&output, TEMPO);
    assert!((dots[1].0.x - dots[0].0.x - 192.0).abs() < 0.1);
}

#[test]
fn minimap_drag_pans_live_without_zooming_seeking_or_changing_repeat() {
    for (inside, delta) in [
        (true, 0.15),
        (true, -0.15),
        (false, 0.15),
        (true, -1.0),
        (true, 1.0),
    ] {
        let mut app = long_song();
        let (ctx, _) = opened_in(&mut app, 0);
        app.tempo_curve.viewport.zoom_edge(
            0,
            true,
            super::super::Axis {
                low: 120.0,
                high: 150.0,
            },
            100.0,
        );
        app.tempo_curve.viewport.pending_scroll = Some(10000.0);
        let output = at(&mut app, &ctx, 10.5, vec![]);
        let dots = circles(&output, TEMPO);
        let zoom = (dots[1].0.x - dots[0].0.x) / 192.0;
        let map = app.tempo_curve.viewport.minimap_rect.unwrap();
        let generation = app.generation;
        let before = app.tempo_curve.viewport.scroll;
        let width = app.tempo_curve.viewport.width;
        let end = 38496.0;
        let first = if inside {
            (before + width * 0.25) / zoom / end
        } else {
            0.6
        };
        let pos = |fraction| egui::pos2(map.left() + map.width() * fraction, map.center().y);
        let from = pos(first);
        let to = pos(first + delta);
        app.playback.repeat = Some((0, 480));
        app.playback.follow = true;
        let held = hold(&mut app, &ctx, 11.0, from, to);
        let expected = if inside {
            before + delta * end * zoom
        } else {
            (first + delta) * end * zoom - width / 2.0
        };
        let expected = expected.clamp(0.0, end * zoom - width - 2.0 * super::super::HIT);
        assert!((app.tempo_curve.viewport.scroll - expected).abs() < 0.1, "inside={inside} delta={delta} before={before} zoom={zoom} actual={} expected={expected}", app.tempo_curve.viewport.scroll);
        assert!(!app.playback.follow);
        let held_dots = circles(&held, TEMPO);
        assert!((held_dots[1].0.x - held_dots[0].0.x - 192.0 * zoom).abs() < 0.1);
        release(&mut app, &ctx, 12.0, to);
        assert!((app.tempo_curve.viewport.scroll - expected).abs() < 0.1, "inside={inside} delta={delta} before={before} zoom={zoom} actual={} expected={expected}", app.tempo_curve.viewport.scroll);
        assert_eq!(app.playback.repeat, Some((0, 480)));
        assert_eq!(app.tempo_curve.viewport.minimap_rect, Some(map));
        assert_eq!(app.tempo_curve.viewport.axis, None);
        assert_eq!(app.generation, generation);
        assert!(app.playback.active.is_none() && app.playback.pending.is_none());
    }
}

#[test]
fn minimap_overlays_tempos_curve_and_references_on_the_notes() {
    let mut app = app(super::with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let map = app.tempo_curve.viewport.minimap_rect.unwrap();
    let lines = |output: &egui::FullOutput, color| {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Path(path)
                    if super::color_of(&path.stroke) == Some(color)
                        && path.points.iter().all(|p| map.contains(*p)) =>
                {
                    Some(path.points.clone())
                }
                egui::epaint::Shape::LineSegment { points, stroke }
                    if stroke.color == color && points.iter().all(|p| map.contains(*p)) =>
                {
                    Some(points.to_vec())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(!lines(&output, TEMPO).is_empty());
    assert!(!lines(&output, super::CURVE).is_empty());
    assert!(!lines(&output, super::super::references::COLOR).is_empty());
    let before = lines(&output, TEMPO);
    app.tempo_curve.viewport.pending_scroll = Some(200.0);
    let output = at(&mut app, &ctx, 11.0, vec![]);
    assert_eq!(lines(&output, TEMPO), before);
    let prepared = super::super::trills::prepare(&mut app).unwrap();
    app.edit_music(|music| {
        *music = (*prepared).clone();
        cat_smf_editor::edit::reference_tempo::set(music, 0, 200.0)
    })
    .unwrap();
    let output = at(&mut app, &ctx, 12.0, vec![]);
    assert!(!lines(&output, super::super::references::MISMATCH).is_empty());
}

#[test]
fn minimap_playhead_moves_without_follow_and_disappears_on_stop() {
    let mut app = long_song();
    let (ctx, _) = opened_in(&mut app, 0);
    app.playback.follow = false;
    crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
    crate::play::transport::advance(&mut app);
    for seconds in [0.25, 0.75] {
        app.audition.set_test_seconds(seconds);
        let tick = crate::play::transport::tick(&app).unwrap();
        let output = at(&mut app, &ctx, 20.0 + seconds, vec![]);
        let map = app.tempo_curve.viewport.minimap_rect.unwrap();
        let positions: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::LineSegment { points, stroke }
                    if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                        && points[0].y == map.top()
                        && points[1].y == map.bottom() =>
                {
                    Some(points[0].x)
                }
                _ => None,
            })
            .collect();
        assert_eq!(positions.len(), 1);
        assert!((positions[0] - (map.left() + tick as f32 / 192480.0 * map.width())).abs() < 0.1);
    }
    crate::play::transport::stop(&mut app);
    let output = at(&mut app, &ctx, 22.0, vec![]);
    let map = app.tempo_curve.viewport.minimap_rect.unwrap();
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::LineSegment { points, stroke }
            if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                && points[0].y == map.top() && points[1].y == map.bottom())));
}

#[test]
fn minimap_drag_while_playing_turns_follow_off_and_keeps_playback_running_at_its_position() {
    let mut app = long_song();
    let (ctx, _) = opened_in(&mut app, 0);
    playback::start(&mut app, playback::Mode::AllTracks);
    transport::advance(&mut app);
    app.audition.set_test_seconds(1.0);
    app.playback.repeat = Some((0, 192000));
    app.playback.follow = true;
    let seconds = app.playback.active.as_ref().unwrap().start_seconds;
    let map = app.tempo_curve.viewport.minimap_rect.unwrap();
    let pos = |fraction| egui::pos2(map.left() + map.width() * fraction, map.center().y);
    hold(&mut app, &ctx, 20.0, pos(0.7), pos(0.8));
    assert!(!app.playback.follow);
    let view = &app.tempo_curve.viewport;
    assert!((view.scroll + view.width / 2.0 - 38496.0 * 0.8).abs() < 0.1);
    release(&mut app, &ctx, 21.0, pos(0.8));
    assert_eq!(app.playback.repeat, Some((0, 192000)));
    assert_eq!(app.playback.active.as_ref().unwrap().start_seconds, seconds);
}
