use super::*;
use cat_smf_editor::edit::tracks::new_song;
fn frame(app: &mut App, ctx: &egui::Context, key: egui::Key) {
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        );
    }
}
#[test]
fn keyboard_range_reverses_excluding_anchor_and_menu_owns_keys() {
    for notes in [true, false] {
        let mut app = crate::view::tests::app(new_song());
        app.note_mode = notes;
        crate::cursor::moves::place(&mut app, 1);
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, egui::Key::S);
        assert_eq!(range(&app), Some(1..2));
        frame(&mut app, &ctx, egui::Key::J);
        assert_eq!(range(&app), Some(1..3));
        frame(&mut app, &ctx, egui::Key::Home);
        assert_eq!(range(&app), Some(0..1));
        frame(&mut app, &ctx, egui::Key::S);
        assert!(app.event_selection.menu);
        frame(&mut app, &ctx, egui::Key::J);
        assert_eq!(app.cursor_row, Some(0));
        frame(&mut app, &ctx, egui::Key::Escape);
        assert!(!app.event_selection.menu);
        frame(&mut app, &ctx, egui::Key::Escape);
        assert!(range(&app).is_none());
    }
}
#[test]
fn whole_track_options_delete_track_and_undo_restore_sound_and_count() {
    let mut music = new_song();
    music.tracks.push(music.tracks[0].clone());
    let mut app = crate::view::tests::app(music);
    app.note_mode = false;
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, egui::Key::Home);
    frame(&mut app, &ctx, egui::Key::S);
    assert!(!whole(&app));
    frame(&mut app, &ctx, egui::Key::End);
    assert!(whole(&app));
    frame(&mut app, &ctx, egui::Key::S);
    frame(&mut app, &ctx, egui::Key::T);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 1);
    assert!(!app.layer_editor.open);
    app.undo_music().unwrap();
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 2);
    assert_eq!(app.loaded.as_ref().unwrap().patches.len(), 2);
}
#[test]
fn selected_note_delete_is_one_undo_step_and_repeat_uses_note_duration() {
    let mut app = crate::view::tests::app(new_song());
    let row = app.loaded.as_ref().unwrap().display_rows[0]
        .event_row(2)
        .unwrap();
    crate::cursor::moves::place(&mut app, row);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, egui::Key::S);
    assert_eq!(actions::ticks(&app), Some((0, 480)));
    frame(&mut app, &ctx, egui::Key::S);
    frame(&mut app, &ctx, egui::Key::D);
    assert!(!app.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .any(cat_smf_editor::score::note_view::note_on));
    app.undo_music().unwrap();
    assert!(app.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .any(cat_smf_editor::score::note_view::note_on));
    assert!(!app.undo.available());
}

#[test]
fn menu_repeat_and_initialization_work_from_keyboard() {
    let mut music = new_song();
    music.tracks.push(music.tracks[0].clone());
    let mut app = crate::view::tests::app(music);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, egui::Key::Home);
    frame(&mut app, &ctx, egui::Key::S);
    frame(&mut app, &ctx, egui::Key::End);
    frame(&mut app, &ctx, egui::Key::S);
    frame(&mut app, &ctx, egui::Key::A);
    assert_eq!(app.playback.repeat, Some((0, 480)));
    frame(&mut app, &ctx, egui::Key::Home);
    frame(&mut app, &ctx, egui::Key::S);
    frame(&mut app, &ctx, egui::Key::End);
    frame(&mut app, &ctx, egui::Key::S);
    frame(&mut app, &ctx, egui::Key::I);
    assert!(!app.mml_input.open);
    assert!(app.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .all(|e| e.tick == 0));
    app.undo_music().unwrap();
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[0]
            .events
            .last()
            .unwrap()
            .tick,
        480
    );
}

#[test]
fn init_keeps_shared_d_minor_and_clears_differing_sound_settings() {
    let mut music = new_song();
    music.tracks.push(music.tracks[0].clone());
    music.tracks.push(music.tracks[0].clone());
    for (i, track) in music.tracks.iter_mut().enumerate().skip(1) {
        track
            .events
            .retain(|e| !(e.status == 0xff && matches!(e.meta_type, 0x51 | 0x58)));
        track.events.insert(
            0,
            cat_smf_editor::model::Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x59,
                data: vec![0xff, 1],
            },
        );
        crate::play::track_volume::write(track, if i == 1 { 0.5 } else { 0.25 });
    }
    crate::play::track_volume::write(&mut music.tracks[0], 0.75);
    let before = cat_smf_editor::smf_io::encode(&music).unwrap();
    let mut app = crate::view::tests::app(music);
    let ctx = egui::Context::default();
    for key in [
        egui::Key::Home,
        egui::Key::S,
        egui::Key::End,
        egui::Key::S,
        egui::Key::I,
    ] {
        frame(&mut app, &ctx, key);
    }
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    let track = &app.loaded.as_ref().unwrap().music.tracks[0];
    assert_eq!(track.events.len(), 2);
    assert_eq!(
        (track.events[0].meta_type, track.events[0].data.as_slice()),
        (0x59, &[0xff, 1][..])
    );
    assert_eq!(crate::play::track_volume::read(track).unwrap(), 1.0);
    app.undo_music().unwrap();
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
