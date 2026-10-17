use crate::{cursor::split, view::tests::app};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

pub(super) fn fixture() -> crate::app::App {
    tracks(4)
}

pub(super) fn tracks(count: u64) -> crate::app::App {
    app(Music {
        format: 1,
        division: 480,
        tracks: (0..count)
            .map(|i| Track {
                events: vec![
                    Event {
                        tick: 480 + i * 240,
                        delta: (480 + i * 240) as u32,
                        status: 0x90,
                        meta_type: 0,
                        data: vec![60, 0],
                    },
                    Event {
                        tick: 480 + count * 240,
                        delta: ((count - i) * 240) as u32,
                        status: 0xff,
                        meta_type: 0x2f,
                        data: vec![],
                    },
                ],
            })
            .collect(),
    })
}

pub(crate) fn press(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    keys: &[(egui::Key, egui::Modifiers)],
) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1900.0, 700.0),
            )),
            events: keys
                .iter()
                .flat_map(|&(key, modifiers)| {
                    [
                        egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers,
                        },
                        egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: false,
                            repeat: false,
                            modifiers,
                        },
                    ]
                })
                .collect(),
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
}

pub(super) fn w_then(app: &mut crate::app::App, ctx: &egui::Context, key: egui::Key) {
    let none = egui::Modifiers::NONE;
    press(app, ctx, &[(egui::Key::W, none)]);
    assert!(app.track_view.split_open);
    press(app, ctx, &[(key, none)]);
    assert!(!app.track_view.split_open);
}

#[test]
fn gt_tracks_preserve_exact_time_and_do_not_open_patch_browser() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    for (key, target) in [
        (egui::Key::Num2, 1),
        (egui::Key::Num4, 3),
        (egui::Key::Num1, 0),
    ] {
        split::go(&mut app, 0, 600);
        press(&mut app, &ctx, &[(egui::Key::G, egui::Modifiers::NONE)]);
        assert!(app.track_view.go_open);
        press(
            &mut app,
            &ctx,
            &[
                (egui::Key::T, egui::Modifiers::NONE),
                (key, egui::Modifiers::NONE),
            ],
        );
        assert_eq!(app.track, target);
        assert_eq!(crate::play::playback::location(&app).0, 600);
        assert!(!app.track_view.go_open);
        assert!(!app.player_selection.open);
    }
}

#[test]
fn go_replaces_only_the_active_column_even_with_the_other_columns_track() {
    let mut app = fixture();
    app.track_view.split = Some([0, 1]);
    split::go(&mut app, 3, 600);
    assert_eq!(app.track_view.split, Some([3, 1]));
    split::go(&mut app, 1, 600);
    assert_eq!(
        (app.track_view.split, app.track_view.active),
        (Some([1, 1]), 0)
    );
    split::focus(&mut app, 1);
    split::go(&mut app, 2, 600);
    assert_eq!((app.track_view.split, app.track), (Some([1, 2]), 2));
}

#[test]
fn gt_track_replaces_only_the_active_pane() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    app.track_view.split = Some([0, 1]);
    let gt = |app: &mut crate::app::App, key| {
        press(
            app,
            &ctx,
            &[
                (egui::Key::G, egui::Modifiers::NONE),
                (egui::Key::T, egui::Modifiers::NONE),
                (key, egui::Modifiers::NONE),
            ],
        );
    };
    gt(&mut app, egui::Key::Num3);
    assert_eq!((app.track_view.split, app.track), (Some([2, 1]), 2));
    gt(&mut app, egui::Key::Num2);
    assert_eq!((app.track_view.split, app.track), (Some([1, 1]), 1));
    gt(&mut app, egui::Key::Num9);
    assert_eq!((app.track_view.split, app.track), (Some([1, 1]), 1));
}

#[test]
fn raw_and_smpte_go_keep_time_without_an_event_at_destination() {
    let mut app = fixture();
    app.note_mode = false;
    split::go(&mut app, 1, 600);
    assert_eq!(crate::play::playback::location(&app).0, 600);
    assert_eq!(app.selected_event, None);
    app.loaded.as_mut().unwrap().music.division = 0xe728;
    app.loaded.as_mut().unwrap().refresh();
    split::go(&mut app, 2, 600);
    assert_eq!(crate::play::playback::location(&app), (600, None));
    crate::cursor::moves::select(&mut app, 0);
    assert_eq!(crate::play::playback::location(&app).0, 960);
}

