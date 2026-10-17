use super::*;

#[test]
fn enter_and_space_in_mixer_open_the_master_effect_chain_without_playing() {
    for key in [egui::Key::Enter, egui::Key::Space] {
        let mut app = fixture();
        let ctx = egui::Context::default();
        render(&mut app, &ctx, Some(egui::Key::M));
        assert!(app.playback.mixer_master);
        render(&mut app, &ctx, Some(key));
        assert!(app.effect_editor.is_open());
        assert!(app.playback.pending.is_none());
    }
}

#[test]
fn mixer_and_view_keys_are_independent_and_mixer_changes_do_not_restart_playback() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    playback::start(&mut app, Mode::Track);
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.75);
    render(&mut app, &ctx, Some(egui::Key::M));
    assert!(app.playback.mixer_open);
    assert!(app.note_mode);
    app.audition.set_gain(0, 0.25);
    assert_eq!(app.audition.gain(0), 0.25);
    assert_eq!(app.audition.gain(1), 1.0);
    assert!(app.audition.auto_gain_enabled());
    app.audition.set_auto_gain_enabled(false);
    assert!(!app.audition.auto_gain_enabled());
    assert_eq!(app.audition.seconds(), Some(0.75));
    render(&mut app, &ctx, Some(egui::Key::F));
    assert!(app.analysis.is_open());
    assert!(app.playback.follow, "f in the mixer leaves follow alone");
    render(&mut app, &ctx, Some(egui::Key::F));
    assert!(!app.analysis.is_open());
    render(&mut app, &ctx, Some(egui::Key::Escape));
    assert!(app.playback.mixer_open);
    assert!(!app.playback.mixer_focused());
    assert!(app.playback.active.is_some());
    render(&mut app, &ctx, Some(egui::Key::V));
    assert!(!app.note_mode);
    render(&mut app, &ctx, Some(egui::Key::M));
    assert!(app.playback.mixer_focused());
    render(&mut app, &ctx, Some(egui::Key::M));
    assert!(!app.playback.mixer_open);
    assert_eq!(transport::tick(&app), Some(720));
    app.audition.set_test_seconds(4.0);
    transport::advance(&mut app);
    assert!(app.playback.active.is_none());
}

#[test]
fn esc_and_clicks_move_keys_between_mixer_and_views() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    render(&mut app, &ctx, Some(egui::Key::M));
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("focused-mixer-fader")));
    render(&mut app, &ctx, Some(egui::Key::Escape));
    assert!(app.playback.mixer_open);
    assert!(!ctx.wants_keyboard_input());
    render(&mut app, &ctx, Some(egui::Key::P));
    assert!(app.playback.open);
    render(&mut app, &ctx, Some(egui::Key::Escape));
    assert!(!app.playback.open);
    assert!(app.playback.mixer_open);
    let mixer = ctx
        .memory(|memory| memory.area_rect(egui::Id::new("Mixer (m)")))
        .unwrap();
    click(&mut app, &ctx, mixer.center());
    assert!(app.playback.mixer_focused());
    click(&mut app, &ctx, egui::pos2(1190.0, 390.0));
    assert!(!mixer.contains(egui::pos2(1190.0, 390.0)));
    assert!(app.playback.mixer_open);
    assert!(!app.playback.mixer_focused());
}

fn click(app: &mut crate::app::App, ctx: &egui::Context, pos: egui::Pos2) {
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 400.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| view::show(app, ctx),
        );
    }
}

#[test]
fn mixer_db_shortcuts_clamp_and_change_only_the_selected_track_without_restarting() {
    let mut music = fixture().loaded.unwrap().music;
    music.tracks.push(music.tracks[0].clone());
    let mut app = view::tests::app(music);
    let ctx = egui::Context::default();
    playback::start(&mut app, Mode::AllTracks);
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.75);
    render(&mut app, &ctx, Some(egui::Key::M));
    render(&mut app, &ctx, Some(egui::Key::L));
    render(&mut app, &ctx, Some(egui::Key::L));
    assert_eq!(app.playback.mixer_track, 1);
    for _ in 0..4 {
        render(&mut app, &ctx, Some(egui::Key::K));
    }
    assert_eq!(
        app.audition.gain(1),
        crate::play::track_volume::db_to_gain(6)
    );
    assert_eq!(app.audition.gain(0), 1.0);
    // Mixer shortcuts must still work when a clicked fader owns keyboard focus.
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("focused-mixer-fader")));
    assert!(ctx.wants_keyboard_input());
    for _ in 0..20 {
        render(&mut app, &ctx, Some(egui::Key::ArrowDown));
    }
    assert_eq!(
        app.audition.gain(1),
        crate::play::track_volume::db_to_gain(-36)
    );
    render(&mut app, &ctx, Some(egui::Key::ArrowUp));
    assert_eq!(
        app.audition.gain(1),
        crate::play::track_volume::db_to_gain(-33)
    );
    render(&mut app, &ctx, Some(egui::Key::H));
    render(&mut app, &ctx, Some(egui::Key::J));
    assert_eq!(
        app.audition.gain(0),
        crate::play::track_volume::db_to_gain(-3)
    );
    assert_eq!(app.audition.seconds(), Some(0.75));
    assert!(app.playback.active.is_some());
    render(&mut app, &ctx, Some(egui::Key::M));
    assert!(!app.playback.mixer_open);
}

#[test]
fn mixer_draws_track_columns_with_db_scale_above_the_values() {
    let mut music = fixture().loaded.unwrap().music;
    music.tracks.push(music.tracks[0].clone());
    let mut app = view::tests::app(music);
    app.loaded.as_mut().unwrap().track_titles = vec!["Mixer A".into(), "Mixer B".into()];
    crate::play::mixer::open(&mut app);
    let ctx = egui::Context::default();
    let mut render_mixer = || {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| crate::play::mixer::show(&mut app, ctx),
        )
    };
    render_mixer();
    let output = render_mixer();
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::epaint::Shape::Text(t) = &s.shape {
                return Some(t);
            }
            None
        })
        .collect();
    let a = texts.iter().find(|t| t.galley.text() == "Mixer A").unwrap();
    assert!(texts.iter().any(|t| t.galley.text() == "Master effects"));
    let b = texts.iter().find(|t| t.galley.text() == "Mixer B").unwrap();
    assert!((a.pos.y - b.pos.y).abs() < 1.0);
    assert!(b.pos.x > a.pos.x + 100.0);
    let top = texts.iter().find(|t| t.galley.text() == "+6 dB").unwrap();
    let bottom = texts.iter().find(|t| t.galley.text() == "-36 dB").unwrap();
    assert!(bottom.pos.y > top.pos.y + 200.0);
    assert!(texts
        .iter()
        .any(|t| t.galley.text() == "+0 dB" && t.pos.y > bottom.pos.y));
    assert!(!texts.iter().any(|t| t.galley.text().contains(" %")));
}
