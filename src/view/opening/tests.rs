use super::*;
use cat_smf_editor::{model::Music, smf_io};
use egui::{Key, Modifiers};

fn frame(app: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    ctx.run(Default::default(), |ctx| crate::view::show(app, ctx))
}

fn text(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

pub(crate) fn finish(app: &mut App, ctx: &egui::Context) {
    for _ in 0..100 {
        if !pending(app) {
            return;
        }
        frame(app, ctx);
    }
    panic!("song opening did not complete: {:?}", app.load_error);
}

fn music() -> Music {
    use cat_smf_editor::model::{Event, Track};
    Music {
        format: 0,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 3,
                    data: b"Old song".to_vec(),
                },
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    tick: 480,
                    delta: 480,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
                Event {
                    tick: 480,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    }
}

fn press(app: &mut App, ctx: &egui::Context, key: Key, modifiers: Modifiers) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn five_tracks() -> Music {
    let mut music = music();
    music.format = 1;
    music.tracks = (0..5)
        .map(|i| {
            let mut track = music.tracks[0].clone();
            track.events[0].data = format!("Instrument {}", i + 1).into_bytes();
            track
        })
        .collect();
    music
}

#[test]
fn recent_selection_paints_score_and_progress_before_loading_each_of_five_tracks() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("next.mid");
    let music = five_tracks();
    std::fs::write(&path, smf_io::encode(&music).unwrap()).unwrap();
    let mut app = crate::view::tests::app(self::music());
    let old = root.path().join("old.mid");
    std::fs::write(&old, smf_io::encode(&self::music()).unwrap()).unwrap();
    app.loaded = None;
    app.open_mid(old);
    app.recent.history = crate::store::recent::History::open(root.path().join("history.json"));
    app.recent.history.record(&path, &music);
    let old_song = app.song_id;
    let ctx = egui::Context::default();
    press(&mut app, &ctx, Key::P, Modifiers::CTRL);
    // Measure the recent modal, then select entirely by keyboard.
    frame(&mut app, &ctx);
    let selected = press(&mut app, &ctx, Key::Enter, Modifiers::NONE);
    assert!(pending(&app));
    assert_eq!(
        app.song_id, old_song,
        "selection must first present progress"
    );
    let selected_text = text(&selected);
    assert!(selected_text.iter().any(|s| s == "Reading file…"));
    assert!(
        selected_text.iter().any(|s| s == "Tracks"),
        "score must remain painted"
    );
    let mut opened = Vec::new();
    for _ in 0..80 {
        let phase = app.opening.job.as_ref().map(|j| &j.phase);
        if let Some(Phase::Audio(_, index)) = phase
            && *index < 5
        {
            let job = app.opening.job.as_ref().unwrap();
            assert_eq!(job.rows[*index].1, "Loading instrument…");
            assert_eq!(job.current, Some(*index));
            assert!(
                app.opening.painted.is_some(),
                "must paint before opening track"
            );
            assert!(job.rows[..*index].iter().all(|row| row.1 == "Ready"));
            opened.push(*index);
        }
        if !pending(&app) {
            break;
        }
        frame(&mut app, &ctx);
    }
    assert_eq!(opened, vec![0, 1, 2, 3, 4]);
    assert!(!pending(&app));
    assert!(app.load_error.is_none());
    assert!(app.playback.active.is_some());
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 5);
    assert_eq!(
        app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&path).unwrap()
    );
}

#[test]
fn layout_retry_cannot_advance_loading_and_input_cannot_edit_old_song() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("song.mid");
    let music = five_tracks();
    let bytes = smf_io::encode(&music).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let mut app = crate::view::tests::app(music);
    let ctx = egui::Context::default();
    request(&mut app, path.clone(), true);
    let song = app.song_id;
    let _ = ctx.run(Default::default(), |ctx| {
        advance(&mut app, ctx);
        assert!(matches!(
            app.opening.job.as_ref().unwrap().phase,
            Phase::Read
        ));
        show(&mut app, ctx, None);
        advance(&mut app, ctx);
        assert!(matches!(
            app.opening.job.as_ref().unwrap().phase,
            Phase::Read
        ));
    });
    press(&mut app, &ctx, Key::N, Modifiers::CTRL);
    assert_eq!(app.song_id, song);
    assert!(!crate::view::busy::pending(&app));
    assert!(!app.saved.dirty());
    finish(&mut app, &ctx);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn invalid_file_keeps_the_previous_document_and_its_edits() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("invalid.mid");
    std::fs::write(&path, b"not MIDI").unwrap();
    let mut app = crate::view::tests::app(five_tracks());
    app.edit_music(|music| {
        music.tracks[0].events[1].data[0] = 67;
        Ok(())
    })
    .unwrap();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let song = app.song_id;
    let ctx = egui::Context::default();
    request(&mut app, path, true);
    frame(&mut app, &ctx);
    frame(&mut app, &ctx);
    assert!(!pending(&app));
    assert_eq!(app.song_id, song);
    assert_eq!(app.invalid_smf_name.as_deref(), Some("invalid.mid"));
    assert!(app.load_error.is_some());
    assert!(app.playback.pending.is_none());
    assert!(app.saved.dirty());
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn dropped_file_uses_progress_without_starting_playback() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dropped.mid");
    let music = five_tracks();
    std::fs::write(&path, smf_io::encode(&music).unwrap()).unwrap();
    let mut app = crate::view::tests::app(music);
    app.loaded = None;
    let ctx = egui::Context::default();
    request_drop(&mut app, path);
    finish(&mut app, &ctx);
    assert!(app.load_error.is_none());
    assert!(app.playback.pending.is_none());
    assert!(app.playback.active.is_none());
}

#[test]
#[ignore = "requires installed Surge XT and default audio output; prepares five instances, no notes sent"]
fn native_five_track_preparation_advances_between_instances_without_playing() {
    let root = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(root.path()).unwrap();
    let path = root.path().join("native-five.mid");
    let music = five_tracks();
    std::fs::write(&path, smf_io::encode(&music).unwrap()).unwrap();
    let mut app = crate::view::tests::app(music);
    app.audition = crate::play::audition::Audition::default();
    let ctx = egui::Context::default();
    request(&mut app, path, true);
    let mut tracks = Vec::new();
    for _ in 0..100 {
        if let Some(job) = &app.opening.job {
            if matches!(job.phase, Phase::Start) {
                assert_eq!(tracks, vec![0, 1, 2, 3, 4]);
                assert!(job.rows.iter().all(|row| row.1 == "Ready"));
                assert!(app.audition.error().is_none());
                assert!(app.playback.pending.is_none());
                assert!(app.playback.active.is_none());
                app.opening = State::default();
                app.audition.reset();
                return;
            }
            if let Phase::Audio(_, index) = job.phase
                && index < 5
            {
                assert!(app.opening.painted.is_some());
                tracks.push(index);
            }
        }
        frame(&mut app, &ctx);
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
    }
    panic!("native preparation did not finish");
}
