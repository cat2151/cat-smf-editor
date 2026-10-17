use super::tests::fixture;
use crate::play::{
    playback::{self, Mode},
    transport,
};

#[test]
fn whole_track_playback_jumps_forward_and_back_keeping_its_mode() {
    let mut app = fixture();
    playback::start(&mut app, Mode::AllTracks);
    transport::advance(&mut app);
    app.audition.set_test_seconds(1.0);
    for tick in [1920, 0] {
        crate::cursor::split::go(&mut app, 0, tick);
        transport::jump(&mut app, tick);
        assert!(app.playback.active.is_none());
        transport::advance(&mut app);
        let active = app.playback.active.as_ref().unwrap();
        assert_eq!(active.start_seconds, tick as f64 / 960.0);
        assert_eq!(active.end, 4.0 - tick as f64 / 960.0);
        assert_eq!(app.playback.last, Mode::AllTracks);
    }
}

#[test]
fn cursor_playback_restarts_at_the_jump_and_stopped_playback_stays_stopped() {
    let mut app = fixture();
    crate::cursor::split::go(&mut app, 0, 1920);
    transport::jump(&mut app, 1920);
    assert!(app.playback.pending.is_none());
    crate::cursor::split::go(&mut app, 0, 0);
    playback::start(&mut app, Mode::Position);
    transport::advance(&mut app);
    crate::cursor::split::go(&mut app, 0, 1920);
    transport::jump(&mut app, 1920);
    transport::advance(&mut app);
    assert_eq!(app.playback.active.as_ref().unwrap().start_seconds, 2.0);
    assert_eq!(app.playback.last, Mode::Position);
}

#[test]
fn all_tracks_from_measure_starts_every_track_at_the_cursor_measure() {
    let mut music = fixture().loaded.unwrap().music;
    music.tracks.push(music.tracks[0].clone());
    let mut app = crate::view::tests::app(music);
    crate::cursor::split::go(&mut app, 0, 2400);
    playback::start(&mut app, Mode::AllTracksFromMeasure);
    transport::advance(&mut app);
    let active = app.playback.active.as_ref().unwrap();
    assert_eq!(active.start_seconds, 2.0);
    assert_eq!(active.end, 2.0);
    let tracks: Vec<_> = active.sources.iter().map(|(t, _)| *t).collect();
    assert_eq!(tracks, [0, 1]);
}

#[test]
fn repeat_and_clear_keep_remembered_method_and_jump_with_all_tracks() {
    for clear in [false, true] {
        let mut app = fixture();
        crate::cursor::split::go(&mut app, 0, 1920);
        playback::start(&mut app, Mode::Position);
        transport::repeat::start(&mut app, (480, 960));
        assert_eq!(app.playback.last, Mode::Position);
        if clear {
            transport::repeat::clear(&mut app);
        }
        crate::cursor::split::go(&mut app, 0, 2400);
        transport::jump(&mut app, 2400);
        assert_eq!(
            app.playback.pending.as_ref().unwrap().active.mode,
            Mode::AllTracks
        );
        transport::stop(&mut app);
        let ctx = eframe::egui::Context::default();
        super::stop_tests::press(
            &mut app,
            &ctx,
            eframe::egui::Key::P,
            eframe::egui::Modifiers::NONE,
        );
        super::stop_tests::press(
            &mut app,
            &ctx,
            eframe::egui::Key::Space,
            eframe::egui::Modifiers::NONE,
        );
        assert_eq!(app.playback.last, Mode::Position);
        let active = app
            .playback
            .pending
            .as_ref()
            .map(|p| &p.active)
            .or(app.playback.active.as_ref())
            .unwrap();
        assert_eq!(active.start_seconds, 2.5);
    }
}
