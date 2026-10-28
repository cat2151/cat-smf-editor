//! Pane backgrounds, curves, notes and scrolling of the overview window.
use super::*;

#[test]
fn panes_tell_notes_events_and_empty_tracks_apart() {
    let app = fixture();
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(
        (0..3)
            .map(|t| kind(&loaded.music.tracks[t].events))
            .collect::<Vec<_>>(),
        vec![Kind::Events, Kind::Notes, Kind::Empty]
    );
}

#[test]
fn reveal_scrolls_only_as_far_as_the_cell_needs() {
    let shown = egui::vec2(100.0, 50.0);
    let cell = |x: f32, y: f32| egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(40.0, 20.0));
    assert_eq!(
        window::reveal_offset(egui::vec2(0.0, 0.0), shown, cell(10.0, 10.0)),
        egui::vec2(0.0, 0.0)
    );
    assert_eq!(
        window::reveal_offset(egui::vec2(0.0, 0.0), shown, cell(200.0, 100.0)),
        egui::vec2(140.0, 70.0)
    );
    assert_eq!(
        window::reveal_offset(egui::vec2(300.0, 300.0), shown, cell(200.0, 100.0)),
        egui::vec2(200.0, 100.0)
    );
}

/// `color` as painted while the opening window still fades in.
fn faded(painted: egui::Color32, color: egui::Color32) -> bool {
    let opacity = f32::from(painted.a()) / 255.0;
    painted.a() > 0
        && (0..3).all(|i| (f32::from(painted[i]) - f32::from(color[i]) * opacity).abs() <= 2.0)
}

/// Lines in `color` that lie within the pane's height and start inside it.
fn lines_in(output: &egui::FullOutput, pane: egui::Rect, color: egui::Color32) -> usize {
    output
        .shapes
        .iter()
        .filter(|s| match &s.shape {
            egui::epaint::Shape::Path(path) => {
                matches!(path.stroke.color, egui::epaint::ColorMode::Solid(c) if faded(c, color))
                    && path.points.len() > 1
                    && pane.contains(path.points[0])
                    && path.points.iter().all(|p| pane.y_range().contains(p.y))
            }
            _ => false,
        })
        .count()
}

#[test]
fn conductor_shows_tempo_and_tracks_with_curves_show_them() {
    use cat_smf_editor::edit::parameter_curve::{self, Curve, Point, Target, ValueSpace};
    use cat_smf_editor::score::{
        macro_curve,
        macro_values::{self, Values, Variations},
    };
    let mut music = fixture().loaded.unwrap().music;
    let lane = |id: &str, value| Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: id.into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Cutoff".into(),
        path: "Filter".into(),
        points: vec![
            Point { tick: 0, value },
            Point {
                tick: 3840,
                value: 1.0 - value,
            },
        ],
        controls: vec![Point {
            tick: 1920,
            value: 0.5,
        }],
    };
    let control = parameter_curve::create(&mut music, 1, lane("cutoff", 0.2)).unwrap();
    let snapshot = |value| {
        let mut c = lane("macro", value);
        c.points.truncate(1);
        c.controls.clear();
        vec![c]
    };
    macro_values::write(
        &mut music.tracks[1],
        &Values {
            version: 1,
            parameters: snapshot(0.2),
            variations: vec![Variations {
                layer: 1,
                selected: 0,
                snapshots: vec![snapshot(0.2), snapshot(0.8)],
            }],
        },
    )
    .unwrap();
    let mut variation = lane("macro-variation", 0.0);
    variation.target.plugin_id = "CLAP:org.surge-synth-team.surge-xt".into();
    variation.points[1].value = 1.0;
    macro_curve::write(&mut music.tracks[1], &variation).unwrap();
    let (mut app, ctx) = opened_from(crate::view::tests::app(music));
    let output = render(&mut app, &ctx, vec![]);
    let tempo = crate::view::tempo_curve::TEMPO;
    let curve = crate::view::tempo_curve::CURVE;
    assert_eq!(lines_in(&output, pane(&ctx, 0), tempo), 1);
    assert_eq!(lines_in(&output, pane(&ctx, 1), curve), 1);
    assert_eq!(lines_in(&output, pane(&ctx, 2), curve), 0);
    assert_eq!(lines_in(&output, pane(&ctx, control), curve), 1);
}

