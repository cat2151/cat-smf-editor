use super::*;
use cat_smf_editor::model::{Music, Track};

fn fixture() -> App {
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }; 2],
    });
    app.select_track(1);
    open(&mut app);
    app
}

fn press(app: &mut App, ctx: &egui::Context, key: egui::Key) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            events: vec![
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
}

#[test]
fn shortcuts_change_selected_layer_allow_multiple_solos_mutes_and_undo() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    for key in [
        egui::Key::A,
        egui::Key::O,
        egui::Key::CloseBracket,
        egui::Key::S,
        egui::Key::M,
        egui::Key::K,
        egui::Key::S,
        egui::Key::M,
    ] {
        press(&mut app, &ctx, key);
    }
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    assert!(patch
        .layers
        .iter()
        .all(|l| l.settings.solo && l.settings.mute));
    assert_eq!(patch.layers[0].settings.octave, 0);
    assert_eq!(patch.layers[1].settings.octave, 1);
    assert_eq!(patch.layers[1].settings.volume_db, 6.0);
    let saved = cat_smf_editor::instrument_layers::read(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    assert_eq!(saved, patch.saved().unwrap());
    app.undo_music().unwrap();
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    assert!(!patch.layers[0].settings.mute);
    assert!(patch.layers[0].settings.solo);
    for key in [
        egui::Key::S,
        egui::Key::J,
        egui::Key::S,
        egui::Key::I,
        egui::Key::OpenBracket,
    ] {
        press(&mut app, &ctx, key);
    }
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    assert!(patch.layers.iter().all(|l| !l.settings.solo));
    assert_eq!(patch.layers[1].settings.octave, 0);
    assert_eq!(patch.layers[1].settings.volume_db, 0.0);
    assert_eq!(app.loaded.as_ref().unwrap().patches[0], Patch::default());
    assert!(!app.playback.mixer_open);
}

#[test]
fn numeric_input_blocks_shortcuts_even_with_help() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    for help in [true, false] {
        app.help_open = help;
        {
            app.layer_editor.field = 1;
            app.layer_editor
                .begin(&app.loaded.as_ref().unwrap().patches[1]);
        }
        for key in [
            egui::Key::O,
            egui::Key::I,
            egui::Key::OpenBracket,
            egui::Key::CloseBracket,
            egui::Key::M,
            egui::Key::S,
        ] {
            press(&mut app, &ctx, key);
        }
        assert_eq!(app.loaded.as_ref().unwrap().patches[1], Patch::default());
    }
}

#[test]
fn overlay_grows_beyond_old_height_and_reveals_last_layer_on_short_screen() {
    let mut app = fixture();
    let patch = &mut app.loaded.as_mut().unwrap().patches[1];
    for _ in 1..24 {
        add(patch).unwrap();
    }
    for (i, layer) in patch.layers.iter_mut().enumerate() {
        layer.settings.octave = 100 + i as i32;
    }
    let ctx = egui::Context::default();
    let mut draw = |height: f32| {
        let mut output = None;
        for _ in 0..8 {
            output = Some(ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, height),
                    )),
                    ..Default::default()
                },
                |ctx| show(&mut app, ctx),
            ));
        }
        output.unwrap()
    };
    let output = draw(900.0);
    let rows: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley
                    .text()
                    .parse::<u32>()
                    .is_ok_and(|n| (100..124).contains(&n)) =>
            {
                Some(
                    s.clip_rect
                        .contains_rect(egui::Rect::from_min_size(t.pos, t.galley.size())),
                )
            }
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), 24);
    assert!(
        rows.iter().all(|visible| *visible),
        "all rows must fit when screen space is available: {rows:?}"
    );
    draw(350.0);
    let mut output = None;
    for step in 0..55 {
        let events = if step < 23 {
            [true, false]
                .map(|pressed| egui::Event::Key {
                    key: egui::Key::J,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .to_vec()
        } else {
            vec![]
        };
        output = Some(ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 350.0),
                )),
                time: Some(10.0 + step as f64 * 0.1),
                events,
                ..Default::default()
            },
            |ctx| show(&mut app, ctx),
        ));
    }
    assert!(
        output.unwrap().shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Text(t) if t.galley.text() == "123" &&
            s.clip_rect.contains_rect(egui::Rect::from_min_size(t.pos, t.galley.size())))),
        "selected final layer must be visible even on a short screen"
    );
    assert_eq!(
        app.layer_editor
            .row(&app.loaded.as_ref().unwrap().patches[1]),
        23
    );
}

#[test]
fn wheel_over_numeric_cell_steps_like_keys_and_selects_it() {
    let mut app = fixture();
    let patch = &mut app.loaded.as_mut().unwrap().patches[1];
    add(patch).unwrap();
    patch.layers[0].settings.volume_db = -7.0;
    patch.layers[1].settings.octave = 100;
    let ctx = egui::Context::default();
    let frame = |app: &mut App, events: Vec<egui::Event>| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| show(app, ctx),
        )
    };
    let wheel = |y: f32| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, y),
        modifiers: egui::Modifiers::NONE,
    };
    let at = |app: &mut App, text: &str, events: Vec<egui::Event>| {
        let mut output = frame(app, vec![]);
        for _ in 0..3 {
            output = frame(app, vec![]);
        }
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(egui::Rect::from_min_size(t.pos, t.galley.size()).center())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{text} is drawn"));
        frame(app, vec![egui::Event::PointerMoved(pos)]);
        frame(app, events);
    };
    let layers = |app: &App| app.loaded.as_ref().unwrap().patches[1].layers.clone();
    at(&mut app, "100", vec![wheel(1.0)]);
    assert_eq!(layers(&app)[1].settings.octave, 101);
    assert_eq!(
        (
            app.layer_editor
                .row(&app.loaded.as_ref().unwrap().patches[1]),
            app.layer_editor.field
        ),
        (1, 1)
    );
    at(&mut app, "101", vec![wheel(-1.0), wheel(-1.0)]);
    assert_eq!(layers(&app)[1].settings.octave, 99);
    at(&mut app, "-7", vec![wheel(1.0)]);
    assert_eq!(layers(&app)[0].settings.volume_db, -1.0);
    assert_eq!(
        (
            app.layer_editor
                .row(&app.loaded.as_ref().unwrap().patches[1]),
            app.layer_editor.field
        ),
        (0, 2)
    );
    let saved = cat_smf_editor::instrument_layers::read(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    assert_eq!(
        saved,
        app.loaded.as_ref().unwrap().patches[1].saved().unwrap()
    );
    app.layer_editor
        .begin(&app.loaded.as_ref().unwrap().patches[1]);
    at(&mut app, "99", vec![wheel(1.0)]);
    assert_eq!(layers(&app)[1].settings.octave, 99);
}
