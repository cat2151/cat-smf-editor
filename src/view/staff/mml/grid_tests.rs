use super::*;

#[test]
fn polyphonic_track_has_one_colored_heading_and_contiguous_high_first_rows() {
    let mut music = fixture().loaded.unwrap().music;
    music.tracks[1].events = [48, 72, 84]
        .into_iter()
        .flat_map(|pitch| {
            [(0, 0x90, 80), (1920, 0x80, 0)].map(|(tick, status, velocity)| Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![pitch, velocity],
            })
        })
        .collect();
    music.tracks[1].events.sort_by_key(|e| e.tick);
    let mut app = crate::view::tests::app(music);
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    let labels: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("track") => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        labels
            .iter()
            .map(|t| t.galley.job.text.as_str())
            .collect::<Vec<_>>(),
        ["track2", "track3"]
    );
    assert_eq!(
        labels[0].galley.job.sections[0].format.color,
        crate::view::track_color(1)
    );
    assert_eq!(
        labels[1].galley.job.sections[0].format.color,
        crate::view::track_color(2)
    );
    assert_ne!(crate::view::track_color(1), crate::view::track_color(2));
    assert_eq!(labels[1].pos.y - labels[0].pos.y, LINE);
    assert_eq!(mml(&output).len(), 2);
    assert_eq!(mml(&output)[0].galley.job.text, "c1");
}

#[test]
fn track_labels_remain_when_the_measure_has_no_notes() {
    let mut app = fixture();
    crate::cursor::split::go(&mut app, 0, 3840);
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    let labels: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("track") => {
                Some(t.galley.job.text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["track2", "track3"]);
    assert!(mml(&output).is_empty());
    assert!(!output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "r1")));
}

#[test]
fn quarter_notes_start_at_four_even_columns_with_the_existing_beat_colors() {
    let mut music = fixture().loaded.unwrap().music;
    music.tracks[1].events = [60, 62, 64, 65]
        .into_iter()
        .enumerate()
        .flat_map(|(beat, pitch)| {
            [
                (beat as u64 * 480, 0x90, 80),
                ((beat as u64 + 1) * 480, 0x80, 0),
            ]
            .map(|(tick, status, velocity)| Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![pitch, velocity],
            })
        })
        .collect();
    let mut app = crate::view::tests::app(music);
    let ctx = egui::Context::default();
    render(&mut app, &ctx);
    let output = render(&mut app, &ctx);
    let label = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "track2" => Some(t),
            _ => None,
        })
        .unwrap();
    let cells: Vec<_> = mml(&output)
        .into_iter()
        .filter(|t| t.pos.y == label.pos.y)
        .collect();
    assert_eq!(
        cells
            .iter()
            .map(|t| t.galley.job.text.as_str())
            .collect::<Vec<_>>(),
        ["l4 c", "d", "e", "f"]
    );
    assert!(label.pos.x < cells[0].pos.x);
    let width = cells[1].pos.x - cells[0].pos.x;
    assert!(width > 0.0);
    for (beat, text) in cells.iter().enumerate() {
        assert!((text.pos.x - cells[0].pos.x - width * beat as f32).abs() < 0.1);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == crate::view::beat_color(beat as u128 + 1)
                    && (points[0].x - (text.pos.x - PAD)).abs() < 0.1
                    && points[0].y <= text.pos.y && points[1].y >= text.pos.y + 14.0)));
    }
}
