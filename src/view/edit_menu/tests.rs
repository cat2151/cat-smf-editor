use crate::view::{
    edit_menu::Command,
    note_edit::{
        tests::{chord_song, editing, enter, render},
        Field,
    },
    tests::app,
};
use eframe::egui;

fn press(app: &mut crate::app::App, ctx: &egui::Context, key: egui::Key) {
    render(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect(),
    );
}

/// The chord song with the cursor on track 1 at `tick`.
fn at(tick: u64) -> (crate::app::App, egui::Context) {
    let mut app = app(chord_song());
    let ctx = egui::Context::default();
    crate::cursor::split::go(&mut app, 1, tick);
    render(&mut app, &ctx, vec![]);
    (app, ctx)
}

#[test]
fn e_m_and_e_d_open_the_cursor_notes_mbt_and_duration() {
    let (mut app, ctx) = at(0);
    press(&mut app, &ctx, egui::Key::E);
    assert!(app.edit_menu.open);
    press(&mut app, &ctx, egui::Key::M);
    assert!(!app.edit_menu.open);
    assert_eq!(editing(&app), Some((Field::Mbt, 0, "1:1:0".into())));
    enter(&mut app, &ctx, "1:2:0");
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[1].events[2].tick,
        480
    );

    let (mut app, ctx) = at(0);
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::D);
    assert_eq!(editing(&app), Some((Field::Duration, 0, "960".into())));
    enter(&mut app, &ctx, "240");
    assert_eq!(
        app.loaded.as_ref().unwrap().note_views[1].durations[0],
        Some(240)
    );
}

#[test]
fn period_repeats_the_last_command_on_the_new_cursor_without_its_number() {
    let (mut app, ctx) = at(1920);
    press(&mut app, &ctx, egui::Key::Period);
    assert_eq!(editing(&app), None);
    app.last_command = Some(Command::Note(Field::Duration));
    press(&mut app, &ctx, egui::Key::Period);
    assert_eq!(editing(&app), Some((Field::Duration, 6, "480".into())));
    enter(&mut app, &ctx, "960");
    crate::cursor::split::go(&mut app, 1, 0);
    press(&mut app, &ctx, egui::Key::Period);
    assert_eq!(editing(&app), Some((Field::Duration, 0, "960".into())));
}

#[test]
fn raw_note_menu_edits_notes_and_off_note_requests_do_not_leak_to_later_rows() {
    let (mut app, ctx) = at(0);
    crate::view::patterns::mode::toggle(&mut app);
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::D);
    assert_eq!(editing(&app), Some((Field::Duration, 0, "960".into())));
    enter(&mut app, &ctx, "240");
    assert_eq!(
        app.loaded.as_ref().unwrap().note_views[1].durations[0],
        Some(240)
    );
    let off = app.loaded.as_ref().unwrap().visible().tracks[1]
        .events
        .iter()
        .position(|e| !cat_smf_editor::score::note_view::note_on(e))
        .unwrap();
    crate::cursor::moves::place(&mut app, off);
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::D);
    assert_eq!(editing(&app), None);
    crate::cursor::moves::place(&mut app, 0);
    render(&mut app, &ctx, vec![]);
    assert_eq!(editing(&app), None);
    crate::view::patterns::mode::toggle(&mut app);
    render(&mut app, &ctx, vec![]);
    assert_eq!(editing(&app), None);
}
