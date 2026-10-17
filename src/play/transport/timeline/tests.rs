use super::*;
use crate::{
    play::{playback, transport},
    view::tests::app,
};
use cat_smf_editor::model::{Event, Music, Track};

fn fixture() -> App {
    let mut events = Vec::new();
    for i in 0..8 {
        for (tick, status, velocity) in [(i * 480, 0x90, 90), ((i + 1) * 480, 0x80, 0)] {
            events.push(Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![60 + (i % 2) as u8, velocity],
            });
        }
    }
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    })
}

#[test]
fn stopped_click_inherits_ending_family_without_changing_last_menu_mode() {
    for (mode, end) in [
        (Mode::BeatOnly, 960),
        (Mode::Measure, 1920),
        (Mode::Position, 3840),
        (Mode::Beat, 3840),
    ] {
        let mut app = fixture();
        app.playback.last = mode;
        crate::view::beat_audition::start(&mut app, 0, 650);
        let prepared = app.playback.pending.as_ref().unwrap();
        assert_eq!(prepared.active.start_seconds, 0.5);
        assert!((prepared.active.end - (end - 480) as f64 / 960.0).abs() < 1e-9);
        assert_eq!(app.playback.last, mode);
    }
}

#[test]
fn playing_click_seeks_exact_tick_and_keeps_repeat_until_clicked_outside() {
    let mut app = fixture();
    transport::repeat::start(&mut app, (480, 1920));
    crate::view::beat_audition::start(&mut app, 0, 750);
    assert_eq!(app.playback.repeat, Some((480, 1920)));
    assert!(
        (app.playback.pending.as_ref().unwrap().active.start_seconds - 750.0 / 960.0).abs() < 1e-10
    );
    crate::view::beat_audition::start(&mut app, 0, 2400);
    assert!(app.playback.repeat.is_none());
    assert!((app.playback.pending.as_ref().unwrap().active.start_seconds - 2.5).abs() < 1e-10);
}

#[test]
fn music_edit_refreshes_running_notes_at_current_tick_and_preserves_repeat() {
    let mut app = fixture();
    transport::repeat::start(&mut app, (480, 1920));
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.25);
    let before = transport::tick(&app).unwrap();
    app.edit_music(|music| {
        for e in &mut music.tracks[0].events {
            if e.status == 0x90 {
                e.data[1] = 55;
            }
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(app.playback.repeat, Some((480, 1920)));
    let p = app.playback.pending.as_ref().unwrap();
    assert!((p.active.start_seconds - before as f64 / 960.0).abs() < 1e-9);
    assert!(p.parts[0].2.notes.iter().all(|n| n.velocity == 55));
    transport::advance(&mut app);
    let end = app.playback.active.as_ref().unwrap().end;
    app.audition.set_test_seconds(end + 0.01);
    transport::advance(&mut app);
    assert_eq!(
        app.playback.pending.as_ref().unwrap().active.start_seconds,
        0.5
    );
    transport::stop(&mut app);
    app.edit_music(|_| Ok(())).unwrap();
    assert!(!running(&app));
    playback::start(&mut app, Mode::Measure);
    let before = app.playback.pending.as_ref().unwrap().active.start_seconds;
    app.edit_music(|_| Ok(())).unwrap();
    assert_eq!(
        app.playback.pending.as_ref().unwrap().active.start_seconds,
        before
    );
}

#[test]
fn editing_a_single_note_playback_does_not_add_other_chord_notes() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().music.tracks[0].events.extend([
        Event {
            tick: 0,
            delta: 0,
            status: 0x90,
            meta_type: 0,
            data: vec![64, 70],
        },
        Event {
            tick: 480,
            delta: 0,
            status: 0x80,
            meta_type: 0,
            data: vec![64, 0],
        },
    ]);
    app.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .sort_by_key(|e| e.tick);
    app.loaded.as_mut().unwrap().refresh();
    app.selected_event = Some(0);
    playback::start(&mut app, Mode::Event);
    app.edit_music(|_| Ok(())).unwrap();
    let notes = &app.playback.pending.as_ref().unwrap().parts[0].2.notes;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].pitch, 60);
}
