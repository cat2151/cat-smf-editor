//! Playback following shares the main view's toggle and respects timeline zoom.
use super::{music, opened_in};
use crate::{
    play::{playback, transport},
    view::{
        piano_roll::tests::{at, press},
        tests::app,
    },
};
use eframe::egui::{self, epaint::Shape};

fn marker(output: &egui::FullOutput) -> (f32, f32) {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke }
                if stroke.color == super::super::repeat::PLAYHEAD
                    && stroke.width == 2.0
                    && (points[1].y - points[0].y).abs() > 56.0 =>
            {
                Some((points[0].x, s.clip_rect.center().x))
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn f_centers_playback_continuously_at_each_zoom_and_off_keeps_scroll() {
    for zoom in [1.0, 2.0] {
        let mut song = music();
        for track in &mut song.tracks {
            track.events.last_mut().unwrap().tick = 192_000;
            cat_smf_editor::edit::refresh_deltas(track).unwrap();
        }
        let mut app = app(song);
        app.playback.follow = false;
        let (ctx, _) = opened_in(&mut app, 0);
        if zoom > 1.0 {
            app.tempo_curve.viewport.zoom_edge(
                0,
                true,
                super::super::Axis {
                    low: 120.0,
                    high: 150.0,
                },
                0.0,
            );
        }
        at(&mut app, &ctx, 11.0, vec![]);
        playback::start(&mut app, playback::Mode::AllTracks);
        transport::advance(&mut app);
        app.audition.set_test_seconds(30.0);
        at(&mut app, &ctx, 12.0, vec![]);
        let before = app.tempo_curve.viewport.scroll;
        let output = press(&mut app, &ctx, egui::Key::F);
        assert!(app.playback.follow);
        assert!(app.tempo_curve.viewport.scroll > before + 1000.0);
        let (x, center) = marker(&output);
        assert!((x - center).abs() < 2.0, "{x} vs {center}");
        // Follow both advancing playback and a repeat jumping backwards.
        for seconds in [30.1, 10.0] {
            app.audition.set_test_seconds(seconds);
            let output = at(&mut app, &ctx, 13.0, vec![]);
            let (x, center) = marker(&output);
            assert!((x - center).abs() < 2.0, "{x} vs {center}");
        }
        press(&mut app, &ctx, egui::Key::F);
        assert!(!app.playback.follow);
        let stopped = app.tempo_curve.viewport.scroll;
        app.audition.set_test_seconds(40.0);
        at(&mut app, &ctx, 14.0, vec![]);
        assert_eq!(app.tempo_curve.viewport.scroll, stopped);
        assert_eq!(app.audition.seconds(), Some(40.0));
        assert!(app.tempo_curve.open);
    }
}
