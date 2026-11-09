//! The header background opens folded panes and focuses open ones, leaving child controls alone.
use super::*;

#[test]
fn a_folded_lines_name_and_far_edge_both_open_it() {
    for at_name in [true, false] {
        let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
        toggle(&mut a, ViewKind::Mixer);
        redraw(&mut a, &c);
        let label = text(&c, dock(), "mixer").unwrap();
        let pos = if at_name {
            label.center()
        } else {
            egui::pos2(850.0, label.center().y)
        };
        frame(&mut a, &c, dock(), click(pos));
        redraw(&mut a, &c);
        assert!(!folded(&a, ViewKind::Mixer));
        assert!(text(&c, dock(), "Master effects").is_some());
    }
}

#[test]
fn the_empty_part_of_a_folded_vertical_strip_opens_it() {
    let (mut a, c) = app(r#"row = [{ view = "piano_roll" }, { view = "mixer" }]"#);
    toggle(&mut a, ViewKind::Mixer);
    redraw(&mut a, &c);
    let x = toggles(&c, dock())
        .into_iter()
        .find(|t| t.left() > 860.0)
        .unwrap()
        .center()
        .x;
    frame(&mut a, &c, dock(), click(egui::pos2(x, 500.0)));
    assert!(!folded(&a, ViewKind::Mixer));
}

#[test]
fn an_open_headers_name_and_whitespace_focus_the_pane_without_folding() {
    for at_name in [true, false] {
        let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
        focus::set(&mut a, ViewKind::PianoRoll);
        redraw(&mut a, &c);
        let label = text(&c, dock(), "mixer").unwrap();
        let pos = if at_name {
            label.center()
        } else {
            egui::pos2(850.0, label.center().y)
        };
        frame(&mut a, &c, dock(), click(pos));
        assert_eq!(focus::focused(&a), ViewKind::Mixer);
        assert!(!folded(&a, ViewKind::Mixer));
        // The keyboard now acts on the pane selected by the header.
        key(
            &mut a,
            &c,
            dock(),
            Key::F6,
            Modifiers::CTRL.plus(Modifiers::COMMAND),
        );
        assert!(folded(&a, ViewKind::Mixer));
        assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    }
}

#[test]
fn tab_buttons_keep_their_click_and_header_whitespace_focuses_the_shown_tab() {
    let (mut a, c) = app(
        r#"column = [{ tabs = [{ view = "mixer" }, { view = "tempo_curve" }] }, { view = "piano_roll" }]"#,
    );
    let pos = text(&c, dock(), "tempo_curve").unwrap().center();
    frame(&mut a, &c, dock(), click(pos));
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::TempoCurve);
    assert!(!folded(&a, ViewKind::TempoCurve));

    focus::set(&mut a, ViewKind::EventList);
    redraw(&mut a, &c);
    // Closing the tempo curve returns the group to its first tab, mixer.
    let y = text(&c, dock(), "mixer").unwrap().center().y;
    frame(&mut a, &c, dock(), click(egui::pos2(850.0, y)));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(!folded(&a, ViewKind::Mixer));
}
