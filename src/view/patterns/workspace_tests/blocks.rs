use super::*;
fn cell_rect(c: &egui::Context, row: usize, measure: u64) -> egui::Rect {
    c.data(|d| d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), "cell", row, measure))))
        .unwrap()
}
/// Stroke widths of the frames drawn exactly around `rect`.
fn frames(c: &egui::Context, rect: egui::Rect) -> Vec<f32> {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
            "workspace-shapes",
            Pane::Song.viewport(),
        )))
    })
    .unwrap()
    .iter()
    .filter_map(|s| match &s.shape {
        egui::Shape::Rect(r) if r.stroke.width > 0.0 && r.rect == rect => Some(r.stroke.width),
        _ => None,
    })
    .collect()
}
#[test]
fn one_placement_is_one_frame_and_repeats_stay_apart() {
    let (mut a, c, _, second, _) = verification_song();
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    // Song row 0: the two-measure scene at 1-2, then `second` at 3 and again at 4.
    grid::place(
        &mut a.loaded.as_mut().unwrap().music,
        None,
        lane,
        4,
        Some(PatternOrScene::Pattern(second)),
    )
    .unwrap();
    cell(&mut a, &c, Pane::Song, lane, 6);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    let long = cell_rect(&c, 0, 1).union(cell_rect(&c, 0, 2));
    assert_eq!(frames(&c, long), vec![1.0]);
    // Measures abut: no blank between bars that would read as a rest.
    assert_eq!(cell_rect(&c, 0, 2).left(), cell_rect(&c, 0, 1).right());
    assert!(frames(&c, cell_rect(&c, 0, 1)).is_empty());
    assert!(frames(&c, cell_rect(&c, 0, 2)).is_empty());
    assert_eq!(frames(&c, cell_rect(&c, 0, 3)), vec![1.0]);
    assert_eq!(frames(&c, cell_rect(&c, 0, 4)), vec![1.0]);
    // The cursor inside the long placement selects its whole frame.
    cell(&mut a, &c, Pane::Song, lane, 2);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert_eq!(frames(&c, long), vec![2.0]);
    assert_eq!(frames(&c, cell_rect(&c, 0, 3)), vec![1.0]);
}
