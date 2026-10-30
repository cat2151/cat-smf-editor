use crate::view::tests::app;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

fn note(tick: u64, pitch: u8, length: u64) -> [Event; 2] {
    [(0x90, 100, 0), (0x80, 0, length)].map(|(status, velocity, offset)| Event {
        delta: 0,
        tick: tick + offset,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    })
}

/// A conductor track shows the other tracks' notes in the minimap too, as the score does.
#[test]
fn noteless_track_minimap_shows_other_tracks() {
    let tempo = Event {
        delta: 0,
        tick: 0,
        status: 0xff,
        meta_type: 0x51,
        data: vec![0x07, 0xa1, 0x20],
    };
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![tempo],
            },
            Track {
                events: (0..8u64)
                    .flat_map(|i| note(i * 480, 60 + i as u8, 240))
                    .collect(),
            },
        ],
    });
    let ctx = egui::Context::default();
    let mut output = None;
    for _ in 0..2 {
        output = Some(ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| crate::view::show(&mut app, ctx),
        ));
    }
    let minimap = app.staff.viewport.minimap_state.rect.expect("minimap");
    let notes = output
        .unwrap()
        .shapes
        .into_iter()
        .filter(|s| {
            matches!(&s.shape, egui::epaint::Shape::Rect(r)
                if minimap.contains_rect(r.rect) && r.rect.width() < minimap.width() / 2.0)
        })
        .count();
    assert_eq!(notes, 8);
}

fn arpeggio() -> crate::app::App {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            Track {
                events: (0..8u64)
                    .flat_map(|i| note(i * 480, 60 + i as u8, 240))
                    .collect(),
            },
        ],
    });
    app.track = 1;
    app.playback.follow = false;
    app
}

/// A point on the staff body, right of the clefs and below the A–B row.
fn on_staff(app: &crate::app::App) -> egui::Pos2 {
    let minimap = app.staff.viewport.minimap_state.rect.expect("minimap");
    egui::pos2(
        minimap.left() + minimap.width() * 0.6,
        minimap.bottom() + crate::view::timeline_repeat::HEIGHT + 80.0,
    )
}

#[test]
fn stopped_staff_click_plays_from_the_clicked_beat() {
    use crate::view::piano_roll::tests::at;
    let mut app = arpeggio();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    at(&mut app, &ctx, 10.1, vec![]);
    let pos = on_staff(&app);
    click(&mut app, &ctx, 11.0, pos);
    assert!(crate::play::transport::timeline::running(&app));
    let tick = crate::play::playback::location(&app).0;
    assert!(tick > 0);
    assert_eq!(tick % 480, 0, "stopped clicks start at the beat");
}

#[test]
fn playing_staff_click_restarts_at_the_clicked_tick() {
    use crate::play::{playback, transport};
    use crate::view::piano_roll::tests::at;
    use cat_smf_editor::score::audition_timing::TempoMap;
    let mut app = arpeggio();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    at(&mut app, &ctx, 10.1, vec![]);
    playback::start(&mut app, playback::Mode::AllTracks);
    transport::advance(&mut app);
    let pos = on_staff(&app);
    click(&mut app, &ctx, 11.0, pos);
    let tick = playback::location(&app).0;
    assert!(tick > 0);
    let map = TempoMap::new(&app.loaded.as_ref().unwrap().music, 0);
    let active = app.playback.active.as_ref().unwrap();
    assert!((active.start_seconds - map.duration(0, tick)).abs() < 1e-8);
}

fn click(app: &mut crate::app::App, ctx: &egui::Context, time: f64, pos: egui::Pos2) {
    use crate::view::piano_roll::tests::at;
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(app, ctx, time + 0.01, vec![button(true)]);
    at(app, ctx, time + 0.02, vec![button(false)]);
    at(app, ctx, time + 0.03, vec![]);
}

#[test]
fn minimap_click_and_drag_while_playing_stop_follow_and_the_click_seeks() {
    use crate::play::{playback, transport};
    use crate::view::piano_roll::tests::at;
    let mut app = arpeggio();
    app.playback.follow = true;
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    at(&mut app, &ctx, 10.1, vec![]);
    playback::start(&mut app, playback::Mode::AllTracks);
    transport::advance(&mut app);
    let minimap = app.staff.viewport.minimap_state.rect.expect("minimap");
    click(&mut app, &ctx, 11.0, minimap.center());
    assert!(!app.playback.follow);
    assert!(playback::location(&app).0 > 0, "the click seeks");
    app.playback.follow = true;
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let (a, b) = (minimap.center(), minimap.center() + egui::vec2(60.0, 0.0));
    at(&mut app, &ctx, 12.0, vec![egui::Event::PointerMoved(a)]);
    at(&mut app, &ctx, 12.01, vec![button(a, true)]);
    at(&mut app, &ctx, 12.2, vec![egui::Event::PointerMoved(b)]);
    at(&mut app, &ctx, 12.3, vec![button(b, false)]);
    assert!(!app.playback.follow);
}

