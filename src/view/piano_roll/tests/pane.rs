//! The roll placed in a pane of the main window behaves like its own window.
use super::drag::drag;
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::ViewKind;
use crate::view::timeline_controls::Screen as Timeline;

/// A conductor and two note tracks, with the cursor on the first note track.
fn two_tracks() -> crate::app::App {
    let track = |notes: &[(u64, u64, u8)]| {
        with_notes(notes)
            .loaded
            .take()
            .unwrap()
            .music
            .tracks
            .remove(0)
    };
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            cat_smf_editor::model::Track {
                events: vec![event(15360, 0xff, vec![])],
            },
            track(&NOTES),
            track(&[(0, 480, 48), (960, 480, 50)]),
        ],
    });
    app.select_track(1);
    app
}

fn beside_the_list() -> crate::store::layout::Spec {
    crate::store::layout::parse(
        "[root]\nrow = [{ view = \"event_list\" }, { view = \"piano_roll\" }]",
    )
    .unwrap()
}

fn key(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    key: egui::Key,
    modifiers: egui::Modifiers,
) {
    let events = [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
        .collect();
    at(app, ctx, time, events);
    at(app, ctx, time + 0.01, vec![]);
}

fn click(app: &mut crate::app::App, ctx: &egui::Context, time: f64, pos: egui::Pos2) {
    for (i, pressed) in [true, false].into_iter().enumerate() {
        at(
            app,
            ctx,
            time + i as f64 * 0.01,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    at(app, ctx, time + 0.02, vec![]);
}

fn minimap(ctx: &egui::Context) -> egui::Rect {
    ctx.data(|d| d.get_temp(egui::Id::new((Timeline::PianoRoll, "minimap"))))
        .unwrap()
}

/// Notes drawn inside the roll's scrolled panel, leaving out what other panes draw.
fn roll_notes(app: &crate::app::App, output: &egui::FullOutput) -> Vec<egui::Rect> {
    let panel = app.piano_roll.viewport.panel.unwrap();
    note_rects(output)
        .into_iter()
        .filter(|r| panel.intersects(*r))
        .collect()
}

fn bytes(app: &crate::app::App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
}

/// What the user can observe after each step of one input sequence.
#[derive(Debug, PartialEq)]
struct Step {
    track: usize,
    open: bool,
    bytes: Vec<u8>,
    zoom: egui::Vec2,
    /// Tick at the centre of the visible range, in units of 10 ticks.
    centre: Option<u64>,
}

fn observe(app: &crate::app::App, ctx: &egui::Context) -> Step {
    let centre = ctx
        .data(|d| {
            d.get_temp::<crate::view::timeline_controls::Snapshot>(egui::Id::new((
                Timeline::PianoRoll,
                "snapshot",
            )))
        })
        .filter(|s| s.scroll > 0.0)
        .map(|s| {
            let end = app.loaded.as_ref().unwrap().music.tracks[0]
                .events
                .last()
                .unwrap()
                .tick as f32;
            ((s.scroll + s.width / 2.0) / s.end_x * end / 10.0).round() as u64
        });
    Step {
        track: app.track,
        open: app.piano_roll.open,
        bytes: bytes(app),
        zoom: app.piano_roll.viewport.zoom(),
        centre,
    }
}

/// Opens the roll with `R`, drags a note, zooms, clicks the minimap, changes the track,
/// undoes and leaves with `Escape`.
fn run(pane: bool) -> Vec<Step> {
    let mut app = two_tracks();
    if pane {
        app.layout.spec = beside_the_list();
    }
    let ctx = egui::Context::default();
    let mut steps = Vec::new();
    at(&mut app, &ctx, 0.0, vec![]);
    key(&mut app, &ctx, 1.0, egui::Key::R, egui::Modifiers::NONE);
    let output = at(&mut app, &ctx, 10.0, vec![]);
    assert_eq!(owner(&app, &ctx), Screen::Timeline, "pane {pane}");
    let map = minimap(&ctx);
    assert_eq!(map.left() > 750.0, pane, "{map:?}");
    steps.push(observe(&app, &ctx));

    let note = roll_notes(&app, &output)[2];
    let from = egui::pos2(note.left() + 48.0, note.center().y);
    drag(&mut app, &ctx, 11.0, from, from + egui::vec2(24.0, 0.0));
    steps.push(observe(&app, &ctx));

    click(
        &mut app,
        &ctx,
        12.0,
        egui::pos2(map.center().x, map.top() - 12.0),
    );
    steps.push(observe(&app, &ctx));

    click(
        &mut app,
        &ctx,
        13.0,
        egui::pos2(map.left() + map.width() * 0.6, map.center().y),
    );
    steps.push(observe(&app, &ctx));

    key(
        &mut app,
        &ctx,
        14.0,
        egui::Key::ArrowRight,
        egui::Modifiers::CTRL,
    );
    steps.push(observe(&app, &ctx));

    key(&mut app, &ctx, 15.0, egui::Key::Z, egui::Modifiers::CTRL);
    steps.push(observe(&app, &ctx));

    key(
        &mut app,
        &ctx,
        16.0,
        egui::Key::Escape,
        egui::Modifiers::NONE,
    );
    assert_eq!(owner(&app, &ctx), Screen::Main, "pane {pane}");
    steps.push(observe(&app, &ctx));
    steps
}

#[test]
fn the_pane_and_the_window_give_the_same_results_for_the_same_input() {
    let window = run(false);
    let pane = run(true);
    // Each step changed what it should, so equal results are not both "nothing happened".
    assert_ne!(window[0].bytes, window[1].bytes, "drag edits");
    assert!(window[2].zoom.x > window[1].zoom.x, "zoom");
    assert!(window[3].centre > window[2].centre, "minimap");
    assert_eq!((window[3].track, window[4].track), (1, 2), "track");
    assert_eq!(window[5].bytes, window[0].bytes, "undo");
    assert!(!window[6].open);
    assert_eq!(window.len(), pane.len());
    for (i, (w, p)) in window.iter().zip(&pane).enumerate() {
        if i == 2 {
            // The zoom keeps the tick under the pointer, so the centre depends on the width,
            // which the pane shares with the event list.
            assert_eq!(
                (w.track, w.open, &w.bytes, w.zoom, w.centre.is_some()),
                (p.track, p.open, &p.bytes, p.zoom, p.centre.is_some()),
                "step {i}"
            );
        } else {
            assert_eq!(w, p, "step {i}");
        }
    }
}

#[test]
fn keys_and_clicks_move_the_focus_between_the_list_and_the_roll() {
    let mut app = two_tracks();
    app.layout.spec = beside_the_list();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    at(&mut app, &ctx, 1.0, vec![]);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    key(&mut app, &ctx, 2.0, egui::Key::F6, egui::Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
    assert!(app.piano_roll.open);
    key(&mut app, &ctx, 3.0, egui::Key::F6, egui::Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    assert!(!app.piano_roll.open);
    key(&mut app, &ctx, 4.0, egui::Key::F6, egui::Modifiers::SHIFT);
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
    key(
        &mut app,
        &ctx,
        5.0,
        egui::Key::Escape,
        egui::Modifiers::NONE,
    );
    assert_eq!(owner(&app, &ctx), Screen::Main);
    // The roll stays drawn while the list has the keys, and a press on it takes them.
    let map = minimap(&ctx);
    click(
        &mut app,
        &ctx,
        6.0,
        egui::pos2(map.center().x, map.bottom() + 40.0),
    );
    assert_eq!(crate::view::dock::focus::focused(&app), ViewKind::PianoRoll);
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
    click(&mut app, &ctx, 7.0, egui::pos2(200.0, 400.0));
    assert_eq!(owner(&app, &ctx), Screen::Main);
}

#[test]
fn leaving_the_layout_brings_the_window_back() {
    let window = || {
        |ctx: &egui::Context| {
            ["Piano roll", "piano-roll-maximized"]
                .iter()
                .any(|id| ctx.memory(|m| m.area_rect(egui::Id::new(id)).is_some()))
        }
    };
    let mut app = two_tracks();
    app.layout.spec = beside_the_list();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    key(&mut app, &ctx, 1.0, egui::Key::R, egui::Modifiers::NONE);
    at(&mut app, &ctx, 2.0, vec![]);
    assert!(!window()(&ctx));
    assert!(minimap(&ctx).left() > 750.0);
    app.layout.spec = Default::default();
    at(&mut app, &ctx, 3.0, vec![]);
    at(&mut app, &ctx, 10.0, vec![]);
    assert!(app.piano_roll.open);
    assert!(window()(&ctx));
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
}
