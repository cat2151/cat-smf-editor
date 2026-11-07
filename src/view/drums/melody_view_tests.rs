use super::melody_tests::{fixture, frame, plain};
use super::*;
use cat_smf_editor::score::harmony::Set;
use egui::{epaint::Shape, Pos2, Rect};

/// The playhead's orange, as drawn through the window's opacity.
fn playhead(color: egui::Color32) -> bool {
    let want = crate::view::timeline_repeat::PLAYHEAD;
    [
        (color.r(), want.r()),
        (color.g(), want.g()),
        (color.b(), want.b()),
    ]
    .iter()
    .all(|(c, w)| c.abs_diff(*w) <= 8)
}

fn cell(ctx: &egui::Context, pitch: u8, step: usize) -> Option<Rect> {
    ctx.data(|d| d.get_temp(egui::Id::new(("melody-cell", pitch, step))))
}

fn temp(ctx: &egui::Context, id: &str) -> Rect {
    ctx.data(|d| d.get_temp(egui::Id::new(id)))
        .unwrap_or_else(|| panic!("{id} not drawn"))
}

/// Press and release `button` at `pos`, one frame each, as a mouse does.
fn click(a: &mut App, ctx: &egui::Context, pos: Pos2, button: egui::PointerButton) {
    frame(a, ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            a,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

fn settled() -> (App, egui::Context) {
    let mut a = fixture();
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&mut a, &ctx, vec![]);
    }
    (a, ctx)
}

#[test]
fn the_playing_step_is_one_line_inside_the_view() {
    let (mut a, ctx) = settled();
    plain(&mut a, &ctx, Key::F);
    a.drum_editor.loop_enabled = true;
    playback::restart(&mut a).unwrap();
    crate::play::transport::advance(&mut a);
    a.audition.set_test_seconds(0.01);
    let out = frame(&mut a, &ctx, vec![]);
    let clip = temp(&ctx, "melody-clip");
    let column = temp(&ctx, "drum-playing");
    // The column is taller than the view: a frame around it showed as two side lines.
    assert!(column.top() < clip.top() && column.bottom() > clip.bottom());
    let lines: Vec<[Pos2; 2]> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke } if playhead(stroke.color) => Some(*points),
            Shape::Rect(r) if playhead(r.stroke.color) => panic!("framed: {r:?}"),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 1);
    let [top, bottom] = lines[0];
    assert_eq!(top.x, column.center().x);
    assert_eq!(bottom.x, column.center().x);
    // Only the visible part: the scroll area's clip reaches a few px past its inner rect.
    assert!((top.y - clip.top()).abs() <= 4.0 && (bottom.y - clip.bottom()).abs() <= 4.0);
}

#[test]
fn rows_out_of_view_are_not_drawn() {
    let (mut a, ctx) = settled();
    let out = frame(&mut a, &ctx, vec![]);
    let names: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .filter(|t| (0..=127).any(|p| melody::name(p) == *t))
        .collect();
    // Chromatic: 128 rows, opened around C4; only the rows in view get a name.
    assert!(names.contains(&"C4".into()));
    assert!(!names.contains(&"G9".into()) && !names.contains(&"C-1".into()));
    let clip = temp(&ctx, "melody-clip");
    assert!(
        names.len() as f32 <= clip.height() / 18.0 + 2.0,
        "{names:?}"
    );
}

#[test]
fn each_container_keeps_its_own_row_set() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let (mut a, ctx) = settled();
    a.config = crate::store::config::Store::open(path.clone());
    let first = a.patterns.container.unwrap();
    // Nothing chosen yet: Chord; `/` steps it to Chromatic for this container only.
    assert_eq!(a.config.melody_rows(Some(first)), Set::Chord);
    plain(&mut a, &ctx, Key::Slash);
    assert_eq!(a.config.melody_rows(Some(first)), Set::Chromatic);
    crate::view::patterns::library::add_container_with(&mut a, None).unwrap();
    let second = a.patterns.container.unwrap();
    assert_ne!(first, second);
    open(&mut a);
    frame(&mut a, &ctx, vec![]);
    assert!(is_open(&a) && melody::active(&a), "{:?}", a.load_error);
    assert_eq!(a.config.melody_rows(Some(second)), Set::Chord);
    assert_ne!(melody::rows(&mut a).len(), 128);
    plain(&mut a, &ctx, Key::Slash);
    plain(&mut a, &ctx, Key::Slash);
    plain(&mut a, &ctx, Key::Slash);
    let saved = crate::store::config::Store::open(path);
    assert_eq!(saved.melody_rows(Some(first)), Set::Chromatic);
    assert_eq!(saved.melody_rows(Some(second)), Set::Penta);
}

#[test]
fn clicking_a_note_name_sounds_it_and_moves_the_cursor_without_editing() {
    let (mut a, ctx) = settled();
    let notes = |a: &App| {
        cat_smf_editor::score::step_notes::notes(
            &a.loaded.as_ref().unwrap().visible().tracks[a.track],
        )
        .len()
    };
    let before = (notes(&a), a.undo.pushes());
    assert_eq!(melody::pitch(&a), 60);
    let d4 = cell(&ctx, 62, 0).unwrap();
    click(
        &mut a,
        &ctx,
        egui::pos2(d4.left() - 20.0, d4.center().y),
        egui::PointerButton::Primary,
    );
    assert_eq!(a.drum_editor.error, None);
    assert_eq!(a.drum_editor.kit.preview.map(|p| p.0), Some(62));
    assert_eq!(melody::pitch(&a), 62);
    assert_eq!((notes(&a), a.undo.pushes()), before);
}

