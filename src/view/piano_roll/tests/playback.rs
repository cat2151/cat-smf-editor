use super::navigation::{click, zone};
use super::*;
use crate::play::{playback, transport};

fn marker(output: &egui::FullOutput, panel: egui::Rect) -> f32 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke }
                if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                    && stroke.width == 2.0
                    && points[1].y > panel.top() =>
            {
                Some(points[0].x)
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn follow_centers_playhead_at_each_zoom_and_off_preserves_scroll_through_repeat_jumps() {
    for zoom in [false, true] {
        let mut app = arpeggio();
        app.loaded.as_mut().unwrap().music.tracks[0]
            .events
            .last_mut()
            .unwrap()
            .tick = 192000;
        app.loaded.as_mut().unwrap().refresh();
        app.playback.follow = false;
        let (ctx, _) = opened_in(&mut app);
        if zoom {
            let pos = zone(&app, 0);
            click(&mut app, &ctx, 11.0, pos, false);
        }
        playback::start(&mut app, playback::Mode::AllTracks);
        transport::advance(&mut app);
        app.audition.set_test_seconds(30.0);
        at(&mut app, &ctx, 12.0, vec![]);
        let before = app.piano_roll.viewport.scroll;
        let output = press(&mut app, &ctx, egui::Key::F);
        assert!(app.playback.follow);
        assert!(app.piano_roll.viewport.scroll.x > before.x + 1000.0);
        assert!(
            (marker(&output, app.piano_roll.viewport.panel.unwrap())
                - app.piano_roll.viewport.panel.unwrap().center().x)
                .abs()
                < 1.0
        );
        for seconds in [30.1, 10.0] {
            app.audition.set_test_seconds(seconds);
            let output = at(&mut app, &ctx, 13.0, vec![]);
            assert!(
                (marker(&output, app.piano_roll.viewport.panel.unwrap())
                    - app.piano_roll.viewport.panel.unwrap().center().x)
                    .abs()
                    < 1.0
            );
        }
        press(&mut app, &ctx, egui::Key::F);
        let offset = app.piano_roll.viewport.scroll;
        app.audition.set_test_seconds(40.0);
        at(&mut app, &ctx, 14.0, vec![]);
        assert_eq!(app.piano_roll.viewport.scroll, offset);
    }
}

#[test]
fn playback_menu_space_escape_and_stop_preserve_the_roll_and_last_playback_mode() {
    let mut app = arpeggio();
    let (ctx, _) = opened_in(&mut app);
    press(&mut app, &ctx, egui::Key::P);
    assert!(app.playback.open && app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::A);
    assert!(!app.playback.open && app.piano_roll.open);
    assert_eq!(app.playback.last, playback::Mode::AllTracks);
    assert!(app.playback.pending.is_some() || app.playback.active.is_some());
    press(&mut app, &ctx, egui::Key::P);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.piano_roll.open && !app.playback.open);
    assert!(app.playback.active.is_some());
    press(&mut app, &ctx, egui::Key::Space);
    assert_eq!(app.playback.last, playback::Mode::AllTracks);
    press(&mut app, &ctx, egui::Key::P);
    press(&mut app, &ctx, egui::Key::D);
    assert!(app.piano_roll.open);
    assert!(app.playback.pending.is_none() && app.playback.active.is_none());
}

#[test]
fn help_has_roll_operations_and_closes_without_closing_the_roll_or_starting_playback() {
    let mut app = arpeggio();
    let (ctx, _) = opened_in(&mut app);
    at(&mut app, &ctx, 11.0, vec![egui::Event::Text("?".into())]);
    assert!(app.help_open && app.piano_roll.open);
    at(&mut app, &ctx, 12.0, vec![]);
    assert_eq!(crate::view::help::test_target(&app), "Piano");
    press(&mut app, &ctx, egui::Key::Space);
    assert!(app.playback.active.is_some() || app.playback.pending.is_some());
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.help_open && !app.piano_roll.open);
}

#[test]
fn click_auditions_the_beat_without_editing_and_focuses_the_clicked_split_track() {
    let mut app = super::split::fixture();
    app.playback.last = playback::Mode::BeatOnly;
    let selected = app.track_view.split;
    let (ctx, output) = opened_in(&mut app);
    let note = super::all_notes(&output)[0];
    let before = app.generation;
    let last = app.playback.last;
    click(&mut app, &ctx, 11.0, note.center(), false);
    assert_eq!(app.track, 2);
    assert_eq!(app.track_view.active, 1);
    assert_eq!(app.track_view.split, selected);
    assert_eq!(app.track_view.cursor_tick, Some(0));
    assert_eq!(app.generation, before);
    assert_eq!(app.playback.last, last);
    let active = app.playback.active.as_ref().unwrap();
    assert_eq!(active.sources.len(), 3);
    assert!((active.end - 0.5).abs() < 1e-9);
    assert!(active.sources[1..]
        .iter()
        .all(|(_, source)| !source.notes.is_empty()));
}
