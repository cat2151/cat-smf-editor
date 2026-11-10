//! The mixer placed in a pane of the main window behaves like its own window.
use crate::view::commands::{owner, Screen};
use crate::view::dock::ViewKind;
use crate::view::piano_roll::tests::at;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui::{self, Key, Modifiers};

mod faders;

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type: if status == 0xff { data[0] } else { 0 },
        data: if status == 0xff {
            data[1..].into()
        } else {
            data.into()
        },
    }
}

fn app() -> crate::app::App {
    let track = |key: u8| Track {
        events: vec![
            event(0, 0x90, &[key, 100]),
            event(480, 0x80, &[key, 0]),
            event(1920, 0xff, &[0x2f]),
        ],
    };
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![track(60), track(64)],
    };
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    crate::view::tests::app(music)
}

fn beside_the_list() -> crate::store::layout::Spec {
    crate::store::layout::parse("[root]\nrow = [{ view = \"event_list\" }, { view = \"mixer\" }]")
        .unwrap()
}

fn key(app: &mut crate::app::App, ctx: &egui::Context, time: f64, key: Key, modifiers: Modifiers) {
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
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    at(
        app,
        ctx,
        time,
        vec![egui::Event::PointerMoved(pos), button(true)],
    );
    at(app, ctx, time + 0.01, vec![button(false)]);
    at(app, ctx, time + 0.02, vec![]);
}

fn text(output: &egui::FullOutput, text: &str) -> Option<egui::Pos2> {
    output.shapes.iter().find_map(|s| match &s.shape {
        egui::epaint::Shape::Text(t) if t.galley.text() == text => {
            Some(t.pos + t.galley.rect.center().to_vec2())
        }
        _ => None,
    })
}

fn window(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.area_rect(egui::Id::new("Mixer (m)")).is_some())
}

/// What the user can observe after each step of one input sequence.
#[derive(Debug, PartialEq)]
struct Step {
    owner: Screen,
    open: bool,
    focused: bool,
    track: usize,
    master: bool,
    gains: [f32; 2],
    bytes: Vec<u8>,
}

fn observe(app: &crate::app::App, ctx: &egui::Context) -> Step {
    Step {
        owner: owner(app, ctx),
        open: app.playback.mixer_open,
        focused: app.playback.mixer_focused(),
        track: app.playback.mixer_track,
        master: app.playback.mixer_master,
        gains: [app.audition.gain(0), app.audition.gain(1)],
        bytes: cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
    }
}

/// Opens the mixer with `M`, walks the tracks, changes two volumes, opens the master
/// chain, leaves it, hands the keys back with `Escape`, takes them again and closes.
fn run(pane: bool) -> Vec<Step> {
    let mut app = app();
    if pane {
        app.layout.spec = beside_the_list();
    }
    let ctx = egui::Context::default();
    let mut steps = Vec::new();
    at(&mut app, &ctx, 0.0, vec![]);
    let mut time = 1.0;
    for (k, m) in [
        (Key::M, Modifiers::NONE),
        (Key::L, Modifiers::NONE),
        (Key::K, Modifiers::NONE),
        (Key::K, Modifiers::NONE),
        (Key::L, Modifiers::NONE),
        (Key::J, Modifiers::NONE),
        (Key::H, Modifiers::NONE),
        (Key::H, Modifiers::NONE),
        (Key::K, Modifiers::NONE),
        (Key::X, Modifiers::NONE),
        (Key::Escape, Modifiers::NONE),
        (Key::Escape, Modifiers::NONE),
        (Key::M, Modifiers::NONE),
        (Key::M, Modifiers::NONE),
    ] {
        key(&mut app, &ctx, time, k, m);
        let output = at(&mut app, &ctx, time + 5.0, vec![]);
        time += 10.0;
        if steps.is_empty() {
            assert_eq!(window(&ctx), !pane);
            let master = text(&output, "Master effects").unwrap();
            assert_eq!(master.x > 750.0, pane, "{master:?}");
        }
        steps.push(observe(&app, &ctx));
    }
    steps
}