/// Left edges of the note heads drawn on the staff, ascending: each filled head is a
/// 20-point ellipse whose center lies `0.62` gaps right of the note's x.
fn head_xs(app: &crate::app::App, output: &egui::FullOutput) -> Vec<f32> {
    let gap = super::GAP * app.staff.viewport.zoom().y;
    let mut xs: Vec<f32> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Path(p) if p.closed && p.points.len() == 20 => {
                Some(p.points.iter().map(|q| q.x).sum::<f32>() / 20.0 - 0.62 * gap)
            }
            _ => None,
        })
        .collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    xs
}

/// Dragging the A–B row from one note head to another repeats exactly those notes' ticks,
/// also for notes inside a measure, and the drawn A and B lines sit on those heads.
#[test]
fn repeat_row_drag_between_note_heads_selects_their_ticks() {
    use crate::view::piano_roll::tests::at;
    let mut app = arpeggio();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    let output = at(&mut app, &ctx, 10.1, vec![]);
    let heads = head_xs(&app, &output);
    assert!(heads.len() >= 7, "{heads:?}");
    let minimap = app.staff.viewport.minimap_state.rect.expect("minimap");
    let y = minimap.bottom() + crate::view::timeline_repeat::HEIGHT / 2.0;
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    for (i, (t, u)) in [(1usize, 5usize), (2, 3), (0, 6)].into_iter().enumerate() {
        let time = 11.0 + i as f64;
        let (a, b) = (egui::pos2(heads[t], y), egui::pos2(heads[u], y));
        at(&mut app, &ctx, time, vec![egui::Event::PointerMoved(a)]);
        at(&mut app, &ctx, time + 0.01, vec![button(a, true)]);
        at(
            &mut app,
            &ctx,
            time + 0.2,
            vec![egui::Event::PointerMoved(b)],
        );
        at(&mut app, &ctx, time + 0.3, vec![button(b, false)]);
        let output = at(&mut app, &ctx, time + 0.4, vec![]);
        assert_eq!(app.playback.repeat, Some((t as u64 * 480, u as u64 * 480)));
        let lines: Vec<f32> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::LineSegment { points, stroke }
                    if stroke.color == crate::view::timeline_repeat::RANGE
                        && points[0].y >= minimap.bottom()
                        && points[1].y <= y + crate::view::timeline_repeat::HEIGHT =>
                {
                    Some(points[0].x)
                }
                _ => None,
            })
            .collect();
        for x in [heads[t], heads[u]] {
            assert!(
                lines.iter().any(|l| (l - x).abs() <= 1.0),
                "line at {x} in {lines:?}"
            );
        }
    }
}

/// Sixteen 4/4 measures of four sixteenths and a long rest, so the engraved x is far from
/// linear in ticks.
fn sixteenths() -> crate::app::App {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            Track {
                events: (0..64u64)
                    .flat_map(|i| note(i / 4 * 1920 + i % 4 * 120, 60 + (i % 12) as u8, 120))
                    .collect(),
            },
        ],
    });
    app.track = 1;
    app
}

/// The engraved x the staff draws `sixteenths()` with at zoom 1.
fn sixteenths_spacing(app: &crate::app::App) -> super::spacing::Spacing {
    let loaded = app.loaded.as_ref().unwrap();
    let onsets: Vec<u64> = (0..64u64).map(|i| i / 4 * 1920 + i % 4 * 120).collect();
    assert_eq!(app.staff.viewport.zoom(), egui::Vec2::splat(1.0));
    super::spacing::Spacing::new(
        super::spacing::bars(loaded, 1, 40_000, 480),
        &onsets,
        crate::view::piano_roll::layout(loaded, 1, 0).scale,
        super::GAP,
    )
}

/// x on the minimap of `tick` and back, read from the minimap's drawn notes of `sixteenths()`.
fn minimap_axis(
    output: &egui::FullOutput,
    minimap: egui::Rect,
) -> (impl Fn(u64) -> f32, impl Fn(f32) -> u64) {
    let lefts: Vec<f32> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if minimap.contains_rect(r.rect) && r.rect.height() < minimap.height() / 2.0 =>
            {
                Some(r.rect.left())
            }
            _ => None,
        })
        .collect();
    assert_eq!(lefts.len(), 64);
    let first = lefts.iter().copied().fold(f32::MAX, f32::min);
    let per_tick = (lefts.iter().copied().fold(f32::MIN, f32::max) - first) / 29_160.0;
    (
        move |tick| first + tick as f32 * per_tick,
        move |x| ((x - first) / per_tick).round() as u64,
    )
}

