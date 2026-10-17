use super::*;
use crate::view::{
    piano_roll::tests::{at, press},
    tests::app,
};
use cat_smf_editor::{
    edit,
    model::{Event, Music, Track},
    score::note_view::note_on,
};

fn song() -> Music {
    let mut music = Music {
        format: 1,
        division: 384,
        tracks: vec![
            Track {
                events: vec![Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x51,
                    data: vec![7, 161, 32],
                }],
            },
            Track { events: vec![] },
        ],
    };
    for i in 0..8 {
        let pitch = 60 + (i % 2) as u8;
        for (tick, status, vel) in [(96 + i * 24, 0x90, 90), (120 + i * 24, 0x80, 0)] {
            music.tracks[1].events.push(Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![pitch, vel],
            });
        }
    }
    trill::initialize(&mut music).unwrap();
    edit::regenerate(&mut music).unwrap();
    music
}

fn position(output: &egui::FullOutput, needle: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text().contains(needle) => Some(
                t.pos + egui::vec2(t.galley.size().x.min(100.0) * 0.5, t.galley.size().y * 0.5),
            ),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing {needle}"))
}

fn click(app: &mut App, ctx: &egui::Context, time: f64, pos: egui::Pos2) {
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    for (dt, pressed) in [(0.01, true), (0.02, false)] {
        at(
            app,
            ctx,
            time + dt,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    at(app, ctx, time + 0.03, vec![]);
}

#[test]
fn first_pitch_selection_updates_repeat_and_saved_metadata_and_cancel_restores() {
    use crate::play::transport;
    let mut app = app(song());
    let original = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    transport::repeat::start(&mut app, (96, 288));
    transport::advance(&mut app);
    let index = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .position(trill::owned)
        .unwrap();
    begin(&mut app, 1, index).unwrap();
    assert!(!app.trill_edit.as_ref().unwrap().upper);
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    for (i, label, pitch) in [(0, "Upper", 61), (1, "Lower", 60), (2, "Upper", 61)] {
        let time = 1.0 + i as f64 * 2.0;
        let output = at(&mut app, &ctx, time, vec![]);
        click(&mut app, &ctx, time + 0.1, position(&output, label));
        assert!(app.trill_edit.is_some());
        assert_eq!(app.playback.repeat, Some((96, 288)));
        let music = &app.loaded.as_ref().unwrap().music;
        let region = trill::read(music).unwrap().remove(0);
        assert_eq!(region.metadata.pattern.as_ref().unwrap().first_note, pitch);
        let ons: Vec<_> = music.tracks[1]
            .events
            .iter()
            .filter(|e| note_on(e))
            .collect();
        assert_eq!(ons[0].data[0], pitch);
        assert_eq!(ons[1].data[0], if pitch == 61 { 60 } else { 61 });
        assert_eq!((ons[0].tick, ons[1].tick), (96, 120));
        let encoded = cat_smf_editor::smf_io::encode(music).unwrap();
        assert_eq!(
            trill::read(&cat_smf_editor::ffi::parse(&encoded).unwrap()).unwrap(),
            [region]
        );
        let end = app.playback.active.as_ref().unwrap().end;
        app.audition.set_test_seconds(end + 0.01);
        transport::advance(&mut app);
        let next = app.playback.pending.as_ref().unwrap();
        assert_eq!(
            next.parts.iter().find(|p| p.0 == 1).unwrap().2.notes[0].pitch,
            pitch
        );
    }
    let output = at(&mut app, &ctx, 8.0, vec![]);
    click(&mut app, &ctx, 8.1, position(&output, "Cancel"));
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
    let index = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .position(trill::owned)
        .unwrap();
    begin(&mut app, 1, index).unwrap();
    assert!(!app.trill_edit.as_ref().unwrap().upper);
}

#[test]
fn notes_and_raw_meta_start_and_end_open_same_editor() {
    for note_mode in [true, false] {
        for label in ["Trill Start:", "Trill End:"] {
            let mut app = app(song());
            app.select_track(1);
            app.note_mode = note_mode;
            crate::play::transport::repeat::start(&mut app, (96, 288));
            let ctx = egui::Context::default();
            at(&mut app, &ctx, 0.0, vec![]);
            let output = at(&mut app, &ctx, 1.0, vec![]);
            let pos = position(&output, label);
            click(&mut app, &ctx, 2.0, pos);
            let editing = app.trill_edit.as_ref().expect("meta click opens editor");
            assert_eq!(app.playback.repeat, Some((96, 288)));
            assert!(
                app.playback.active.is_some(),
                "opening a meta must not stop repeat"
            );
            assert_eq!((editing.track, editing.id), (1, 1));
            assert_eq!(editing.first, editing.interval);
            let output = at(&mut app, &ctx, 3.0, vec![]);
            position(&output, "First note");
            position(&output, "Interval");
            let bytes =
                cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
            app.trill_edit.as_mut().unwrap().first = 0.2;
            let cancel = position(&output, "Cancel");
            click(&mut app, &ctx, 4.0, cancel);
            assert!(app.trill_edit.is_none());
            assert_eq!(
                cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
                bytes
            );
        }
    }
}

#[test]
fn apply_saves_independent_first_duration_and_reflows_and_errors_keep_editor_open() {
    let mut app = app(song());
    let index = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .position(trill::owned)
        .unwrap();
    begin(&mut app, 1, index).unwrap();
    app.trill_edit.as_mut().unwrap().first = 0.0625;
    app.trill_edit.as_mut().unwrap().interval = 0.03125;
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    let output = at(&mut app, &ctx, 1.0, vec![]);
    click(&mut app, &ctx, 2.0, position(&output, "Apply"));
    assert!(app.trill_edit.is_none());
    let music = &app.loaded.as_ref().unwrap().music;
    let ons: Vec<_> = music.tracks[1]
        .events
        .iter()
        .filter(|e| note_on(e))
        .map(|e| e.tick)
        .collect();
    assert_eq!(&ons[..3], &[96, 144, 168]);
    let r = trill::read(music).unwrap().remove(0);
    assert_eq!(r.metadata.first_note_seconds, Some(0.0625));
    assert_eq!(r.metadata.interval_seconds, 0.03125);
    let data = cat_smf_editor::smf_io::encode(music).unwrap();
    assert_eq!(
        trill::read(&cat_smf_editor::ffi::parse(&data).unwrap()).unwrap(),
        [r]
    );
    let index = music.tracks[1]
        .events
        .iter()
        .position(trill::owned)
        .unwrap();
    begin(&mut app, 1, index).unwrap();
    app.trill_edit.as_mut().unwrap().first = -1.0;
    let output = at(&mut app, &ctx, 3.0, vec![]);
    click(&mut app, &ctx, 4.0, position(&output, "Apply"));
    assert!(app.trill_edit.as_ref().unwrap().error.is_some());
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        data
    );
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.trill_edit.is_none());
}