#[test]
fn the_pane_and_the_window_give_the_same_results_for_the_same_keys() {
    let window = run(false);
    let pane = run(true);
    let db = crate::play::track_volume::db_to_gain;
    // Each step did something: the keys reached the mixer.
    let at = |i: usize| (&pane[i].owner, pane[i].track, pane[i].master);
    assert_eq!(at(0), (&Screen::Mixer, 0, true));
    assert_eq!(at(1), (&Screen::Mixer, 0, false));
    assert_eq!(pane[3].gains, [db(6), 1.0]);
    assert_eq!(pane[5].gains, [db(6), db(-3)]);
    assert_ne!(pane[3].bytes, pane[0].bytes);
    assert!(pane[7].master);
    assert_eq!(pane[9].owner, Screen::Effects);
    assert_eq!(pane[10].owner, Screen::Mixer);
    assert_eq!(pane[11].owner, Screen::Main);
    assert!(pane[12].focused);
    assert!(!pane[13].open);
    for (i, (w, p)) in window.iter().zip(&pane).enumerate() {
        assert_eq!(w, p, "step {i}");
    }
}

#[test]
fn keys_and_clicks_move_the_focus_between_the_list_and_the_mixer() {
    let mut app = app();
    app.layout.spec = beside_the_list();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    let output = at(&mut app, &ctx, 1.0, vec![]);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    key(&mut app, &ctx, 2.0, Key::F6, Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Mixer);
    assert_eq!(crate::view::dock::focus::focused(&app), ViewKind::Mixer);
    // The event list does not see the keys while the mixer has them.
    let row = app.cursor_row;
    key(&mut app, &ctx, 3.0, Key::L, Modifiers::NONE);
    key(&mut app, &ctx, 4.0, Key::J, Modifiers::NONE);
    assert_eq!(app.cursor_row, row);
    assert_eq!(
        app.audition.gain(0),
        crate::play::track_volume::db_to_gain(-3)
    );
    // A menu owns the keys above the focused pane.
    app.edit_menu.open = true;
    assert_eq!(owner(&app, &ctx), Screen::Edit);
    app.edit_menu.open = false;
    key(&mut app, &ctx, 5.0, Key::F6, Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    assert!(!app.playback.mixer_focused());
    key(&mut app, &ctx, 6.0, Key::F6, Modifiers::SHIFT);
    assert_eq!(owner(&app, &ctx), Screen::Mixer);
    key(&mut app, &ctx, 7.0, Key::Escape, Modifiers::NONE);
    assert_eq!(owner(&app, &ctx), Screen::Main);
    key(&mut app, &ctx, 8.0, Key::J, Modifiers::NONE);
    assert_ne!(app.cursor_row, row);
    // The mixer stays drawn while the list has the keys, and a press on it takes them.
    let scale = text(&output, "+6 dB").unwrap();
    click(&mut app, &ctx, 9.0, scale);
    assert_eq!(owner(&app, &ctx), Screen::Mixer);
    click(&mut app, &ctx, 10.0, egui::pos2(200.0, 400.0));
    assert_eq!(owner(&app, &ctx), Screen::Main);
}

#[test]
fn leaving_the_layout_brings_the_window_back() {
    let mut app = app();
    app.layout.spec = beside_the_list();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 0.0, vec![]);
    key(&mut app, &ctx, 1.0, Key::M, Modifiers::NONE);
    at(&mut app, &ctx, 2.0, vec![]);
    assert!(!window(&ctx));
    app.layout.spec = Default::default();
    at(&mut app, &ctx, 3.0, vec![]);
    at(&mut app, &ctx, 10.0, vec![]);
    assert!(app.playback.mixer_open);
    assert!(window(&ctx));
    assert_eq!(owner(&app, &ctx), Screen::Mixer);
}
