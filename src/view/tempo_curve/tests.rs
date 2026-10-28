use super::{Axis, CONTROL, CURVE, GRID, GRID_LABEL, LABEL_FONT, POINT, TEMPO};
use crate::view::{
    note_edit::tests::render,
    piano_roll::tests::{at, press},
    tests::app,
};
use cat_smf_editor::{
    edit::tempo_curve::{self, Curve, Point},
    model::{Event, Music, Track},
};
use eframe::egui::{
    self,
    epaint::{ColorMode, Shape},
};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

fn tempo(tick: u64, micros: u32) -> Event {
    event(tick, 0xff, 0x51, &micros.to_be_bytes()[1..])
}

fn notes(list: &[(u64, u64, u8)]) -> Track {
    let mut events: Vec<Event> = list
        .iter()
        .flat_map(|&(tick, duration, key)| {
            [
                event(tick, 0x90, 0, &[key, 80]),
                event(tick + duration, 0x80, 0, &[key, 0]),
            ]
        })
        .collect();
    events.sort_by_key(|e| e.tick);
    events.push(event(3840, 0xff, 0x2f, &[]));
    Track { events }
}

/// Conductor at 120 then 150 BPM, and two tracks of two notes each, at 480 PPQN.
fn music() -> Music {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    tempo(0, 500_000),
                    tempo(960, 400_000),
                    event(3840, 0xff, 0x2f, &[]),
                ],
            },
            notes(&[(0, 480, 60), (480, 480, 64)]),
            notes(&[(0, 960, 67), (960, 960, 72)]),
        ],
    };
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    music
}

fn with_curve() -> Music {
    let mut music = music();
    let curve = Curve {
        version: 1,
        points: vec![
            Point {
                tick: 0,
                bpm: 100.0,
            },
            Point {
                tick: 960,
                bpm: 140.0,
            },
            Point {
                tick: 1920,
                bpm: 120.0,
            },
        ],
        controls: vec![
            Point {
                tick: 480,
                bpm: 120.0,
            },
            Point {
                tick: 1440,
                bpm: 130.0,
            },
        ],
    };
    tempo_curve::write(&mut music, Some(&curve)).unwrap();
    tempo_curve::regenerate(&mut music, &curve).unwrap();
    music
}

fn tempo_count(app: &crate::app::App) -> usize {
    super::tempo_events(app.loaded.as_ref().unwrap()).len()
}

/// Presses `r` on `track` and returns the context and a frame after the window's fade-in.
fn opened_in(app: &mut crate::app::App, track: usize) -> (egui::Context, egui::FullOutput) {
    app.select_track(track);
    let ctx = egui::Context::default();
    render(app, &ctx, vec![]);
    press(app, &ctx, egui::Key::R);
    at(app, &ctx, 0.0, vec![]);
    let output = at(app, &ctx, 10.0, vec![]);
    (ctx, output)
}

fn color_of(stroke: &egui::epaint::PathStroke) -> Option<egui::Color32> {
    match stroke.color {
        ColorMode::Solid(color) => Some(color),
        ColorMode::UV(_) => None,
    }
}

/// Grid labels: text in the label color and font, since other windows share the color.
fn labels(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Text(t)
                if t.fallback_color == GRID_LABEL
                    && t.galley.job.sections[0].format.font_id == LABEL_FONT =>
            {
                Some(t.galley.text().to_owned())
            }
            _ => None,
        })
        .collect()
}

/// Points of every open path stroked in `color`.
fn paths(output: &egui::FullOutput, color: egui::Color32) -> Vec<Vec<egui::Pos2>> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Path(path) if color_of(&path.stroke) == Some(color) => Some(path.points.clone()),
            _ => None,
        })
        .collect()
}

/// Circles in `color`: (center, radius, filled).
fn circles(output: &egui::FullOutput, color: egui::Color32) -> Vec<(egui::Pos2, f32, bool)> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Circle(circle) if circle.fill == color => {
                Some((circle.center, circle.radius, true))
            }
            Shape::Circle(circle) if circle.stroke.color == color => {
                Some((circle.center, circle.radius, false))
            }
            _ => None,
        })
        .collect()
}

/// Rectangles in `color`: (rect, filled).
fn squares(output: &egui::FullOutput, color: egui::Color32) -> Vec<(egui::Rect, bool)> {
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Rect(rect) if rect.fill == color => Some((rect.rect, true)),
            Shape::Rect(rect) if rect.stroke.color == color => Some((rect.rect, false)),
            _ => None,
        })
        .collect()
}

fn note_rects(output: &egui::FullOutput) -> Vec<(egui::Rect, bool)> {
    let colors: Vec<_> = [60, 64, 67]
        .into_iter()
        .map(|pitch| crate::view::pitch_keyboard::roll_color(pitch, None))
        .collect();
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Rect(r)
                if colors.contains(&r.fill) && r.corner_radius == egui::CornerRadius::ZERO =>
            {
                Some((r.rect, true))
            }
            _ => None,
        })
        .collect()
}

fn piano_notes(output: &egui::FullOutput) -> usize {
    note_rects(output)
        .iter()
        .filter(|(_, filled)| *filled)
        .count()
}

#[test]
fn r_opens_the_tempo_curve_only_on_the_conductor_of_format_1() {
    let mut app = app(music());
    let ctx = egui::Context::default();
    render(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.tempo_curve.open);
    assert!(!app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::T);
    assert!(!app.player_selection.open);
    press(&mut app, &ctx, egui::Key::R);
    assert!(!app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::R);
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    press(&mut app, &ctx, egui::Key::V);
    assert!(!app.note_mode && !app.tempo_curve.open);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    press(&mut app, &ctx, egui::Key::V);
    assert!(app.note_mode);

    app.select_track(1);
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.piano_roll.open);
    assert!(!app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::Escape);

    let mut single = music();
    single.format = 0;
    single.tracks.truncate(1);
    let mut format_0 = crate::view::tests::app(single);
    let ctx = egui::Context::default();
    render(&mut format_0, &ctx, vec![]);
    press(&mut format_0, &ctx, egui::Key::R);
    assert!(format_0.piano_roll.open);
    assert!(!format_0.tempo_curve.open);
}

