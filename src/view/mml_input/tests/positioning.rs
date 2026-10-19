use super::*;

fn anchors(output: &egui::FullOutput) -> Vec<egui::Pos2> {
    let positions: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if matches!(
                    text.galley.text(),
                    "MML / Chord progression" | "Enter: Confirm" | "Esc: Cancel"
                ) =>
            {
                Some(text.pos)
            }
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 3);
    positions
}

#[test]
fn chord_input_keeps_modal_position_and_height_during_and_after_preparation() {
    let mut app = keyed_fixture(4, false);
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    // Let the initially centered modal settle before starting to type.
    raw_frame(&mut app, &ctx, vec![]);
    let expected = anchors(&raw_frame(&mut app, &ctx, vec![]));
    for chord in ["I", " V", " IV", " I"] {
        let output = raw_frame(&mut app, &ctx, vec![egui::Event::Text(chord.into())]);
        assert!(app.mml_input.pending_preview.is_some());
        assert_eq!(
            anchors(&output),
            expected,
            "preparation changed modal geometry"
        );
        for _ in 0..3 {
            let output = raw_frame(&mut app, &ctx, vec![]);
            assert_eq!(
                anchors(&output),
                expected,
                "completion changed modal geometry"
            );
        }
        assert!(app.mml_input.pending_preview.is_none());
    }
}
