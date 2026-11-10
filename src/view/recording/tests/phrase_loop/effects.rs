use super::*;
use crate::view::effects::{tests::effect, Target};

fn chain_key(a: &mut App, ctx: &egui::Context, k: Key) {
    let _ = ctx.run(
        egui::RawInput {
            events: vec![
                key(k, true, Modifiers::NONE),
                key(k, false, Modifiers::NONE),
            ],
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000., 700.),
            )),
            ..Default::default()
        },
        |ctx| crate::view::effects::show(a, ctx),
    );
}

#[test]
fn keyboard_effects_follow_the_playing_container_and_the_instrument_visibility() {
    for count in [0usize, 1, 3] {
        let (mut a, track) = container();
        let ctx = egui::Context::default();
        open_key(&mut a, &ctx);
        press(&mut a, &ctx, Key::X, Modifiers::CTRL);
        assert!(
            !a.effect_editor.is_open(),
            "same availability as instrument browse"
        );
        assert!(ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("keyboard-effects")))
            .is_none());
        start_loop(&mut a, &ctx);
        // The synthetic effects have no player-generated preset files; create the Loop
        // before attaching them so this UI fixture does not invoke the external player.
        let mut patch = a.loaded.as_ref().unwrap().patches[track].clone();
        patch.effects = (0..count).map(|i| effect(&format!("fx{i}"))).collect();
        crate::player::selection::store_track(&mut a, track, &patch).unwrap();
        a.track = 0;
        press(&mut a, &ctx, Key::X, Modifiers::CTRL);
        assert_eq!(a.track, track);
        assert_eq!(
            a.effect_editor.command_target(),
            (Some(Target::Track(track)), count.saturating_sub(1))
        );
        assert!(!a.player_selection.open);
        assert!(phrase::looping(session(&a)));
        if count > 0 {
            chain_key(&mut a, &ctx, Key::B);
            assert!(a.loaded.as_ref().unwrap().patches[track].effects[count - 1].bypass);
            assert!(a.loaded.as_ref().unwrap().patches[0].effects.is_empty());
        }
        chain_key(&mut a, &ctx, Key::Escape);
        assert!(!a.effect_editor.is_open());
        frame(&mut a, &ctx, vec![], true);
        assert!(a.patterns.recording.is_open());
        assert!(phrase::looping(session(&a)));
        assert_eq!(
            session(&a).sound().patch.effects,
            a.loaded.as_ref().unwrap().patches[track].effects
        );
    }
}

#[test]
fn keyboard_effects_button_sits_beside_the_sound_and_opens_its_chain() {
    for layered in [false, true] {
        let (mut a, track) = container();
        if layered {
            let mut patch = a.loaded.as_ref().unwrap().patches[track].clone();
            crate::view::layers::add(&mut patch).unwrap();
            crate::player::selection::store_track(&mut a, track, &patch).unwrap();
        }
        let ctx = egui::Context::default();
        open_key(&mut a, &ctx);
        start_loop(&mut a, &ctx);
        for _ in 0..2 {
            frame(&mut a, &ctx, vec![], true);
        }
        let at = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("keyboard-effects")))
            .unwrap()
            .center();
        for pressed in [true, false] {
            frame(
                &mut a,
                &ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
                true,
            );
        }
        assert_eq!(
            a.effect_editor.command_target().0,
            Some(Target::Track(track))
        );
    }
}

#[test]
fn a_keyboard_pane_yields_its_keys_to_the_chain_and_resumes_playing_after_close() {
    use crate::view::dock::{focus, ViewKind};
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    a.layout.spec = crate::store::layout::parse("[root]\nview = \"keyboard\"").unwrap();
    focus::set(&mut a, ViewKind::Keyboard);
    crate::view::effects::tests::frame(&mut a, &ctx, vec![]);
    crate::view::effects::tests::press(&mut a, &ctx, Key::X, Modifiers::CTRL);
    assert_eq!(
        a.effect_editor.command_target().0,
        Some(Target::Track(track))
    );
    assert_eq!(commands::owner(&a, &ctx), Screen::Effects);
    crate::view::effects::tests::press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    assert!(!a.effect_editor.is_open());
    assert_eq!(commands::owner(&a, &ctx), Screen::Recording);
    assert!(a.patterns.recording.is_open());
    crate::view::effects::tests::frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)]);
    assert!(recorder(&a).held_pads().next().is_some());
}
