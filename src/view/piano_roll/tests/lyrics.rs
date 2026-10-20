use super::*;

#[test]
fn format_one_singer_on_track_zero_opens_the_piano_roll_with_moras() {
    let mut app = crate::view::note_lyrics::tests::singing();
    app.loaded.as_mut().unwrap().music.format = 1;
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.piano_roll.open);
    assert!(!app.tempo_curve.open);
    at(&mut app, &ctx, 10.0, vec![]);
    let output = at(&mut app, &ctx, 10.1, vec![]);
    let lyrics: Vec<_> = output
        .shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match &s.shape {
            Shape::Text(t) if ["きゃ", "り"].contains(&t.galley.text()) => Some((i, t)),
            _ => None,
        })
        .collect();
    assert_eq!(lyrics.len(), 3);
    for (i, text) in lyrics {
        assert!(matches!(&output.shapes[i - 1].shape, Shape::Rect(bg)
            if bg.fill == egui::Color32::WHITE
                && bg.rect.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size()))));
    }
}

#[test]
fn lyric_overlays_follow_note_starts_including_drag_and_hover() {
    let app = crate::view::note_lyrics::tests::singing();
    let layout = super::super::layout(app.loaded.as_ref().unwrap(), 0, 0);
    assert_eq!(
        layout.lyrics,
        [Some("きゃ".into()), Some("きゃ".into()), Some("り".into())]
    );
    let ctx = egui::Context::default();
    let origin = egui::pos2(20.0, 30.0);
    let moved = layout.notes[2].1.translate(egui::vec2(50.0, 0.0));
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            super::super::draw(
                ui.painter(),
                egui::Rect::from_min_size(origin, layout.size),
                &layout,
                Some((2, moved)),
                Some((2, Part::Start)),
            );
        });
    });
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(text) => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 3);
    for (i, text) in texts.iter().enumerate() {
        let rect = if i == 2 { moved } else { layout.notes[i].1 };
        assert!((text.pos.x - (origin.x + rect.left() + 2.0)).abs() < 0.01);
        assert!(
            (text.pos.y + text.galley.size().y / 2.0 - (origin.y + rect.center().y)).abs() < 0.01
        );
    }
}

#[test]
fn r_opens_with_black_moras_on_white_backgrounds_above_note_rectangles() {
    let mut app = crate::view::note_lyrics::tests::singing();
    let (_, output) = opened_in(&mut app);
    let mut count = 0;
    for (i, shape) in output.shapes.iter().enumerate() {
        let Shape::Text(text) = &shape.shape else {
            continue;
        };
        if !["きゃ", "り"].contains(&text.galley.text()) {
            continue;
        }
        count += 1;
        let Shape::Rect(background) = &output.shapes[i - 1].shape else {
            panic!("white background immediately before lyric");
        };
        assert_eq!(background.fill, egui::Color32::WHITE);
        assert!(background
            .rect
            .contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())));
        assert!(shape.clip_rect.intersects(background.rect));
        assert!(text
            .galley
            .job
            .sections
            .iter()
            .all(|section| section.format.color == egui::Color32::BLACK
                && section.format.font_id.size >= 14.0));
        assert!(!output.shapes[i + 1..].iter().any(|later| matches!(&later.shape, Shape::Rect(rect) if rect.stroke.width == 0.5 && rect.rect.intersects(background.rect))));
    }
    assert_eq!(count, 3, "each mora reaches the actual r-key window");
}
