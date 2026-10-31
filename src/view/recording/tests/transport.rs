use super::super::play as playing;
use super::*;
use window::Button;

/// Clicks a transport button as the window does after drawing; `true` closes it.
fn click(a: &mut App, button: Button) -> bool {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let done = playing::button(a, state.session.as_mut().unwrap(), button);
    a.patterns.recording = state;
    done
}
fn patterns(a: &App) -> usize {
    Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .len()
}
fn to_loop(a: &mut App, ctx: &egui::Context) {
    frame(a, ctx, vec![key(Key::R, true, Modifiers::CTRL)], true);
}

#[test]
fn free_rec_starts_the_clock_and_stop_commits_and_closes() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let before = patterns(&a);
    assert!(!playing::armed(session(&a)));
    assert!(!click(&mut a, Button::Rec));
    assert!(recorder(&a).started() && playing::armed(session(&a)));
    // The silence between Rec and the first note is part of the take.
    crate::view::recording::rewind(&mut a, Duration::from_millis(500));
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    crate::view::recording::rewind(&mut a, Duration::from_millis(250));
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    assert!(click(&mut a, Button::Stop));
    assert_eq!(patterns(&a), before + 1);
    let music = &a.loaded.as_ref().unwrap().music;
    let p = &Catalog::read(music).unwrap().patterns[&a.patterns.selected.unwrap()];
    let body = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    let first = body.iter().find(|e| e.status == 0x90).unwrap();
    assert!(first.tick >= 400, "{}", first.tick);
}

#[test]
fn space_in_free_range_starts_the_clock_like_rec() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Space, true, Modifiers::NONE)],
        true,
    );
    assert!(recorder(&a).started());
}

#[test]
fn stop_is_inert_until_armed_and_loop_rec_counts_in_once() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let before = patterns(&a);
    assert!(!click(&mut a, Button::Stop));
    assert!(a.patterns.recording.is_open() && !recorder(&a).started());
    assert_eq!(patterns(&a), before);
    to_loop(&mut a, &ctx);
    assert!(!click(&mut a, Button::Rec));
    assert!(playing::armed(session(&a)) && !recorder(&a).started());
    crate::view::recording::render_click(&mut a);
    let origin = recorder(&a).origin().unwrap();
    assert!(!click(&mut a, Button::Rec));
    crate::view::recording::render_click(&mut a);
    assert_eq!(recorder(&a).origin(), Some(origin));
    // Nothing was played, so Stop closes without a pattern.
    assert!(click(&mut a, Button::Stop));
    assert_eq!(patterns(&a), before);
}

#[test]
fn pads_only_sound_until_rec_by_default() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let before = patterns(&a);
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(recorder(&a).sounding(60));
    assert!(!playing::armed(session(&a)) && !recorder(&a).recorded());
    frame(
        &mut a,
        &ctx,
        vec![
            key(Key::Z, false, Modifiers::NONE),
            key(Key::Escape, true, Modifiers::NONE),
        ],
        true,
    );
    assert!(!a.patterns.recording.is_open());
    assert_eq!(patterns(&a), before);
}

#[test]
fn key_start_records_from_the_first_pad_and_counts_in_a_loop() {
    for looping in [false, true] {
        let mut a = app();
        let ctx = egui::Context::default();
        open_key(&mut a, &ctx);
        let mut keys = vec![Key::S];
        if looping {
            keys.push(Key::R);
        }
        for k in keys {
            frame(&mut a, &ctx, vec![key(k, true, Modifiers::CTRL)], true);
        }
        assert!(!playing::armed(session(&a)));
        frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
        assert!(playing::armed(session(&a)), "loop {looping}");
        // A loop's first pad only starts the count-in.
        assert_eq!(recorder(&a).started(), !looping);
        assert_eq!(recorder(&a).recorded(), !looping);
    }
}
