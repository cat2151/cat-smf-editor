use super::stop_tests::stop_with_shift_space;
use crate::{
    play::playback::{self, Mode},
    play::transport,
    view,
};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

pub(super) fn fixture() -> crate::app::App {
    view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 1920,
                    delta: 1920,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 80],
                },
                Event {
                    tick: 2400,
                    delta: 480,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
                Event {
                    tick: 3840,
                    delta: 1440,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    })
}

fn render(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    key: Option<egui::Key>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 400.0),
            )),
            events: key
                .map(|key| {
                    vec![
                        egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: false,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]
                })
                .unwrap_or_default(),
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

#[test]
fn start_has_a_preparing_frame_and_silent_intro_advances_without_auditioning_rows() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let texts = |output: &egui::FullOutput| -> Vec<String> {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    };
    let overlay = |output: &egui::FullOutput| {
        texts(output)
            .iter()
            .filter(|t| *t == "Preparing playback…")
            .count()
    };
    let output = render(&mut app, &ctx, Some(egui::Key::Space));
    assert!(app.playback.pending.is_some());
    assert!(app.playback.active.is_none());
    // Status line plus the overlay on the start row; no green marker yet.
    assert_eq!(overlay(&output), 2);
    assert_eq!(transport::preparing_row_for(&app, 0), Some(0));
    // The stream is open but has not rendered a frame: still preparing.
    let output = render(&mut app, &ctx, None);
    assert!(app.playback.active.is_some());
    assert_eq!(transport::tick(&app), None);
    assert_eq!(transport::marker_row(&app), None);
    assert_eq!(overlay(&output), 2);
    app.audition.set_test_seconds(1.0);
    assert_eq!(transport::preparing_row_for(&app, 0), None);
    assert_eq!(overlay(&render(&mut app, &ctx, None)), 0);
    // 1:3 lies in the empty first measure, which is a single row.
    assert_eq!(transport::marker_row(&app), Some(0));
    assert_eq!(transport::tick(&app), Some(960));
    assert!(app.playback.follow);
    render(&mut app, &ctx, Some(egui::Key::F));
    assert!(!app.playback.follow);
    assert_eq!(app.cursor_row, None);
    assert_eq!(app.selected_event, None);
    assert_eq!(transport::tick(&app), Some(960));
    render(&mut app, &ctx, Some(egui::Key::F));
    assert!(app.playback.follow);
    render(&mut app, &ctx, Some(egui::Key::Escape));
    assert!(app.playback.active.is_some());
    stop_with_shift_space(&mut app, &ctx);
    assert!(app.playback.active.is_none());
    assert!(app.audition.seconds().is_none());
}

#[test]
fn missing_player_shows_guide_and_preserves_existing_playback() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    render(&mut app, &ctx, Some(egui::Key::Space));
    render(&mut app, &ctx, Some(egui::Key::T));
    assert!(app.player_selection.open);
    assert!(app.playback.pending.is_none());
    assert!(app.playback.active.is_some());
    assert!(app.note_mode);
}

#[test]
fn track_selection_keeps_pending_start_and_row_selection_stops_playback() {
    let mut app = fixture();
    playback::start(&mut app, Mode::Track);
    app.select_track(0);
    transport::advance(&mut app);
    assert!(app.playback.active.is_some());
    assert!(app.audition.seconds().is_some());
    crate::cursor::moves::select(&mut app, 0);
    assert!(app.playback.active.is_none());
    assert!(app.audition.seconds().is_none());
}

#[test]
fn follow_scrolls_to_distant_playhead_and_off_preserves_the_viewport() {
    let mut app = fixture();
    app.playback.follow = false;
    app.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .last_mut()
        .unwrap()
        .tick = 192000;
    app.loaded.as_mut().unwrap().refresh();
    let ctx = egui::Context::default();
    render(&mut app, &ctx, None);
    playback::start(&mut app, Mode::Track);
    transport::advance(&mut app);
    app.audition.set_test_seconds(80.0);
    let contains = |output: &egui::FullOutput, text: &str| {
        output.shapes.iter().any(
            |s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text().trim() == text),
        )
    };
    assert!(!contains(&render(&mut app, &ctx, None), "41"));
    let centered = render(&mut app, &ctx, Some(egui::Key::F));
    assert!(contains(&centered, "41"));
    let marker_center = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::LineSegment { points, stroke }
                    if stroke.color == egui::Color32::LIGHT_GREEN && stroke.width == 4.0 =>
                {
                    Some(((points[0].y + points[1].y) * 0.5, s.clip_rect.center().y))
                }
                _ => None,
            })
            .unwrap()
    };
    let (playhead, viewport) = marker_center(&centered);
    assert!(
        (playhead - viewport).abs() < 2.0,
        "playhead={playhead}, viewport center={viewport}"
    );
    // Keep centering as playback advances, even while the old marker is still visible.
    app.audition.set_test_seconds(81.0);
    let following = render(&mut app, &ctx, None);
    let (playhead, viewport) = marker_center(&following);
    assert!((playhead - viewport).abs() < 2.0);
    assert_eq!(app.selected_event, None);
    assert_eq!(app.audition.seconds(), Some(81.0));
    render(&mut app, &ctx, Some(egui::Key::F));
    app.audition.set_test_seconds(100.0);
    let output = render(&mut app, &ctx, None);
    assert!(contains(&output, "41"));
    assert!(!contains(&output, "51:1:0"));
}

#[test]
fn switching_tracks_preserves_audio_and_projects_playhead_through_each_tracks_tempo() {
    for mode in [Mode::Position, Mode::AllTracks] {
        let mut music = fixture().loaded.unwrap().music;
        music.format = 2;
        let mut other = music.tracks[0].clone();
        other.events.insert(
            0,
            Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x51,
                data: vec![0x0f, 0x42, 0x40],
            },
        );
        music.tracks.push(other);
        let mut app = view::tests::app(music);
        app.cursor_position = Some(
            app.loaded
                .as_ref()
                .unwrap()
                .time
                .map(0)
                .unwrap()
                .position(2400),
        );
        assert!(app.playback.follow);
        playback::start(&mut app, mode);
        assert_eq!(app.playback.pending.as_ref().unwrap().parts[0].0, 0);
        app.select_track(1);
        assert!(app.playback.pending.is_some());
        transport::advance(&mut app);
        app.audition.set_test_seconds(0.5);
        assert_eq!(
            transport::tick(&app),
            Some(if mode == Mode::Position { 1440 } else { 240 })
        );
        assert!(transport::marker_row(&app).is_some());
        app.select_track(0);
        assert_eq!(app.audition.seconds(), Some(0.5));
        assert!(app.playback.active.is_some());
        assert_eq!(
            transport::tick(&app),
            Some(if mode == Mode::Position { 2880 } else { 480 })
        );
        assert!(app.playback.follow);
        stop_with_shift_space(&mut app, &egui::Context::default());
        assert!(app.audition.seconds().is_none());
    }
}

#[test]
fn preparing_overlay_stays_on_the_cursor_row_among_rows_at_the_same_tick() {
    let events = [(0, 60), (480, 60), (480, 64), (480, 67), (960, 60)]
        .into_iter()
        .map(|(tick, pitch)| Event {
            tick,
            delta: 0,
            status: 0x90,
            meta_type: 0,
            data: vec![pitch, 100],
        })
        .collect();
    let mut app = view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    for cursor in 1..=3 {
        crate::cursor::moves::place(&mut app, cursor);
        playback::start(&mut app, Mode::Position);
        assert_eq!(transport::preparing_row_for(&app, 0), Some(cursor));
    }
}

mod mixer;
mod status;
