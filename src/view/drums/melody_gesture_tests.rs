use super::melody_tests::{fixture, frame, plain};
use super::*;
use cat_smf_editor::score::step_notes;
use egui::{Event, Pos2, Rect};

fn setup() -> (App, egui::Context) {
    let mut a = fixture();
    a.undo.clear();
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![]);
    }
    (a, ctx)
}

fn cell(ctx: &egui::Context, pitch: u8, step: usize) -> Rect {
    ctx.data(|d| d.get_temp(egui::Id::new(("melody-cell", pitch, step))))
        .unwrap()
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

fn begin(a: &mut App, ctx: &egui::Context, pos: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(pos)]);
    frame(a, ctx, vec![button(pos, true)]);
}

fn drag(a: &mut App, ctx: &egui::Context, from: Pos2, to: Pos2) {
    begin(a, ctx, from);
    frame(a, ctx, vec![Event::PointerMoved(to)]);
    frame(a, ctx, vec![button(to, false)]);
    frame(a, ctx, vec![]);
}

fn notes(a: &App) -> Vec<(u8, u64, u64)> {
    step_notes::notes(&a.loaded.as_ref().unwrap().visible().tracks[a.track])
        .iter()
        .map(|n| (n.pitch, n.start, n.end))
        .collect()
}

#[test]
fn horizontal_drawing_previews_one_band_and_commits_one_undo() {
    let (mut a, ctx) = setup();
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 60, 4).center();
    let origin = cell(&ctx, 60, 0).center();
    begin(&mut a, &ctx, from);
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)]);
    frame(&mut a, &ctx, vec![]);
    assert!(notes(&a).is_empty(), "preview must not mutate the document");
    assert!(!a.undo.available());
    assert_eq!(cell(&ctx, 60, 0).center(), origin);
    assert_eq!(
        cell(&ctx, 60, 4).left(),
        cell(&ctx, 60, 3).right(),
        "preview joins the cells"
    );
    frame(&mut a, &ctx, vec![button(to, false)]);
    assert_eq!(notes(&a), [(60, 0, 600)]);
    a.undo_music().unwrap();
    assert!(notes(&a).is_empty());
    assert!(!a.undo.available());
}

#[test]
fn diagonal_drawing_fills_skipped_columns_without_scrolling() {
    let (mut a, ctx) = setup();
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 64, 4).center();
    let origin = cell(&ctx, 60, 0).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(
        notes(&a),
        [
            (60, 0, 120),
            (61, 120, 240),
            (62, 240, 360),
            (63, 360, 480),
            (64, 480, 600)
        ]
    );
    assert_eq!(cell(&ctx, 60, 0).center(), origin);
    a.undo_music().unwrap();
    assert!(notes(&a).is_empty());
    assert!(!a.undo.available());
}

#[test]
fn right_edge_extends_and_shortens_without_moving_the_head() {
    let (mut a, ctx) = setup();
    plain(&mut a, &ctx, Key::F);
    let from = cell(&ctx, 60, 0).right_center() - egui::vec2(2.0, 0.0);
    let to = cell(&ctx, 60, 4).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 0, 600)]);
    let from = cell(&ctx, 60, 4).right_center() - egui::vec2(2.0, 0.0);
    let to = cell(&ctx, 60, 0).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 0, 120)]);
    a.undo_music().unwrap();
    assert_eq!(notes(&a), [(60, 0, 600)]);
}

#[test]
fn resizing_stops_at_the_next_note_and_pattern_end() {
    let (mut a, ctx) = setup();
    let track = a.track;
    a.edit_music(|m| {
        step_notes::toggle(m, track, 60, 1, 0, 90)?;
        step_notes::toggle(m, track, 60, 1, 6, 80)
    })
    .unwrap();
    frame(&mut a, &ctx, vec![]);
    let from = cell(&ctx, 60, 0).right_center() - egui::vec2(2.0, 0.0);
    let to = cell(&ctx, 60, 8).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 0, 720), (60, 720, 840)]);
    let from = cell(&ctx, 60, 6).right_center() - egui::vec2(2.0, 0.0);
    let to = cell(&ctx, 60, 15).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 0, 720), (60, 720, 1920)]);
    let ns = step_notes::notes(&a.loaded.as_ref().unwrap().visible().tracks[track]);
    assert_eq!(ns.iter().map(|n| n.velocity).collect::<Vec<_>>(), [90, 80]);
}

