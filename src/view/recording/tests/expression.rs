use super::super::play as playing;
use super::*;
use cat_smf_editor::score::pattern::recording::{Modulation, Velocity};

fn cc1_now(a: &App) -> Option<u8> {
    let s = session(a);
    s.cc1(s.clock.elapsed())
}
/// CC1 values written anywhere in the song, in track order.
fn written_cc1(a: &App) -> Vec<u8> {
    a.loaded
        .as_ref()
        .unwrap()
        .music
        .tracks
        .iter()
        .flat_map(|t| &t.events)
        .filter(|e| e.status == 0xb0 && e.data[0] == 1)
        .map(|e| e.data[1])
        .collect()
}

#[test]
fn the_cc1_sweep_waits_at_its_start_until_the_keys_are_recorded() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert_eq!(recorder(&a).settings().modulation, Modulation::Sweep);
    let ms = Duration::from_millis;
    rewind(&mut a, ms(1000));
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(session(&a).sweep, None);
    assert_eq!(cc1_now(&a), Some(0));
    // A key starts a Capture phrase, and the sweep with it.
    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], true);
    assert!(session(&a).sweep.is_some());
    rewind(&mut a, ms(1000));
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Z, false, Modifiers::NONE)],
        true,
    );
    // A second up the sweep, plus the milliseconds the frames really took.
    let up = cc1_now(&a).unwrap();
    assert!((63..=66).contains(&up), "{up}");
    // Silence ends the phrase, and the sweep waits at its start again.
    rewind(&mut a, ms(1000));
    frame(&mut a, &ctx, vec![], true);
    assert!(session(&a).sweep.is_some());
    rewind(&mut a, ms(1500));
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(session(&a).sweep, None);
    assert_eq!(cc1_now(&a), Some(0));
    // The phrase keeps the CC1 the key was played under.
    let cc1 = written_cc1(&a);
    assert_eq!(cc1.first(), Some(&0));
    assert!(cc1.windows(2).all(|w| w[0] < w[1]), "{cc1:?}");
    assert!((60..=up).contains(cc1.last().unwrap()), "{cc1:?}");
}

#[test]
fn the_keyboard_window_chooses_velocity_and_modulation_but_a_take_fixes_the_modulation() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let mut state = std::mem::take(&mut a.patterns.recording);
    let s = state.session.as_mut().unwrap();
    playing::button(&mut a, s, window::Button::Velocity(Velocity::V127));
    playing::button(
        &mut a,
        s,
        window::Button::Modulation(Modulation::Fixed(127)),
    );
    a.patterns.recording = state;
    let t = *recorder(&a).settings();
    assert_eq!(
        (t.velocity, t.modulation),
        (Velocity::V127, Modulation::Fixed(127))
    );
    assert_eq!(cc1_now(&a), Some(127));
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Space, true, Modifiers::NONE)],
        true,
    );
    let mut state = std::mem::take(&mut a.patterns.recording);
    let s = state.session.as_mut().unwrap();
    assert!(playing::armed(s));
    playing::button(&mut a, s, window::Button::Velocity(Velocity::V100));
    playing::button(&mut a, s, window::Button::Modulation(Modulation::Off));
    a.patterns.recording = state;
    let t = *recorder(&a).settings();
    assert_eq!(
        (t.velocity, t.modulation),
        (Velocity::V100, Modulation::Fixed(127))
    );
}
