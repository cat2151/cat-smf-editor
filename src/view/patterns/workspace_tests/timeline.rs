use super::*;
use crate::play::{playback::Mode, transport};

fn rect(c: &egui::Context, name: &str) -> egui::Rect {
    c.data(|d| d.get_temp(egui::Id::new((Pane::Song.title(), name))))
        .unwrap()
}

fn cell_rect(c: &egui::Context, measure: u64) -> egui::Rect {
    c.data(|d| d.get_temp(egui::Id::new((Pane::Song.title(), "cell", 0usize, measure))))
        .expect("rendered measure")
}

#[test]
fn song_zoom_out_shrinks_cells_and_reveals_more_measures_without_changing_scene() {
    let (mut a, c, _, second, _) = verification_song();
    let song = Pane::Song.viewport();
    a.patterns.workspace.song_cell.measure = 1;
    frame(&mut a, &c, song, vec![]);
    let before = cell_rect(&c, 1);
    let scene_before = c
        .data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new((Pane::Scene.title(), "cell", 0usize, 1u64)))
        })
        .unwrap();
    let saved = bytes(&a);
    for _ in 0..4 {
        key(&mut a, &c, song, Key::PageDown, Modifiers::NONE);
    }
    // Let the grid's cached column widths settle after changing zoom.
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    let small = cell_rect(&c, 1);
    assert!((small.width() / before.width() - (2.0f32 / 3.0).powi(4)).abs() < 0.01);
    assert_eq!(small.height(), before.height());
    let distant = cell_rect(&c, 20);
    assert!(distant.right() < rect(&c, "zoom").right());
    let scene_after = c
        .data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new((Pane::Scene.title(), "cell", 0usize, 1u64)))
        })
        .unwrap();
    assert_eq!(scene_before.size(), scene_after.size());
    assert_eq!(bytes(&a), saved);

    a.patterns.workspace.song_cell.placing = true;
    a.patterns.workspace.song_cell.choice = Some(PatternOrScene::Pattern(second));
    for _ in 0..19 {
        key(&mut a, &c, song, Key::L, Modifiers::NONE);
    }
    assert_eq!(a.patterns.workspace.song_cell.measure, 20);
    key(&mut a, &c, song, Key::F, Modifiers::NONE);
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        grid::cell(&collection, None, collection.song_lanes[0], 20),
        Some(PatternOrScene::Pattern(second))
    );
    key(&mut a, &c, song, Key::Delete, Modifiers::NONE);
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        grid::cell(&collection, None, collection.song_lanes[0], 20),
        None
    );
}

fn pointer(
    a: &mut App,
    c: &egui::Context,
    pos: egui::Pos2,
    button: egui::PointerButton,
    pressed: bool,
) {
    frame(
        a,
        c,
        Pane::Song.viewport(),
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ],
    );
}