#[test]
fn panes_draw_notes_in_the_track_color_with_measure_lines_only() {
    let (mut app, ctx) = opened();
    let output = render(&mut app, &ctx, vec![]);
    let pane = pane(&ctx, 1);
    let color = crate::view::track_color(1);
    let mut notes: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if pane.contains(r.rect.min)
                    && r.rect.height() < pane.height() / 2.0
                    && faded(r.fill, color) =>
            {
                Some(r)
            }
            _ => None,
        })
        .collect();
    assert_eq!(notes.len(), 4);
    notes.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
    let measure = notes[1].rect.left() - notes[0].rect.left();
    let lines: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::LineSegment { points, .. } if pane.contains(points[0]) => {
                Some(*points)
            }
            _ => None,
        })
        .collect();
    let edge = |y: f32| (y - pane.top()).abs() < 3.0 || (pane.bottom() - y).abs() < 3.0;
    assert!(
        lines.iter().all(|p| p[0].x == p[1].x || edge(p[0].y)),
        "no staff lines"
    );
    // Inside the first four measures, past the lane's left frame.
    let first = notes[0].rect.left();
    let within = lines
        .iter()
        .filter(|p| p[0].x == p[1].x)
        .filter(|p| (first + 1.0..first + 4.0 * measure - 1.0).contains(&p[0].x))
        .count();
    assert_eq!(within, 3, "one line per measure, none per beat");
}

/// Shapes far longer than the view lose precision and flicker while scrolling.
#[test]
fn pane_shapes_stay_within_the_visible_area() {
    let (mut app, ctx) = opened();
    let output = render(&mut app, &ctx, vec![]);
    let pane = pane(&ctx, 1);
    let view = pane.expand(2.0);
    for s in output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
    {
        let bounds = s.shape.visual_bounding_rect();
        if !bounds.is_finite() {
            continue;
        }
        assert!(
            view.contains_rect(bounds),
            "{bounds:?} reaches past {pane:?}"
        );
    }
}

#[test]
fn drum_notes_fade_after_their_first_thirty_second() {
    let mut app = fixture();
    app.loaded.as_mut().unwrap().patches[1].drums = Some(Default::default());
    let (mut app, ctx) = opened_from(app);
    let output = render(&mut app, &ctx, vec![]);
    let pane = pane(&ctx, 1);
    let color = crate::view::track_color(1);
    // Head and tail share the hue; the tail is fainter.
    let mut notes: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| pane.contains_rect(s.clip_rect))
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if r.rect.height() < pane.height() / 2.0 && faded(r.fill, color) =>
            {
                Some((r.rect, r.fill.a()))
            }
            _ => None,
        })
        .collect();
    notes.sort_by(|a, b| a.0.left().total_cmp(&b.0.left()));
    let head = notes.iter().map(|n| n.1).max().unwrap();
    let (heads, tails): (Vec<&(egui::Rect, u8)>, Vec<_>) = notes.iter().partition(|n| n.1 == head);
    let heads: Vec<_> = heads.iter().map(|n| n.0).collect();
    let tails: Vec<_> = tails.iter().map(|n| n.0).collect();
    assert_eq!((heads.len(), tails.len()), (4, 4));
    for (head, tail) in heads.iter().zip(&tails) {
        assert_eq!(head.right(), tail.left());
        // A 32nd of the 960-tick note.
        let ratio = head.width() / (head.width() + tail.width());
        assert!((ratio - 0.0625).abs() < 0.01, "{ratio}");
    }
}
