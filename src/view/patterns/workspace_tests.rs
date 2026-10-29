//! Vertical UI contracts: no pattern/scene metadata is injected into the fixture.
use super::workspace::{choices, Pane};
use super::*;
use cat_smf_editor::score::{
    pattern::Catalog,
    scene::{
        self,
        arrangement::{self as grid, PatternOrScene},
    },
};
use eframe::egui::{self, Key, Modifiers};
mod arrange;
mod comment_out;
mod deletion;
mod input;
mod keyboard;
mod minimap;
mod recording;
mod scrolling;
mod single_cursor;
mod timeline;

pub(super) fn context() -> egui::Context {
    let c = egui::Context::default();
    c.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let id = viewport.ids.this;
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("workspace-visible", id)),
                viewport.builder.visible.unwrap_or(true),
            )
        });
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(("window-builder", id)),
                viewport.builder.clone(),
            )
        });
        let geometry = ctx.data(|d| d.get_temp::<[f32; 4]>(egui::Id::new(("window-geometry", id))));
        let events = ctx
            .data_mut(|d| d.remove_temp::<Vec<egui::Event>>(egui::Id::new(("workspace-input", id))))
            .unwrap_or_default();
        let mut input = egui::RawInput {
            viewport_id: id,
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                viewport
                    .builder
                    .inner_size
                    .unwrap_or(egui::vec2(760.0, 440.0)),
            )),
            ..Default::default()
        };
        input.viewports.insert(
            id,
            egui::ViewportInfo {
                parent: Some(egui::ViewportId::ROOT),
                focused: Some(true),
                outer_rect: geometry.map(|[x, y, w, h]| {
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w + 16.0, h + 40.0))
                }),
                inner_rect: geometry.map(|[x, y, w, h]| {
                    egui::Rect::from_min_size(egui::pos2(x + 8.0, y + 32.0), egui::vec2(w, h))
                }),
                ..Default::default()
            },
        );
        let out = ctx.run(input, |ctx| (viewport.viewport_ui_cb)(ctx));
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new(("workspace-shapes", id)), out.shapes.clone())
        });
        record(ctx, &out);
    });
    c
}
pub(crate) fn frame(
    a: &mut App,
    c: &egui::Context,
    id: egui::ViewportId,
    events: Vec<egui::Event>,
) {
    let root = if id == egui::ViewportId::ROOT {
        events
    } else {
        c.data_mut(|d| d.insert_temp(egui::Id::new(("workspace-input", id)), events));
        vec![]
    };
    let out = c.run(
        egui::RawInput {
            events: root,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            ..Default::default()
        },
        |ctx| crate::view::show(a, ctx),
    );
    record(c, &out);
}
fn record(c: &egui::Context, out: &egui::FullOutput) {
    for (&id, out) in &out.viewport_output {
        if out.commands.contains(&egui::ViewportCommand::Focus) {
            c.data_mut(|d| d.insert_temp(egui::Id::new(("workspace-focus", id)), true));
        }
    }
}
pub(crate) fn key(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key, m: Modifiers) {
    // Native Windows input sets both ctrl and command. egui's text selection
    // uses command, while the application's physical bindings use ctrl.
    let m = Modifiers {
        command: m.command || m.ctrl,
        ..m
    };
    frame(
        a,
        c,
        id,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key: k,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: m,
            })
            .collect(),
    );
}
pub(crate) fn text(a: &mut App, c: &egui::Context, id: egui::ViewportId, s: &str) {
    frame(a, c, id, vec![]);
    frame(a, c, id, vec![egui::Event::Text(s.into())]);
}
pub(crate) fn command(a: &mut App, c: &egui::Context, id: egui::ViewportId, s: &str) {
    key(a, c, id, Key::P, Modifiers::CTRL | Modifiers::SHIFT);
    text(a, c, id, s);
    key(a, c, id, Key::Enter, Modifiers::NONE);
}
fn click(a: &mut App, c: &egui::Context, id: egui::ViewportId, rect: egui::Id) {
    frame(a, c, id, vec![]);
    let pos = c
        .data(|d| d.get_temp::<egui::Rect>(rect))
        .expect("visible control")
        .center();
    for pressed in [true, false] {
        frame(
            a,
            c,
            id,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}
fn button(a: &mut App, c: &egui::Context, p: Pane, id: &'static str) {
    click(a, c, p.viewport(), egui::Id::new((p.title(), id)));
}
fn cell(a: &mut App, c: &egui::Context, p: Pane, lane: u64, measure: u64) {
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let row = workspace::lanes(&collection, workspace::scene_id(a, p))
        .iter()
        .position(|l| *l == lane)
        .expect("saved lane");
    row_cell(a, c, p, row, measure);
}
pub(super) fn row_cell(a: &mut App, c: &egui::Context, p: Pane, row: usize, measure: u64) {
    click(
        a,
        c,
        p.viewport(),
        egui::Id::new((p.title(), "cell", row, measure)),
    );
}
pub(super) fn candidate(a: &mut App, c: &egui::Context, p: Pane, part: PatternOrScene) {
    let rect = egui::Id::new((p.title(), "part", format!("{part:?}")));
    // A filtered-out candidate must not reuse its coordinates from an older frame.
    c.data_mut(|d| d.remove::<egui::Rect>(rect));
    click(a, c, p.viewport(), rect);
}
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn body(a: &App, id: u64) -> Vec<u8> {
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let p = &c.patterns[&id];
    a.loaded.as_ref().unwrap().music.tracks[p.track]
        .events
        .iter()
        .filter(|e| {
            cat_smf_editor::score::note_view::note_on(e)
                && e.tick >= p.tick
                && e.tick < p.tick + 1920
        })
        .map(|e| e.data[0])
        .collect()
}
fn add(a: &mut App, c: &egui::Context, mml: &str) -> u64 {
    let root = egui::ViewportId::ROOT;
    command(a, c, root, "add pattern");
    if a.patterns.confirming_creation() {
        key(a, c, root, Key::Enter, Modifiers::NONE);
    }
    let id = a.patterns.selected.unwrap();
    frame(a, c, root, vec![]);
    assert!(a.mml_input.open, "{:?}", a.load_error);
    text(a, c, root, mml);
    key(a, c, root, Key::Enter, Modifiers::NONE);
    for _ in 0..3 {
        frame(a, c, root, vec![]);
    }
    assert!(!a.mml_input.open, "{:?}", a.load_error);
    id
}
fn create_scene(a: &mut App, c: &egui::Context, name: &str, length: &str) {
    button(a, c, Pane::Scene, "create");
    assert!(a.patterns.workspace.input_open());
    text(a, c, Pane::Scene.viewport(), name);
    key(a, c, Pane::Scene.viewport(), Key::Tab, Modifiers::NONE);
    key(a, c, Pane::Scene.viewport(), Key::A, Modifiers::CTRL);
    text(a, c, Pane::Scene.viewport(), length);
    key(a, c, Pane::Scene.viewport(), Key::Enter, Modifiers::NONE);
    assert!(
        !a.patterns.workspace.input_open(),
        "{:?}",
        a.patterns.workspace.error
    );
}
pub(crate) fn verification_song() -> (App, egui::Context, u64, u64, u64) {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let c = context();
    let first = add(&mut a, &c, "o4 l4 c");
    let first_body = body(&a, first);
    let second = add(&mut a, &c, "o4 l4 g");
    assert_eq!(body(&a, first), first_body);
    let root_track = a.track;
    let edit = a.patterns.editing.clone();
    let visible = |c: &egui::Context, id| {
        c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
            .unwrap_or(false)
    };
    assert!(!visible(&c, Pane::Scene.viewport()));
    let auto_save = a.saved.auto_save_enabled();
    key(&mut a, &c, Pane::Song.viewport(), Key::S, Modifiers::NONE);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert_eq!(a.saved.auto_save_enabled(), auto_save);
    for id in [Pane::Scene.viewport(), Pane::Song.viewport()] {
        assert!(
            c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
                .unwrap_or(false),
            "viewport {id:?}; owner {:?}, mode {}, mml {}, dialog {}",
            crate::view::commands::owner(&a, &c),
            mode::workspace_open(&a),
            a.mml_input.open,
            a.patterns.workspace.input_open()
        );
    }
    create_scene(&mut a, &c, "A", "2");
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let s = &collection.scenes[0];
    assert_eq!(s.meters.len(), 2);
    assert_eq!(s.lanes.len(), 1);
    let (scene, lanes) = (s.id, s.lanes.clone());
    assert!(s.cells.is_empty());
    assert_eq!(a.track, root_track);
    assert_eq!(a.patterns.editing, edit);
    let before = bytes(&a);
    candidate(&mut a, &c, Pane::Scene, PatternOrScene::Pattern(first));
    assert_eq!(bytes(&a), before);
    assert_eq!(
        a.playback
            .pending
            .as_ref()
            .expect("candidate audition")
            .active
            .mode,
        crate::play::playback::Mode::Pattern(first)
    );
    cell(&mut a, &c, Pane::Scene, lanes[0], 1);
    key(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        Key::Enter,
        Modifiers::NONE,
    );
    // A second lane is saved by placing on the unsaved row below.
    row_cell(&mut a, &c, Pane::Scene, 1, 2);
    click(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        egui::Id::new((Pane::Scene.title(), "query")),
    );
    text(&mut a, &c, Pane::Scene.viewport(), "Pattern 2");
    key(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        Key::Enter,
        Modifiers::NONE,
    );
    assert!(!a.patterns.workspace.input_open());
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        collection.scenes[0].cells.len(),
        2,
        "cells {:?}; selection {:?}/{}; error {:?}",
        collection.scenes[0].cells,
        a.patterns.workspace.scene_cell.row,
        a.patterns.workspace.scene_cell.measure,
        a.patterns.workspace.error
    );
    assert_eq!(
        grid::cell(&collection, Some(scene), collection.scenes[0].lanes[1], 2),
        Some(PatternOrScene::Pattern(second))
    );
    assert_eq!(a.track, root_track);
    // Clicking the candidate opened it, unbound to any placement.
    let editing = a.patterns.editing.as_ref().expect("candidate opened");
    assert_eq!((editing.pattern, editing.placement), (first, None));
    assert!(scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .song_lanes
        .is_empty());
    candidate(&mut a, &c, Pane::Song, PatternOrScene::Scene(scene));
    row_cell(&mut a, &c, Pane::Song, 0, 1);
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Enter,
        Modifiers::NONE,
    );
    let song_lane = workspace::lane(&a, Pane::Song).unwrap();
    candidate(&mut a, &c, Pane::Song, PatternOrScene::Pattern(second));
    cell(&mut a, &c, Pane::Song, song_lane, 3);
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Enter,
        Modifiers::NONE,
    );
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(collection.song.len(), 2);
    assert_eq!(
        grid::cell(&collection, None, song_lane, 1),
        Some(PatternOrScene::Scene(scene))
    );
    assert_eq!(
        grid::cell(&collection, None, song_lane, 3),
        Some(PatternOrScene::Pattern(second))
    );
    (a, c, first, second, scene)
}
#[test]
fn workspace_vertical_two_patterns_empty_scene_song_play_stop_and_explicit_placement() {
    let (mut a, c, first, _, _) = verification_song();
    let first_body = body(&a, first);
    for (p, end, count) in [(Pane::Scene, 4.0, 2), (Pane::Song, 6.0, 3)] {
        button(&mut a, &c, p, "transport");
        let prepared = a.playback.pending.as_ref().unwrap();
        assert_eq!(prepared.active.end, end);
        assert_eq!(
            prepared
                .parts
                .iter()
                .map(|p| p.2.notes.len())
                .sum::<usize>(),
            count
        );
        assert!(prepared.parts.iter().all(|p| p.1 == Default::default()));
        button(&mut a, &c, p, "transport");
        assert!(a.playback.pending.is_none() && a.playback.active.is_none());
    }
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-focus", Pane::Scene.viewport()))))
        .unwrap_or(false));
    assert_eq!(body(&a, first), first_body);
    assert_eq!(choices(&a, Pane::Scene).len(), 2);
}
