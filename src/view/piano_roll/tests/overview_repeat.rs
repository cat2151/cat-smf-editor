use super::navigation::{click, wheel, zone};
use super::*;

#[test]
fn minimap_pans_live_without_changing_zoom_playback_or_repeat() {
    for (inside, delta) in [
        (true, 0.2),
        (true, -0.2),
        (false, 0.2),
        (true, -1.0),
        (true, 1.0),
    ] {
        let mut app = arpeggio();
        let (ctx, _) = opened_in(&mut app);
        let pos = zone(&app, 1);
        click(&mut app, &ctx, 11.0, pos, false);
        let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
        let generation = app.generation;
        let center = egui::pos2(map.left() + map.width() * 0.6, map.center().y);
        app.playback.follow = true;
        click(&mut app, &ctx, 12.0, center, false);
        assert!(!app.playback.follow);
        let panel = app.piano_roll.viewport.panel.unwrap();
        let end = 15360.0 * 0.2 + 96.0;
        let before = app.piano_roll.viewport.scroll;
        assert!((before.x + panel.width() / 2.0 - end * 0.6).abs() < 0.1);
        let output = at(&mut app, &ctx, 12.1, vec![]);
        let note_size = all_notes(&output)[0].size();
        let first = if inside {
            (before.x + panel.width() * 0.25) / end
        } else {
            0.2
        };
        let pos = |fraction| egui::pos2(map.left() + map.width() * fraction, map.center().y);
        app.playback.repeat = Some((0, 480));
        app.playback.follow = true;
        let held = super::drag::drag(&mut app, &ctx, 13.0, pos(first), pos(first + delta));
        let expected = if inside {
            before.x + delta * end
        } else {
            (first + delta) * end - panel.width() / 2.0
        };
        let expected = expected.clamp(0.0, end - panel.width());
        let held_notes = all_notes(&held);
        assert!((held_notes[0].left() - (panel.left() - expected)).abs() < 0.2);
        assert!((app.piano_roll.viewport.scroll.x - expected).abs() < 0.2);
        assert_eq!(app.piano_roll.viewport.scroll.y, before.y);
        assert_eq!(held_notes[0].size(), note_size);
        assert!(!app.playback.follow);
        assert_eq!(app.playback.repeat, Some((0, 480)));
        assert_eq!(app.piano_roll.viewport.minimap_state.rect, Some(map));
        assert_eq!(app.generation, generation);
        assert!(app.playback.active.is_none() && app.playback.pending.is_none());
    }
}

#[test]
fn repeat_uses_zoomed_scrolled_ticks_and_stays_fixed_during_pitch_scroll() {
    for reverse in [false, true] {
        let mut app = with_notes(&[(0, 480, 0), (3840, 960, 127)]);
        app.playback.follow = false;
        crate::cursor::split::go(&mut app, 0, 3840);
        let (ctx, _) = opened_in(&mut app);
        let mut pos = zone(&app, 0);
        pos.x = app.piano_roll.viewport.panel.unwrap().left() + 100.0;
        click(&mut app, &ctx, 11.0, pos, false);
        let output = at(&mut app, &ctx, 11.1, vec![]);
        let notes = all_notes(&output);
        let scale = notes[1].width() / 960.0;
        let origin = notes[1].left() - 3840.0 * scale;
        let row = app.piano_roll.repeat_rect.unwrap();
        let (a, b) = (
            egui::pos2(origin + 3840.0 * scale, row.center().y),
            egui::pos2(origin + 4320.0 * scale, row.center().y),
        );
        let before = app.generation;
        let (from, to) = if reverse { (b, a) } else { (a, b) };
        super::drag::drag(&mut app, &ctx, 12.0, from, to);
        assert_eq!(app.playback.repeat, Some((3840, 4320)));
        assert!(app.playback.active.is_some() || app.playback.pending.is_some());
        let panel = app.piano_roll.viewport.panel.unwrap();
        let bar = app.piano_roll.viewport.bars[1].unwrap();
        wheel(&mut app, &ctx, 13.0, bar.center(), -100.0);
        assert!(app.piano_roll.viewport.scroll.y > 0.0);
        assert_eq!(app.piano_roll.repeat_rect.unwrap(), row);
        assert_eq!(app.piano_roll.viewport.panel.unwrap(), panel);
        let x = origin + 3960.0 * scale;
        click(&mut app, &ctx, 14.0, egui::pos2(x, row.center().y), true);
        assert_eq!(app.playback.repeat, Some((3960, 4320)));
        click(&mut app, &ctx, 15.0, a, false);
        assert_eq!(app.playback.repeat, None);
        assert_eq!(app.generation, before);
    }
}

#[test]
fn minimap_playhead_moves_without_follow_and_disappears_on_stop() {
    let mut app = arpeggio();
    let (ctx, _) = opened_in(&mut app);
    app.playback.follow = false;
    crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
    crate::play::transport::advance(&mut app);
    for seconds in [0.25, 0.75] {
        app.audition.set_test_seconds(seconds);
        let tick = crate::play::transport::tick(&app).unwrap();
        let output = at(&mut app, &ctx, 20.0 + seconds, vec![]);
        let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
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
        assert!((positions[0] - (map.left() + tick as f32 / 15840.0 * map.width())).abs() < 0.1);
    }
    crate::play::transport::stop(&mut app);
    let output = at(&mut app, &ctx, 22.0, vec![]);
    let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::LineSegment { points, stroke }
            if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                && points[0].y == map.top() && points[1].y == map.bottom())));
}

#[test]
fn minimap_drag_while_playing_turns_follow_off_without_seeking_or_changing_repeat() {
    let mut app = arpeggio();
    let (ctx, _) = opened_in(&mut app);
    crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
    crate::play::transport::advance(&mut app);
    app.audition.set_test_seconds(1.0);
    app.playback.repeat = Some((0, 15360));
    app.playback.follow = true;
    let seconds = app.playback.active.as_ref().unwrap().start_seconds;
    let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
    let pos = |fraction| egui::pos2(map.left() + map.width() * fraction, map.center().y);
    super::drag::drag(&mut app, &ctx, 20.0, pos(0.5), pos(0.7));
    assert!(!app.playback.follow);
    let view = &app.piano_roll.viewport;
    assert!((view.scroll.x + view.panel.unwrap().width() / 2.0 - 3168.0 * 0.7).abs() < 0.2);
    assert_eq!(app.playback.repeat, Some((0, 15360)));
    assert_eq!(app.playback.active.as_ref().unwrap().start_seconds, seconds);
}
