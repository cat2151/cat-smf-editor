use super::*;

fn lines(output: &egui::FullOutput) -> Vec<([egui::Pos2; 2], egui::Stroke)> {
    let panel_lines: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::LineSegment { points, stroke }
                if points[0].y == points[1].y && stroke.color == STAFF_COLOR =>
            {
                Some((*points, *stroke))
            }
            _ => None,
        })
        .collect();
    assert_eq!(panel_lines.len(), 4);
    panel_lines
}

#[test]
fn staff_stays_gray_and_stationary_during_hover_blinks_and_time_scroll() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let expected = lines(&output);
    assert!(expected.iter().all(|(_, stroke)| {
        stroke.color == egui::Color32::from_gray(90) && stroke.width == 1.0
    }));
    let note = note_rects(&output)[2];
    at(
        &mut app,
        &ctx,
        11.0,
        vec![egui::Event::PointerMoved(note.center())],
    );
    for time in [11.0, 11.3, 11.5, 11.8] {
        assert_eq!(lines(&at(&mut app, &ctx, time, vec![])), expected);
    }
    at(&mut app, &ctx, 12.0, vec![egui::Event::PointerGone]);
    // At zoom 1, measure 5 in 4/4 starts at 1536 points. Exercise both
    // sides of it, including fractional scroll positions.
    for (i, offset) in [0.2, 500.0, 1400.3, 1535.8, 1536.2, 1600.7, 1700.0]
        .into_iter()
        .enumerate()
    {
        app.piano_roll.viewport.pending = Some(egui::vec2(offset, 0.0));
        let output = at(&mut app, &ctx, 13.0 + i as f64, vec![]);
        // The entire visible line must stay fixed, including its endpoints.
        // Sending long, moving offscreen segments to the GPU needlessly
        // changes the triangles covering these stationary background lines.
        assert_eq!(lines(&output), expected, "scroll {offset}");
    }
}
