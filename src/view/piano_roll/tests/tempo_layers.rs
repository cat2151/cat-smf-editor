use super::navigation::{click, zone};
use super::*;
use crate::view::{
    tempo_curve::{CURVE, TEMPO},
    timeline_minimap::tempos::{REFERENCE, REFERENCE_MISMATCH},
};
use cat_smf_editor::edit::{
    reference_tempo,
    tempo_curve::{self, Curve, Point},
};

type EventSnapshot = (u64, u8, u8, Vec<u8>);

fn fingerprint(app: &crate::app::App) -> Vec<Vec<EventSnapshot>> {
    app.loaded
        .as_ref()
        .unwrap()
        .music
        .tracks
        .iter()
        .map(|t| {
            t.events
                .iter()
                .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
                .collect()
        })
        .collect()
}

fn lines(output: &egui::FullOutput, map: egui::Rect, color: egui::Color32) -> Vec<Vec<egui::Pos2>> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke }
                if stroke.color == color && points.iter().all(|p| map.contains(*p)) =>
            {
                Some(points.to_vec())
            }
            Shape::Path(path)
                if matches!(path.stroke.color, egui::epaint::ColorMode::Solid(c) if c == color)
                    && path.points.iter().all(|p| map.contains(*p)) =>
            {
                Some(path.points.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn minimap_draws_tempo_curve_and_reference_layers_without_writing_and_is_independent_of_navigation()
{
    let source = super::split::fixture();
    let mut music = source.loaded.unwrap().music;
    let mut tempo = event(960, 0xff, vec![0x06, 0x1a, 0x80]);
    tempo.meta_type = 0x51;
    music.tracks[0].events.insert(0, tempo);
    cat_smf_editor::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
    reference_tempo::initialize(&mut music).unwrap();
    let curve = Curve {
        version: 1,
        points: vec![
            Point {
                tick: 0,
                bpm: 100.0,
            },
            Point {
                tick: 1920,
                bpm: 180.0,
            },
        ],
        controls: vec![Point {
            tick: 960,
            bpm: 110.0,
        }],
    };
    tempo_curve::write(&mut music, Some(&curve)).unwrap();
    tempo_curve::regenerate(&mut music, &curve).unwrap();
    let mut app = app(music);
    app.select_track(1);
    let before = fingerprint(&app);
    let (ctx, output) = opened_in(&mut app);
    let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
    assert_eq!(fingerprint(&app), before);
    for color in [TEMPO, CURVE, REFERENCE] {
        assert!(!lines(&output, map, color).is_empty());
    }
    let layers = [TEMPO, CURVE, REFERENCE].map(|c| lines(&output, map, c));
    let pos = zone(&app, 0);
    click(&mut app, &ctx, 11.0, pos, false);
    let pos = zone(&app, 1);
    click(&mut app, &ctx, 12.0, pos, false);
    app.piano_roll.viewport.pending = Some(egui::vec2(400.0, 20.0));
    let output = at(&mut app, &ctx, 13.0, vec![]);
    assert_eq!(
        [TEMPO, CURVE, REFERENCE].map(|c| lines(&output, map, c)),
        layers
    );
    reference_tempo::set(&mut app.loaded.as_mut().unwrap().music, 0, 200.0).unwrap();
    let output = at(&mut app, &ctx, 14.0, vec![]);
    assert!(!lines(&output, map, REFERENCE_MISMATCH).is_empty());
}

#[test]
fn opening_and_navigating_a_song_without_reference_metadata_does_not_initialize_it() {
    let mut app = super::split::fixture();
    let before = fingerprint(&app);
    let (ctx, _) = opened_in(&mut app);
    let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
    click(&mut app, &ctx, 11.0, map.center(), false);
    assert_eq!(fingerprint(&app), before);
    assert!(reference_tempo::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .is_empty());
}

#[test]
fn curve_points_beyond_track_ends_remain_reachable_on_the_shared_timeline() {
    let mut music = super::split::fixture().loaded.unwrap().music;
    let curve = Curve {
        version: 1,
        points: vec![
            Point {
                tick: 0,
                bpm: 100.0,
            },
            Point {
                tick: 30000,
                bpm: 180.0,
            },
        ],
        controls: vec![Point {
            tick: 15000,
            bpm: 120.0,
        }],
    };
    tempo_curve::write(&mut music, Some(&curve)).unwrap();
    let mut app = app(music);
    app.select_track(1);
    let (ctx, _) = opened_in(&mut app);
    let map = app.piano_roll.viewport.minimap_state.rect.unwrap();
    click(
        &mut app,
        &ctx,
        11.0,
        egui::pos2(map.left() + map.width() * 0.8, map.center().y),
        false,
    );
    let panel = app.piano_roll.viewport.panel.unwrap();
    assert!(
        (app.piano_roll.viewport.scroll.x + panel.width() / 2.0 - (30000.0 * 0.2 + 96.0) * 0.8)
            .abs()
            < 0.1
    );
    assert_eq!(
        tempo_curve::read(&app.loaded.as_ref().unwrap().music).unwrap(),
        Some(curve)
    );
}
