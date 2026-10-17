use crate::view::tests::app;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;
fn fixture() -> crate::app::App {
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 480,
                    delta: 480,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    tick: 48_000,
                    delta: 47_520,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
            ],
        }],
    })
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: egui::Modifiers::NONE,
    }
}
#[test]
fn navigation_counts_empty_rows_pages_clamps_and_does_not_move_with_help() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let press = |app: &mut crate::app::App, k| {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(k)],
                ..Default::default()
            },
            |ctx| crate::cursor::moves::shortcuts(app, ctx),
        );
    };
    press(&mut app, egui::Key::J);
    assert_eq!((app.cursor_row, app.selected_event), (Some(0), None));
    press(&mut app, egui::Key::ArrowDown);
    assert_eq!((app.cursor_row, app.selected_event), (Some(1), Some(0)));
    press(&mut app, egui::Key::PageDown);
    assert_eq!((app.cursor_row, app.selected_event), (Some(11), None));
    press(&mut app, egui::Key::PageUp);
    assert_eq!(app.cursor_row, Some(1));
    press(&mut app, egui::Key::K);
    press(&mut app, egui::Key::ArrowUp);
    assert_eq!(app.cursor_row, Some(0));
    let last = app.loaded.as_ref().unwrap().display_rows[0].len() - 1;
    press(&mut app, egui::Key::End);
    assert_eq!(app.cursor_row, Some(last));
    press(&mut app, egui::Key::Home);
    assert_eq!(app.cursor_row, Some(0));
    app.select_track(0);
    press(&mut app, egui::Key::End);
    assert_eq!(app.cursor_row, Some(last));
    press(&mut app, egui::Key::Home);
    app.help_open = true;
    press(&mut app, egui::Key::PageDown);
    assert_eq!(app.cursor_row, Some(0));
    app.select_track(0);
    assert_eq!(app.cursor_row, None);
}
#[test]
fn virtual_scroll_reveals_cursor_after_page_moves_and_keeps_it_visible() {
    let mut app = fixture();
    // Empty measures take one row each, so stretch the rest past eight pages.
    app.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .last_mut()
        .unwrap()
        .tick = 192_000;
    app.loaded.as_mut().unwrap().refresh();
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    render(&mut app, vec![]);
    crate::cursor::moves::select(&mut app, 0);
    for _ in 0..8 {
        render(&mut app, vec![key(egui::Key::PageDown)]);
    }
    assert_eq!(app.cursor_row, Some(80));
    let output = render(&mut app, vec![]);
    assert!(
        output.shapes.iter().any(
            |s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text().trim() == "78")
        ),
        "destination was not rendered"
    );
    for _ in 0..8 {
        render(&mut app, vec![key(egui::Key::PageUp)]);
    }
    let output = render(&mut app, vec![]);
    assert!(output.shapes.iter().any(
        |s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text().trim() == "1:1")
    ));
}

#[test]
fn clicking_a_note_then_pressing_j_moves_from_the_clicked_row() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    render(&mut app, vec![]);
    let output = render(&mut app, vec![]);
    let position = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == "C4" => {
                Some(t.pos + egui::vec2(3.0, 5.0))
            }
            _ => None,
        })
        .unwrap();
    render(
        &mut app,
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    render(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!((app.cursor_row, app.selected_event), (Some(1), Some(0)));
    render(&mut app, vec![key(egui::Key::J)]);
    assert_eq!((app.cursor_row, app.selected_event), (Some(2), None));
}

fn press(app: &mut crate::app::App, ctx: &egui::Context, keys: &[egui::Key]) {
    let _ = ctx.run(
        egui::RawInput {
            // Released in the same frame, so pressing the key again is not a repeat.
            events: [true, false]
                .into_iter()
                .flat_map(|pressed| {
                    keys.iter().map(move |&key| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                })
                .collect(),
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
}

/// Track 0: chord C4+E4 at tick 0, G4 in measure 2, C5 in measure 3; track 1: A3 in measure 2.
fn two_measures() -> crate::app::App {
    let note = |tick, status, pitch, velocity| Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    };
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    note(0, 0x90, 60, 100),
                    note(0, 0x90, 64, 100),
                    note(480, 0x80, 60, 0),
                    note(480, 0x80, 64, 0),
                    note(1920, 0x90, 67, 100),
                    note(2400, 0x80, 67, 0),
                    note(3840, 0x90, 72, 100),
                    note(4320, 0x80, 72, 0),
                ],
            },
            Track {
                events: vec![note(1920, 0x90, 57, 100), note(2400, 0x80, 57, 0)],
            },
        ],
    });
    app.note_mode = false;
    app
}

