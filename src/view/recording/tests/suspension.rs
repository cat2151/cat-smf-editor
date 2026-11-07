use super::*;

#[test]
fn hidden_keyboard_releases_pads_and_keeps_take_without_processing_escape() {
    let mut a = app();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    frame(
        &mut a,
        &c,
        vec![
            key(Key::Space, true, Modifiers::NONE),
            key(Key::Z, true, Modifiers::NONE),
        ],
        true,
    );
    assert!(recorder(&a).held_pads().next().is_some());
    let at = session(&a).clock;
    a.edit_menu.open = true;
    frame(
        &mut a,
        &c,
        vec![key(Key::Escape, true, Modifiers::NONE)],
        true,
    );
    assert!(a.patterns.recording.is_open());
    assert!(recorder(&a).held_pads().next().is_none());
    assert!(crate::view::recording::play::armed(session(&a)));
    assert_eq!(session(&a).clock, at);
    a.edit_menu.open = false;
    frame(&mut a, &c, vec![key(Key::X, true, Modifiers::NONE)], true);
    assert!(recorder(&a).held_pads().next().is_some());
}

#[test]
fn popup_keys_do_not_play_pads_or_close_the_recording_session() {
    let mut a = app();
    let c = egui::Context::default();
    open_key(&mut a, &c);
    c.memory_mut(|m| m.open_popup(egui::Id::new("recording-popup")));
    frame(&mut a, &c, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(recorder(&a).held_pads().next().is_none());
    frame(
        &mut a,
        &c,
        vec![key(Key::Escape, true, Modifiers::NONE)],
        true,
    );
    assert!(a.patterns.recording.is_open());
}
