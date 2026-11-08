//! The template's example: the Song and the keyboard share one dock window, the analysis has another.
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, window, ViewKind};
use cat_smf_editor::score::pattern::recording::Pad;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;
/// The background of the spectrum, the Lissajous and the correlation bar.
const PLOT: egui::Color32 = egui::Color32::from_rgb(15, 20, 24);

fn song_window() -> egui::ViewportId {
    window::viewport(0)
}

fn analysis_window() -> egui::ViewportId {
    window::viewport(1)
}

fn builder(c: &egui::Context, id: egui::ViewportId) -> Option<egui::ViewportBuilder> {
    c.data_mut(|d| d.remove_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
}

fn visible(c: &egui::Context, id: egui::ViewportId) -> Option<bool> {
    c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
}

/// `id` (a `Pad` or a `(title, "transport")` pair) was drawn since the last call.
fn drawn(c: &egui::Context, id: impl std::hash::Hash) -> bool {
    let id = egui::Id::new(id);
    c.data_mut(|d| {
        let seen = d.get_temp::<egui::Rect>(id).is_some();
        d.remove::<egui::Rect>(id);
        seen
    })
}

fn song_drawn(c: &egui::Context) -> bool {
    drawn(c, (Pane::Song.title(), "transport"))
}

fn pads_drawn(c: &egui::Context) -> bool {
    drawn(c, ("pad-rect", Pad::Row { row: 0, column: 0 }))
}

fn plots(c: &egui::Context, id: egui::ViewportId) -> usize {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
    .iter()
    .filter(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == PLOT))
    .count()
}

#[test]
fn the_template_demo_draws_the_song_the_keyboard_and_the_analysis_in_two_dock_windows() {
    let (mut a, c, ..) = verification_song();
    a.layout.spec = crate::store::layout::parse(&crate::store::layout::tests::demo()).unwrap();
    crate::view::analysis::toggle(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    for id in [song_window(), analysis_window()] {
        builder(&c, id);
    }
    builder(&c, Pane::Song.viewport());
    builder(&c, a.analysis.viewport());
    song_drawn(&c);
    frame(&mut a, &c, ROOT, vec![]);
    let title = |id| builder(&c, id).and_then(|b| b.title);
    assert_eq!(
        title(song_window()).as_deref(),
        Some("cat-smf-editor · song · keyboard")
    );
    assert_eq!(
        title(analysis_window()).as_deref(),
        Some("cat-smf-editor · analysis")
    );
    assert!(song_drawn(&c));
    assert_eq!(plots(&c, analysis_window()), 3);
    assert_eq!(plots(&c, song_window()), 0);
    assert!(builder(&c, Pane::Song.viewport()).is_none());
    assert!(builder(&c, a.analysis.viewport()).is_none());

    // The first key in the Song window gives the focus to its first pane, the Song.
    key(&mut a, &c, song_window(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    key(&mut a, &c, song_window(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
    pads_drawn(&c);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(pads_drawn(&c));
    assert!(builder(&c, crate::view::recording::window::viewport()).is_none());
    // F6 cycles between the Song and the keyboard within their window.
    key(&mut a, &c, song_window(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    key(&mut a, &c, song_window(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
    key(&mut a, &c, song_window(), Key::F6, Modifiers::SHIFT);
    assert_eq!(focus::focused(&a), ViewKind::Song);

    // A key in the other window hands the focus to the analysis.
    key(&mut a, &c, analysis_window(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Analysis);
    assert_eq!(owner(&a, &c), Screen::Analysis);

    // A menu hides both dock windows, and closing it brings them back with the focus kept.
    a.edit_menu.open = true;
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(visible(&c, song_window()), Some(false));
    assert_eq!(visible(&c, analysis_window()), Some(false));
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(visible(&c, song_window()), Some(true));
    assert_eq!(visible(&c, analysis_window()), Some(true));
    assert_eq!(focus::focused(&a), ViewKind::Analysis);
    assert!(a.patterns.recording.is_open());
}
