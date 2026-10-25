use super::*;
use crate::view::note_edit::tests::{chord_song, render};
use cat_smf_editor::smf_io;

fn key(key: egui::Key, modifiers: egui::Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn keyboard_rename_unicode_cancel_and_undo() {
    let mut app = crate::view::tests::app(chord_song());
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::ArrowRight, egui::Modifiers::CTRL)],
    );
    assert_eq!(app.track, 1);
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::F2, egui::Modifiers::NONE)],
    );
    assert!(app.track_rename.is_some());
    render(&mut app, &ctx, vec![Event::Text("日本語 Piano".into())]);
    assert_eq!(app.track_rename.as_ref().unwrap().text, "日本語 Piano");
    assert!(!app.playback.open);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.track_rename.is_none());
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[1]
            .name()
            .as_deref(),
        Some("日本語 Piano")
    );
    assert!(app.loaded.as_ref().unwrap().track_titles[1].contains("日本語 Piano"));
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::F2, egui::Modifiers::NONE)],
    );
    render(&mut app, &ctx, vec![Event::Text("discarded".into())]);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(app.track_rename.is_none());
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Z, egui::Modifiers::CTRL)],
    );
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn ime_enter_and_stale_document_cannot_commit() {
    let mut app = crate::view::tests::app(chord_song());
    let ctx = egui::Context::default();
    begin(&mut app, 1);
    render(&mut app, &ctx, vec![]);
    render(
        &mut app,
        &ctx,
        vec![Event::Ime(ImeEvent::Preedit("なまえ".into()))],
    );
    render(
        &mut app,
        &ctx,
        vec![
            Event::Ime(ImeEvent::Commit("名前".into())),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    assert!(app.track_rename.is_some());
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks[1].name(), None);
    app.song_id += 1;
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.track_rename.is_none());
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks[1].name(), None);
}

#[test]
fn tracks_context_menu_renames_the_clicked_track() {
    let mut app = crate::view::tests::app(chord_song());
    let ctx = egui::Context::default();
    let output = render(&mut app, &ctx, vec![]);
    let position = |out: &egui::FullOutput, text: &str| {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing UI text: {text}"))
    };
    let row = position(&output, &app.loaded.as_ref().unwrap().track_titles[1]);
    let click = |app: &mut App, at, button| {
        render(
            app,
            &ctx,
            vec![
                Event::PointerMoved(at),
                Event::PointerButton {
                    pos: at,
                    button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        render(
            app,
            &ctx,
            vec![Event::PointerButton {
                pos: at,
                button,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
    };
    click(&mut app, row, egui::PointerButton::Secondary);
    let output = render(&mut app, &ctx, vec![]);
    let menu = position(&output, "Rename…");
    click(&mut app, menu, egui::PointerButton::Primary);
    assert_eq!(app.track_rename.as_ref().unwrap().track, 1);
    render(&mut app, &ctx, vec![Event::Text("Clicked track".into())]);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[1]
            .name()
            .as_deref(),
        Some("Clicked track")
    );
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks[0].name(), None);
}

#[test]
fn timeline_keeps_its_keys_out_of_the_name_field() {
    let mut app = crate::view::tests::app(chord_song());
    app.select_track(1);
    crate::view::piano_roll::open(&mut app);
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::F2, egui::Modifiers::NONE)],
    );
    render(
        &mut app,
        &ctx,
        vec![
            key(egui::Key::P, egui::Modifiers::NONE),
            key(egui::Key::R, egui::Modifiers::NONE),
            key(egui::Key::I, egui::Modifiers::NONE),
            key(egui::Key::J, egui::Modifiers::NONE),
            key(egui::Key::Space, egui::Modifiers::NONE),
            Event::Text("p r ijkl space".into()),
        ],
    );
    assert_eq!(app.track_rename.as_ref().unwrap().text, "p r ijkl space");
    assert!(app.piano_roll.open);
    assert!(!app.playback.open && !app.mml_input.open);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(app.track_rename.is_none());
    assert!(app.piano_roll.open);
}

#[test]
fn track_list_renames_the_current_track_and_closes() {
    let mut app = crate::view::tests::app(chord_song());
    app.select_track(1);
    app.track_view.go_open = true;
    app.track_view.track_list = true;
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::F2, egui::Modifiers::NONE)],
    );
    assert_eq!(app.track_rename.as_ref().unwrap().track, 1);
    assert!(!app.track_view.go_open);
    render(&mut app, &ctx, vec![Event::Text("Bass".into())]);
    render(
        &mut app,
        &ctx,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[1]
            .name()
            .as_deref(),
        Some("Bass")
    );
}
