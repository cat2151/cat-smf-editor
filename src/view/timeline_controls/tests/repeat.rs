use super::{fixtures::Fixture, *};
use egui::{
    epaint::Shape,
    PointerButton::{Primary, Secondary},
};

fn markers(f: &Fixture) -> (f32, f32, (u64, u64)) {
    let row = f.rect("repeat");
    match f.screen {
        Screen::Staff => {
            let mut xs: Vec<f32> = f
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Path(p)
                        if p.closed
                            && p.points.len() == 20
                            && p.points.iter().map(|p| p.y).sum::<f32>() / 20.0 > row.bottom() =>
                    {
                        Some(p.points.iter().map(|p| p.x).sum::<f32>() / 20.0)
                    }
                    _ => None,
                })
                .collect();
            // Horizontal-only zoom leaves the fixture staff gap at 9px.
            let gap = 9.0;
            xs.sort_by(f32::total_cmp);
            xs.dedup_by(|a, b| (*a - *b).abs() < 0.5);
            let n = xs.len();
            (xs[n - 2] - 0.62 * gap, xs[n - 1] - 0.62 * gap, (480, 960))
        }
        Screen::PianoRoll => {
            let r = f
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Rect(r) if r.stroke.width == 0.5 && r.rect.top() > row.bottom() => {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .max_by(|a, b| a.left().total_cmp(&b.left()))
                .unwrap();
            (r.left(), r.right(), (480, 960))
        }
        Screen::Tempo => {
            let mut xs: Vec<f32> = f
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Circle(c)
                        if c.fill == crate::view::tempo_curve::TEMPO
                            && c.center.y > row.bottom() =>
                    {
                        Some(c.center.x)
                    }
                    _ => None,
                })
                .collect();
            xs.sort_by(f32::total_cmp);
            xs.dedup_by(|a, b| (*a - *b).abs() < 0.5);
            let n = xs.len();
            let distance = xs[n - 1] - xs[n - 2];
            (xs[n - 1], xs[n - 1] + distance / 2.0, (960, 1440))
        }
        Screen::Parameter => {
            let point = f
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Circle(c)
                        if c.fill == crate::view::tempo_curve::POINT
                            && c.center.y > row.bottom() =>
                    {
                        Some(c.center.x)
                    }
                    _ => None,
                })
                .max_by(f32::total_cmp)
                .unwrap();
            let control = f
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    Shape::Rect(r)
                        if r.fill == crate::view::tempo_curve::CONTROL
                            && r.rect.top() > row.bottom() =>
                    {
                        Some(r.rect.center().x)
                    }
                    _ => None,
                })
                .unwrap();
            (control, point, (240, 480))
        }
        Screen::TrackOverview => {
            let pane = f
                .ctx
                .read_response(egui::Id::new(("track-overview-pane", 1)))
                .unwrap()
                .rect;
            let r = f
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Rect(r)
                        if r.fill == crate::view::track_color(1)
                            && pane.y_range().contains(r.rect.center().y) =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .max_by(|a, b| a.left().total_cmp(&b.left()))
                .unwrap();
            (r.left(), r.right(), (480, 960))
        }
        Screen::Song => {
            let cell = |measure| {
                f.ctx.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new((
                        crate::view::patterns::workspace::Pane::Song.title(),
                        "cell",
                        0usize,
                        measure,
                    )))
                })
            };
            for measure in 1u64..80 {
                if let Some((left, right)) = cell(measure).zip(cell(measure + 1))
                    && row.x_range().contains(left.left())
                    && row.x_range().contains(right.left())
                {
                    return (
                        left.left(),
                        right.left(),
                        ((measure - 1) * 1920, measure * 1920),
                    );
                }
            }
            panic!("two current Song measure boundaries must be visible");
        }
        Screen::Scene => unreachable!("Scene has no repeat row"),
    }
}

#[test]
fn every_repeat_adapter_uses_drawn_markers_after_zoom_and_scroll() {
    for screen in Screen::ALL.into_iter().filter(|s| s.has_bars()) {
        let mut f = Fixture::new(screen);
        let map = f.rect("minimap");
        let zoom = egui::pos2(f.rect("repeat").left() + 20.0, map.top() - 12.0);
        f.pointer(zoom, Primary, true);
        f.pointer(zoom, Primary, false);
        f.frame(vec![]);
        let row = f.rect("repeat");
        f.frame(vec![
            egui::Event::PointerMoved(row.center() + egui::vec2(0.0, 60.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(-20.0, 0.0),
                modifiers: Default::default(),
            },
        ]);
        for _ in 0..30 {
            f.frame(vec![]);
        }
        let (left, right, (a, b)) = markers(&f);
        let row = f.rect("repeat");
        assert!(
            row.x_range().contains(left) && row.x_range().contains(right),
            "visible markers {screen:?}: {left}..{right} in {row:?}"
        );
        let left = egui::pos2(left, row.center().y);
        let right = egui::pos2(right, row.center().y);
        let before = f.bytes();
        for reverse in [false, true] {
            crate::play::transport::stop(&mut f.app);
            f.app.playback.repeat = None;
            let (from, to) = if reverse {
                (right, left)
            } else {
                (left, right)
            };
            f.pointer(from, Primary, true);
            f.frame(vec![egui::Event::PointerMoved(to)]);
            f.pointer(to, Primary, false);
            assert_eq!(
                f.app.playback.repeat,
                Some((a, b)),
                "{screen:?} reverse={reverse}"
            );
        }
        let adjust = left.lerp(right, 0.75);
        f.pointer(adjust, Secondary, true);
        f.pointer(adjust, Secondary, false);
        assert_eq!(
            f.app.playback.repeat,
            Some((a, a + (b - a) * 3 / 4)),
            "right endpoint {screen:?}"
        );
        f.pointer(adjust, Primary, true);
        f.pointer(adjust, Primary, false);
        assert_eq!(f.app.playback.repeat, None, "clear {screen:?}");
        assert_eq!(f.bytes(), before, "{screen:?}");
    }
}
