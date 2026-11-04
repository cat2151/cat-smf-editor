use super::*;

fn cell(c: &egui::Context, p: Pane, measure: u64) -> egui::Rect {
    c.data(|d| d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "cell", 0usize, measure))))
        .unwrap()
}

fn offset(c: &egui::Context) -> f32 {
    let id = c
        .data(|d| d.get_temp(egui::Id::new((Pane::Song.title(), "matrix-scroll"))))
        .unwrap();
    egui::scroll_area::State::load(c, id).unwrap().offset.x
}

/// The playhead lines across the first row of cells; the minimap line sits above them.
fn lines(a: &mut App, c: &egui::Context, p: Pane) -> Vec<f32> {
    let id = p.viewport();
    frame(a, c, id, vec![]);
    let y = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "matrix-viewport"))))
        .unwrap()
        .center()
        .y;
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap()
    .into_iter()
    .filter_map(|s| match s.shape {
        egui::epaint::Shape::LineSegment { points, stroke }
            if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                && points[0].y <= y
                && points[1].y >= y =>
        {
            Some(points[0].x)
        }
        _ => None,
    })
    .collect()
}

/// Reveals measure one by moving the cell cursor there, and returns the view's offset.
fn scroll_home(a: &mut App, c: &egui::Context) -> f32 {
    let song = Pane::Song.viewport();
    a.patterns.workspace.song_cell.placing = true;
    for measure in [2, 1] {
        a.patterns.workspace.song_cell.measure = measure;
        for _ in 0..3 {
            frame(a, c, song, vec![]);
        }
    }
    offset(c)
}

/// A Song wider than the view, shown from measure one and just started.
fn zoomed_song_playing() -> (App, egui::Context, f32) {
    let (mut a, c, _, _, _) = verification_song();
    c.style_mut(|style| style.spacing.scroll.floating = false);
    let song = Pane::Song.viewport();
    frame(&mut a, &c, song, vec![]);
    for _ in 0..12 {
        key(&mut a, &c, song, Key::PageUp, Modifiers::NONE);
    }
    let home = scroll_home(&mut a, &c);
    crate::play::arrangement::start(&mut a, None);
    crate::play::transport::advance(&mut a);
    a.audition.set_test_seconds(0.1);
    assert_eq!(lines(&mut a, &c, Pane::Song).len(), 1);
    (a, c, home)
}

#[test]
fn song_cells_show_the_playing_position_until_stop() {
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    frame(&mut a, &c, song, vec![]);
    assert!(lines(&mut a, &c, Pane::Song).is_empty());
    crate::play::arrangement::start(&mut a, None);
    crate::play::transport::advance(&mut a);
    // Half of the six-second song: the middle of its second measure.
    a.audition.set_test_seconds(3.0);
    frame(&mut a, &c, song, vec![]);
    let x = lines(&mut a, &c, Pane::Song);
    assert_eq!(x.len(), 1);
    let measure = cell(&c, Pane::Song, 2);
    assert!((x[0] - measure.center().x).abs() < 0.5, "{x:?} {measure:?}");
    crate::play::transport::stop(&mut a);
    assert!(lines(&mut a, &c, Pane::Song).is_empty());
}

#[test]
fn the_view_turns_a_page_when_the_playhead_runs_off_it() {
    let (mut a, c, home) = zoomed_song_playing();
    let song = Pane::Song.viewport();
    a.audition.set_test_seconds(5.0);
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    assert!(offset(&c) > home);
    let x = lines(&mut a, &c, Pane::Song);
    let area = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), "matrix-viewport"))))
        .unwrap();
    assert_eq!(x.len(), 1);
    assert!(area.x_range().contains(x[0]), "{x:?} {area:?}");
}

#[test]
fn a_view_scrolled_away_from_the_playhead_stays_there() {
    let (mut a, c, home) = zoomed_song_playing();
    let song = Pane::Song.viewport();
    a.audition.set_test_seconds(5.0);
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    assert!(offset(&c) > home);
    let home = scroll_home(&mut a, &c);
    a.audition.set_test_seconds(5.5);
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    assert_eq!(offset(&c), home);
}

#[test]
fn each_pane_shows_only_its_own_arrangement() {
    let (mut a, c, _, _, scene) = verification_song();
    workspace::open_scene(&mut a, scene).unwrap();
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    frame(&mut a, &c, Pane::Scene.viewport(), vec![]);
    crate::play::arrangement::start(&mut a, Some(scene));
    crate::play::transport::advance(&mut a);
    // Half of the scene's two-second first measure.
    a.audition.set_test_seconds(1.0);
    let x = lines(&mut a, &c, Pane::Scene);
    assert_eq!(x.len(), 1);
    let measure = cell(&c, Pane::Scene, 1);
    assert!((x[0] - measure.center().x).abs() < 0.5, "{x:?} {measure:?}");
    assert!(lines(&mut a, &c, Pane::Song).is_empty());
    crate::play::arrangement::start(&mut a, None);
    crate::play::transport::advance(&mut a);
    a.audition.set_test_seconds(1.0);
    assert_eq!(lines(&mut a, &c, Pane::Song).len(), 1);
    assert!(lines(&mut a, &c, Pane::Scene).is_empty());
}
