use crate::app::App;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

const WIDTH: f32 = 1400.0;

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type: if status == 0xff { data[0] } else { 0 },
        data: if status == 0xff {
            data[1..].into()
        } else {
            data.into()
        },
    }
}

fn app() -> App {
    let track = || Track {
        events: vec![
            event(0, 0x90, &[60, 100]),
            event(480, 0x80, &[60, 0]),
            event(960, 0xff, &[0x2f]),
        ],
    };
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![track(), track()],
    })
}

/// The panel's content shapes, after enough frames for the layout to settle.
fn shapes(app: &mut App) -> Vec<egui::epaint::ClippedShape> {
    let ctx = egui::Context::default();
    let mut out = Vec::new();
    for _ in 0..3 {
        out = ctx
            .run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(WIDTH, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        super::show(app, ui, &mut false, &mut None);
                    });
                },
            )
            .shapes;
    }
    out
}

/// Below the panes' header line, which every pane of a window holding several has.
const BELOW_HEADER: f32 = 30.0;

fn right_half(shapes: &[egui::epaint::ClippedShape]) -> usize {
    shapes
        .iter()
        .map(|s| s.shape.visual_bounding_rect())
        .filter(|r| r.is_finite() && r.min.x > WIDTH / 2.0 + 10.0 && r.min.y > BELOW_HEADER)
        .count()
}

#[test]
fn note_mode_puts_the_score_beside_the_list() {
    let mut a = app();
    a.note_mode = true;
    assert!(right_half(&shapes(&mut a)) > 0);
    a.piano_roll.open = true;
    assert_eq!(right_half(&shapes(&mut a)), 0);
}

#[test]
fn split_tracks_fill_the_list_without_the_score() {
    let mut a = app();
    a.note_mode = true;
    a.track_view.split = Some([0, 1]);
    let drawn = shapes(&mut a);
    assert!(drawn.iter().any(|s| matches!(
        &s.shape,
        egui::Shape::Rect(r) if r.stroke.color == crate::view::ACTIVE_PANE
    )));
    assert!(right_half(&drawn) > 0);
}

#[test]
fn the_layout_file_decides_the_panes() {
    let mut a = app();
    a.note_mode = true;
    a.layout.spec = crate::store::layout::parse(
        "[root]\nrow = [{ view = \"event_list\", width = 3 }, { view = \"score\" }]",
    )
    .unwrap();
    let narrow_list = right_half(&shapes(&mut a));
    a.layout.spec = crate::store::layout::parse("[root]\nview = \"event_list\"").unwrap();
    let list_only = right_half(&shapes(&mut a));
    a.layout.spec = Default::default();
    let built_in = right_half(&shapes(&mut a));
    assert!(narrow_list > built_in, "{narrow_list} {built_in}");
    assert!(built_in > list_only, "{built_in} {list_only}");
}

#[test]
fn a_folded_score_draws_nothing_beside_the_list() {
    let mut a = app();
    a.note_mode = true;
    assert!(right_half(&shapes(&mut a)) > 0);
    crate::view::dock::collapse::toggle(&mut a, crate::view::dock::ViewKind::Score);
    assert_eq!(right_half(&shapes(&mut a)), 0);
}

#[test]
fn the_focus_frame_encloses_the_pane_header() {
    let mut a = app();
    a.note_mode = true;
    let drawn = shapes(&mut a);
    let frame = drawn
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.stroke.color == crate::view::ACTIVE_PANE => {
                Some(r.rect.intersect(s.clip_rect))
            }
            _ => None,
        })
        .expect("the focused pane has a frame");
    let header = crate::view::dock::collapse::thickness(&egui::Ui::new(
        egui::Context::default(),
        egui::Id::NULL,
        egui::UiBuilder::new(),
    ));
    assert!(frame.min.y < header, "{frame:?}");
}
