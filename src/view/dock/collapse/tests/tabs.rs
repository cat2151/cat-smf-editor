//! A tab group folds as one pane: every tab with it, the focus passing over all of them.
use super::*;

const GROUP: &str = r#"column = [{ tabs = [{ view = "mixer" }, { view = "tempo_curve" }] }, { view = "piano_roll" }]"#;

#[test]
fn folding_a_tab_group_folds_every_tab_and_the_focus_moves_on() {
    let (mut a, c) = app(GROUP);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    toggle(&mut a, ViewKind::Mixer);
    assert!(folded(&a, ViewKind::Mixer));
    assert!(folded(&a, ViewKind::TempoCurve));
    assert!(!folded(&a, ViewKind::PianoRoll));
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    // Any tab names the group: the hidden one opens it again.
    toggle(&mut a, ViewKind::TempoCurve);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(!folded(&a, ViewKind::TempoCurve));
}

#[test]
fn f6_passes_over_a_folded_tab_group() {
    let (mut a, c) = app(GROUP);
    focus::set(&mut a, ViewKind::PianoRoll);
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);

    toggle(&mut a, ViewKind::Mixer);
    redraw(&mut a, &c);
    for shift in [Modifiers::NONE, Modifiers::SHIFT] {
        key(&mut a, &c, dock(), Key::F6, shift);
        assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    }
}

#[test]
fn a_window_of_one_tab_group_does_not_fold() {
    let (mut a, _c) = app(r#"tabs = [{ view = "mixer" }, { view = "tempo_curve" }]"#);
    toggle(&mut a, ViewKind::Mixer);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(!folded(&a, ViewKind::TempoCurve));
}