#[test]
fn j_and_k_audition_only_the_landing_event_while_click_keeps_the_chord() {
    let mut app = two_measures();
    let ctx = egui::Context::default();
    crate::cursor::moves::select(&mut app, 1);
    assert_eq!(app.audition.auditioned, vec![60, 64]);
    press(&mut app, &ctx, &[egui::Key::K]);
    assert_eq!(app.selected_event, Some(0));
    assert_eq!(app.audition.auditioned, vec![60]);
    press(&mut app, &ctx, &[egui::Key::J]);
    assert_eq!(app.selected_event, Some(1));
    assert_eq!(app.audition.auditioned, vec![64]);
}

#[test]
fn h_and_l_play_only_the_current_tracks_landing_measure() {
    let mut app = two_measures();
    let ctx = egui::Context::default();
    crate::cursor::moves::select(&mut app, 0);
    press(&mut app, &ctx, &[egui::Key::L]);
    assert_eq!(app.selected_event, Some(4));
    let prepared = app.playback.pending.as_ref().unwrap();
    assert_eq!(prepared.parts.len(), 1);
    assert_eq!(prepared.parts[0].0, 0);
    let pitches: Vec<_> = prepared.parts[0].2.notes.iter().map(|n| n.pitch).collect();
    assert_eq!(pitches, vec![67]);
    // Space keeps repeating the method chosen in the play overlay.
    assert_eq!(app.playback.last, crate::play::playback::Mode::Position);
    press(&mut app, &ctx, &[egui::Key::H]);
    let prepared = app.playback.pending.as_ref().unwrap();
    let pitches: Vec<_> = prepared.parts[0].2.notes.iter().map(|n| n.pitch).collect();
    assert_eq!(pitches, vec![60, 64]);
}

#[test]
fn g_then_h_or_e_goes_to_the_current_tracks_first_or_last_row() {
    let mut app = two_measures();
    let ctx = egui::Context::default();
    crate::cursor::moves::select(&mut app, 4);
    press(&mut app, &ctx, &[egui::Key::G]);
    press(&mut app, &ctx, &[egui::Key::E]);
    assert!(!app.track_view.go_open);
    assert_eq!(app.cursor_row, Some(7));
    press(&mut app, &ctx, &[egui::Key::G]);
    press(&mut app, &ctx, &[egui::Key::H]);
    assert!(!app.track_view.go_open);
    assert_eq!(app.cursor_row, Some(0));
    assert_eq!(app.track, 0);
}

#[test]
fn ctrl_down_and_up_move_by_measure_like_l_and_h() {
    let mut app = two_measures();
    let ctx = egui::Context::default();
    // Windows reports Ctrl as both ctrl and command.
    let ctrl = egui::Modifiers::CTRL | egui::Modifiers::COMMAND;
    crate::cursor::moves::select(&mut app, 0);
    crate::cursor::split::tests::press(&mut app, &ctx, &[(egui::Key::ArrowDown, ctrl)]);
    assert_eq!(app.selected_event, Some(4));
    let prepared = app.playback.pending.as_ref().unwrap();
    let pitches: Vec<_> = prepared.parts[0].2.notes.iter().map(|n| n.pitch).collect();
    assert_eq!(pitches, vec![67]);
    crate::cursor::split::tests::press(&mut app, &ctx, &[(egui::Key::ArrowUp, ctrl)]);
    assert_eq!(app.selected_event, Some(0));
}
