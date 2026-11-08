//! The curve placed in a pane of the main window behaves like its own window.
use super::circles;
use crate::view::commands::{owner, Screen};
use crate::view::dock::ViewKind;
use crate::view::piano_roll::tests::at;
use crate::view::tempo_curve::TEMPO;
use crate::view::tests::app;
use crate::view::timeline_controls::Screen as Timeline;
use eframe::egui;

/// Long enough that the minimap scrolls the curve.
const END: u64 = 153600;

fn music() -> cat_smf_editor::model::Music {
    let mut music = super::music();
    for track in &mut music.tracks {
        track.events.last_mut().unwrap().tick = END;
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    music
}

fn beside_the_list() -> crate::store::layout::Spec {
    crate::store::layout::parse(
        "[root]\nrow = [{ view = \"event_list\" }, { view = \"tempo_curve\" }]",
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

fn click(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    pos: egui::Pos2,
) -> egui::FullOutput {
    super::edit::click(app, ctx, time, pos, egui::PointerButton::Primary)
}

fn minimap(ctx: &egui::Context) -> egui::Rect {
    ctx.data(|d| d.get_temp(egui::Id::new((Timeline::Tempo, "minimap"))))
        .unwrap()
}

fn bytes(app: &crate::app::App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
}

/// What the user can observe after each step of one input sequence.
#[derive(Debug, PartialEq)]
struct Step {
    track: usize,
    open: bool,
    roll: bool,
    bytes: Vec<u8>,
    zoom: f32,
    /// Tick at the centre of the visible range, in units of 10 ticks.
    centre: Option<u64>,
}

fn observe(app: &crate::app::App, ctx: &egui::Context) -> Step {
    let centre = ctx
        .data(|d| {
            d.get_temp::<crate::view::timeline_controls::Snapshot>(egui::Id::new((
                Timeline::Tempo,
                "snapshot",
            )))
        })
        .filter(|s| s.scroll > 0.0)
        .map(|s| ((s.scroll + s.width / 2.0) / s.end_x * END as f32 / 10.0).round() as u64);
    Step {
        track: app.track,
        open: app.tempo_curve.open,
        roll: app.piano_roll.open,
        bytes: bytes(app),
        zoom: app.tempo_curve.viewport.time_zoom(),
        centre,
    }
}

/// Opens the curve with `R` on the conductor, adds a point, zooms, clicks the minimap,
/// undoes, moves to the next track and leaves with `Escape`.
fn run(pane: bool) -> Vec<Step> {
    let mut app = app(music());
    if pane {
        app.layout.spec = beside_the_list();
    }
    app.select_track(0);
    let ctx = egui::Context::default();
    let mut steps = Vec::new();
    at(&mut app, &ctx, 0.0, vec![]);
    key(&mut app, &ctx, 1.0, egui::Key::R, egui::Modifiers::NONE);
    let output = at(&mut app, &ctx, 10.0, vec![]);
    assert_eq!(owner(&app, &ctx), Screen::Timeline, "pane {pane}");
    let map = minimap(&ctx);
    assert_eq!(map.left() > 750.0, pane, "{map:?}");
    steps.push(observe(&app, &ctx));

    // The 120 BPM tempo at tick 0: a click one quarter later at that height adds a point.
    let first = circles(&output, TEMPO)[0].0;
    click(&mut app, &ctx, 11.0, first + egui::vec2(96.0, 0.0));
    steps.push(observe(&app, &ctx));

    click(
        &mut app,
        &ctx,
        12.0,
        egui::pos2(map.center().x, map.top() - 12.0),
    );
    steps.push(observe(&app, &ctx));

    // The pane moves once the song has a curve, so the minimap is read again.
    let map = minimap(&ctx);
    click(
        &mut app,
        &ctx,
        13.0,
        egui::pos2(map.left() + map.width() * 0.6, map.center().y),
    );
    steps.push(observe(&app, &ctx));

    key(&mut app, &ctx, 14.0, egui::Key::Z, egui::Modifiers::CTRL);
    steps.push(observe(&app, &ctx));

    key(
        &mut app,
        &ctx,
        15.0,
        egui::Key::ArrowRight,
        egui::Modifiers::CTRL,
    );
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
    assert_ne!(window[0].bytes, window[1].bytes, "point added");
    assert!(window[2].zoom > window[1].zoom, "zoom");
    assert!(window[3].centre > window[2].centre, "minimap");
    assert_eq!(window[4].bytes, window[0].bytes, "undo");
    assert_eq!((window[5].track, window[5].roll), (1, true), "track");
    assert!(!window[5].open && !window[6].roll);
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
fn keys_and_clicks_move_the_focus_between_the_list_and_the_curve() {
    let mut app = app(music());
    app.layout.spec = beside_the_list();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    at(&mut app, &ctx, 1.0, vec![]);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    key(&mut app, &ctx, 2.0, egui::Key::F6, egui::Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
    assert!(app.tempo_curve.open);
    key(&mut app, &ctx, 3.0, egui::Key::F6, egui::Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    assert!(!app.tempo_curve.open);
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
    // The curve stays drawn while the list has the keys, and a press on it takes them.
    let map = minimap(&ctx);
    click(
        &mut app,
        &ctx,
        6.0,
        egui::pos2(map.center().x, map.bottom() + 40.0),
    );
    assert_eq!(
        crate::view::dock::focus::focused(&app),
        ViewKind::TempoCurve
    );
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
    click(&mut app, &ctx, 7.0, egui::pos2(200.0, 400.0));
    assert_eq!(owner(&app, &ctx), Screen::Main);
}

#[test]
fn the_roll_and_the_curve_share_the_focus_when_both_are_placed() {
    let mut app = app(music());
    app.layout.spec = crate::store::layout::parse(
        "[root]\nrow = [{ view = \"event_list\" }, { column = [{ view = \"piano_roll\" }, { view = \"tempo_curve\" }] }]",
    )
    .unwrap();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    let order: Vec<_> = (0..3)
        .map(|i| {
            key(
                &mut app,
                &ctx,
                1.0 + i as f64,
                egui::Key::F6,
                egui::Modifiers::NONE,
            );
            (
                crate::view::dock::focus::focused(&app),
                app.piano_roll.open,
                app.tempo_curve.open,
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            (ViewKind::PianoRoll, true, false),
            (ViewKind::TempoCurve, false, true),
            (ViewKind::EventList, false, false),
        ]
    );
}

#[test]
fn leaving_the_layout_brings_the_window_back() {
    let window = |ctx: &egui::Context| {
        ["Tempo curve", "tempo-curve-maximized"]
            .iter()
            .any(|id| ctx.memory(|m| m.area_rect(egui::Id::new(id)).is_some()))
    };
    let mut app = app(music());
    app.layout.spec = beside_the_list();
    app.select_track(0);
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    key(&mut app, &ctx, 1.0, egui::Key::R, egui::Modifiers::NONE);
    at(&mut app, &ctx, 2.0, vec![]);
    assert!(!window(&ctx));
    assert!(minimap(&ctx).left() > 750.0);
    app.layout.spec = Default::default();
    at(&mut app, &ctx, 3.0, vec![]);
    at(&mut app, &ctx, 10.0, vec![]);
    assert!(app.tempo_curve.open);
    assert!(window(&ctx));
    assert_eq!(owner(&app, &ctx), Screen::Timeline);
}
