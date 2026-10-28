use super::*;
use crate::play::{
    audio_sequence::{Note, Sequence},
    playback::Mode,
    transport::Active,
};
use cat_smf_editor::{
    model::{Music, Track},
    score::audition_timing::TempoMap,
    voicevox::Voice,
};
fn fixture() -> (Music, Active) {
    let mut music = Music {
        format: 2,
        division: 480,
        tracks: vec![Track { events: vec![] }; 3],
    };
    Voice::write(&mut music.tracks[0], Some(&Voice::default())).unwrap();
    let other = Voice {
        speaker_uuid: "other".into(),
        name: "他キャラ".into(),
        ..Voice::default()
    };
    Voice::write(&mut music.tracks[1], Some(&other)).unwrap();
    let seq = |start, duration| {
        Sequence::new(
            vec![Note {
                start,
                duration,
                pitch: 60,
                velocity: 100,
            }],
            4.0,
        )
    };
    let active = Active {
        automation: Default::default(),
        mode: Mode::AllTracks,
        clock_track: 0,
        tempos: (0..3).map(|t| TempoMap::new(&music, t)).collect(),
        start_seconds: 0.0,
        end: 4.0,
        sources: vec![(0, seq(0.5, 1.0)), (1, seq(0.0, 4.0)), (2, seq(0.0, 4.0))],
    };
    (music, active)
}
#[test]
fn singing_uses_only_played_zundamon_intervals_and_ignores_style() {
    let (mut music, mut active) = fixture();
    assert!(!state_at(&active, &music, 0.49).singing);
    assert!(state_at(&active, &music, 0.5).singing);
    assert!(!state_at(&active, &music, 1.5).singing);
    let v = Voice {
        style_id: 999,
        ..Voice::default()
    };
    Voice::write(&mut music.tracks[0], Some(&v)).unwrap();
    assert!(state_at(&active, &music, 0.75).singing);
    active.sources.retain(|s| s.0 != 0);
    assert!(!state_at(&active, &music, 0.75).singing);
}
#[test]
fn overlapping_tracks_form_union_and_seek_loop_rebase_immediately() {
    let (mut music, mut active) = fixture();
    Voice::write(&mut music.tracks[2], Some(&Voice::default())).unwrap();
    active.sources[2].1.notes[0].start = 1.0;
    active.sources[2].1.notes[0].duration = 1.0;
    for t in [0.5, 1.0, 1.49, 1.5, 1.99] {
        assert!(state_at(&active, &music, t).singing);
    }
    assert!(!state_at(&active, &music, 2.0).singing);
    assert_eq!(state_at(&active, &music, 1.0).beat, Some(2.0));
    active.start_seconds = 4.0;
    assert_eq!(state_at(&active, &music, 0.25).beat, Some(8.5));
    active.start_seconds = 0.0;
    assert_eq!(state_at(&active, &music, 0.25).beat, Some(0.5));
}
#[test]
fn clock_track_not_view_track_and_smpte_has_no_quarter_beat() {
    let (mut music, mut active) = fixture();
    music.tracks[1].events.push(cat_smf_editor::model::Event {
        tick: 480,
        delta: 480,
        status: 0xff,
        meta_type: 0x51,
        data: vec![0x0f, 0x42, 0x40],
    });
    active.tempos = (0..3).map(|t| TempoMap::new(&music, t)).collect();
    assert_eq!(state_at(&active, &music, 1.0).beat, Some(2.0));
    active.clock_track = 1;
    let s = state_at(&active, &music, 1.0);
    assert_eq!(s.beat, Some(1.5));
    assert_eq!(s.beats_per_second, 1.0);
    music.division = 0xe728;
    assert_eq!(state_at(&active, &music, 1.0).beat, None);
}
#[test]
fn pending_and_audio_before_first_callback_do_not_play() {
    let (music, active) = fixture();
    let mut app = crate::view::tests::app(music);
    assert!(!snapshot(&app).playing);
    app.playback.pending = Some(super::super::transport::Prepared {
        parts: vec![],
        active,
        origin: None,
    });
    assert!(!snapshot(&app).playing);
    app.playback.active = app.playback.pending.take().map(|p| p.active);
    assert!(!snapshot(&app).playing);
}
#[test]
fn unsent_states_coalesce_without_waiting_for_network() {
    let (wake, _rx) = mpsc::sync_channel(1);
    let worker = Worker {
        latest: Arc::new(Mutex::new(None)),
        wake: Some(wake),
        thread: None,
    };
    let mut request = stopped();
    request.playing = true;
    for _ in 0..100 {
        worker.publish(request.clone());
    }
    worker.publish(stopped());
    assert!(!worker.latest.lock().unwrap().as_ref().unwrap().playing);
}

#[test]
fn app_exit_sends_stop_and_waits_for_worker_cleanup() {
    let (music, _) = fixture();
    let mut app = crate::view::tests::app(music);
    let (sent, received) = mpsc::channel();
    app.mascot.worker = Some(Worker::spawn(move |request| {
        sent.send(request.playing).unwrap();
    }));
    eframe::App::on_exit(&mut app, None);
    assert!(app.mascot.worker.is_none());
    assert!(!received.try_recv().unwrap());
    assert!(matches!(
        received.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
}

#[test]
fn settings_key_toggles_and_closes_without_leaking_to_main() {
    use eframe::egui::{self, Key, Modifiers};
    let (music, _) = fixture();
    let mut app = crate::view::tests::app(music);
    let ctx = egui::Context::default();
    fn press(app: &mut App, ctx: &egui::Context, key: Key, modifiers: Modifiers) {
        let _ = ctx.run(
            egui::RawInput {
                events: [true, false]
                    .into_iter()
                    .map(|pressed| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers,
                    })
                    .collect(),
                ..Default::default()
            },
            |ctx| crate::view::commands::route(app, ctx),
        );
    }
    press(&mut app, &ctx, Key::Comma, Modifiers::CTRL);
    assert!(app.comma_menu.open);
    press(&mut app, &ctx, Key::B, Modifiers::NONE);
    assert!(app.mascot.enabled);
    press(&mut app, &ctx, Key::B, Modifiers::NONE);
    assert!(!app.mascot.enabled);
    press(&mut app, &ctx, Key::Escape, Modifiers::NONE);
    assert!(!app.comma_menu.open);
}

#[test]
fn unavailable_server_does_not_block_publish_and_only_latest_stop_follows() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let (arrived, rx) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut first = true;
    let worker = Worker::spawn(move |r| {
        if first {
            first = false;
            arrived.send(true).unwrap();
            blocked.recv().unwrap();
        }
        let failed = mascot_render_client::sync_playback_at(address, &r).is_err();
        let _ = arrived.send(failed && !r.playing);
    });
    let mut playing = stopped();
    playing.playing = true;
    worker.publish(playing.clone());
    assert!(rx.recv_timeout(Duration::from_secs(2)).unwrap());
    for _ in 0..100 {
        worker.publish(playing.clone());
    }
    worker.publish(stopped());
    release.send(()).unwrap();
    assert!(!rx.recv_timeout(Duration::from_secs(3)).unwrap());
    assert!(rx.recv_timeout(Duration::from_secs(3)).unwrap());
    drop(worker);
}
