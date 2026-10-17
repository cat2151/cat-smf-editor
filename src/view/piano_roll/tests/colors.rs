use super::*;

#[test]
fn note_colors_follow_each_notes_key_signature_and_update_after_key_edits() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let fills = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                Shape::Rect(r) if r.stroke.width == 0.5 => Some(r.fill),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let before = fills(&output);
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(
        before,
        NOTES
            .iter()
            .map(|n| crate::view::pitch_keyboard::roll_color(n.2, None))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        before.len(),
        loaded.note_views[0].durations.iter().flatten().count()
    );
    let mut key = event(240, 0xff, vec![1, 0]);
    key.meta_type = 0x59;
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks[0].events.push(key);
    loaded.music.tracks[0].events.sort_by_key(|e| e.tick);
    cat_smf_editor::edit::refresh_deltas(&mut loaded.music.tracks[0]).unwrap();
    loaded.refresh();
    let output = at(&mut app, &ctx, 11.0, vec![]);
    let after = fills(&output);
    let loaded = app.loaded.as_ref().unwrap();
    let expected = NOTES
        .iter()
        .map(|n| {
            crate::view::pitch_keyboard::roll_color(
                n.2,
                loaded.keys.map(0).and_then(|keys| keys.at(n.0)),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(after, expected);
    assert_eq!(after[0], before[0]);
    assert_ne!(after[2], before[2]);
}

#[test]
fn hover_keeps_a_visible_outline_when_the_part_color_equals_the_notes_color() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let note = all_notes(&output)[1];
    at(
        &mut app,
        &ctx,
        11.0,
        vec![egui::Event::PointerMoved(note.center())],
    );
    let output = at(&mut app, &ctx, 11.0, vec![]);
    assert_eq!(
        rects_in(&output, Part::Middle.color()),
        vec![part_rect(note, Part::Middle)]
    );
}
