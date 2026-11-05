use super::*;

fn muted(app: &App, row: usize) -> bool {
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    let id = patch.drums.as_ref().unwrap().rows[row].layer;
    patch
        .layers
        .iter()
        .find(|l| l.settings.id == id)
        .unwrap()
        .settings
        .mute
}

#[test]
fn mute_follows_row_id_roundtrips_and_undoes_without_changing_notes() {
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    app.loaded.as_mut().unwrap().patches[1].layers.reverse();
    let before = app.loaded.as_ref().unwrap().music.clone();
    let ctx = egui::Context::default();
    tests::press(&mut app, &ctx, Key::M);
    assert!(muted(&app, 0));
    assert!((1..4).all(|row| !muted(&app, row)));
    let music = &app.loaded.as_ref().unwrap().music;
    let notes = |music: &cat_smf_editor::model::Music| {
        music.tracks[1]
            .events
            .iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (e.tick, e.status, e.data.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(notes(music), notes(&before));
    let restored =
        cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(music).unwrap()).unwrap();
    let saved = cat_smf_editor::instrument_layers::parse(&restored.tracks[1]).unwrap();
    assert!(saved.layers.iter().find(|l| l.id == 1).unwrap().mute);
    tests::press(&mut app, &ctx, Key::M);
    assert!(!muted(&app, 0));
    app.undo_music().unwrap();
    assert!(muted(&app, 0));
    app.undo_music().unwrap();
    assert!(!muted(&app, 0));
}

#[test]
fn clicking_header_toggles_mute_and_dims_active_steps() {
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    let ctx = egui::Context::default();
    let input = |events| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        events,
        ..Default::default()
    };
    for _ in 0..3 {
        let _ = ctx.run(input(vec![]), |ctx| show(&mut app, ctx));
    }
    let bounds = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new(("drum-mute", 0usize)))
            .unwrap()
    });
    for expected in [true, false] {
        for pressed in [true, false] {
            let _ = ctx.run(
                input(vec![
                    egui::Event::PointerMoved(bounds.center()),
                    egui::Event::PointerButton {
                        pos: bounds.center(),
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ]),
                |ctx| show(&mut app, ctx),
            );
        }
        assert_eq!(muted(&app, 0), expected);
        let output = ctx.run(input(vec![]), |ctx| show(&mut app, ctx));
        let cells = ctx.data(|d| {
            d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                "drum-grid-bounds",
            ))
            .unwrap()
            .2
        });
        let cell = cells[3 * 18 + 2];
        let fill = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.rect == cell => Some(r.fill),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            fill,
            if expected {
                egui::Color32::from_rgb(101, 105, 94)
            } else {
                egui::Color32::from_rgb(166, 226, 46)
            }
        );
    }
}

#[test]
fn muting_during_loop_keeps_clock_scope_and_repeat() {
    use crate::play::transport;
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    app.drum_editor.loop_enabled = true;
    playback::restart(&mut app).unwrap();
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.08);
    let tick = transport::tick(&app);
    let repeat = app.playback.repeat;
    values::toggle_mute(&mut app, 0).unwrap();
    assert!(muted(&app, 0));
    assert!(app.playback.pending.is_none());
    assert_eq!(transport::tick(&app), tick);
    assert_eq!(app.audition.seconds(), Some(0.08));
    assert_eq!(app.playback.repeat, repeat);
}