#[test]
fn song_zoom_changes_cells_and_repeat_but_keeps_the_whole_song_overview() {
    let (mut a, c, _, _, _) = verification_song();
    let id = Pane::Song.viewport();
    frame(&mut a, &c, id, vec![]);
    let before = rect(&c, "minimap");
    let cell_before = cell_rect(&c, 3);
    let zoom = rect(&c, "zoom");
    assert_eq!(before.left(), zoom.left(), "no vertical zoom bar");
    for pressed in [true, false] {
        pointer(
            &mut a,
            &c,
            zoom.center(),
            egui::PointerButton::Primary,
            pressed,
        );
    }
    for _ in 0..3 {
        frame(&mut a, &c, id, vec![]);
    }
    let enlarged = rect(&c, "minimap");
    assert_eq!(enlarged, before);
    let stride = cell_rect(&c, 4).left() - cell_rect(&c, 3).left();
    assert!((rect(&c, "repeat").left() + 2.0 * stride - cell_rect(&c, 3).left()).abs() < 0.5);
    assert!((cell_rect(&c, 3).width() / cell_before.width() - 1.5).abs() < 0.01);
    for pressed in [true, false] {
        pointer(
            &mut a,
            &c,
            zoom.center(),
            egui::PointerButton::Secondary,
            pressed,
        );
    }
    assert!((rect(&c, "minimap").width() - before.width()).abs() < 1.0);
    assert!((cell_rect(&c, 3).width() - cell_before.width()).abs() < 0.01);
    let from = egui::pos2(zoom.left() + zoom.width() * 0.25, zoom.center().y);
    let to = egui::pos2(zoom.left() + zoom.width() * 0.75, zoom.center().y);
    pointer(&mut a, &c, from, egui::PointerButton::Primary, true);
    frame(&mut a, &c, id, vec![egui::Event::PointerMoved(to)]);
    pointer(&mut a, &c, to, egui::PointerButton::Primary, false);
    assert_eq!(rect(&c, "minimap"), before);
    assert!((cell_rect(&c, 3).width() / cell_before.width() - 1.8).abs() < 0.02);
    assert!(a.playback.repeat.is_none());
    // Seeking stays in whole-Song coordinates even after range zoom and scrolling.
    let point = before.center();
    for pressed in [true, false] {
        pointer(&mut a, &c, point, egui::PointerButton::Primary, pressed);
    }
    let active = &a.playback.pending.as_ref().expect("overview seek").active;
    assert_eq!(active.mode, Mode::Arrangement(None));
    assert!((active.start_seconds - 3.0).abs() < 0.02);
}

#[test]
fn song_repeat_uses_arrangement_with_candidate_open_and_keeps_scope_on_loop_and_clear() {
    let (mut a, c, _, _, _) = verification_song();
    let before = bytes(&a);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    let row = rect(&c, "repeat");
    let first = cell_rect(&c, 1);
    let second = cell_rect(&c, 2);
    let stride = second.left() - first.left();
    assert!((row.left() - first.left()).abs() < 0.5);
    assert!(row.bottom() < first.top());
    assert!(row.top() > rect(&c, "minimap").bottom());
    // Measure 1 + 3/4 to measure 3 + 1/4, in cell coordinates.
    let point = |fraction| egui::pos2(first.left() + stride * fraction * 3.0, row.center().y);
    pointer(&mut a, &c, point(0.25), egui::PointerButton::Primary, true);
    frame(
        &mut a,
        &c,
        Pane::Song.viewport(),
        vec![egui::Event::PointerMoved(point(0.75))],
    );
    pointer(&mut a, &c, point(0.75), egui::PointerButton::Primary, false);
    let range = a.playback.repeat.expect("repeat range");
    let prepared = &a.playback.pending.as_ref().unwrap().active;
    assert_eq!(prepared.mode, Mode::Arrangement(None));
    assert!((prepared.start_seconds - 1.5).abs() < 0.02);
    assert!((prepared.end - 3.0).abs() < 0.02);
    assert_eq!(
        transport::repeat::prepare(&a, range).unwrap().active.mode,
        Mode::Arrangement(None)
    );
    for pressed in [true, false] {
        pointer(
            &mut a,
            &c,
            point(0.9),
            egui::PointerButton::Secondary,
            pressed,
        );
    }
    assert!(a.playback.repeat.unwrap().1 > range.1);
    for pressed in [true, false] {
        pointer(
            &mut a,
            &c,
            point(0.5),
            egui::PointerButton::Primary,
            pressed,
        );
    }
    assert!(a.playback.repeat.is_none());
    assert_eq!(
        a.playback.pending.as_ref().unwrap().active.mode,
        Mode::Arrangement(None)
    );
    assert_eq!(bytes(&a), before);
}

