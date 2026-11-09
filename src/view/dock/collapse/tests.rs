//! Folding panes: the toggle in each pane's header, what a folded pane leaves drawn, and the
//! focus passing over it.
use super::*;
use crate::view::dock::window::viewport;
use crate::view::patterns::workspace_tests::{frame, key, verification_song};
use eframe::egui::{Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

fn dock() -> egui::ViewportId {
    viewport(0)
}

/// A song in the main window's event list, with `node` (`column = [...]` or `row = [...]`) in a
/// 900×700 dock window.
fn app(node: &str) -> (App, egui::Context) {
    let (mut a, c, ..) = verification_song();
    a.layout.spec = crate::store::layout::parse(&format!(
        "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"Dock\"\nrect = [0, 0, 900, 700]\n{node}"
    ))
    .unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    (a, c)
}

fn shapes(c: &egui::Context, id: egui::ViewportId) -> Vec<egui::epaint::ClippedShape> {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
}

fn text(c: &egui::Context, id: egui::ViewportId, text: &str) -> Option<egui::Rect> {
    shapes(c, id).into_iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == text => Some(t.visual_bounding_rect()),
        _ => None,
    })
}

/// The toggles' chevrons: their three points, the tip in the middle.
fn chevrons(c: &egui::Context, id: egui::ViewportId) -> Vec<Vec<egui::Pos2>> {
    shapes(c, id)
        .into_iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Path(p)
                if p.points.len() == 3 && !p.closed && p.stroke.width == CHEVRON_WIDTH =>
            {
                Some(p.points.clone())
            }
            _ => None,
        })
        .filter(|p| {
            let r = egui::Rect::from_points(p);
            r.width() <= 20.0 && r.height() <= 20.0
        })
        .collect()
}

fn toggles(c: &egui::Context, id: egui::ViewportId) -> Vec<egui::Rect> {
    chevrons(c, id)
        .iter()
        .map(|p| egui::Rect::from_points(p))
        .collect()
}

/// The toggle on the header line of `name`: the nearest one left of the name.
fn toggle_of(c: &egui::Context, id: egui::ViewportId, name: &str) -> egui::Pos2 {
    let label = text(c, id, name).unwrap_or_else(|| panic!("no header {name}"));
    toggles(c, id)
        .into_iter()
        .filter(|t| t.right() <= label.left() && (t.center().y - label.center().y).abs() < 6.0)
        .max_by(|a, b| a.right().total_cmp(&b.right()))
        .unwrap_or_else(|| panic!("no toggle beside {name}"))
        .center()
}

fn click(pos: egui::Pos2) -> Vec<egui::Event> {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    vec![egui::Event::PointerMoved(pos), button(true), button(false)]
}

fn redraw(a: &mut App, c: &egui::Context) {
    frame(a, c, ROOT, vec![]);
    frame(a, c, ROOT, vec![]);
}

#[test]
fn the_toggle_folds_a_pane_in_a_column_to_one_line_and_opens_it_again() {
    let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
    assert!(text(&c, dock(), "Master effects").is_some());
    let mixer = text(&c, dock(), "mixer").unwrap();
    assert!(mixer.top() < 400.0, "{mixer:?}");

    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "mixer")));
    redraw(&mut a, &c);
    assert!(folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_none());
    // One line at the bottom; the roll above took the rest.
    let mixer = text(&c, dock(), "mixer").unwrap();
    assert!(mixer.top() > 700.0 - 40.0, "{mixer:?}");

    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "mixer")));
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_some());
}

#[test]
fn a_pane_folded_in_a_row_keeps_one_column_with_its_toggle() {
    let (mut a, c) = app(r#"row = [{ view = "piano_roll" }, { view = "mixer" }]"#);
    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "mixer")));
    redraw(&mut a, &c);
    assert!(folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_none());
    // The name went into the tooltip; the toggle alone sits at the right edge, at the top.
    assert!(text(&c, dock(), "mixer").is_none());
    let right = toggles(&c, dock())
        .into_iter()
        .find(|t| t.left() > 900.0 - 40.0)
        .expect("the folded pane's toggle");
    assert!(right.top() < 40.0, "{right:?}");

    frame(&mut a, &c, dock(), click(right.center()));
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_some());
}

/// The way each toggle on the top line points, left to right, as `<`, `>` or `v`.
fn arrows(c: &egui::Context, id: egui::ViewportId) -> String {
    let mut tips: Vec<(f32, char)> = chevrons(c, id)
        .into_iter()
        .filter(|p| egui::Rect::from_points(p).top() < 40.0)
        .map(|p| {
            let tip = p[1] - p[0].lerp(p[2], 0.5);
            let way = if tip.y.abs() > tip.x.abs() {
                'v'
            } else if tip.x < 0.0 {
                '<'
            } else {
                '>'
            };
            (egui::Rect::from_points(&p).center().x, way)
        })
        .collect();
    tips.sort_by(|a, b| a.0.total_cmp(&b.0));
    tips.into_iter().map(|(_, way)| way).collect()
}

