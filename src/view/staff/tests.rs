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