#[test]
fn song_timeline_keyboard_zoom_and_repeat_commands_preserve_cell_navigation() {
    let (mut a, c, _, _, _) = verification_song();
    let id = Pane::Song.viewport();
    frame(&mut a, &c, id, vec![]);
    let width = rect(&c, "minimap").width();
    let cell_width = cell_rect(&c, 3).width();
    key(&mut a, &c, id, Key::PageUp, Modifiers::NONE);
    assert_eq!(rect(&c, "minimap").width(), width);
    assert!(cell_rect(&c, 3).width() > cell_width);
    key(&mut a, &c, id, Key::PageDown, Modifiers::NONE);
    assert!((rect(&c, "minimap").width() - width).abs() < 1.0);
    a.patterns.workspace.song_cell.measure = 1;
    command(&mut a, &c, id, "Song repeat A");
    a.patterns.workspace.song_cell.measure = 3;
    command(&mut a, &c, id, "Song repeat B");
    assert_eq!(a.playback.repeat, Some((0, 3840)));
    command(&mut a, &c, id, "Song repeat解除");
    assert!(a.playback.repeat.is_none());
}

#[test]
fn song_minimap_pans_cells_live_and_repeat_tracks_the_scrolled_cells() {
    for outside in [false, true] {
        let (mut a, c, _, _, _) = verification_song();
        let music = &mut a.loaded.as_mut().unwrap().music;
        cat_smf_editor::score::scene::sizing::song(music, 40).unwrap();
        let id = Pane::Song.viewport();
        for _ in 0..3 {
            frame(&mut a, &c, id, vec![]);
        }
        let before = bytes(&a);
        let selected = a.patterns.workspace.song_cell.measure;
        let map = rect(&c, "minimap");
        let visible = rect(&c, "minimap-visible");
        let first_cell = cell_rect(&c, 1);
        let stride = cell_rect(&c, 2).left() - first_cell.left();
        let label = first_cell.left() - rect(&c, "matrix-viewport").left();
        assert!(visible.width() < map.width() / 2.0);
        let from = if outside {
            map.center()
        } else {
            visible.center()
        };
        let to = from + egui::vec2(map.width() / 4.0, 0.0);
        a.playback.repeat = Some((0, 480));
        a.playback.follow = true;
        pointer(&mut a, &c, from, egui::PointerButton::Primary, true);
        frame(&mut a, &c, id, vec![egui::Event::PointerMoved(to)]);
        frame(&mut a, &c, id, vec![]);
        let shifted = rect(&c, "minimap-visible");
        assert!(shifted.left() > visible.left() + map.width() * 0.2);
        // Scrolling hides the row label, exposing that much more music.
        let span = visible.width() + label / stride * map.width() / 40.0;
        assert!(
            (shifted.width() - span).abs() < 0.5,
            "{shifted:?} {visible:?} {span}"
        );
        assert!(!a.playback.follow);
        assert_eq!(a.playback.repeat, Some((0, 480)));
        assert_eq!(a.patterns.workspace.song_cell.measure, selected);
        assert!(a.playback.pending.is_none() && a.playback.active.is_none());
        pointer(&mut a, &c, to, egui::PointerButton::Primary, false);
        // After scrolling, choose known measure boundaries from the actual grid.
        let measure = if outside { 30 } else { 12 };
        let first = cell_rect(&c, measure);
        let second = cell_rect(&c, measure + 1);
        let row = rect(&c, "repeat");
        let from = egui::pos2(first.left(), row.center().y);
        let to = egui::pos2(second.left(), row.center().y);
        let viewport = rect(&c, "matrix-viewport");
        assert!(viewport.x_range().contains(from.x) && viewport.x_range().contains(to.x));
        pointer(&mut a, &c, from, egui::PointerButton::Primary, true);
        frame(&mut a, &c, id, vec![egui::Event::PointerMoved(to)]);
        pointer(&mut a, &c, to, egui::PointerButton::Primary, false);
        assert_eq!(
            a.playback.repeat,
            Some(((measure - 1) * 1920, measure * 1920))
        );
        assert_eq!(bytes(&a), before);
    }
}