#[test]
fn without_a_curve_only_the_existing_tempo_events_are_drawn() {
    let mut app = app(music());
    let (_, output) = opened_in(&mut app, 0);
    let dots = circles(&output, TEMPO);
    assert_eq!(dots.len(), 2);
    assert!(paths(&output, CURVE).is_empty());
    assert!(circles(&output, POINT).is_empty());
    assert!(squares(&output, CONTROL).is_empty());
    // 150 BPM sits above 120 BPM.
    assert!(dots[1].0.y < dots[0].0.y);
    assert_eq!(piano_notes(&output), 4);
}

#[test]
fn bpm_grid_lines_every_10_bpm_carry_only_their_number() {
    let mut app = app(music());
    let (_, output) = opened_in(&mut app, 0);
    // Tempos 120 and 150 are the exact axis bounds.
    assert_eq!(labels(&output), ["120", "130", "140", "150"]);
    // The tempo at tick 0 marks the roll's left edge, where the grid lines start.
    let dots = circles(&output, TEMPO);
    let left = dots[0].0.x;
    let lines: Vec<f32> = output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::LineSegment { points, stroke }
                if points[0].y == points[1].y
                    && stroke.color == GRID
                    && (points[0].x - left).abs() < 0.01 =>
            {
                Some(points[0].y)
            }
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 4);
    // Linear: equal spacing, higher BPM higher up.
    let gap = lines[0] - lines[1];
    assert!(gap > 0.0);
    assert!(lines.windows(2).all(|w| ((w[0] - w[1]) - gap).abs() < 0.01));
    // The 120 and 150 BPM tempo dots sit on their grid lines.
    assert!((dots[0].0.y - lines[0]).abs() < 0.01);
    assert!((dots[1].0.y - lines[3]).abs() < 0.01);
}

#[test]
fn the_axis_uses_song_bounds_and_handles_constant_or_missing_tempo() {
    let near = |axis: Axis, low: f64, high: f64| {
        assert!(
            (axis.low - low).abs() < 1e-9 && (axis.high - high).abs() < 1e-9,
            "{axis:?}"
        )
    };
    near(Axis::new([120.0, 150.0].into_iter()), 120.0, 150.0);
    // A constant tempo gets a nonzero editing span.
    near(Axis::new([50.0].into_iter()), 49.0, 51.0);
    // No tempo at all means the SMF default of 120 BPM.
    near(Axis::new(std::iter::empty()), 119.0, 121.0);
}

#[test]
fn hovering_either_kind_draws_it_larger_without_c() {
    let mut app = app(with_curve());
    let (ctx, output) = opened_in(&mut app, 0);
    let second = circles(&output, POINT)[1].0;
    at(
        &mut app,
        &ctx,
        11.0,
        vec![egui::Event::PointerMoved(second + egui::vec2(3.0, 0.0))],
    );
    let hovered = at(&mut app, &ctx, 11.0, vec![]);
    let radii: Vec<f32> = circles(&hovered, POINT).iter().map(|c| c.1).collect();
    assert_eq!(radii, [5.0, 7.5, 5.0]);
    // Control points are directly accessible without a keyboard mode.
    let control = squares(&hovered, CONTROL)[0].0.center();
    at(
        &mut app,
        &ctx,
        12.0,
        vec![egui::Event::PointerMoved(control)],
    );
    let other = at(&mut app, &ctx, 12.0, vec![]);
    assert_eq!(squares(&other, CONTROL)[0].0.width(), 12.0);
    assert_eq!(squares(&other, CONTROL)[1].0.width(), 8.0);
}

#[test]
fn play_overlay_opens_over_the_curve_and_owns_its_keys() {
    use crate::play::playback::Mode;

    let mut app = app(music());
    let (ctx, _) = opened_in(&mut app, 0);
    press(&mut app, &ctx, egui::Key::P);
    assert!(app.playback.open);
    assert!(app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.playback.open);
    assert!(app.tempo_curve.open);

    press(&mut app, &ctx, egui::Key::P);
    press(&mut app, &ctx, egui::Key::C);
    assert_eq!(app.playback.last, Mode::AllTracksFromMeasure);
    assert!(app.tempo_curve.open);
    assert!(!app.playback.open);
    // Avoid opening an audio device on the next frame.
    app.playback.pending = None;

    press(&mut app, &ctx, egui::Key::P);
    press(&mut app, &ctx, egui::Key::M);
    assert_eq!(app.playback.last, Mode::Measure);
    assert!(!app.playback.mixer_open);
    let pending = app.playback.pending.take().unwrap();
    assert_eq!(
        pending
            .parts
            .iter()
            .filter(|(_, _, s)| !s.notes.is_empty())
            .count(),
        2
    );
    assert!(app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::Space);
    assert_eq!(app.playback.last, Mode::Measure);
    assert!(app.playback.pending.take().is_some());
    press(&mut app, &ctx, egui::Key::C);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.tempo_curve.open);
}

mod collision_delete;
mod edit;

mod viewport;

mod references;

mod gestures;

mod follow;

mod note_guides;

mod wheel;

mod minimap;

mod rendering;

mod trills;

mod deferred;
mod grid;
mod initial_meta;
mod note_references;
