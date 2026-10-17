use crate::view::{note_edit::Field, tests::app};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

fn note(tick: u64, status: u8, key: u8) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type: 0,
        data: vec![key, if status == 0x90 { 100 } else { 0 }],
    }
}

/// Conductor plus a C-E-G chord held for two beats.
pub(crate) fn chord_song() -> Music {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![Event {
                    delta: 0,
                    tick: 0,
                    status: 0xff,
                    meta_type: 0x51,
                    data: vec![0x07, 0xa1, 0x20],
                }],
            },
            Track {
                events: vec![
                    note(0, 0x90, 60),
                    note(0, 0x90, 64),
                    note(0, 0x90, 67),
                    note(960, 0x80, 60),
                    note(960, 0x80, 64),
                    note(960, 0x80, 67),
                    note(1920, 0x90, 60),
                    note(2400, 0x80, 60),
                ],
            },
        ],
    };
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    music
}

pub(crate) fn render(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
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
}

fn texts(output: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some((t.galley.text().to_owned(), t.pos)),
            _ => None,
        })
        .collect()
}

#[test]
fn clicking_the_mbt_of_one_chord_note_and_entering_a_new_mbt_moves_only_that_start() {
    let mut app = app(chord_song());
    app.select_track(1);
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    let output = render(&mut app, &ctx, vec![]);
    let shapes = texts(&output);
    let e4 = shapes.iter().find(|(t, _)| t == "E4").unwrap().1;
    let mbt = shapes
        .iter()
        .find(|(t, p)| t.trim() == "1:1:0" && (p.y - e4.y).abs() < 2.0)
        .unwrap()
        .1
        + egui::vec2(3.0, 5.0);
    let button = |pressed| egui::Event::PointerButton {
        pos: mbt,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    render(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(mbt), button(true)],
    );
    render(&mut app, &ctx, vec![button(false)]);
    let editing = app.note_edit.as_mut().unwrap();
    assert_eq!(
        (editing.track, editing.event, editing.text.as_str()),
        (1, 1, "1:1:0")
    );
    editing.text = "1:2:0".into();
    let enter = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    render(&mut app, &ctx, vec![enter(true), enter(false)]);
    assert!(app.note_edit.is_none());
    let loaded = app.loaded.as_ref().unwrap();
    let starts: Vec<_> = loaded.music.tracks[1]
        .events
        .iter()
        .map(|e| (e.tick, e.status, e.data[0]))
        .collect();
    assert_eq!(
        starts,
        vec![
            (0, 0x90, 60),
            (0, 0x90, 67),
            (480, 0x90, 64),
            (960, 0x80, 60),
            (960, 0x80, 64),
            (960, 0x80, 67),
            (1920, 0x90, 60),
            (2400, 0x80, 60),
        ]
    );
    assert_eq!(loaded.note_views[1].durations[2], Some(480));
}

#[test]
fn an_invalid_mbt_keeps_the_editor_open_and_the_song_unchanged() {
    let mut app = app(chord_song());
    app.select_track(1);
    let ctx = egui::Context::default();
    let before = format!("{:?}", app.loaded.as_ref().unwrap().music.tracks);
    crate::view::note_edit::begin(
        &mut app,
        crate::view::note_edit::Field::Mbt,
        1,
        1,
        egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(90.0, 20.0)),
        " 1: 3:  0",
    );
    render(&mut app, &ctx, vec![]);
    // 1:3:0 is tick 960, the note's own Note Off.
    let enter = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    render(&mut app, &ctx, vec![enter(true), enter(false)]);
    assert!(app.note_edit.as_ref().unwrap().error.is_some());
    assert_eq!(
        format!("{:?}", app.loaded.as_ref().unwrap().music.tracks),
        before
    );
}

/// The open field's kind, event, and text.
pub(crate) fn editing(app: &crate::app::App) -> Option<(Field, usize, String)> {
    app.note_edit
        .as_ref()
        .map(|editing| (editing.field, editing.event, editing.text.clone()))
}

/// Replaces the open field's text and presses Enter.
pub(crate) fn enter(app: &mut crate::app::App, ctx: &egui::Context, text: &str) {
    app.note_edit.as_mut().unwrap().text = text.into();
    let enter = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    render(app, ctx, vec![enter(true), enter(false)]);
}

#[test]
fn clicking_the_duration_of_one_chord_note_and_entering_ticks_moves_only_its_end() {
    let mut app = app(chord_song());
    app.select_track(1);
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    let output = render(&mut app, &ctx, vec![]);
    let shapes = texts(&output);
    let e4 = shapes.iter().find(|(t, _)| t == "E4").unwrap().1;
    let duration = shapes
        .iter()
        .find(|(t, p)| t == "960" && (p.y - e4.y).abs() < 2.0)
        .unwrap()
        .1
        + egui::vec2(3.0, 5.0);
    let button = |pressed| egui::Event::PointerButton {
        pos: duration,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    render(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(duration), button(true)],
    );
    render(&mut app, &ctx, vec![button(false)]);
    assert_eq!(editing(&app), Some((Field::Duration, 1, "960".into())));
    enter(&mut app, &ctx, "480");
    assert!(app.note_edit.is_none());
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(loaded.music.tracks[1].events[3].tick, 480);
    assert_eq!(loaded.note_views[1].durations[1], Some(480));
    assert_eq!(loaded.note_views[1].durations[0], Some(960));
}