#[test]
fn drawing_preserves_existing_notes_and_reverse_drawing_joins_cells() {
    let (mut a, ctx) = setup();
    let track = a.track;
    a.edit_music(|m| step_notes::toggle(m, track, 60, 1, 2, 70))
        .unwrap();
    frame(&mut a, &ctx, vec![]);
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 60, 4).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 0, 240), (60, 240, 360), (60, 360, 600)]);
    let from = cell(&ctx, 62, 4).center();
    let to = cell(&ctx, 62, 0).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a).last(), Some(&(62, 0, 600)));
}

#[test]
fn escape_cancels_only_the_pending_drawing_then_closes() {
    let (mut a, ctx) = setup();
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 64, 4).center();
    begin(&mut a, &ctx, from);
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)]);
    plain(&mut a, &ctx, Key::Escape);
    assert!(is_open(&a));
    frame(&mut a, &ctx, vec![button(to, false)]);
    assert!(notes(&a).is_empty());
    assert!(!a.undo.available());
    plain(&mut a, &ctx, Key::Escape);
    assert!(!is_open(&a));
}

#[test]
fn a_complete_drag_between_frames_still_draws_all_steps() {
    let (mut a, ctx) = setup();
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 64, 4).center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(from)]);
    frame(
        &mut a,
        &ctx,
        vec![
            button(from, true),
            Event::PointerMoved(to),
            button(to, false),
        ],
    );
    assert_eq!(notes(&a).len(), 5);
    a.undo_music().unwrap();
    assert!(notes(&a).is_empty());
}

#[test]
fn diagonal_drawing_uses_the_filtered_rows() {
    let (mut a, ctx) = setup();
    plain(&mut a, &ctx, Key::Slash);
    plain(&mut a, &ctx, Key::Slash);
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![]);
    }
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 67, 3).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(
        notes(&a),
        [(60, 0, 120), (62, 120, 240), (64, 240, 360), (67, 360, 480)]
    );
}

#[test]
fn resizing_a_continuation_in_the_second_measure_preserves_its_start() {
    let (mut a, ctx) = setup();
    let track = a.track;
    let id = crate::view::patterns::mode::editing(&a, track)
        .unwrap()
        .pattern;
    a.edit_tracks(|m| {
        use cat_smf_editor::score::pattern;
        pattern::operations::resize(
            m,
            id,
            vec![
                pattern::Meter {
                    numerator: 4,
                    exponent: 2,
                    ticks: 1920
                };
                2
            ],
        )
    })
    .unwrap();
    a.edit_music(|m| {
        step_notes::toggle(m, track, 60, 1, 15, 85)?;
        for _ in 0..3 {
            step_notes::resize(m, track, 60, 1, 15, step_notes::Length::Longer, 3840)?;
        }
        Ok(())
    })
    .unwrap();
    plain(&mut a, &ctx, Key::PageDown);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(notes(&a), [(60, 1800, 2280)]);
    let from = cell(&ctx, 60, 2).right_center() - egui::vec2(2.0, 0.0);
    let to = cell(&ctx, 60, 0).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 1800, 2040)]);
}

#[test]
fn off_grid_resize_keeps_the_start_and_velocity() {
    let (mut a, ctx) = setup();
    let track = a.track;
    a.edit_music(|m| {
        step_notes::toggle(m, track, 60, 1, 0, 73)?;
        let n = step_notes::notes(&m.tracks[track])[0].clone();
        m.tracks[track].events[n.on].tick = 30;
        m.tracks[track].events[n.off].tick = 270;
        cat_smf_editor::edit::refresh_deltas(&mut m.tracks[track])
    })
    .unwrap();
    frame(&mut a, &ctx, vec![]);
    let from = cell(&ctx, 60, 2).right_center() - egui::vec2(2.0, 0.0);
    let to = cell(&ctx, 60, 4).center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(notes(&a), [(60, 30, 600)]);
    assert_eq!(
        step_notes::notes(&a.loaded.as_ref().unwrap().visible().tracks[track])[0].velocity,
        73
    );
}

#[test]
fn page_change_cancels_drawing_and_wheel_still_scrolls() {
    let (mut a, ctx) = setup();
    let from = cell(&ctx, 60, 0).center();
    let to = cell(&ctx, 64, 4).center();
    begin(&mut a, &ctx, from);
    frame(&mut a, &ctx, vec![Event::PointerMoved(to)]);
    plain(&mut a, &ctx, Key::PageDown);
    frame(&mut a, &ctx, vec![button(to, false)]);
    assert!(notes(&a).is_empty());
    assert!(!a.undo.available());
    let before = cell(&ctx, 60, 0).center();
    frame(
        &mut a,
        &ctx,
        vec![
            Event::PointerMoved(from),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -100.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    frame(&mut a, &ctx, vec![]);
    assert_ne!(cell(&ctx, 60, 0).center(), before);
    assert!(notes(&a).is_empty());
}