/// Clicking the minimap centers the clicked tick's engraved x, while stopped and, from the
/// frame of the click, while playing; the minimap frame spans the ticks at the staff's left
/// and right edges.
#[test]
fn minimap_click_centers_the_clicked_tick_and_frames_the_shown_ticks() {
    use crate::play::{playback, transport};
    use crate::view::piano_roll::tests::at;
    let mut app = sixteenths();
    app.playback.follow = false;
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    let output = at(&mut app, &ctx, 10.1, vec![]);
    let spacing = sixteenths_spacing(&app);
    let minimap = app.staff.viewport.minimap_state.rect.expect("minimap");
    let (minimap_x, tick_at) = minimap_axis(&output, minimap);
    for (i, (fraction, playing)) in [(0.37f32, false), (0.71, false), (0.55, true), (0.42, true)]
        .into_iter()
        .enumerate()
    {
        if playing && !transport::timeline::running(&app) {
            playback::start(&mut app, playback::Mode::AllTracks);
            transport::advance(&mut app);
            app.playback.follow = true;
        }
        let time = 11.0 + i as f64;
        let pos = minimap.lerp_inside(egui::vec2(fraction, 0.5));
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        at(&mut app, &ctx, time, vec![egui::Event::PointerMoved(pos)]);
        at(&mut app, &ctx, time + 0.01, vec![button(true)]);
        at(&mut app, &ctx, time + 0.02, vec![button(false)]);
        assert!(!app.playback.follow);
        let tick = tick_at(pos.x);
        if playing {
            assert!(
                tick.abs_diff(playback::location(&app).0) <= 1,
                "the click seeks"
            );
        }
        let viewport = &app.staff.viewport;
        let half = viewport.panel.expect("panel").width() / 2.0;
        let center = spacing.x(tick) - viewport.scroll.x;
        assert!(
            (center - half).abs() <= 2.0,
            "tick {tick} at {center}, half width {half}"
        );
        let output = at(&mut app, &ctx, time + 0.5, vec![]);
        let viewport = &app.staff.viewport;
        let width = viewport.panel.expect("panel").width();
        let expected =
            [viewport.scroll.x, viewport.scroll.x + width].map(|x| minimap_x(spacing.tick(x)));
        let frame = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r)
                    if r.stroke.color == crate::view::timeline_minimap::COLOR =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .expect("minimap frame");
        for (edge, want) in [frame.left(), frame.right()].into_iter().zip(expected) {
            assert!(
                (edge - want).abs() <= 1.0,
                "frame {frame:?}, expected {expected:?}"
            );
        }
    }
}

mod pattern_edit;

/// Notes far above and below the staves, so the staff needs ledger room.
fn wide_range() -> crate::app::App {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            Track {
                events: [20u8, 110, 40, 90]
                    .iter()
                    .enumerate()
                    .flat_map(|(i, &pitch)| note(i as u64 * 480, pitch, 240))
                    .collect(),
            },
        ],
    });
    app.track = 1;
    app.playback.follow = false;
    app
}

fn vertical_zoom(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    button: egui::PointerButton,
) {
    use crate::view::piano_roll::tests::at;
    let pos = app.staff.viewport.zoom_bars[1]
        .expect("vertical zoom bar")
        .center();
    let event = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(app, ctx, time + 0.01, vec![event(true)]);
    at(app, ctx, time + 0.02, vec![event(false)]);
    at(app, ctx, time + 0.03, vec![]);
    at(app, ctx, time + 0.5, vec![]);
}

/// Scrolls down over the vertical scrollbar and returns the vertical offset it reached.
fn scrolled_down(app: &mut crate::app::App, ctx: &egui::Context, time: f64) -> f32 {
    use crate::view::piano_roll::tests::at;
    let pos = app.staff.viewport.bars[1]
        .expect("vertical scrollbar")
        .center();
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(
        app,
        ctx,
        time + 0.01,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for i in 2..40 {
        at(app, ctx, time + f64::from(i) * 0.05, vec![]);
    }
    app.staff.viewport.scroll.y
}

#[test]
fn vertical_zoom_keeps_the_staff_area_and_zooming_out_fits_the_whole_staff() {
    use crate::view::piano_roll::tests::at;
    let mut app = wide_range();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    at(&mut app, &ctx, 10.1, vec![]);
    let height = app.staff.viewport.panel.expect("panel").height();
    assert_eq!(
        scrolled_down(&mut app, &ctx, 11.0),
        0.0,
        "natural size fits"
    );
    for i in 0..3 {
        vertical_zoom(
            &mut app,
            &ctx,
            20.0 + f64::from(i),
            egui::PointerButton::Secondary,
        );
        assert_eq!(app.staff.viewport.panel.unwrap().height(), height);
    }
    assert_eq!(scrolled_down(&mut app, &ctx, 30.0), 0.0, "zoomed out fits");
    for i in 0..6 {
        vertical_zoom(
            &mut app,
            &ctx,
            40.0 + f64::from(i),
            egui::PointerButton::Primary,
        );
        assert_eq!(app.staff.viewport.panel.unwrap().height(), height);
    }
    assert!(
        scrolled_down(&mut app, &ctx, 50.0) > 0.0,
        "zoomed in scrolls"
    );
}
