use crate::app::App;
use crate::view::tests::app;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

mod pattern_edit;
mod tempo;
mod variations;

/// The map outlines only the ticks the list shows, right of the pane.
#[test]
fn track_map_outlines_visible_ticks() {
    let events = (0..400u64)
        .flat_map(|i| {
            [(0x90, 100), (0x80, 0)].map(|(status, velocity)| Event {
                delta: 0,
                tick: i * 480 + if status == 0x80 { 240 } else { 0 },
                status,
                meta_type: 0,
                data: vec![60 + (i % 12) as u8, velocity],
            })
        })
        .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    let ctx = egui::Context::default();
    let render = |app: &mut App| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    render(&mut app);
    let output = render(&mut app);
    let (first, last) = app.track_view.visible_ticks.unwrap();
    assert_eq!(first, 0);
    assert!(0 < last && last < 399 * 480, "{last}");
    let frame = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if r.stroke.color == egui::Color32::from_gray(64) && r.stroke.width > 0.0 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .expect("visible range outline");
    // The score takes the right column, so the pane and its map end mid-screen.
    assert!(frame.left() > 600.0 && frame.right() < 850.0, "{frame:?}");
    assert!(frame.height() < 450.0, "{frame:?}");
}

const DARK: egui::Color32 = egui::Color32::from_gray(64);

fn note(tick: u64, pitch: u8, length: u64) -> [Event; 2] {
    [(0x90, 100, 0), (0x80, 0, length)].map(|(status, velocity, offset)| Event {
        delta: 0,
        tick: tick + offset,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    })
}

/// Track 0: a note per measure (4/4, 480 ppqn) over pitches 60–72 for 8 measures and a marker at measure 5.
/// Track 1: one note inside that range, one above it and a tempo change at measure 3.
fn two_tracks() -> App {
    let mut first: Vec<Event> = (0..8u64)
        .flat_map(|m| note(m * 1920, 60 + (m as u8 % 2) * 12, 480))
        .collect();
    first.push(Event {
        delta: 0,
        tick: 4 * 1920,
        status: 0xff,
        meta_type: 0x06,
        data: b"B".to_vec(),
    });
    first.sort_by_key(|e| e.tick);
    let mut second = [note(0, 64, 960), note(1920, 100, 960)].concat();
    second.push(Event {
        delta: 0,
        tick: 2 * 1920,
        status: 0xff,
        meta_type: 0x51,
        data: vec![0x07, 0xa1, 0x20],
    });
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: first }, Track { events: second }],
    })
}

fn render(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn filled(output: &egui::FullOutput, color: egui::Color32) -> Vec<egui::Rect> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r) if r.fill == color => Some(r.rect),
            _ => None,
        })
        .collect()
}

fn outlines(output: &egui::FullOutput) -> Vec<egui::Rect> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r) if r.stroke.color == DARK && r.stroke.width > 0.0 => {
                Some(r.rect)
            }
            _ => None,
        })
        .collect()
}

/// Other tracks show only inside this track's range; meta events and the playback position fill a whole row.
#[test]
fn map_layers_other_tracks_meta_rows_and_position() {
    let mut app = two_tracks();
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let output = render(&ctx, &mut app, Vec::new());
    let map = outlines(&output)[0];
    let dark = filled(&output, DARK);
    let rows: Vec<_> = dark.iter().filter(|r| r.width() == map.width()).collect();
    assert_eq!(rows.len(), 2, "{dark:?}");
    assert!(rows.iter().all(|r| r.height() == 1.0));
    // Track 1's tempo at measure 3 and this track's marker at measure 5, in a track ending at 13920.
    let mut fractions: Vec<f32> = rows
        .iter()
        .map(|r| (r.top() - map.top()) / map.height())
        .collect();
    fractions.sort_by(f32::total_cmp);
    for (fraction, tick) in fractions.into_iter().zip([3840.0, 7680.0]) {
        assert!((fraction - tick / 13920.0).abs() < 0.01, "{fraction}");
    }
    // Only track 1's pitch 64 note; pitch 100 is outside 60–72.
    assert_eq!(dark.len() - rows.len(), 1, "{dark:?}");
    let white: Vec<_> = filled(&output, egui::Color32::WHITE)
        .into_iter()
        .filter(|r| r.width() == map.width())
        .collect();
    assert_eq!(white.len(), 1);
    assert_eq!(white[0].top(), map.top());
}

/// Two-column view draws each pane's map on that pane's right: mid-screen and at the right edge.
#[test]
fn split_view_shows_a_map_on_each_side() {
    let mut app = two_tracks();
    app.track_view.split = Some([0, 1]);
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let output = render(&ctx, &mut app, Vec::new());
    let mut frames = outlines(&output);
    frames.sort_by(|a, b| a.left().total_cmp(&b.left()));
    assert_eq!(frames.len(), 2, "{frames:?}");
    let active = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Rect(rect) if rect.stroke.color == crate::view::ACTIVE_PANE => {
                Some(rect.rect)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        (frames[0].right() - active.right()).abs() < 12.0,
        "{frames:?}; active={active:?}"
    );
    assert!(frames[1].right() > 1480.0, "{frames:?}");
}

fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    render(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    render(ctx, app, vec![button(true)]);
    render(ctx, app, vec![button(false)]);
}

/// A click mid-measure puts the cursor on that measure's first tick; in two columns it also focuses that pane.
#[test]
fn click_jumps_to_the_measure_start() {
    let mut app = two_tracks();
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let map = outlines(&render(&ctx, &mut app, Vec::new()))[0];
    // Measure 3 runs over ticks 3840–5760 of 13920.
    let y = map.top() + map.height() * (4800.0 / 13920.0);
    click(&ctx, &mut app, egui::pos2(map.center().x, y));
    assert_eq!(app.track_view.cursor_tick, Some(3840));

    app.track_view.split = Some([0, 1]);
    app.track_view.active = 1;
    app.select_track(1);
    render(&ctx, &mut app, Vec::new());
    let output = render(&ctx, &mut app, Vec::new());
    let mut frames = outlines(&output);
    frames.sort_by(|a, b| a.left().total_cmp(&b.left()));
    // The right map shows all of track 1, so its outline spans the full map height.
    let (left, full) = (frames[0], frames[1]);
    click(
        &ctx,
        &mut app,
        egui::pos2(
            left.center().x,
            full.top() + full.height() * (2000.0 / 13920.0),
        ),
    );
    assert_eq!(app.track_view.active, 0);
    assert_eq!(app.track, 0);
    assert_eq!(app.track_view.cursor_tick, Some(1920));
}

/// During whole-song playback a click restarts it from the clicked measure.
#[test]
fn click_while_playing_moves_playback_to_the_measure() {
    let mut app = two_tracks();
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let map = outlines(&render(&ctx, &mut app, Vec::new()))[0];
    crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
    render(&ctx, &mut app, Vec::new());
    app.audition.set_test_seconds(1.0);
    let y = map.top() + map.height() * (4800.0 / 13920.0);
    click(&ctx, &mut app, egui::pos2(map.center().x, y));
    render(&ctx, &mut app, Vec::new());
    let active = app.playback.active.as_ref().expect("playback continues");
    assert_eq!(active.start_seconds, 3840.0 / 960.0);
}

/// While stopped, a click scrolls the rows to the clicked measure.
#[test]
fn click_while_stopped_scrolls_the_rows_to_the_measure() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: (0..400u64).flat_map(|i| note(i * 480, 60, 240)).collect(),
        }],
    });
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let map = outlines(&render(&ctx, &mut app, Vec::new()))[0];
    let (_, before) = app.track_view.visible_ticks.unwrap();
    click(&ctx, &mut app, egui::pos2(map.center().x, 800.0));
    render(&ctx, &mut app, Vec::new());
    let cursor = app.track_view.cursor_tick.unwrap();
    assert!(before < cursor, "{before} {cursor}");
    let (first, last) = app.track_view.visible_ticks.unwrap();
    assert!((first..=last).contains(&cursor), "{first}..{last} {cursor}");
}

/// A track without notes spans every track's range, so all other notes show.
#[test]
fn noteless_track_map_shows_every_note() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![Event {
                    delta: 0,
                    tick: 3840,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: Vec::new(),
                }],
            },
            Track {
                events: [note(0, 40, 480), note(1920, 90, 480)].concat(),
            },
            Track {
                events: note(960, 60, 480).to_vec(),
            },
        ],
    });
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let output = render(&ctx, &mut app, Vec::new());
    let map = outlines(&output)[0];
    let notes: Vec<_> = filled(&output, DARK)
        .into_iter()
        .filter(|r| r.width() < map.width())
        .collect();
    assert_eq!(notes.len(), 3, "{notes:?}");
    let low = notes.iter().map(|r| r.left()).fold(f32::MAX, f32::min);
    let high = notes.iter().map(|r| r.right()).fold(f32::MIN, f32::max);
    assert!(
        low - map.left() < 5.0 && map.right() - high < 5.0,
        "{notes:?} {map:?}"
    );
}

mod repeat;

/// Effect buttons wider than the pane are cut off, so the rows stay left of the map and its clicks still navigate.
#[test]
fn overflowing_track_header_leaves_the_map_clickable() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: (0..400u64).flat_map(|i| note(i * 480, 60, 240)).collect(),
        }],
    });
    app.loaded.as_mut().unwrap().patches[0].effects = (0..6)
        .map(|i| cat_smf_editor::chain_settings::PluginState {
            reference: None,
            plugin: cat_smf_editor::host::PluginKey {
                format: "CLAP".into(),
                id: format!("effect {i}"),
                name: format!("A long effect name to overflow the pane {i}"),
                vendor: "fixture".into(),
                bundle_path: "X:/plugins/fixture.clap".into(),
            },
            state: vec![1],
            preset_name: None,
            bypass: false,
        })
        .collect();
    let ctx = egui::Context::default();
    render(&ctx, &mut app, Vec::new());
    let map = outlines(&render(&ctx, &mut app, Vec::new()))[0];
    click(&ctx, &mut app, egui::pos2(map.center().x, 700.0));
    // Measure 78 of the 400 quarter notes in 4/4 lies under y = 700.
    assert_eq!(app.track_view.cursor_tick, Some(77 * 1920));
}