#[test]
fn repeat_keeps_playing_while_editor_is_open_and_apply_updates_its_current_pass() {
    use crate::play::transport;
    let mut app = app(song());
    transport::repeat::start(&mut app, (96, 288));
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.03125);
    let index = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .position(trill::owned)
        .unwrap();
    begin(&mut app, 1, index).unwrap();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    assert!(app.playback.active.is_some());
    assert_eq!(app.playback.repeat, Some((96, 288)));
    app.trill_edit.as_mut().unwrap().first = 0.0625;
    let output = at(&mut app, &ctx, 1.0, vec![]);
    click(&mut app, &ctx, 2.0, position(&output, "Apply"));
    assert_eq!(app.playback.repeat, Some((96, 288)));
    let active = app.playback.active.as_ref().unwrap();
    assert!((active.start_seconds - 0.15625).abs() < 1e-10);
    assert_eq!(
        active.sources.iter().find(|s| s.0 == 1).unwrap().1.notes[0].pitch,
        60
    );
    let end = active.end;
    app.audition.set_test_seconds(end + 0.01);
    transport::advance(&mut app);
    assert!((app.playback.pending.as_ref().unwrap().active.start_seconds - 0.125).abs() < 1e-10);
}

#[test]
fn dragging_first_note_updates_running_repeat_before_release_or_apply_and_cancel_restores_it() {
    use crate::play::transport;
    let mut app = app(song());
    let original = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    transport::repeat::start(&mut app, (96, 288));
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.03125);
    let index = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .position(trill::owned)
        .unwrap();
    begin(&mut app, 1, index).unwrap();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    let output = at(&mut app, &ctx, 1.0, vec![]);
    let from = position(&output, "0.03125");
    at(&mut app, &ctx, 2.0, vec![egui::Event::PointerMoved(from)]);
    at(
        &mut app,
        &ctx,
        2.01,
        vec![egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for (i, dx) in [20.0, 40.0, 60.0].into_iter().enumerate() {
        let generation = app.generation;
        at(
            &mut app,
            &ctx,
            2.02 + i as f64 * 0.01,
            vec![egui::Event::PointerMoved(from + egui::vec2(dx, 0.0))],
        );
        assert!(
            app.generation > generation,
            "held mouse movement must apply the timing"
        );
        assert!(
            app.trill_edit.is_some(),
            "live updates must keep the editor and drag alive"
        );
        assert_eq!(app.playback.repeat, Some((96, 288)));
        let first = trill::read(&app.loaded.as_ref().unwrap().music).unwrap()[0]
            .metadata
            .first_note_seconds
            .unwrap();
        assert!(first > 0.03125);
        let pending = app
            .playback
            .pending
            .as_ref()
            .expect("new audio must be scheduled while mouse is held");
        assert_eq!(
            pending.parts.iter().find(|p| p.0 == 1).unwrap().2.notes[0].pitch,
            60
        );
    }
    let to = from + egui::vec2(60.0, 0.0);
    at(
        &mut app,
        &ctx,
        2.1,
        vec![egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    // Finish the current pass and inspect the freshly generated next loop.
    let end = app.playback.active.as_ref().unwrap().end;
    app.audition.set_test_seconds(end + 0.01);
    transport::advance(&mut app);
    let first = trill::read(&app.loaded.as_ref().unwrap().music).unwrap()[0]
        .metadata
        .first_note_seconds
        .unwrap();
    let next = app.playback.pending.as_ref().unwrap();
    let sequence = &next.parts.iter().find(|p| p.0 == 1).unwrap().2;
    assert!((sequence.notes[0].duration - first).abs() < 0.0014);
    assert!(sequence.notes.len() < 8);
    let output = at(&mut app, &ctx, 3.0, vec![]);
    click(&mut app, &ctx, 4.0, position(&output, "Cancel"));
    assert!(app.trill_edit.is_none());
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
    assert_eq!(app.playback.repeat, Some((96, 288)));
    assert!(app.playback.active.is_some() || app.playback.pending.is_some());
}
