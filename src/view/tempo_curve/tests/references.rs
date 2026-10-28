use super::edit::{hold, release};
use super::*;
use cat_smf_editor::edit::reference_tempo;

fn reference_labels(output: &egui::FullOutput, color: egui::Color32) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(t)
                if t.galley.text().ends_with("BPM")
                    && t.galley
                        .job
                        .sections
                        .iter()
                        .any(|s| s.format.color == color) =>
            {
                Some(t.galley.text().to_owned())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn dragging_reference_previews_rank_colors_and_keeps_tick_curve_and_tempos() {
    let mut app = app(music());
    let (ctx, output) = opened_in(&mut app, 0);
    let color = super::super::references::COLOR;
    let mismatch = super::super::references::MISMATCH;
    assert_eq!(reference_labels(&output, color), ["120.0 BPM", "150.0 BPM"]);
    let top = circles(&output, TEMPO)[1].0.y;
    assert!(output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.text() == "150.0 BPM" => Some(t.pos.y),
            _ => None,
        })
        .all(|y| y >= top));
    let before = app.loaded.as_ref().unwrap().music.clone();
    let original =
        reference_tempo::read(&super::super::trills::prepare(&mut app).unwrap()).unwrap();
    let generation = app.generation;
    let first = circles(&output, TEMPO)[0].0;
    // Drag horizontally as well: the guide must stay at tick 0.
    let from = first + egui::vec2(96.0, 0.0);
    let to = from + egui::vec2(20.0, -30.0);
    let held = hold(&mut app, &ctx, 11.0, from, to);
    assert_eq!(
        reference_tempo::read(&app.loaded.as_ref().unwrap().music).unwrap(),
        reference_tempo::read(&before).unwrap()
    );
    assert_eq!(app.generation, generation);
    assert_eq!(reference_labels(&held, color).len(), 2);
    release(&mut app, &ctx, 11.1, to);
    let music = &app.loaded.as_ref().unwrap().music;
    let changed = reference_tempo::read(music).unwrap();
    assert_eq!(changed[0].tick, 0);
    assert_eq!(changed[0].rank, original[0].rank);
    assert!(changed[0].bpm > original[0].bpm);
    assert_eq!(changed[1], original[1]);
    assert!(tempo_curve::read(music).unwrap().is_none());
    assert_eq!(
        super::super::tempo_events(app.loaded.as_ref().unwrap()),
        vec![(0, 120.0), (960, 150.0)]
    );
    // Cross the faster guide; both current ranks now differ from their stored ranks.
    let output = at(&mut app, &ctx, 12.0, vec![]);
    let guide = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t)
                if t.galley.text().ends_with("BPM")
                    && t.galley
                        .job
                        .sections
                        .iter()
                        .any(|s| s.format.color == color) =>
            {
                Some(egui::pos2(first.x + 96.0, t.pos.y + t.galley.size().y))
            }
            _ => None,
        })
        .unwrap();
    let target = guide
        - egui::vec2(
            0.0,
            app.tempo_curve.viewport.tempo_rect.unwrap().height() * 1.2,
        );
    let held = hold(&mut app, &ctx, 13.0, guide, target);
    assert_eq!(reference_labels(&held, mismatch).len(), 2);
    release(&mut app, &ctx, 13.1, target);
    let changed = reference_tempo::read(&app.loaded.as_ref().unwrap().music).unwrap();
    assert!(changed[0].bpm > changed[1].bpm);
    assert_eq!(changed[0].rank, 2);
    crate::view::piano_roll::tests::press(&mut app, &ctx, egui::Key::R);
    crate::view::piano_roll::tests::press(&mut app, &ctx, egui::Key::R);
    assert_eq!(
        reference_tempo::read(&app.loaded.as_ref().unwrap().music).unwrap(),
        changed
    );
}

#[test]
fn reference_drag_with_a_curve_preserves_all_performance_events() {
    let mut song = music();
    cat_smf_editor::edit::initial_meta::initialize(&mut song).unwrap();
    reference_tempo::initialize(&mut song).unwrap();
    let curve = tempo_curve::read(&with_curve()).unwrap().unwrap();
    tempo_curve::write(&mut song, Some(&curve)).unwrap();
    tempo_curve::regenerate(&mut song, &curve).unwrap();
    let mut app = app(song);
    let (ctx, output) = opened_in(&mut app, 0);
    let first = circles(&output, POINT)[0].0;
    // The last curve point shares the first guide's 120 BPM height.
    let last = circles(&output, POINT)[2].0;
    let from = egui::pos2(first.x + 48.0, last.y);
    let before = app.loaded.as_ref().unwrap().music.clone();
    let to = from + egui::vec2(20.0, -20.0);
    hold(&mut app, &ctx, 11.0, from, to);
    release(&mut app, &ctx, 11.1, to);
    let after = &app.loaded.as_ref().unwrap().music;
    assert_ne!(
        reference_tempo::read(after).unwrap(),
        reference_tempo::read(&before).unwrap()
    );
    assert_eq!(tempo_curve::read(after).unwrap(), Some(curve));
    let without_references = |music: &Music| {
        let mut music = music.clone();
        music.tracks[0]
            .events
            .retain(|e| !reference_tempo::owned(e));
        cat_smf_editor::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
        format!("{:?}", music.tracks)
    };
    assert_eq!(without_references(after), without_references(&before));
}

#[test]
fn horizontal_reference_drag_does_not_save_or_split_equal_ranks() {
    let mut song = music();
    song.tracks[0].events[1].data = 500_000u32.to_be_bytes()[1..].to_vec();
    let mut app = app(song);
    let (ctx, output) = opened_in(&mut app, 0);
    let first = circles(&output, TEMPO)[0].0;
    let from = first + egui::vec2(48.0, 0.0);
    let before = app.generation;
    hold(&mut app, &ctx, 11.0, from, from + egui::vec2(40.0, 0.0));
    release(&mut app, &ctx, 11.1, from + egui::vec2(40.0, 0.0));
    assert_eq!(app.generation, before);
    assert!(reference_tempo::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .is_empty());
    let points = reference_tempo::read(&super::super::trills::prepare(&mut app).unwrap()).unwrap();
    assert_eq!(reference_tempo::ranks(&points), [1, 1]);
}
