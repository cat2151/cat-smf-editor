use crate::view::{
    commands::{owner, Screen},
    note_edit::tests::render,
    piano_roll::tests::press,
    tests::app,
};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui::{self, Key};

fn event(tick: u64, status: u8, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: if status == 0xff { 0x2f } else { 0 },
        data,
    }
}

/// C-E-G at 0 and F-A at 1920 (measure 2), each a half note, at 480 PPQN in 4/4.
const NOTES: [(u64, u8); 5] = [(0, 60), (0, 64), (0, 67), (1920, 65), (1920, 69)];

fn song() -> crate::app::App {
    let mut events: Vec<Event> = NOTES
        .iter()
        .flat_map(|&(tick, key)| {
            [
                event(tick, 0x90, vec![key, 80]),
                event(tick + 960, 0x80, vec![key, 0]),
            ]
        })
        .collect();
    events.sort_by_key(|e| e.tick);
    events.push(event(7680, 0xff, vec![]));
    let mut track = Track { events };
    cat_smf_editor::edit::refresh_deltas(&mut track).unwrap();
    app(Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    })
}

/// `(start, end, pitch)` of each note, sorted.
fn spans(app: &crate::app::App) -> Vec<(u64, u64, u8)> {
    let events = &app.loaded.as_ref().unwrap().music.tracks[0].events;
    let mut open = vec![];
    let mut out = vec![];
    for e in events {
        match e.status & 0xf0 {
            0x90 if e.data[1] > 0 => open.push((e.tick, e.data[0])),
            0x80 | 0x90 => {
                let i = open.iter().position(|o| o.1 == e.data[0]).unwrap();
                let (start, key) = open.remove(i);
                out.push((start, e.tick, key));
            }
            _ => {}
        }
    }
    out.sort();
    out
}

fn shift(app: &mut crate::app::App, ctx: &egui::Context, key: Key) {
    let events = [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        })
        .collect();
    render(app, ctx, events);
}

pub(crate) fn open(ctx: &egui::Context) -> crate::app::App {
    let mut a = song();
    press(&mut a, ctx, Key::R);
    assert!(a.piano_roll.open);
    press(&mut a, ctx, Key::C);
    assert_eq!(owner(&a, ctx), Screen::ChordTiming);
    a
}

#[test]
fn keys_move_the_chord_and_one_undo_returns_the_whole_mode() {
    let ctx = egui::Context::default();
    let mut a = open(&ctx);
    let before = spans(&a);
    assert_eq!(a.playback.repeat, Some((0, 1920)));
    assert_eq!(super::highlight(&a, 0).len(), 3);
    // Later twice, longer once: the chord keeps moving as one, Note Off with it.
    press(&mut a, &ctx, Key::L);
    press(&mut a, &ctx, Key::L);
    shift(&mut a, &ctx, Key::L);
    assert_eq!(a.piano_roll.error, None);
    assert_eq!(
        &spans(&a)[..3],
        &[(240, 1320, 60), (240, 1320, 64), (240, 1320, 67)]
    );
    // The next chord is in measure 2, so the loop follows it.
    press(&mut a, &ctx, Key::J);
    assert_eq!(a.playback.repeat, Some((1920, 3840)));
    press(&mut a, &ctx, Key::H);
    assert_eq!(&spans(&a)[3..], &[(1800, 2760, 65), (1800, 2760, 69)]);
    // The moved chord now starts in measure 1 and the loop goes with it.
    assert_eq!(a.playback.repeat, Some((0, 1920)));
    press(&mut a, &ctx, Key::Enter);
    assert_eq!(owner(&a, &ctx), Screen::Timeline);
    assert_eq!(a.playback.repeat, None);
    crate::view::undo::apply(&mut a);
    assert_eq!(spans(&a), before);
    assert!(!a.undo.available());
}

#[test]
fn escape_restores_the_song_from_before_the_mode() {
    let ctx = egui::Context::default();
    let mut a = open(&ctx);
    let before = spans(&a);
    press(&mut a, &ctx, Key::L);
    shift(&mut a, &ctx, Key::H);
    press(&mut a, &ctx, Key::J);
    press(&mut a, &ctx, Key::L);
    assert_ne!(spans(&a), before);
    press(&mut a, &ctx, Key::Escape);
    assert_eq!(owner(&a, &ctx), Screen::Timeline);
    assert_eq!(spans(&a), before);
    assert!(!a.undo.available());
    assert_eq!(a.playback.repeat, None);
}

#[test]
fn quantize_snaps_the_selected_chord_only() {
    let ctx = egui::Context::default();
    let mut a = song();
    a.edit_music(|m| {
        cat_smf_editor::edit::chord_timing::apply(
            &mut m.tracks[0],
            0,
            cat_smf_editor::edit::chord_timing::Change::Move(50),
        )
        .map(|_| ())
    })
    .unwrap();
    press(&mut a, &ctx, Key::R);
    press(&mut a, &ctx, Key::C);
    press(&mut a, &ctx, Key::Q);
    assert_eq!(&spans(&a)[..1], &[(0, 960, 60)]);
    assert_eq!(&spans(&a)[3..], &[(1920, 2880, 65), (1920, 2880, 69)]);
}

#[test]
fn a_rejected_move_reports_and_keeps_the_song() {
    let ctx = egui::Context::default();
    let mut a = open(&ctx);
    let before = spans(&a);
    press(&mut a, &ctx, Key::H);
    assert!(a.piano_roll.error.is_some());
    assert_eq!(spans(&a), before);
    assert!(!a.undo.available());
}

/// Each arrow does what its h j k l does, Shift included.
#[test]
fn arrows_do_what_hjkl_do() {
    let ctx = egui::Context::default();
    let mut letters = open(&ctx);
    let mut arrows = open(&ctx);
    for (letter, arrow, shifted) in [
        (Key::L, Key::ArrowRight, false),
        (Key::L, Key::ArrowRight, true),
        (Key::J, Key::ArrowDown, false),
        (Key::H, Key::ArrowLeft, false),
        (Key::H, Key::ArrowLeft, true),
        (Key::K, Key::ArrowUp, false),
        (Key::L, Key::ArrowRight, false),
    ] {
        for (a, key) in [(&mut letters, letter), (&mut arrows, arrow)] {
            if shifted {
                shift(a, &ctx, key);
            } else {
                press(a, &ctx, key);
            }
        }
        assert_eq!(spans(&letters), spans(&arrows));
        assert_eq!(letters.playback.repeat, arrows.playback.repeat);
    }
    assert_ne!(spans(&letters), spans(&song()));
}
