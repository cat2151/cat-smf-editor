use super::variation_tests::key;
use super::*;
use egui::Key;

fn setup() -> App {
    let mut app = crate::view::tests::app(tracks::new_song());
    let mut s = session(&app);
    for i in 1..8 {
        let mut curve = s.rows[0].curve.clone();
        curve.target.parameter_id = format!("macro-{i}");
        s.rows.push(Row {
            curve,
            value: Arc::new(AtomicU64::new(0.25f64.to_bits())),
            bipolar: false,
            readback: None,
        });
    }
    app.macro_editor.session = Some(s);
    app
}
fn question(app: &mut App, ctx: &egui::Context) {
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Text("?".into())],
            ..Default::default()
        },
        |ctx| {
            crate::view::commands::route(app, ctx);
            show(app, ctx);
        },
    );
}

#[test]
fn counted_hjkl_steps_boundaries_and_variation_switching() {
    let mut app = setup();
    let ctx = egui::Context::default();
    let cursor = app.cursor_row;
    for k in [Key::Num3, Key::J, Key::L] {
        key(&mut app, &ctx, k);
    }
    let s = app.macro_editor.session.as_ref().unwrap();
    assert_eq!(s.selected, 3);
    assert_eq!(s.rows[3].get(), 0.375);
    for k in [Key::OpenBracket, Key::Num2, Key::H] {
        key(&mut app, &ctx, k);
    }
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[3].get(),
        0.25
    );
    for k in [
        Key::CloseBracket,
        Key::Num2,
        Key::K,
        Key::D,
        Key::L,
        Key::PageUp,
    ] {
        key(&mut app, &ctx, k);
    }
    let s = app.macro_editor.session.as_ref().unwrap();
    assert_eq!(s.selected, 1);
    assert_eq!(s.variation, 0);
    assert_eq!(s.rows[1].get(), 0.25);
    key(&mut app, &ctx, Key::PageDown);
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[1].get(),
        0.375
    );
    for k in [Key::Num9, Key::Num9, Key::J, Key::Num9, Key::Num9, Key::L] {
        key(&mut app, &ctx, k);
    }
    let s = app.macro_editor.session.as_ref().unwrap();
    assert_eq!(s.selected, 7);
    assert_eq!(s.rows[7].get(), 1.0);
    key(&mut app, &ctx, Key::H);
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[7].get(),
        0.875
    );
    for k in [Key::Num9, Key::Num9, Key::K, Key::Num9, Key::Num9, Key::H] {
        key(&mut app, &ctx, k);
    }
    let s = app.macro_editor.session.as_ref().unwrap();
    assert_eq!(s.selected, 0);
    assert_eq!(s.rows[0].get(), 0.0);
    assert_eq!(app.cursor_row, cursor);
}

#[test]
fn count_matches_repeated_keys_and_saturates_without_looping() {
    let mut counted = setup();
    let mut repeated = setup();
    let a = egui::Context::default();
    let b = egui::Context::default();
    for _ in 0..7 {
        key(&mut counted, &a, Key::OpenBracket);
        key(&mut repeated, &b, Key::OpenBracket);
    }
    for k in [Key::Num9, Key::Num9, Key::L] {
        key(&mut counted, &a, k);
    }
    for _ in 0..99 {
        key(&mut repeated, &b, Key::L);
    }
    assert_eq!(
        counted.macro_editor.session.as_ref().unwrap().rows[0].get(),
        repeated.macro_editor.session.as_ref().unwrap().rows[0].get()
    );
    for _ in 0..30 {
        key(&mut counted, &a, Key::Num9);
    }
    key(&mut counted, &a, Key::J);
    assert_eq!(counted.macro_editor.session.as_ref().unwrap().selected, 7);
    key(&mut counted, &a, Key::Num2);
    key(&mut counted, &a, Key::Escape);
    assert!(counted.macro_editor.open());
    key(&mut counted, &a, Key::K);
    assert_eq!(counted.macro_editor.session.as_ref().unwrap().selected, 6);
}

#[test]
fn question_overlay_owns_input_and_clears_count() {
    let mut app = setup();
    let ctx = egui::Context::default();
    key(&mut app, &ctx, Key::Num9);
    question(&mut app, &ctx);
    assert!(app.macro_editor.help);
    for k in [
        Key::J,
        Key::L,
        Key::Num3,
        Key::D,
        Key::PageDown,
        Key::OpenBracket,
        Key::Enter,
    ] {
        key(&mut app, &ctx, k);
    }
    let s = app.macro_editor.session.as_ref().unwrap();
    assert_eq!(s.selected, 0);
    assert_eq!(s.rows[0].get(), 0.25);
    assert_eq!(s.keyboard.step, 0.125);
    assert_eq!(s.variation, 0);
    key(&mut app, &ctx, Key::Escape);
    assert!(!app.macro_editor.help);
    assert!(app.macro_editor.open());
    key(&mut app, &ctx, Key::L);
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[0].get(),
        0.375
    );
    question(&mut app, &ctx);
    question(&mut app, &ctx);
    assert!(!app.macro_editor.help);
    key(&mut app, &ctx, Key::Enter);
    assert!(!app.macro_editor.open());
}

#[test]
fn step_toast_shows_latest_value_for_500ms_after_last_change() {
    let mut app = setup();
    let ctx = egui::Context::default();
    let mut frame = |time, key: Option<Key>| {
        ctx.run(
            egui::RawInput {
                time: Some(time),
                events: key
                    .into_iter()
                    .map(|key| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                    .collect(),
                ..Default::default()
            },
            |ctx| {
                crate::view::commands::route(&mut app, ctx);
                show(&mut app, ctx);
            },
        )
    };
    let contains = |output: &egui::FullOutput, label: &str| {
        output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == label))
    };
    frame(0.0, None::<Key>);
    assert!(!contains(&frame(0.1, None), "変化量 0.125"));
    frame(1.0, Some(Key::OpenBracket));
    let visible = frame(1.001, None);
    let bar = visible
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if (r.rect.width() - 280.0).abs() < 0.1 && (r.rect.height() - 26.0).abs() < 0.1 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .unwrap();
    let toast = visible
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == "変化量 0.0625" => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(toast.pos.x > bar.left() + bar.width() * 0.25);
    assert!(toast.pos.y + toast.galley.size().y < bar.top());
    assert!(contains(&frame(1.499, None), "変化量 0.0625"));
    frame(1.5, Some(Key::CloseBracket));
    let updated = frame(1.501, None);
    assert!(contains(&updated, "変化量 0.125"));
    assert!(!contains(&updated, "変化量 0.0625"));
    assert!(contains(&frame(1.999, None), "変化量 0.125"));
    assert!(!contains(&frame(2.0, None), "変化量 0.125"));
    assert!(app
        .macro_editor
        .session
        .as_ref()
        .unwrap()
        .keyboard
        .step_toast_until
        .is_none());
}