#[test]
fn missing_track_stays_open_and_escape_cancels_without_moving() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    press(
        &mut app,
        &ctx,
        &[
            (egui::Key::G, egui::Modifiers::NONE),
            (egui::Key::T, egui::Modifiers::NONE),
            (egui::Key::Num9, egui::Modifiers::NONE),
        ],
    );
    assert!(app.track_view.go_open);
    assert_eq!(app.track, 0);
    press(
        &mut app,
        &ctx,
        &[(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert!(!app.track_view.go_open);
    assert_eq!(app.track, 0);
}

#[test]
fn unequal_event_density_scrolls_from_either_pane_without_idle_drift() {
    let mut app = fixture();
    for track in &mut app.loaded.as_mut().unwrap().music.tracks {
        track.events.last_mut().unwrap().tick = 96_000;
    }
    // Many rows at the same tick must not be rounded away on the next frame.
    let events = &mut app.loaded.as_mut().unwrap().music.tracks[0].events;
    for tick in (0..96_000).step_by(480) {
        for _ in 0..3 {
            events.push(Event {
                tick,
                delta: 0,
                status: 0xb0,
                meta_type: 0,
                data: vec![1, 0],
            });
        }
    }
    events.sort_by_key(|e| e.tick);
    app.loaded.as_mut().unwrap().refresh();
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1900.0, 420.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    split::go(&mut app, 0, 48_000);
    render(&mut app, vec![]);
    app.track_view.split = Some([0, 1]);
    render(&mut app, vec![]);
    let output = render(&mut app, vec![]);
    let before = (app.track_view.scroll_tick, app.track_view.scroll_source);
    let right = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t)
                if t.pos.x > 1000.0
                    && t.pos.y > 160.0
                    && !t.galley.text().trim().is_empty()
                    && t.galley
                        .text()
                        .chars()
                        .all(|c| c.is_ascii_digit() || matches!(c, ':' | ' ')) =>
            {
                Some(t.pos + egui::vec2(5.0, 5.0))
            }
            _ => None,
        })
        .expect("right track time rows were not painted");
    for _ in 0..3 {
        render(&mut app, vec![egui::Event::PointerMoved(right)]);
    }
    assert_eq!(
        (app.track_view.scroll_tick, app.track_view.scroll_source),
        before
    );
    render(
        &mut app,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -180.0),
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for _ in 0..40 {
        render(&mut app, vec![]);
    }
    assert!(
        app.track_view.scroll_tick > before.0,
        "scrolling the inactive pane did not move the shared time"
    );
    let anchor = app.track_view.scroll_tick;
    let measure = app
        .loaded
        .as_ref()
        .unwrap()
        .time
        .map(0)
        .unwrap()
        .position(anchor)
        .measure
        .to_string();
    let output = render(&mut app, vec![]);
    // The sparse track shows an empty measure as its number alone.
    let painted = |right: bool| {
        output.shapes.iter().any(|s| {
            matches!(&s.shape,
        egui::epaint::Shape::Text(t) if (t.pos.x > 950.0) == right
            && t.galley.text().replace(' ', "").split(':').next() == Some(measure.as_str()))
        })
    };
    assert!(
        painted(false) && painted(true),
        "both tracks must paint the shared time anchor"
    );
    let stable = (app.track_view.scroll_tick, app.track_view.scroll_source);
    for _ in 0..5 {
        render(&mut app, vec![]);
    }
    assert_eq!(
        (app.track_view.scroll_tick, app.track_view.scroll_source),
        stable
    );
}

#[test]
fn horizontal_wheel_on_inactive_pane_moves_shared_offset() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1300.0, 420.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        );
    };
    app.track_view.split = Some([0, 1]);
    render(&mut app, vec![]);
    render(
        &mut app,
        vec![egui::Event::PointerMoved(egui::pos2(1150.0, 300.0))],
    );
    render(
        &mut app,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(-180.0, 0.0),
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for _ in 0..40 {
        render(&mut app, vec![]);
    }
    assert!(app.track_view.horizontal_offset > 0.0);
}

#[test]
fn gt_letters_skip_n_and_p_and_shift_letters_follow_z() {
    let mut app = tracks(50);
    let ctx = egui::Context::default();
    let shift = egui::Modifiers::SHIFT;
    let none = egui::Modifiers::NONE;
    for ((key, modifiers), target) in [
        ((egui::Key::A, none), 9),
        ((egui::Key::M, none), 21),
        ((egui::Key::O, none), 22),
        ((egui::Key::Q, none), 23),
        ((egui::Key::T, none), 26),
        ((egui::Key::Z, none), 32),
        ((egui::Key::A, shift), 33),
        ((egui::Key::D, shift), 36),
        ((egui::Key::N, shift), 46),
        ((egui::Key::P, shift), 48),
        ((egui::Key::Num9, none), 8),
    ] {
        split::go(&mut app, 0, 600);
        press(
            &mut app,
            &ctx,
            &[(egui::Key::G, none), (egui::Key::T, none), (key, modifiers)],
        );
        assert_eq!(app.track, target, "{key:?} {modifiers:?}");
        assert_eq!(crate::play::playback::location(&app).0, 600);
        assert!(!app.track_view.go_open);
    }
    // Shift+t does not open the list, Shift+digit and missing letters keep it open.
    split::go(&mut app, 0, 600);
    press(
        &mut app,
        &ctx,
        &[(egui::Key::G, none), (egui::Key::T, shift)],
    );
    assert!(app.track_view.go_open && !app.track_view.track_list);
    press(
        &mut app,
        &ctx,
        &[
            (egui::Key::T, none),
            (egui::Key::Num2, shift),
            (egui::Key::Z, shift),
        ],
    );
    assert!(app.track_view.go_open);
    assert_eq!(app.track, 0);
    assert_eq!(split::key_label(9), Some('a'));
    assert_eq!(split::key_label(22), Some('o'));
    assert_eq!(split::key_label(58), Some('Z'));
    assert_eq!(split::key_label(59), None);
}