/// One frame at `time`: the wheel's smoothing needs the clock to advance.
fn timed(a: &mut App, ctx: &egui::Context, time: f64, events: Vec<egui::Event>) {
    let _ = ctx.run(
        egui::RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 700.0))),
            events,
            ..Default::default()
        },
        |ctx| show(a, ctx),
    );
}

fn zoom(a: &App) -> f32 {
    a.config.melody_zoom(a.patterns.container)
}

#[test]
fn the_zoom_bar_zooms_around_the_clicked_height() {
    let (mut a, ctx) = settled();
    let bar = temp(&ctx, "melody-zoom-bar");
    let clip = temp(&ctx, "melody-clip");
    assert!(bar.right() <= clip.left());
    assert!(bar.height() >= clip.height());
    let height = |ctx: &egui::Context| cell(ctx, 60, 0).unwrap().height();
    let unzoomed = height(&ctx);
    // A click away from the middle zooms in, and G3 under the pointer stays under it.
    let y = cell(&ctx, 55, 0).unwrap().center().y;
    click(
        &mut a,
        &ctx,
        egui::pos2(bar.center().x, y),
        egui::PointerButton::Primary,
    );
    frame(&mut a, &ctx, vec![]);
    assert!(height(&ctx) > unzoomed * 1.4, "{}", height(&ctx));
    let after = cell(&ctx, 55, 0).unwrap().center().y;
    assert!((after - y).abs() < 6.0, "G3 moved from {y} to {after}");
    // A right click zooms back out around the same height.
    click(
        &mut a,
        &ctx,
        egui::pos2(bar.center().x, y),
        egui::PointerButton::Secondary,
    );
    frame(&mut a, &ctx, vec![]);
    assert!((height(&ctx) - unzoomed).abs() < 0.5, "{}", height(&ctx));
    let after = cell(&ctx, 55, 0).unwrap().center().y;
    assert!((after - y).abs() < 6.0, "G3 moved from {y} to {after}");
    // The wheel over the bar zooms too.
    timed(
        &mut a,
        &ctx,
        1.0,
        vec![
            egui::Event::PointerMoved(bar.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 40.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for i in 1..4 {
        timed(&mut a, &ctx, 1.0 + f64::from(i) * 0.1, vec![]);
    }
    frame(&mut a, &ctx, vec![]);
    assert!(height(&ctx) > unzoomed, "{}", height(&ctx));
}

#[test]
fn dragging_on_the_zoom_bar_fits_the_span_to_the_view() {
    let (mut a, ctx) = settled();
    let bar = temp(&ctx, "melody-zoom-bar");
    // From E4 to C4: five rows, Chromatic.
    let (top, bottom) = (cell(&ctx, 64, 0).unwrap(), cell(&ctx, 60, 0).unwrap());
    let x = bar.center().x;
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    // Cells sit 1 px inside their rows.
    let (from, to) = (
        egui::pos2(x, top.top() - 1.0),
        egui::pos2(x, bottom.bottom() + 1.0),
    );
    frame(&mut a, &ctx, vec![egui::Event::PointerMoved(from)]);
    frame(&mut a, &ctx, vec![button(from, true)]);
    for step in 1..=4 {
        let pos = from.lerp(to, step as f32 / 4.0);
        frame(&mut a, &ctx, vec![egui::Event::PointerMoved(pos)]);
    }
    frame(&mut a, &ctx, vec![button(to, false)]);
    for _ in 0..2 {
        frame(&mut a, &ctx, vec![]);
    }
    // The five dragged rows now fill the view.
    let clip = temp(&ctx, "melody-clip");
    let (top, bottom) = (
        cell(&ctx, 64, 0).unwrap().top(),
        cell(&ctx, 60, 0).unwrap().bottom(),
    );
    assert!(
        (top - clip.top()).abs() < 2.0 && (bottom - clip.bottom()).abs() < 2.0,
        "E4..C4 {top}..{bottom}, view {clip:?}"
    );
}

#[test]
fn each_container_keeps_its_own_zoom() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let (mut a, ctx) = settled();
    a.config = crate::store::config::Store::open(path.clone());
    let first = a.patterns.container.unwrap();
    let bar = temp(&ctx, "melody-zoom-bar");
    click(&mut a, &ctx, bar.center(), egui::PointerButton::Primary);
    assert_eq!(zoom(&a), timeline_zoom_click());
    crate::view::patterns::library::add_container_with(&mut a, None).unwrap();
    open(&mut a);
    frame(&mut a, &ctx, vec![]);
    assert!(is_open(&a) && melody::active(&a), "{:?}", a.load_error);
    assert_eq!(zoom(&a), 1.0);
    a.config.persist();
    let saved = crate::store::config::Store::open(path);
    assert_eq!(saved.melody_zoom(Some(first)), timeline_zoom_click());
    assert_eq!(saved.melody_zoom(a.patterns.container), 1.0);
}

fn timeline_zoom_click() -> f32 {
    crate::view::timeline_zoom::CLICK
}
