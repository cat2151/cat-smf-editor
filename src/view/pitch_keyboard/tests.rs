use crate::view::tests::app;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

#[test]
fn key_fills_follow_conductor_changes_and_selection_preserves_pitch_color() {
    use cat_smf_editor::score::key_signature::KeySignature;
    let signature = |tick, sharps| Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x59,
        data: vec![sharps, 1],
    };
    let d_minor = KeySignature::from_event(&signature(0, 255)).unwrap();
    let a_minor = KeySignature::from_event(&signature(10, 0)).unwrap();
    let color = |pitch, key| crate::view::pitch_keyboard::note_color(pitch, Some(key));
    assert_eq!(color(62, d_minor), color(74, d_minor));
    // Monokai: tonic cyan, tritone pink; minor turns the sharp side toward purple.
    assert_eq!(
        color(62, d_minor),
        egui::Color32::from_rgb(0x66, 0xd9, 0xef)
    );
    assert_eq!(
        color(59, d_minor),
        egui::Color32::from_rgb(0xae, 0x81, 0xff)
    );
    assert_ne!(color(57, d_minor), color(67, d_minor));
    assert_eq!(
        color(68, d_minor),
        egui::Color32::from_rgb(0xf9, 0x26, 0x72)
    );
    let events = [(0, 62), (0, 57), (0, 67), (0, 68), (10, 62)]
        .into_iter()
        .map(|(tick, pitch)| Event {
            tick,
            delta: 0,
            status: 0x90,
            meta_type: 0,
            data: vec![pitch, 100],
        })
        .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![signature(0, 255), signature(10, 0)],
            },
            Track { events },
        ],
    });
    app.select_track(1);
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(2000.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let render = |app: &mut crate::app::App| ctx.run(input(), |ctx| crate::view::show(app, ctx));
    let before = render(&mut app);
    app.selected_event = Some(0);
    let after = render(&mut app);
    let lit = |pitch, key| color(pitch, key).gamma_multiply(crate::view::pitch_keyboard::LIT);
    let palette = [
        lit(62, d_minor),
        lit(57, d_minor),
        lit(67, d_minor),
        lit(68, d_minor),
        lit(62, a_minor),
    ];
    let fills = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(key) if palette.contains(&key.fill) => {
                    Some((key.rect, key.fill))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(fills(&before), fills(&after));
    let painted = fills(&after);
    assert_eq!(painted.len(), 5);
    assert_eq!(
        painted
            .iter()
            .filter(|(rect, _)| rect.top() == painted[0].0.top())
            .count(),
        4
    );
    let tonic = painted
        .iter()
        .find(|(_, fill)| *fill == lit(62, d_minor))
        .unwrap();
    let changed = painted
        .iter()
        .rev()
        .find(|(_, fill)| *fill == lit(62, a_minor))
        .unwrap();
    assert!(changed.0.top() > tonic.0.top());
    assert!(after.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::Rect(key) if key.stroke.color == egui::Color32::from_rgb(0x27, 0x28, 0x22) && key.stroke.width == 2.0)));
}

#[test]
fn simultaneous_chord_is_drawn_once_on_trailing_cc_row_with_selection() {
    let events = [
        (0, 0x90, 60, 100),
        (0, 0x91, 60, 90),
        (0, 0x90, 64, 80),
        (0, 0xb0, 64, 127),
        (10, 0x90, 67, 70),
    ]
    .into_iter()
    .map(|(tick, status, pitch, velocity)| Event {
        delta: 0,
        tick,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    })
    .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    app.selected_event = Some(0);
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(2000.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let output = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let gray = egui::Color32::from_gray(150).gamma_multiply(crate::view::pitch_keyboard::LIT);
    let keys: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Rect(key) if key.fill == gray => Some(key),
            _ => None,
        })
        .collect();
    assert_eq!(
        keys.len(),
        3,
        "one filled key per pitch, with no repeated keyboard"
    );
    assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::epaint::Shape::Rect(key) if key.stroke.color == egui::Color32::from_rgb(0x27, 0x28, 0x22) && key.stroke.width == 2.0)));
    // Only the score in the right column draws circles, for its clefs.
    assert!(output
        .shapes
        .iter()
        .all(|s| !matches!(&s.shape, egui::epaint::Shape::Circle(c) if c.center.x < 1000.0)));
    assert!((keys[0].rect.center().y - keys[1].rect.center().y).abs() < 0.1);
    assert!(keys[2].rect.center().y > keys[1].rect.center().y);
    let cc = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "Sustain on" => Some(text),
            _ => None,
        })
        .unwrap();
    assert!((keys[0].rect.center().y - (cc.pos.y + cc.galley.size().y * 0.5)).abs() < 1.0);
    let keyboard_rows: std::collections::BTreeSet<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Rect(key) if key_fill(key.fill) => {
                Some(key.rect.top().round() as i32)
            }
            _ => None,
        })
        .collect();
    assert_eq!(keyboard_rows.len(), 2);
}

