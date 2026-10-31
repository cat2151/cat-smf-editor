use super::*;
use cat_smf_editor::score::pattern::recording::{pitches, Pad};

/// A key event as the measured Japanese keyboard delivers it.
fn jis(key: Key, physical: Option<Key>, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: physical,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
fn lower(vk: u8) -> bool {
    vk == 0xE2
}
fn upper(vk: u8) -> bool {
    vk == 0xDC
}
fn nothing(_: u8) -> bool {
    false
}
fn tap(a: &mut App, ctx: &egui::Context, k: Key, physical: Option<Key>) {
    frame(a, ctx, vec![jis(k, physical, true)], true);
    frame(a, ctx, vec![jis(k, physical, false)], true);
}
/// Sorted Note On pitches of a pattern.
fn notes(a: &App, id: u64) -> Vec<u8> {
    let music = &a.loaded.as_ref().unwrap().music;
    let p = &Catalog::read(music).unwrap().patterns[&id];
    let b = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    let mut on: Vec<_> = b
        .iter()
        .filter(|e| e.status == 0x90)
        .map(|e| e.data[0])
        .collect();
    on.sort_unstable();
    on
}

#[test]
fn rows_keys_easily_confused_on_a_japanese_keyboard_record_their_own_pitches() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let settings = *recorder(&a).settings();
    tap(&mut a, &ctx, Key::Space, Some(Key::Space));
    tap(&mut a, &ctx, Key::Z, Some(Key::Z));
    tap(&mut a, &ctx, Key::CloseBracket, Some(Key::Backslash));
    tap(&mut a, &ctx, Key::OpenBracket, Some(Key::OpenBracket));
    tap(&mut a, &ctx, Key::OpenBracket, Some(Key::CloseBracket));
    for down in [lower as fn(u8) -> bool, upper] {
        a.patterns.recording.session.as_mut().unwrap().down = down;
        frame(&mut a, &ctx, vec![jis(Key::Backslash, None, true)], true);
        a.patterns.recording.session.as_mut().unwrap().down = nothing;
        frame(&mut a, &ctx, vec![jis(Key::Backslash, None, false)], true);
    }
    assert!((0..128).all(|p| !recorder(&a).sounding(p)));
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Escape, true, Modifiers::NONE)],
        true,
    );
    assert!(!a.patterns.recording.is_open());
    let mut expected: Vec<u8> = [(0, 0), (1, 11), (2, 10), (2, 11), (0, 10), (3, 12)]
        .into_iter()
        .flat_map(|(row, column)| pitches(&settings, Pad::Row { row, column }))
        .collect();
    expected.sort_unstable();
    assert_eq!(expected.len(), 6);
    assert!(expected.windows(2).all(|w| w[0] < w[1]), "{expected:?}");
    assert_eq!(notes(&a, a.patterns.selected.unwrap()), expected);
}

#[test]
fn tab_and_octave_keys_while_holding_pads_leave_nothing_sounding() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let plain = |k| key(k, true, Modifiers::NONE);
    for events in [
        vec![plain(Key::Z)],
        vec![plain(Key::Tab), plain(Key::X)],
        vec![plain(Key::ArrowUp), key(Key::Z, false, Modifiers::NONE)],
        vec![plain(Key::C), plain(Key::ArrowDown), plain(Key::Tab)],
        vec![
            key(Key::X, false, Modifiers::NONE),
            key(Key::C, false, Modifiers::NONE),
        ],
    ] {
        frame(&mut a, &ctx, events, true);
    }
    assert!((0..128).all(|p| !recorder(&a).sounding(p)));
    let queued: Vec<u32> = session(&a)
        .queue
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .iter()
        .copied()
        .collect();
    // The chord pad sent several notes; every pitch that started has stopped.
    assert!(queued.len() > 6, "{queued:x?}");
    for pitch in 0..128u32 {
        let count = |status| {
            queued
                .iter()
                .filter(|&&m| (m >> 16) & 0xff == status && (m >> 8) & 0xff == pitch)
                .count()
        };
        assert_eq!(count(0x90), count(0x80), "pitch {pitch}");
    }
}

#[test]
fn help_switches_between_preparing_and_playing_keys() {
    let mut a = app();
    let ctx = egui::Context::default();
    let help = |a: &mut App| {
        let c = egui::Context::default();
        let output = c.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 2000.0),
                )),
                ..Default::default()
            },
            |c| {
                egui::CentralPanel::default().show(c, |ui| crate::view::help::body(a, ui));
            },
        );
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    open_key(&mut a, &ctx);
    let playing = help(&mut a);
    assert_eq!(playing[0], "Virtual MIDI keyboard");
    assert!(playing.iter().any(|t| t == "Ctrl+,"));
    // The settings window shows the item shortcuts beside the items.
    assert!(!playing.iter().any(|t| t == "Tab" || t == "Enter"));
}

#[test]
fn space_waits_for_the_audio_threads_first_click_before_fixing_the_loop_origin() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(&mut a, &ctx, vec![key(Key::R, true, Modifiers::CTRL)], true);
    assert_eq!(
        recorder(&a).settings().range,
        cat_smf_editor::score::pattern::recording::Range::Loop { measures: 4 }
    );
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Space, true, Modifiers::NONE)],
        true,
    );
    // No audio block has run yet, so the press before the first click is not recorded.
    tap(&mut a, &ctx, Key::Z, Some(Key::Z));
    assert!(!recorder(&a).started());
    let before = session(&a).clock.elapsed();
    crate::view::recording::render_click(&mut a);
    let after = session(&a).clock.elapsed();
    let measure = Duration::from_micros(u64::from(recorder(&a).settings().tempo) * 4);
    let origin = recorder(&a).origin().unwrap();
    assert!(
        before + measure <= origin + Duration::from_millis(1) && origin <= after + measure,
        "{before:?} {origin:?} {after:?}"
    );
    assert!(!recorder(&a).recorded());
    // A second Space neither restarts the click nor moves the origin.
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Space, true, Modifiers::NONE)],
        true,
    );
    crate::view::recording::render_click(&mut a);
    assert_eq!(recorder(&a).origin(), Some(origin));
}
