use super::*;

#[test]
fn every_tracks_notes_are_drawn_in_the_upper_piano_roll() {
    let mut app = app(with_curve());
    let (_, output) = opened_in(&mut app, 0);
    assert_eq!(piano_notes(&output), 4);
}

#[test]
fn the_curve_its_tempo_events_and_its_points_are_drawn() {
    let mut app = app(with_curve());
    let (_, output) = opened_in(&mut app, 0);
    let curve = paths(&output, CURVE);
    assert_eq!(curve.len(), 2);
    // Straight segments retain their endpoints without a fixed sampling grid.
    let line = &curve[1];
    assert!(line.windows(2).all(|w| w[1].x >= w[0].x));
    let tempos = tempo_count(&app);
    assert!(
        tempos > 2,
        "the curve renders more than the two original tempos"
    );
    let dots = circles(&output, TEMPO);
    assert_eq!(dots.len(), tempos);
    assert!(dots.iter().all(|&(_, r, filled)| filled && r == 2.0));
    let steps: Vec<_> = paths(&output, TEMPO)
        .into_iter()
        .filter(|points| {
            points
                .iter()
                .all(|p| p.y > app.tempo_curve.viewport.minimap_rect.unwrap().bottom())
        })
        .collect();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].len(), tempos * 2);
    // Each step stays flat until the next event.
    assert!(steps[0].chunks(2).all(|p| p[0].y == p[1].y));
    for (dot, step) in dots.iter().zip(steps[0].chunks(2)) {
        assert!((dot.0 - step[0]).length() < 0.01);
    }
    // Both kinds are directly draggable and filled.
    let points = circles(&output, POINT);
    assert_eq!(points.len(), 3);
    assert!(points.iter().all(|&(_, r, filled)| filled && r == 5.0));
    let controls = squares(&output, CONTROL);
    assert_eq!(controls.len(), 2);
    assert!(controls
        .iter()
        .all(|&(r, filled)| filled && r.width() == 8.0 && r.height() == 8.0));
    // The curve passes through each tempo point.
    for &(center, _, _) in &points {
        let nearest = line
            .iter()
            .min_by(|a, b| (a.x - center.x).abs().total_cmp(&(b.x - center.x).abs()))
            .unwrap();
        if (nearest.x - center.x).abs() < 0.01 {
            assert!(
                (nearest.y - center.y).abs() < 0.5,
                "{nearest:?} vs {center:?}"
            );
        }
    }
    // 100 BPM is the lowest point, so it is drawn lowest.
    let lowest = points.iter().map(|p| p.0.y).fold(f32::MIN, f32::max);
    assert_eq!(points[0].0.y, lowest);
}

#[test]
fn tempos_and_references_pin_to_both_edges_with_direction_marks() {
    let mut app = app(with_curve());
    let (ctx, _) = opened_in(&mut app, 0);
    let before = app.loaded.as_ref().unwrap().music.clone();
    app.tempo_curve.viewport.axis = Some(Axis {
        low: 115.0,
        high: 125.0,
    });
    let output = at(&mut app, &ctx, 11.0, vec![]);
    let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
    let points = circles(&output, POINT);
    assert_eq!(points[0].0.y, panel.bottom());
    assert_eq!(points[1].0.y, panel.top());
    assert!(points[2].0.y > panel.top() && points[2].0.y < panel.bottom());
    for color in [
        POINT,
        CONTROL,
        CURVE,
        TEMPO,
        super::super::references::COLOR,
    ] {
        let marks: Vec<_> = paths(&output, color)
            .into_iter()
            .filter(|p| p.len() == 3 && (p[0].x - p[1].x).abs() == 4.0)
            .collect();
        assert!(!marks.is_empty(), "missing edge markers for {color:?}");
        assert!(marks
            .iter()
            .all(|p| p.iter().all(|p| panel.y_range().contains(p.y))));
    }
    let marks = paths(&output, POINT);
    assert!(marks.iter().any(|p| p.len() == 3 && p[1].y < p[0].y));
    assert!(marks.iter().any(|p| p.len() == 3 && p[1].y > p[0].y));
    for clipped in &output.shapes {
        // The event list also shows BPM text, outside the panel's clip.
        if let Shape::Text(t) = &clipped.shape
            && clipped.clip_rect.intersects(panel)
            && t.galley.text().ends_with("BPM")
        {
            assert!(t.pos.y >= panel.top());
            assert!(t.pos.y + t.galley.size().y <= panel.bottom() + 0.1);
        }
    }
    assert_eq!(
        format!("{before:?}"),
        format!("{:?}", app.loaded.as_ref().unwrap().music)
    );
    // Hit testing uses the pinned location; drag offsets still use the true BPM position.
    let curve = tempo_curve::read(&before).unwrap().unwrap();
    let roll = super::super::Roll {
        rect: panel,
        scale: 0.1,
        axis: Axis {
            low: 115.0,
            high: 125.0,
        },
    };
    assert_eq!(
        roll.hit(&curve, roll.pinned(roll.pos(960.0, 140.0))),
        Some((super::super::Target::Point, 1))
    );
}
