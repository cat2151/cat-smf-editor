use super::*;
use egui::{Key, Modifiers};

#[test]
fn parameter_navigation_repeats_and_ime_does_not_move_selection() {
    let mut a = tests::app();
    let mut p = tests::picker();
    p.song = a.song_id;
    p.generation = a.generation;
    p.parameters = (0..25)
        .map(|i| {
            let mut parameter = p.parameters[0].clone();
            parameter.stable_id = i.to_string();
            parameter.name = format!("Parameter {i}");
            parameter
        })
        .collect();
    a.parameter_editor.picker = Some(p);
    let c = egui::Context::default();
    for (key, modifiers, expected) in [
        (Key::PageDown, Modifiers::NONE, 10),
        (Key::PageDown, Modifiers::NONE, 20),
        (Key::ArrowDown, Modifiers::NONE, 21),
        (Key::End, Modifiers::CTRL, 24),
        (Key::PageUp, Modifiers::NONE, 14),
        (Key::Home, Modifiers::CTRL, 0),
    ] {
        let _ = c.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: true,
                    modifiers,
                }],
                ..Default::default()
            },
            |ctx| crate::view::commands::route(&mut a, ctx),
        );
        assert_eq!(
            a.parameter_editor.picker.as_ref().unwrap().selected,
            expected
        );
    }
    a.parameter_editor.picker.as_mut().unwrap().composing = true;
    let _ = c.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::End,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            }],
            ..Default::default()
        },
        |ctx| crate::view::commands::route(&mut a, ctx),
    );
    assert_eq!(a.parameter_editor.picker.as_ref().unwrap().selected, 0);
}

#[test]
fn picker_draws_position_and_separate_enter_keycap() {
    let mut a = tests::app();
    a.parameter_editor.picker = Some(tests::picker());
    let c = egui::Context::default();
    let _ = c.run(egui::RawInput::default(), |ctx| {
        picker_ui::picker_ui(&mut a, ctx)
    });
    let output = c.run(egui::RawInput::default(), |ctx| {
        picker_ui::picker_ui(&mut a, ctx)
    });
    let text: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect();
    assert!(text.contains(&"1 / 1"), "{text:?}");
    assert!(text.contains(&"Create automation track"));
    assert!(text.contains(&"Enter"));
    a.parameter_editor.picker.as_mut().unwrap().query = "no-match".into();
    let output = c.run(egui::RawInput::default(), |ctx| {
        picker_ui::picker_ui(&mut a, ctx)
    });
    assert!(output.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::Text(t) if t.galley.text() == "0 / 0")));
}
