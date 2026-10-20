use super::*;
use cat_smf_editor::model::{Music, Track};

fn app() -> App {
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }; 3],
    })
}

fn press(a: &mut App, c: &egui::Context, key: Key, modifiers: Modifiers) {
    let _ = c.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events: [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                })
                .collect(),
            ..Default::default()
        },
        |c| crate::view::show(a, c),
    );
}

#[test]
fn settings_require_ctrl_comma_in_score_roll_and_tempo_map() {
    for track in [None, Some(0), Some(1)] {
        let mut a = app();
        let c = egui::Context::default();
        if let Some(track) = track {
            a.select_track(track);
            crate::view::piano_roll::open(&mut a);
        }
        press(&mut a, &c, Key::Comma, Modifiers::NONE);
        assert!(!a.comma_menu.open);
        press(&mut a, &c, Key::Comma, Modifiers::CTRL | Modifiers::COMMAND);
        assert!(a.comma_menu.open);
        press(&mut a, &c, Key::Escape, Modifiers::NONE);
        assert!(!a.comma_menu.open);
        assert_eq!(a.piano_roll.open, track == Some(1));
        assert_eq!(a.tempo_curve.open, track == Some(0));
    }
}

#[test]
fn timeline_track_steps_wrap_preserve_tick_and_switch_overlay() {
    let mut a = app();
    let c = egui::Context::default();
    let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
    crate::cursor::split::go(&mut a, 0, 600);
    crate::view::piano_roll::open(&mut a);
    for (key, expected) in [
        (Key::ArrowRight, 1),
        (Key::ArrowRight, 2),
        (Key::ArrowRight, 0),
        (Key::ArrowLeft, 2),
        (Key::ArrowLeft, 1),
        (Key::ArrowLeft, 0),
    ] {
        press(&mut a, &c, key, ctrl);
        assert_eq!(a.track, expected);
        assert_eq!(crate::play::playback::location(&a).0, 600);
        assert_eq!(a.tempo_curve.open, expected == 0);
        assert_eq!(a.piano_roll.open, expected != 0);
    }
    a.track_view.split = Some([0, 2]);
    a.track_view.active = 1;
    crate::cursor::split::go(&mut a, 2, 600);
    crate::view::piano_roll::open(&mut a);
    press(&mut a, &c, Key::ArrowLeft, ctrl);
    assert_eq!(a.track_view.split, Some([0, 1]));
    press(&mut a, &c, Key::ArrowLeft, ctrl);
    assert_eq!(a.track_view.split, Some([0, 0]));
    assert!(a.tempo_curve.open);
    press(&mut a, &c, Key::Escape, Modifiers::NONE);
    assert!(!a.piano_roll.open && !a.tempo_curve.open);
}

#[test]
fn timeline_track_steps_keep_piano_roll_on_first_track_with_notes() {
    let mut music = app().loaded.unwrap().music;
    music.tracks[0].events.push(cat_smf_editor::model::Event {
        tick: 0,
        delta: 0,
        status: 0x90,
        meta_type: 0,
        data: vec![60, 80],
    });
    let mut a = crate::view::tests::app(music);
    let c = egui::Context::default();
    let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
    press(&mut a, &c, Key::R, Modifiers::NONE);
    assert!(a.piano_roll.open);
    assert!(!a.tempo_curve.open);
    for (key, expected) in [(Key::ArrowRight, 1), (Key::ArrowLeft, 0)] {
        press(&mut a, &c, key, ctrl);
        assert_eq!(a.track, expected);
        assert!(a.piano_roll.open);
        assert!(!a.tempo_curve.open);
    }
    press(&mut a, &c, Key::Escape, Modifiers::NONE);
    assert!(!a.piano_roll.open && !a.tempo_curve.open);
}
