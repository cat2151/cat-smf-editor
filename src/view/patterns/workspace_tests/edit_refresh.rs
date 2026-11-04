//! The arrangement windows draw from the arrangement kept with the music, so an edit that
//! keeps the number of patterns and scenes must still show in the next frame.
use super::*;

fn shapes(a: &mut App, c: &egui::Context, p: Pane) -> Vec<egui::epaint::ClippedShape> {
    frame(a, c, p.viewport(), vec![]);
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
            "workspace-shapes",
            p.viewport(),
        )))
    })
    .unwrap()
}

/// The 3px note bars painted in `p`'s minimap.
fn minimap_notes(a: &mut App, c: &egui::Context, p: Pane) -> usize {
    let shapes = shapes(a, c, p);
    let map = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "minimap"))))
        .unwrap();
    shapes
        .iter()
        .filter(|s| match &s.shape {
            egui::epaint::Shape::Rect(r) => r.rect.height() == 3.0 && map.contains_rect(r.rect),
            _ => false,
        })
        .count()
}

fn labels(a: &mut App, c: &egui::Context) -> Vec<String> {
    shapes(a, c, Pane::Song)
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn removing_a_placement_changes_the_minimap_in_the_next_frame_and_undo_restores_it() {
    let (mut a, c, _, _, scene) = verification_song();
    workspace::open_scene(&mut a, scene).unwrap();
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let scene_lane = collection.scenes[0].lanes[1];
    let song_lane = collection.song[0].lane;
    for (p, id, lane, measure) in [
        (Pane::Song, None, song_lane, 3),
        (Pane::Scene, Some(scene), scene_lane, 2),
    ] {
        let before = minimap_notes(&mut a, &c, p);
        assert!(before > 0, "{p:?}");
        workspace::mutate(&mut a, |m| grid::place(m, id, lane, measure, None)).unwrap();
        assert!(minimap_notes(&mut a, &c, p) < before, "{p:?}");
        a.undo_music().unwrap();
        assert_eq!(minimap_notes(&mut a, &c, p), before, "{p:?}");
    }
}

#[test]
fn replacing_a_scene_changes_the_candidates_in_the_next_frame_and_undo_restores_them() {
    let (mut a, c, _, _, _) = verification_song();
    a.patterns.workspace.song_cell.query.clear();
    let mut spare = 0;
    workspace::mutate(&mut a, |m| {
        spare = grid::create_scene(m, "Spare", 2, 1)?;
        Ok(())
    })
    .unwrap();
    assert!(labels(&mut a, &c).contains(&"Spare · 2 measures".to_string()));
    workspace::mutate(&mut a, |m| {
        grid::delete_scene(m, spare)?;
        grid::create_scene(m, "Other", 3, 1).map(|_| ())
    })
    .unwrap();
    let shown = labels(&mut a, &c);
    assert!(
        shown.contains(&"Other · 3 measures".to_string()),
        "{shown:?}"
    );
    assert!(
        !shown.contains(&"Spare · 2 measures".to_string()),
        "{shown:?}"
    );
    a.undo_music().unwrap();
    let shown = labels(&mut a, &c);
    assert!(
        shown.contains(&"Spare · 2 measures".to_string()),
        "{shown:?}"
    );
    assert!(
        !shown.contains(&"Other · 3 measures".to_string()),
        "{shown:?}"
    );
}