#[test]
fn the_arrow_points_the_way_the_pane_folds() {
    let (mut a, c) =
        app(r#"row = [{ view = "piano_roll" }, { view = "mixer" }, { view = "tempo_curve" }]"#);
    // The last pane narrows toward the right edge, the others toward their left.
    assert_eq!(arrows(&c, dock()), "<<>");
    toggle(&mut a, ViewKind::PianoRoll);
    toggle(&mut a, ViewKind::TempoCurve);
    redraw(&mut a, &c);
    assert_eq!(arrows(&c, dock()), "><<");

    let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
    assert_eq!(arrows(&c, dock()), "v");
    toggle(&mut a, ViewKind::PianoRoll);
    redraw(&mut a, &c);
    // The mixer's header came up under the folded line.
    assert_eq!(arrows(&c, dock()), ">v");
}

#[test]
fn a_lone_pane_has_no_toggle() {
    let (mut a, c) = app(r#"view = "mixer""#);
    assert!(text(&c, dock(), "Master effects").is_some());
    assert!(text(&c, dock(), "mixer").is_none());
    assert!(toggles(&c, dock()).iter().all(|t| t.top() > 40.0));
    // A fold left over from a layout with more panes does not hide it.
    a.pane_collapse.toggle(&Node::View(ViewKind::Mixer));
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_some());
    // The main window's lone event list has none either.
    assert!(text(&c, ROOT, "event_list").is_none());
}

#[test]
fn folding_the_mixer_ends_its_volume_gesture() {
    let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
    let music = a.loaded.as_ref().unwrap().music.clone();
    // Held down, the button keeps a gesture going through the mixer's frame.
    let label = text(&c, dock(), "piano_roll").unwrap().center();
    frame(
        &mut a,
        &c,
        dock(),
        vec![
            egui::Event::PointerMoved(label),
            egui::Event::PointerButton {
                pos: label,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    a.undo.record_volume(music.clone(), 0, true);
    frame(&mut a, &c, dock(), vec![]);
    let pushes = a.undo.pushes();
    a.undo.record_volume(music.clone(), 0, true);
    assert_eq!(a.undo.pushes(), pushes, "the gesture went on");

    toggle(&mut a, ViewKind::Mixer);
    frame(&mut a, &c, dock(), vec![]);
    let pushes = a.undo.pushes();
    a.undo.record_volume(music, 0, true);
    assert_eq!(a.undo.pushes(), pushes + 1, "the folded mixer ended it");
}

#[test]
fn folding_the_focused_pane_hands_the_focus_on_and_f6_passes_over_it() {
    let (mut a, c) =
        app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }, { view = "tempo_curve" }]"#);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    toggle(&mut a, ViewKind::Mixer);
    assert_eq!(focus::focused(&a), ViewKind::TempoCurve);
    assert!(a.tempo_curve.open);
    redraw(&mut a, &c);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::TempoCurve);
    key(&mut a, &c, dock(), Key::F6, Modifiers::SHIFT);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    // A click on the folded pane's name opens it without taking the focus.
    let line = text(&c, dock(), "mixer").unwrap().center();
    frame(&mut a, &c, dock(), click(line));
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    assert!(!folded(&a, ViewKind::Mixer));
}

#[test]
fn with_no_other_pane_to_take_it_the_focus_goes_back_to_the_event_list() {
    let (mut a, c) = app(r#"column = [{ view = "mixer" }, { view = "piano" }]"#);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(focus::panes(&a), 1);
    toggle(&mut a, ViewKind::Mixer);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert!(!a.playback.mixer_focused());
    // The window's keys no longer name the folded mixer.
    assert_eq!(focus::dock_view(&a, 0), None);
}

mod header;
mod keys;
mod tab_panes;
mod tabs;

#[test]
fn hovering_the_toggle_tells_what_a_click_does() {
    let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
    c.style_mut(|s| s.interaction.tooltip_delay = 0.0);
    let hover = |a: &mut App, pos| {
        for _ in 0..3 {
            frame(a, &c, dock(), vec![egui::Event::PointerMoved(pos)]);
        }
    };
    let at = toggle_of(&c, dock(), "mixer");
    hover(&mut a, at);
    assert!(text(&c, dock(), "折りたたむ").is_some());

    frame(&mut a, &c, dock(), click(at));
    redraw(&mut a, &c);
    hover(&mut a, egui::pos2(450.0, 300.0));
    hover(&mut a, toggle_of(&c, dock(), "mixer"));
    assert!(text(&c, dock(), "折りたたみを解除する").is_some());
}
