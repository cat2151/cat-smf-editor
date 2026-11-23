use super::*;
use crate::view::palette::tests as palette;
use cat_smf_editor::{
    edit::tracks,
    score::{note_view::note_on, pattern::Catalog, scene::arrangement},
    smf_io,
};
use egui::{Event, Modifiers};

fn key(key: Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn has_label(output: &egui::FullOutput, label: &str) -> bool {
    labels(output, label).next().is_some()
}

fn labels<'a>(
    output: &'a egui::FullOutput,
    label: &'a str,
) -> impl Iterator<Item = &'a egui::epaint::TextShape> {
    output
        .shapes
        .iter()
        .filter_map(move |shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => Some(text),
            _ => None,
        })
}

#[test]
fn palette_to_timeout_saves_edits_and_requires_a_fresh_enter_after_the_lock() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = crate::view::tests::app(tracks::new_song());
    let kick = 24;
    let _kit = crate::player::default_kit::fixture::provide(&mut app.state_cache, kick);
    app.loaded = None;
    app.saved.directory = Some(directory.path().to_owned());
    let ctx = egui::Context::default();
    palette::open(&mut app, &ctx);
    let id = "file.start-song-challenge";
    assert_eq!(palette::name_of(&app, id), "start 15-Minute Song Challenge");
    palette::choose(&mut app, &ctx, id);
    palette::press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let State::Duration(input) = &app.song_challenge else {
        panic!("duration dialog did not open")
    };
    assert_eq!(input.minutes, "15");
    let path = app.loaded.as_ref().unwrap().original.clone();
    assert_eq!(
        path.parent(),
        Some(directory.path().join("song-challenges").as_path())
    );
    let starter = std::fs::read(&path).unwrap();
    let music = &app.loaded.as_ref().unwrap().music;
    assert_eq!(starter, smf_io::encode(music).unwrap());
    assert!(crate::view::patterns::mode::workspace_open(&app));
    let catalog = Catalog::read(music).unwrap();
    assert_eq!(catalog.sources.len(), 4);
    assert_eq!(
        music.tracks[1..]
            .iter()
            .map(|t| t.name().unwrap())
            .collect::<Vec<_>>(),
        ["Drum kit", "Bass", "Chord", "Lead"]
    );
    assert_eq!(catalog.patterns.len(), 1);
    let (&id, pattern) = catalog.patterns.iter().next().unwrap();
    assert_eq!(pattern.track, 1);
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    assert_eq!(patch.kit.as_ref().unwrap().notes[0].note, kick);
    assert_eq!(patch.label(), "default-kit.sfz");
    let notes: Vec<_> = music
        .tracks
        .iter()
        .flat_map(|track| &track.events)
        .filter(|e| note_on(e))
        .map(|e| (e.tick, e.data[0]))
        .collect();
    assert_eq!(notes, [(0, kick)]);
    assert_eq!(
        arrangement::occurrences(music, None).unwrap(),
        (vec![(id, 0)], 1920)
    );

    assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("song-challenge-minutes"))));
    frame(&mut app, &ctx, vec![Event::Text("0".into())]);
    assert!(matches!(&app.song_challenge, State::Duration(input) if input.minutes == "0"));
    palette::press(&mut app, &ctx, Key::Enter);
    let State::Duration(input) = &app.song_challenge else {
        panic!("zero must be rejected")
    };
    assert_eq!(input.minutes, "0");
    assert!(input.invalid);
    frame(
        &mut app,
        &ctx,
        vec![key(Key::Backspace, true), key(Key::Backspace, false)],
    );
    frame(&mut app, &ctx, vec![Event::Text("1".into())]);
    palette::press(&mut app, &ctx, Key::Enter);
    let State::Running { start, limit, .. } = &mut app.song_challenge else {
        panic!("timer did not start")
    };
    assert_eq!(*limit, Duration::from_secs(60));
    assert_eq!(app.playback.repeat, Some((0, 1920)));
    assert_eq!(
        app.playback.repeat_mode,
        Some(crate::play::playback::Mode::Arrangement(None))
    );
    *start = Instant::now() - Duration::from_millis(1500);
    let output = frame(&mut app, &ctx, vec![]);
    let outlined: Vec<_> = labels(&output, "あと0分59秒").collect();
    assert_eq!(outlined.len(), 9);
    let (white, outline) = outlined.split_last().unwrap();
    assert_eq!(white.fallback_color, Color32::WHITE);
    assert!(outline
        .iter()
        .all(|text| text.override_text_color == Some(Color32::BLACK)));
    let bottom_right = white.pos + white.galley.size();
    assert!((bottom_right - egui::pos2(1184.0, 784.0)).length() <= 1.0);
    app.edit_tracks(|music| tracks::rename(&mut music.tracks[2], "Challenge edit"))
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), starter);

    let State::Running { start, .. } = &mut app.song_challenge else {
        unreachable!()
    };
    *start -= Duration::from_secs(60);
    frame(&mut app, &ctx, vec![key(Key::Enter, true)]);
    let output = frame(&mut app, &ctx, vec![]);
    assert!(has_label(&output, "Time's Up!"));
    assert!(!has_label(&output, "OK"));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
    );
    let State::Finished { shown, error } = &mut app.song_challenge else {
        panic!("timeout dialog did not open")
    };
    assert!(error.is_none());
    *shown = Some(Instant::now() - Duration::from_secs(1));

    let output = frame(&mut app, &ctx, vec![key(Key::Enter, true)]);
    assert!(has_label(&output, "OK"));
    assert!(
        app.song_challenge.modal_open(),
        "held Enter must not close the dialog"
    );
    palette::press(&mut app, &ctx, Key::Escape);
    assert!(app.song_challenge.modal_open());
    frame(&mut app, &ctx, vec![key(Key::Enter, false)]);
    palette::press(&mut app, &ctx, Key::Enter);
    assert!(matches!(app.song_challenge, State::Off));
}

#[test]
fn clicking_ok_dismisses_the_centered_timeout_modal() {
    let mut app = crate::view::tests::app(tracks::new_song());
    app.song_challenge = State::Finished {
        shown: Some(Instant::now() - Duration::from_secs(1)),
        error: None,
    };
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let title = labels(&output, "Time's Up!").next().unwrap();
    let center = title.pos + title.galley.rect.center().to_vec2();
    assert!((center.x - 600.0).abs() < 2.0, "{center:?}");
    assert!((center.y - 400.0).abs() < 40.0, "{center:?}");
    let button = labels(&output, "OK").next().unwrap();
    let pos = button.pos + button.galley.rect.center().to_vec2();
    let click = |pressed| Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![Event::PointerMoved(pos), click(true)]);
    frame(&mut app, &ctx, vec![click(false)]);
    assert!(matches!(app.song_challenge, State::Off));
}

#[test]
fn duration_requires_a_positive_integer_with_representable_seconds() {
    for invalid in ["", "0", "-1", "1.5", "1e2", "18446744073709551615"] {
        assert_eq!(duration(invalid), None, "{invalid}");
    }
    assert_eq!(duration("15"), Some(Duration::from_secs(900)));
}
