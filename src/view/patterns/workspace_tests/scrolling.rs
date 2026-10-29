use super::*;

fn viewport(c: &egui::Context) -> egui::Rect {
    c.data(|d| d.get_temp(egui::Id::new((Pane::Song.title(), "matrix-viewport"))))
        .unwrap()
}

fn offset(c: &egui::Context) -> f32 {
    let id = c
        .data(|d| d.get_temp(egui::Id::new((Pane::Song.title(), "matrix-scroll"))))
        .unwrap();
    egui::scroll_area::State::load(c, id).unwrap().offset.x
}

#[test]
fn song_manual_scroll_returns_to_measure_one_without_moving_selection() {
    let (mut a, c, _, _, _) = verification_song();
    c.style_mut(|style| style.spacing.scroll.floating = false);
    let song = Pane::Song.viewport();
    a.patterns.workspace.song_cell.placing = true;
    a.patterns.workspace.song_cell.measure = 20;
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    assert!(offset(&c) > 0.0);
    let area = viewport(&c);
    frame(
        &mut a,
        &c,
        song,
        vec![
            egui::Event::PointerMoved(area.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(100_000.0, 0.0),
                modifiers: Modifiers::NONE,
            },
        ],
    );
    for _ in 0..5 {
        frame(&mut a, &c, song, vec![]);
    }
    assert_eq!(offset(&c), 0.0);
    assert_eq!(a.patterns.workspace.song_cell.measure, 20);
    let first = c
        .data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), "cell", 0usize, 1u64)))
        })
        .unwrap();
    assert!(viewport(&c).contains_rect(first));
    // Clicking an already visible cell must not shift the content origin.
    row_cell(&mut a, &c, Pane::Song, 0, 6);
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    assert_eq!(a.patterns.workspace.song_cell.measure, 6);
    assert_eq!(offset(&c), 0.0);
    // Keyboard navigation still reveals a cell outside the current viewport.
    for _ in 0..14 {
        key(&mut a, &c, song, Key::L, Modifiers::NONE);
    }
    assert_eq!(a.patterns.workspace.song_cell.measure, 20);
    assert!(offset(&c) > 0.0);

    // Drag the scrollbar itself to its left endpoint while measure 20 stays selected.
    let area = viewport(&c);
    let start = egui::pos2(area.right() - 5.0, area.bottom() + 5.0);
    let left = egui::pos2(area.left() - 50.0, start.y);
    frame(
        &mut a,
        &c,
        song,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(&mut a, &c, song, vec![egui::Event::PointerMoved(left)]);
    frame(
        &mut a,
        &c,
        song,
        vec![egui::Event::PointerButton {
            pos: left,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    for _ in 0..3 {
        frame(&mut a, &c, song, vec![]);
    }
    assert_eq!(offset(&c), 0.0);
    assert_eq!(a.patterns.workspace.song_cell.measure, 20);
}
