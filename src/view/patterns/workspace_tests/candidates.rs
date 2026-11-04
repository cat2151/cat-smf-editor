//! The pattern list in the Song window fills the window height and always shows its scroll bar.
use super::*;

/// The list rectangle and the "+" button rectangle in a Song window of `height`.
fn layout(height: f32) -> (App, egui::Context, egui::Rect, egui::Rect) {
    let (mut a, c, _, _, _) = verification_song();
    c.style_mut(|style| style.spacing.scroll.floating = false);
    let geometry = [460.0, 520.0, 760.0, height];
    a.patterns.workspace.song_cell.opened = Some(geometry);
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("window-geometry", Pane::Song.viewport())),
            geometry,
        )
    });
    for _ in 0..3 {
        frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    }
    let rect = |id: &str| {
        c.data(|d| d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), id))))
            .unwrap()
    };
    let (list, plus) = (rect("parts-list"), rect("new-pattern"));
    (a, c, list, plus)
}

#[test]
fn the_pattern_list_grows_with_the_song_window_down_to_its_bottom() {
    for height in [520.0, 800.0] {
        let (_, _, list, plus) = layout(height);
        assert!(plus.top() > list.bottom(), "{list:?} {plus:?}");
        // Only the panel margin is left under the "+" button.
        assert!(height - plus.bottom() < 16.0, "{height} {plus:?}");
    }
    let small = layout(520.0).2.height();
    let large = layout(800.0).2.height();
    assert!((large - small - 280.0).abs() < 1.0, "{small} {large}");
}

#[test]
fn the_pattern_list_shows_its_scroll_bar_with_few_patterns() {
    let (_, c, list, _) = layout(800.0);
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                "workspace-shapes",
                Pane::Song.viewport(),
            )))
        })
        .unwrap();
    // The bar track spans the whole list height beside the 240 px wide cards.
    let bar = shapes.iter().any(|s| match &s.shape {
        egui::Shape::Rect(r) => {
            r.rect.left() > list.left() + 200.0
                && (r.rect.top() - list.top()).abs() < 1.0
                && (r.rect.bottom() - list.bottom()).abs() < 1.0
        }
        _ => false,
    });
    assert!(bar, "{list:?}");
}
