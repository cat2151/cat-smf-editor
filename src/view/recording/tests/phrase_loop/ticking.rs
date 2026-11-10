use super::*;

fn ticking(a: &App) -> bool {
    session(a).ticking()
}

/// Two seconds of silence end a Capture phrase.
fn past_capture_gap(a: &mut App, ctx: &egui::Context) {
    rewind(a, Duration::from_millis(2_500));
    frame(a, ctx, vec![], true);
}

#[test]
fn an_untouched_keyboard_does_not_tick_and_a_held_key_does_until_its_phrase_is_taken() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    frame(&mut a, &ctx, vec![], true);
    assert!(!ticking(&a));
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(ticking(&a));
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    // The Capture phrase waits for its silence.
    assert!(ticking(&a));
    past_capture_gap(&mut a, &ctx);
    assert!(!ticking(&a));
}

#[test]
fn a_phrase_loop_ticks_while_it_repeats_and_not_once_switched_off() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    // Waiting for its first key.
    assert!(!ticking(&a));
    start_loop_from_waiting(&mut a, &ctx);
    assert!(phrase::looping(session(&a)));
    past_capture_gap(&mut a, &ctx);
    assert!(ticking(&a));
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    assert!(!ticking(&a));
}

/// The first key and a measure later, as `start_loop` does from the switch on.
fn start_loop_from_waiting(a: &mut App, ctx: &egui::Context) {
    frame(a, ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(ticking(a));
    rewind(a, measure(a) / 4);
    frame(a, ctx, vec![key(Key::Z, false, Modifiers::NONE)], true);
    rewind(a, measure(a));
    frame(a, ctx, vec![], true);
}
