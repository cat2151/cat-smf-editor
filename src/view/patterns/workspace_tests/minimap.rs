use super::*;
use crate::play::playback::Mode;
#[test]
fn minimap_click_plays_the_song_or_scene_from_that_point() {
    let (mut a, c, _, _, scene) = verification_song();
    workspace::open_scene(&mut a, scene).unwrap();
    for (p, mode, half) in [
        (Pane::Song, Mode::Arrangement(None), 3.0),
        (Pane::Scene, Mode::Arrangement(Some(scene)), 2.0),
    ] {
        button(&mut a, &c, p, "minimap");
        let active = &a.playback.pending.as_ref().expect("playing").active;
        assert_eq!(active.mode, mode);
        assert_eq!(active.start_seconds, half);
        crate::play::transport::stop(&mut a);
    }
}
#[test]
fn scene_notes_are_painted_measure_by_measure() {
    let (a, c, _, _, scene) = verification_song();
    let music = &a.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(music).unwrap();
    let collection = scene::read(music).unwrap();
    let s = collection.scenes.iter().find(|s| s.id == scene).unwrap();
    let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 30.0));
    let notes = |measures| {
        let out = c.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            super::super::cards::paint_scene(
                &painter,
                area,
                music,
                &catalog,
                s,
                measures,
                &|_, _| egui::Color32::WHITE,
            );
        });
        out.shapes
            .iter()
            .map(|s| s.shape.visual_bounding_rect().left())
            .collect::<Vec<_>>()
    };
    // The first pattern fills measure 1, the second measure 2.
    let whole = notes(None);
    assert_eq!(whole.len(), 2);
    assert!(whole[0] < 50.0 && whole[1] >= 50.0, "{whole:?}");
    assert_eq!(notes(Some((1, 2))).len(), 1);
}
#[test]
fn minimap_playhead_follows_the_arrangement_and_disappears_on_stop() {
    let (mut a, c, _, _, _) = verification_song();
    let id = Pane::Song.viewport();
    let playheads = |a: &mut App| {
        frame(a, &c, id, vec![]);
        let map = c
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), "minimap"))))
            .unwrap();
        let shapes = c
            .data(|d| {
                d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                    "workspace-shapes",
                    id,
                )))
            })
            .unwrap();
        (
            map,
            shapes
                .into_iter()
                .filter_map(|s| match s.shape {
                    egui::epaint::Shape::LineSegment { points, stroke }
                        if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                            && points[0].y == map.top() =>
                    {
                        Some(points[0].x)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>(),
        )
    };
    crate::play::arrangement::start(&mut a, None);
    crate::play::transport::advance(&mut a);
    // Half of the six-second song.
    a.audition.set_test_seconds(3.0);
    let (map, x) = playheads(&mut a);
    assert_eq!(x.len(), 1);
    assert!((x[0] - map.center().x).abs() < 0.5, "{x:?} {map:?}");
    crate::play::transport::stop(&mut a);
    assert!(playheads(&mut a).1.is_empty());
}
#[test]
fn minimap_notes_use_the_other_minimaps_colors() {
    let (mut a, c, _, _, _) = verification_song();
    let id = Pane::Song.viewport();
    frame(&mut a, &c, id, vec![]);
    let map = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), "minimap"))))
        .unwrap();
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
        })
        .unwrap();
    let mut fills = shapes
        .into_iter()
        .filter_map(|s| match s.shape {
            egui::epaint::Shape::Rect(r) if map.contains_rect(r.rect) && r.rect != map => {
                Some(r.fill)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    fills.dedup();
    // Scene A plays c then g; the song then plays g again.
    let (c4, g4) = (
        crate::view::timeline_minimap::note_color(60, None),
        crate::view::timeline_minimap::note_color(67, None),
    );
    assert_ne!(c4, g4);
    assert_eq!(fills, vec![c4, g4]);
}
#[test]
fn scene_notes_use_the_minimap_colors_and_pattern_notes_stay_gray() {
    let (mut a, c, _, second, scene) = verification_song();
    let p = Pane::Song;
    frame(&mut a, &c, p.viewport(), vec![]);
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                "workspace-shapes",
                p.viewport(),
            )))
        })
        .unwrap();
    let fills = |id: egui::Id| {
        let rect = c.data(|d| d.get_temp::<egui::Rect>(id)).unwrap();
        let mut out = shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if rect.contains_rect(r.rect) && r.rect != rect => {
                    Some(r.fill)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        out.dedup();
        out
    };
    let minimap = crate::view::timeline_minimap::note_color;
    let gray = crate::view::pitch_keyboard::note_color(67, None);
    // Scene A plays c in its first measure and g in its second.
    let cell = |measure: u64| egui::Id::new((p.title(), "cell", 0usize, measure));
    assert_eq!(fills(cell(1)), vec![minimap(60, None)]);
    assert_eq!(fills(cell(2)), vec![minimap(67, None)]);
    assert_eq!(fills(cell(3)), vec![gray]);
    let part = |part: PatternOrScene| egui::Id::new((p.title(), "part", format!("{part:?}")));
    assert_eq!(
        fills(part(PatternOrScene::Scene(scene))),
        vec![minimap(60, None), minimap(67, None)]
    );
    assert_eq!(fills(part(PatternOrScene::Pattern(second))), vec![gray]);
}