#[test]
fn keyboard_columns_stay_aligned_with_varying_note_duration_and_velocity_widths() {
    let events = [
        (0, 0x90, 60, 1),
        (1, 0x80, 60, 0),
        (9, 0x90, 61, 127),
        (1009, 0x80, 61, 0),
        (1010, 0x90, 60, 99),
        (1109, 0x80, 60, 0),
    ]
    .into_iter()
    .map(|(tick, status, pitch, velocity)| Event {
        delta: 0,
        tick,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    })
    .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(2000.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let output = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let mut keyboard_extents = std::collections::BTreeMap::<i32, (f32, f32)>::new();
    for shape in &output.shapes {
        if let egui::epaint::Shape::Rect(key) = &shape.shape
            && key_fill(key.fill)
        {
            let row = keyboard_extents
                .entry(key.rect.top().round() as i32)
                .or_insert((key.rect.left(), key.rect.right()));
            row.0 = row.0.min(key.rect.left());
            row.1 = row.1.max(key.rect.right());
        }
    }
    assert_eq!(keyboard_extents.len(), 3);
    let first = keyboard_extents.values().next().unwrap();
    for extent in keyboard_extents.values() {
        assert!(
            (extent.0 - first.0).abs() < 0.1,
            "keyboard left edge shifted: {keyboard_extents:?}"
        );
        assert!(
            (extent.1 - first.1).abs() < 0.1,
            "keyboard right edge shifted: {keyboard_extents:?}"
        );
    }
}

/// Unlit keys of the backdrop keyboard (white, black) and the lit gray of no key signature.
fn key_fill(fill: egui::Color32) -> bool {
    fill == egui::Color32::from_white_alpha(5)
        || fill == egui::Color32::BLACK
        || fill == egui::Color32::from_gray(150).gamma_multiply(crate::view::pitch_keyboard::LIT)
}

#[test]
fn keyboard_is_drawn_behind_the_note_cells_with_solid_black_keys() {
    let events = [(0, 0x90, 60, 100), (0, 0x90, 64, 80), (10, 0x80, 60, 0)]
        .into_iter()
        .map(|(tick, status, pitch, velocity)| Event {
            delta: 0,
            tick,
            status,
            meta_type: 0,
            data: vec![pitch, velocity],
        })
        .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(2000.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let output = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let lit = egui::Color32::from_gray(150).gamma_multiply(crate::view::pitch_keyboard::LIT);
    let keys = |fill| -> Vec<_> {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(key) if key.fill == fill => Some(key.rect),
                _ => None,
            })
            .collect()
    };
    let behind = keys(lit);
    assert_eq!(behind.len(), 2);
    // C#4 / D#4 between the lit C4 and E4.
    assert!(!keys(egui::Color32::BLACK).is_empty());
    let (label, mbt) = output
        .shapes
        .iter()
        .enumerate()
        .find_map(|(index, s)| match &s.shape {
            egui::epaint::Shape::Text(t)
                if t.galley.text().starts_with("1:1:")
                    && behind[0].y_range().contains(t.pos.y + 1.0) =>
            {
                Some((index, t.pos.x))
            }
            _ => None,
        })
        .unwrap();
    // Starts at the row's left edge under the cells, painted before their text.
    assert!(behind
        .iter()
        .all(|k| k.left() >= mbt - 20.0 && k.left() < mbt + 500.0));
    let first_key = output
        .shapes
        .iter()
        .position(|s| matches!(&s.shape, egui::epaint::Shape::Rect(k) if k.fill == lit))
        .unwrap();
    assert!(first_key < label, "keys under text");
}

#[test]
fn only_the_playing_row_lights_keys_in_full_color() {
    let fills = |playing| {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::view::pitch_keyboard::backdrop(
                    ui,
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 20.0)),
                    &[(60, false)],
                    (60, 72),
                    None,
                    playing,
                );
            });
        });
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(key) => Some(key.fill),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let gray = egui::Color32::from_gray(150);
    assert!(fills(true).contains(&gray));
    assert!(!fills(false).contains(&gray));
    assert!(fills(false).contains(&gray.gamma_multiply(crate::view::pitch_keyboard::LIT)));
}
