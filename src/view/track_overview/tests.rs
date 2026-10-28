use super::*;
use crate::view::{commands, tests::app};
use cat_smf_editor::model::{Event, Music, Track};

fn event(tick: u64, status: u8, meta_type: u8, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type,
        data,
    }
}

/// Conductor, a melody of one note per measure for four measures, and an empty track.
fn fixture() -> App {
    let end = || event(7680, 0xff, 0x2f, vec![]);
    let mut melody = Vec::new();
    for measure in 0..4 {
        melody.push(event(measure * 1920, 0x90, 0, vec![60 + measure as u8, 90]));
        melody.push(event(
            measure * 1920 + 960,
            0x80,
            0,
            vec![60 + measure as u8, 0],
        ));
    }
    melody.push(end());
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 0xff, 0x51, vec![0x07, 0xa1, 0x20]), end()],
            },
            Track { events: melody },
            Track {
                events: vec![end()],
            },
        ],
    })
}

fn key(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn render(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
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

fn press(app: &mut App, ctx: &egui::Context, keys: &[egui::Key]) {
    for &k in keys {
        render(app, ctx, vec![key(k, true), key(k, false)]);
    }
}

fn click(app: &mut App, ctx: &egui::Context, pos: egui::Pos2, button: egui::PointerButton) {
    render(app, ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        render(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

fn pane(ctx: &egui::Context, slot: usize) -> egui::Rect {
    ctx.read_response(egui::Id::new(("track-overview-pane", slot)))
        .unwrap()
        .rect
}

fn text_pos(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == text => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text {text}"))
}

fn playing_tracks(app: &App) -> Vec<usize> {
    app.playback
        .pending
        .as_ref()
        .unwrap()
        .parts
        .iter()
        .map(|p| p.0)
        .collect()
}

fn opened() -> (App, egui::Context) {
    opened_from(fixture())
}

fn opened_from(mut app: App) -> (App, egui::Context) {
    let ctx = egui::Context::default();
    app.select_track(1);
    render(&mut app, &ctx, vec![]);
    let command = commands::candidates(&app, &ctx)
        .into_iter()
        .find(|c| c.id == "track-overview")
        .unwrap();
    assert!(commands::execute(&mut app, &ctx, &command));
    render(&mut app, &ctx, vec![]);
    (app, ctx)
}

#[test]
fn palette_opens_the_overview_and_escape_closes_it() {
    let (mut app, ctx) = opened();
    assert!(app.track_overview.open);
    assert_eq!(commands::owner(&app, &ctx), Screen::Overview);
    assert_eq!(app.track_overview.cell, (1, 0));
    press(&mut app, &ctx, &[egui::Key::Escape]);
    assert!(!app.track_overview.open);
    assert_eq!(commands::owner(&app, &ctx), Screen::Main);
}

#[test]
fn the_go_menu_has_no_overview_entry() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::Escape, egui::Key::G]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Go);
    assert!(!commands::definitions(&app, Screen::Go)
        .iter()
        .any(|c| c.name == "Track Overview"));
    press(&mut app, &ctx, &[egui::Key::M]);
    assert!(!app.track_overview.open);
}

#[test]
fn keys_move_the_cell_by_measure_and_track_within_the_song() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::H, egui::Key::K, egui::Key::K]);
    assert_eq!(
        app.track_overview.cell,
        (0, 0),
        "stops at the first measure and track"
    );
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::J]);
    assert_eq!(app.track_overview.cell, (1, 1920));
    press(&mut app, &ctx, &[egui::Key::L; 6]);
    assert_eq!(
        app.track_overview.cell,
        (1, 7680),
        "stops at the last event's measure"
    );
    press(&mut app, &ctx, &[egui::Key::J, egui::Key::J]);
    assert_eq!(app.track_overview.cell, (2, 7680));
}

#[test]
fn enter_plays_every_track_and_t_plays_the_cell_track_from_its_measure() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::Enter]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert!(
        (prepared.active.end - 6.0).abs() < 1e-9,
        "plays to the song end"
    );
    assert!(playing_tracks(&app).contains(&1));
    assert!(app.track_overview.open);
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::T]);
    assert_eq!(playing_tracks(&app), vec![1]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 4.0).abs() < 1e-9);
}

#[test]
fn g_closes_the_overview_and_puts_the_cursor_on_the_cell() {
    let (mut app, ctx) = opened();
    press(
        &mut app,
        &ctx,
        &[egui::Key::L, egui::Key::L, egui::Key::K, egui::Key::G],
    );
    assert!(!app.track_overview.open);
    assert_eq!(app.track, 0);
    assert_eq!(app.track_view.cursor_tick, Some(3840));
    assert_eq!(commands::owner(&app, &ctx), Screen::Main);
}

#[test]
fn play_menu_and_space_play_from_the_cell_and_keep_the_overview_open() {
    let (mut app, ctx) = opened();
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::P]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Play);
    press(&mut app, &ctx, &[egui::Key::S]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert_eq!(playing_tracks(&app), vec![1]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Overview);
    press(&mut app, &ctx, &[egui::Key::L, egui::Key::Space]);
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!(
        (prepared.active.start_seconds - 4.0).abs() < 1e-9,
        "repeats the last mode at the moved cell"
    );
    assert!(app.track_overview.open);
}

#[test]
fn f_toggles_follow_and_ctrl_comma_opens_settings() {
    let (mut app, ctx) = opened();
    let follow = app.playback.follow;
    press(&mut app, &ctx, &[egui::Key::F]);
    assert_eq!(app.playback.follow, !follow);
    let ctrl_comma = egui::Event::Key {
        key: egui::Key::Comma,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    };
    render(&mut app, &ctx, vec![ctrl_comma]);
    assert_eq!(commands::owner(&app, &ctx), Screen::Settings);
}

#[test]
fn left_click_plays_all_tracks_from_the_clicked_measure() {
    let (mut app, ctx) = opened();
    let rect = pane(&ctx, 1);
    // 0.2 px per tick: x 400 is tick 2000 inside the second measure.
    click(
        &mut app,
        &ctx,
        egui::pos2(rect.left() + 400.0, rect.center().y),
        egui::PointerButton::Primary,
    );
    let prepared = app.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert_eq!(app.track_overview.cell, (1, 1920));
}

#[test]
fn right_click_menu_offers_track_play_and_jump() {
    let (mut app, ctx) = opened();
    let rect = pane(&ctx, 1);
    let at = egui::pos2(rect.left() + 800.0, rect.center().y);
    click(&mut app, &ctx, at, egui::PointerButton::Secondary);
    let output = render(&mut app, &ctx, vec![]);
    let button = text_pos(&output, "t — track play");
    click(&mut app, &ctx, button, egui::PointerButton::Primary);
    assert_eq!(playing_tracks(&app), vec![1]);
    assert!(
        (app.playback.pending.as_ref().unwrap().active.start_seconds - 4.0).abs() < 1e-9,
        "x 800 is the third measure"
    );
    click(&mut app, &ctx, at, egui::PointerButton::Secondary);
    let output = render(&mut app, &ctx, vec![]);
    let button = text_pos(&output, "g — jump");
    click(&mut app, &ctx, button, egui::PointerButton::Primary);
    assert!(!app.track_overview.open);
    assert_eq!(app.track, 1);
    assert_eq!(app.track_view.cursor_tick, Some(3840));
}

mod drawing;
mod pattern_edit;
