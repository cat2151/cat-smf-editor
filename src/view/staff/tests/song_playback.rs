//! The score and docked roll use the event list's pattern timeline during Song playback.
use crate::{
    app::App,
    play::{arrangement, playback, transport},
    view::{patterns::library, tests::unplaced},
};
use cat_smf_editor::score::{
    note_view::note_on,
    scene::{self, arrangement as grid, arrangement::PatternOrScene},
};
use eframe::egui::{self, epaint::Shape};

fn song() -> (App, u64) {
    let (mut a, id) = unplaced::fixture();
    a.edit_tracks(|m| {
        let lane = grid::add_lane(m, None)?;
        for measure in [1, 3, 5] {
            grid::place(m, None, lane, measure, Some(PatternOrScene::Pattern(id)))?;
        }
        Ok(())
    })
    .unwrap();
    library::edit(&mut a, id).unwrap();
    assert!(!scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .song
        .is_empty());
    arrangement::start(&mut a, None);
    transport::advance(&mut a);
    a.playback.follow = false;
    (a, id)
}

fn draw(a: &mut App, ctx: &egui::Context, roll: bool) -> Vec<f32> {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 700.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if roll {
                    crate::view::piano_roll::pane(a, ui);
                } else {
                    super::super::show(a, ui);
                }
            });
        },
    );
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke }
                if stroke.color == crate::view::timeline_repeat::PLAYHEAD
                    && stroke.width == 2.0 =>
            {
                Some(points[0].x)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn song_repetitions_keep_score_and_docked_roll_on_the_event_lists_pattern() {
    for roll in [false, true] {
        let (mut a, id) = song();
        let ctx = egui::Context::default();
        let mut first = Vec::new();
        for seconds in [0.5, 4.5, 4.75, 6.5] {
            a.audition.set_test_seconds(seconds);
            let local = if seconds == 4.75 { 720 } else { 480 };
            assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
            let tick = transport::display_tick_for(&a, a.track);
            assert_eq!(tick, (seconds < 6.0).then_some(local));
            let loaded = a.loaded.as_ref().unwrap();
            let notes = |events: &[cat_smf_editor::model::Event]| {
                events
                    .iter()
                    .filter(|e| note_on(e))
                    .map(|e| (e.tick, e.data[0]))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                notes(&playback::performance(&a)[a.track].events),
                notes(&loaded.visible().tracks[a.track].events)
            );
            assert_eq!(transport::marker_for(&a, a.track).is_some(), tick.is_some());
            // Two passes let the scroll area settle before comparing actual drawn markers.
            draw(&mut a, &ctx, roll);
            let markers = draw(&mut a, &ctx, roll);
            if seconds == 0.5 {
                assert!(!markers.is_empty(), "roll={roll}");
                first = markers;
            } else if seconds == 4.5 {
                assert_eq!(markers, first, "roll={roll}: repeated placement");
            } else if seconds == 4.75 {
                assert_eq!(markers.len(), first.len());
                assert!(markers.iter().zip(&first).all(|(now, before)| now > before));
            } else {
                assert!(markers.is_empty(), "roll={roll}: silent gap");
            }
        }
        transport::stop(&mut a);
        assert!(draw(&mut a, &ctx, roll).is_empty());
        assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    }
}

#[test]
#[ignore = "read-only local SMF verification; set PATTERN_DISPLAY_SMF and PATTERN_DISPLAY_ID"]
fn actual_song_keeps_the_selected_pattern_and_local_playhead_in_both_panes() {
    let roots = crate::store::config::directory()
        .unwrap()
        .join("cache/patch-roots-v1.json");
    let _roots = crate::player::patch_roots::Assumed::with_table(
        serde_json::from_slice(&std::fs::read(roots).unwrap()).unwrap(),
    );
    let path = std::path::PathBuf::from(std::env::var_os("PATTERN_DISPLAY_SMF").unwrap());
    let id = std::env::var("PATTERN_DISPLAY_ID")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    for roll in [false, true] {
        let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        a.state_cache =
            cat_smf_editor::score::state_cache::Cache::new(crate::store::config::directory());
        a.load_mid_unchecked(path.clone());
        assert!(a.load_error.is_none(), "{:?}", a.load_error);
        library::edit(&mut a, id).unwrap();
        let loaded = a.loaded.as_ref().unwrap();
        let (catalog, collection) = loaded.arrangement.as_ref().unwrap();
        let length = catalog.patterns[&id].length().unwrap();
        let occurrences = grid::occurrences_in(&loaded.music, catalog, collection, None)
            .unwrap()
            .0;
        let starts: Vec<_> = occurrences
            .into_iter()
            .filter_map(|(p, at)| (p == id).then_some(at))
            .collect();
        assert!(starts.len() >= 2, "select a repeated Song pattern");
        arrangement::start(&mut a, None);
        transport::advance(&mut a);
        a.playback.follow = false;
        let ctx = egui::Context::default();
        let mut first = None;
        for start in starts {
            let active = a.playback.active.as_ref().unwrap();
            let local = length.min(480) / 2;
            let seconds =
                active.tempos[active.clock_track].duration(0, start + local) - active.start_seconds;
            a.audition.set_test_seconds(seconds);
            assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
            assert!(
                transport::display_tick_for(&a, a.track)
                    .unwrap()
                    .abs_diff(local)
                    <= 1
            );
            assert!(transport::marker_for(&a, a.track).is_some());
            draw(&mut a, &ctx, roll);
            let markers = draw(&mut a, &ctx, roll);
            assert!(!markers.is_empty());
            if let Some(first) = &first {
                let first: &Vec<f32> = first;
                assert_eq!(markers.len(), first.len());
                assert!(markers
                    .iter()
                    .zip(first)
                    .all(|(now, before)| (now - before).abs() <= 1.0));
            } else {
                first = Some(markers);
            }
        }
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
}
