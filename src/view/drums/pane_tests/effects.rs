use super::*;
use crate::view::effects::{tests::effect, Target};

#[test]
fn every_grid_opens_its_whole_chain_and_returns_to_the_same_cursor() {
    for kind in KINDS {
        for place in [Place::Floating, Place::Root, Place::Dock] {
            for count in [0usize, 1, 3] {
                let mut a = placed(kind, place);
                let track = a.track;
                let mut patch = a.loaded.as_ref().unwrap().patches[track].clone();
                patch.effects = (0..count).map(|i| effect(&format!("fx{i}"))).collect();
                crate::player::selection::store_track(&mut a, track, &patch).unwrap();
                let c = context();
                frame(&mut a, &c, ROOT, vec![]);
                press(&mut a, &c, host(place), Key::L);
                let cursor = a.drum_editor.command_target();
                press(&mut a, &c, host(place), Key::X);
                assert!(a.effect_editor.is_open());
                assert_eq!(
                    a.effect_editor.command_target(),
                    (Some(Target::Track(track)), count.saturating_sub(1))
                );
                assert_eq!(crate::view::commands::root_owner(&a), Screen::Effects);
                assert!(
                    !a.player_selection.open,
                    "opening the chain must not browse a plugin"
                );
                assert!(is_open(&a));
                if count > 0 {
                    press(&mut a, &c, ROOT, Key::B);
                    assert!(a.loaded.as_ref().unwrap().patches[track].effects[count - 1].bypass);
                }
                press(&mut a, &c, ROOT, Key::Escape);
                assert!(!a.effect_editor.is_open());
                assert!(is_open(&a));
                assert_eq!(a.drum_editor.command_target(), cursor);
                if place != Place::Floating {
                    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
                }
                press(&mut a, &c, host(place), Key::H);
                assert_ne!(a.drum_editor.command_target(), cursor);
            }
        }
    }
}

#[test]
fn every_grids_effects_button_opens_the_chain() {
    for kind in KINDS {
        let mut a = placed(kind, Place::Root);
        let c = context();
        for _ in 0..3 {
            frame(&mut a, &c, ROOT, vec![]);
        }
        let at = shapes(&c, ROOT)
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Effects" => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .expect("Effects button");
        for pressed in [true, false] {
            frame(
                &mut a,
                &c,
                ROOT,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(a.effect_editor.is_open());
    }
}
