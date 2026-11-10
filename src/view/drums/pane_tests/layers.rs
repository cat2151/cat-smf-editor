use super::*;

#[test]
fn melody_sound_button_precedes_effects_and_edits_layers_then_returns_to_grid() {
    for place in [Place::Floating, Place::Root, Place::Dock] {
        let mut a = placed(Kind::Melody, place);
        let c = context();
        for _ in 0..3 {
            frame(&mut a, &c, ROOT, vec![]);
        }
        let output = shapes(&c, host(place));
        let at = |label: &str| {
            output
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{label} button in {place:?}"))
        };
        let sound = at("Inst. layers");
        let effects = at("Effects");
        assert!(sound.x < effects.x);
        assert!((sound.y - effects.y).abs() < 1.0);
        let cursor = a.drum_editor.command_target();
        for pressed in [true, false] {
            frame(
                &mut a,
                &c,
                host(place),
                vec![
                    egui::Event::PointerMoved(sound),
                    egui::Event::PointerButton {
                        pos: sound,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(a.layer_editor.open);
        let track = a.track;
        let before = a.loaded.as_ref().unwrap().patches[track].layers.len();
        press(&mut a, &c, ROOT, Key::A);
        assert_eq!(
            a.loaded.as_ref().unwrap().patches[track].layers.len(),
            before + 1
        );
        press(&mut a, &c, ROOT, Key::Escape);
        assert!(!a.layer_editor.open);
        assert!(is_open(&a));
        assert_eq!(a.drum_editor.command_target(), cursor);
        press(&mut a, &c, host(place), Key::L);
        assert_ne!(a.drum_editor.command_target(), cursor);
        key(&mut a, &c, host(place), Key::T, Modifiers::SHIFT);
        assert!(a.layer_editor.open);
        press(&mut a, &c, ROOT, Key::O);
        assert_eq!(
            a.loaded.as_ref().unwrap().patches[track].layers[0]
                .settings
                .octave,
            1
        );
        press(&mut a, &c, ROOT, Key::Escape);
        assert!(!a.layer_editor.open);
    }
}
