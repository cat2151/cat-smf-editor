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
mod blocks;
mod candidates;
mod comment_out;
mod deletion;
mod edit_refresh;
pub(crate) mod harness;
mod input;
mod keyboard;
mod minimap;
mod pane;
mod placed;
mod playhead;
mod recording;
mod resume;
mod scene_pane;
mod scrolling;
mod single_cursor;
mod storm;
mod timeline;

pub(crate) use harness::{command, context, focus, frame, key, request_close, text};
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
pub(crate) fn button(a: &mut App, c: &egui::Context, p: Pane, id: &'static str) {
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
    command(a, c, root, "pattern.create");
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
/// Places `pattern` on the first Song lane so the Song ends after `measures`.
pub(crate) fn lengthen_song(a: &mut App, pattern: u64, measures: u64) {
    let music = &mut a.loaded.as_mut().unwrap().music;
    let lane = scene::read(music).unwrap().song_lanes[0];
    let length = cat_smf_editor::score::pattern::Catalog::read(music)
        .unwrap()
        .patterns[&pattern]
        .meters
        .len() as u64;
    let at = measures + 1 - length;
    grid::place(
        music,
        None,
        lane,
        at,
        Some(PatternOrScene::Pattern(pattern)),
    )
    .unwrap();
    a.loaded.as_mut().unwrap().refresh();
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
