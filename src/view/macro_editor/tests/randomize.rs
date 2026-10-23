use super::variation_tests::key;
use super::*;

#[test]
fn randomize_all_current_macros_live_preserves_other_variations_and_confirms_or_cancels() {
    for confirm in [false, true] {
        let mut app = crate::view::tests::app(tracks::new_song());
        let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
        let mut s = session(&app);
        let mut curve = s.rows[0].curve.clone();
        curve.target.parameter_id = "macro-2".into();
        curve.name = "Macro 2".into();
        s.rows.push(Row {
            curve,
            value: Arc::new(AtomicU64::new(0.25f64.to_bits())),
            bipolar: true,
            readback: None,
        });
        let bindings: Vec<_> = s.rows.iter().map(|r| r.value.clone()).collect();
        s.duplicate_variation();
        let untouched = s.variations[0].clone();
        app.macro_editor.session = Some(s);
        let ctx = egui::Context::default();
        let mut rng = fastrand::Rng::with_seed(42);
        fastrand::seed(42);
        let mut expected = vec![];
        for _ in 0..2 {
            expected = (0..2).map(|_| rng.f64()).collect();
            random_key(&mut app, &ctx, true);
            let s = app.macro_editor.session.as_ref().unwrap();
            for ((row, binding), expected) in s.rows.iter().zip(&bindings).zip(&expected) {
                assert_eq!(row.get(), *expected);
                assert_eq!(f64::from_bits(binding.load(Ordering::Relaxed)), *expected);
                assert!((0.0..1.0).contains(expected));
            }
            assert_eq!(s.rows[0].signal(), expected[0]);
            assert_eq!(s.rows[1].signal(), expected[1] * 2.0 - 1.0);
            assert_eq!(s.variations[0], untouched);
            assert_eq!(s.variation, 1);
        }
        assert_eq!(
            before,
            cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
        );
        assert!(!app.undo.available());
        // Help owns input and leaves the draft intact.
        key(&mut app, &ctx, egui::Key::F1);
        key(&mut app, &ctx, egui::Key::R);
        let s = app.macro_editor.session.as_ref().unwrap();
        assert_eq!(s.rows.iter().map(Row::get).collect::<Vec<_>>(), expected);
        key(&mut app, &ctx, egui::Key::Escape);
        key(&mut app, &ctx, egui::Key::PageUp);
        assert_eq!(
            app.macro_editor.session.as_ref().unwrap().rows[0].get(),
            0.25
        );
        key(&mut app, &ctx, egui::Key::PageDown);
        assert_eq!(
            app.macro_editor.session.as_ref().unwrap().rows[0].get(),
            expected[0]
        );
        key(
            &mut app,
            &ctx,
            if confirm {
                egui::Key::Enter
            } else {
                egui::Key::Escape
            },
        );
        assert!(!app.macro_editor.open());
        if confirm {
            let bytes =
                cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
            let restored = cat_smf_editor::ffi::parse(&bytes).unwrap();
            let saved = macro_values::read(&restored.tracks[0]).unwrap();
            assert_eq!(saved.variations[0].snapshots[0], untouched);
            assert_eq!(
                saved
                    .parameters
                    .iter()
                    .map(|c| c.points[0].value)
                    .collect::<Vec<_>>(),
                expected
            );
            app.undo_music().unwrap();
        }
        assert_eq!(
            before,
            cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
        );
        assert!(!app.undo.available());
    }
}

fn random_key(app: &mut App, ctx: &egui::Context, full: bool) {
    let modifiers = if full {
        egui::Modifiers::SHIFT
    } else {
        egui::Modifiers::NONE
    };
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                modifiers,
                events: vec![egui::Event::Key {
                    key: egui::Key::R,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            },
            |ctx| {
                crate::view::commands::route(app, ctx);
                show(app, ctx);
            },
        );
    }
}

#[test]
fn random_reads_all_variations_per_macro_and_writes_only_current_variation() {
    let mut app = crate::view::tests::app(tracks::new_song());
    let mut s = session(&app);
    for index in 1..3 {
        let mut curve = s.rows[0].curve.clone();
        curve.target.parameter_id = format!("macro-{index}");
        s.rows.push(Row {
            curve,
            value: Arc::new(AtomicU64::new(0.4_f64.to_bits())),
            bipolar: index == 1,
            readback: None,
        });
    }
    // The extrema come from different variations for each macro; the middle
    // variation is the destination. The last macro has a zero-width range.
    let positions = [[0.1, 0.8, 0.4], [0.3, 0.4, 0.4], [0.7, 0.2, 0.4]];
    s.variations = positions
        .iter()
        .map(|positions| {
            for (row, value) in s.rows.iter().zip(positions) {
                row.set(*value);
            }
            s.rows.iter().map(Row::saved).collect()
        })
        .collect();
    s.variation = 1;
    for (row, value) in s.rows.iter().zip(positions[1]) {
        row.set(value);
    }
    let untouched = [s.variations[0].clone(), s.variations[2].clone()];
    let bindings: Vec<_> = s.rows.iter().map(|r| r.value.clone()).collect();
    app.macro_editor.session = Some(s);
    let ctx = egui::Context::default();
    let mut rng = fastrand::Rng::with_seed(42);
    fastrand::seed(42);
    let expected = [0.1 + 0.6 * rng.f64(), 0.2 + 0.6 * rng.f64(), {
        rng.f64();
        0.4
    }];
    random_key(&mut app, &ctx, false);
    let s = app.macro_editor.session.as_ref().unwrap();
    assert_eq!(s.variation, 1);
    for ((row, binding), expected) in s.rows.iter().zip(&bindings).zip(expected) {
        assert!((row.get() - expected).abs() < 1e-12);
        assert!((f64::from_bits(binding.load(Ordering::Relaxed)) - expected).abs() < 1e-12);
    }
    let saved = s.saved_variations();
    assert_eq!(saved.snapshots[0], untouched[0]);
    assert_eq!(saved.snapshots[2], untouched[1]);
    assert!((saved.snapshots[1][1].points[0].value - expected[1]).abs() < 1e-12);
    assert!((s.rows[1].signal() - (expected[1] * 2.0 - 1.0)).abs() < 1e-12);
    key(&mut app, &ctx, egui::Key::Enter);
    assert!(!app.macro_editor.open(), "{:?}", app.macro_editor.error);
    let saved = macro_values::read(&app.loaded.as_ref().unwrap().music.tracks[0]).unwrap();
    assert_eq!(saved.variations[0].snapshots[0], untouched[0]);
    assert_eq!(saved.variations[0].snapshots[2], untouched[1]);
}

#[test]
fn single_variation_random_keeps_values_and_full_range_can_change_them() {
    let mut app = crate::view::tests::app(tracks::new_song());
    let mut s = session(&app);
    s.rows[0].set(0.73);
    s.variations = vec![s.rows.iter().map(Row::saved).collect()];
    app.macro_editor.session = Some(s);
    let ctx = egui::Context::default();
    random_key(&mut app, &ctx, false);
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[0].get(),
        0.73
    );
    random_key(&mut app, &ctx, true);
    assert_ne!(
        app.macro_editor.session.as_ref().unwrap().rows[0].get(),
        0.73
    );
}
