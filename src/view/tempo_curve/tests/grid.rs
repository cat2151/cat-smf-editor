use super::*;

fn horizontal_grid(output: &egui::FullOutput) -> Vec<([egui::Pos2; 2], egui::Stroke)> {
    let lines: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::LineSegment { points, stroke }
                if points[0].y == points[1].y
                    && points[0].y > 100.0
                    && [crate::view::piano_roll::STAFF_COLOR, GRID].contains(&stroke.color) =>
            {
                Some((*points, *stroke))
            }
            _ => None,
        })
        .collect();
    for color in [crate::view::piano_roll::STAFF_COLOR, GRID] {
        assert!(lines.iter().any(|(_, stroke)| stroke.color == color));
    }
    lines
}

#[test]
fn empty_tracks_staff_and_bpm_grid_stay_fixed_when_scrolling_past_measure_five() {
    for mut music in [music(), with_curve()] {
        for track in &mut music.tracks {
            track.events.last_mut().unwrap().tick = 15360;
            cat_smf_editor::edit::refresh_deltas(track).unwrap();
        }
        let mut app = app(music);
        let (ctx, _) = opened_in(&mut app, 0);
        assert!(app.tempo_curve.open && !app.piano_roll.open);
        app.playback.follow = false;
        app.tempo_curve.viewport.pending_scroll = Some(500.0);
        let expected = horizontal_grid(&at(&mut app, &ctx, 11.0, vec![]));
        for (i, offset) in [500.2, 1400.3, 1535.8, 1536.2, 1600.7]
            .into_iter()
            .enumerate()
        {
            app.tempo_curve.viewport.pending_scroll = Some(offset);
            let output = at(&mut app, &ctx, 12.0 + i as f64, vec![]);
            assert!((app.tempo_curve.viewport.scroll - offset).abs() < 0.001);
            assert_eq!(horizontal_grid(&output), expected, "scroll {offset}");
        }
    }
}
