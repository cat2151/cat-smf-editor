use super::*;
use cat_smf_editor::model::{Event, Music, Track};
mod feedback;
mod migemo;
mod navigation;
mod ranking;
pub(super) fn app() -> App {
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    Event {
                        tick: 0,
                        delta: 0,
                        status: 0x90,
                        meta_type: 0,
                        data: vec![60, 100]
                    },
                    Event {
                        tick: 480,
                        delta: 480,
                        status: 0x80,
                        meta_type: 0,
                        data: vec![60, 0]
                    }
                ]
            };
            3
        ],
    })
}
fn key(k: Key, m: Modifiers) -> egui::Event {
    egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    }
}
pub(crate) fn frame(a: &mut App, c: &egui::Context, mut events: Vec<egui::Event>) {
    let releases = events
        .iter()
        .filter_map(|e| {
            if let egui::Event::Key { key, modifiers, .. } = e {
                Some(egui::Event::Key {
                    key: *key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: *modifiers,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    events.extend(releases);
    let _ = c.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |c| super::super::show(a, c),
    );
}
pub(crate) fn open(a: &mut App, c: &egui::Context) {
    frame(
        a,
        c,
        vec![key(
            Key::P,
            Modifiers::CTRL | Modifiers::SHIFT | Modifiers::COMMAND,
        )],
    );
    assert!(a.palette.is_open());
}
pub(crate) fn query(a: &mut App, c: &egui::Context, q: &str) {
    frame(a, c, vec![egui::Event::Text(q.into())]);
    assert_eq!(a.palette.query, q);
}
/// The labels the palette lists for its current query.
pub(crate) fn listed(a: &mut App) -> Vec<String> {
    filter(
        &a.palette.commands,
        &a.palette.query,
        &mut a.palette.search,
        &a.palette.history,
    )
    .into_iter()
    .map(|i| a.palette.commands[i].label())
    .collect()
}
pub(crate) fn press(a: &mut App, c: &egui::Context, k: Key) {
    frame(a, c, vec![key(k, Modifiers::NONE)]);
}

#[test]
fn strict_chord_filter_navigation_cancel_and_closed_mixer_target() {
    let mut a = app();
    a.track = 2;
    let c = egui::Context::default();
    frame(&mut a, &c, vec![]);
    frame(
        &mut a,
        &c,
        vec![key(
            Key::P,
            Modifiers::CTRL | Modifiers::SHIFT | Modifiers::ALT,
        )],
    );
    assert!(!a.palette.is_open());
    assert!(!a.recent.open);
    open(&mut a, &c);
    assert!(!a.recent.open);
    assert!(!a.playback.mixer_open);
    press(&mut a, &c, Key::ArrowDown);
    assert_eq!(a.palette.selected, 1);
    query(&mut a, &c, "減音");
    assert_eq!(a.palette.selected, 0);
    press(&mut a, &c, Key::Escape);
    assert!(!a.palette.is_open());
    assert!(!a.playback.mixer_open);
    assert_eq!(a.audition.gain(2), 1.0);
    open(&mut a, &c);
    query(&mut a, &c, "減音");
    press(&mut a, &c, Key::Enter);
    assert!(!a.palette.is_open());
    assert!(a.playback.mixer_open);
    assert!(!a.playback.mixer_master);
    assert_eq!(a.playback.mixer_track, 2);
    assert!(a.audition.gain(2) < 1.0);
    assert_eq!(a.audition.gain(1), 1.0);
}

#[test]
fn empty_result_and_stale_generation_never_execute_and_search_matches_keys() {
    let mut a = app();
    let c = egui::Context::default();
    open(&mut a, &c);
    frame(
        &mut a,
        &c,
        vec![
            egui::Event::Text("no-such-operation".into()),
            key(Key::Enter, Modifiers::NONE),
        ],
    );
    assert!(
        !a.file_open_requested,
        "same-batch search never executes an old result"
    );
    press(&mut a, &c, Key::Enter);
    assert!(a.palette.is_open());
    press(&mut a, &c, Key::Escape);
    open(&mut a, &c);
    query(&mut a, &c, "ctrl+shift+n");
    assert_eq!(
        filter(
            &a.palette.commands,
            &a.palette.query,
            &mut a.palette.search,
            &a.palette.history
        )
        .len(),
        1
    );
    a.generation += 1;
    press(&mut a, &c, Key::Enter);
    assert!(a.palette.is_open());
    assert_eq!(a.loaded.as_ref().unwrap().music.tracks.len(), 3);
    press(&mut a, &c, Key::Escape);
    frame(&mut a, &c, vec![key(Key::P, Modifiers::CTRL)]);
    assert!(a.recent.open);
    assert!(!a.palette.is_open());
}

#[test]
fn mml_buffer_focus_selection_cancel_and_context_confirm_are_preserved() {
    let mut a = app();
    let c = egui::Context::default();
    super::super::mml_input::open(&mut a, &c, false);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![egui::Event::Text("cde".into())]);
    frame(&mut a, &c, vec![]);
    let before = a.mml_input.buffer.clone();
    let focus = c.memory(|m| m.focused()).unwrap();
    let state = egui::text_edit::TextEditState::load(&c, focus).unwrap();
    let selection = state.cursor.char_range();
    open(&mut a, &c);
    query(&mut a, &c, "取消");
    press(&mut a, &c, Key::Escape);
    assert!(a.mml_input.open);
    assert_eq!(a.mml_input.buffer, before);
    assert_eq!(c.memory(|m| m.focused()), Some(focus));
    assert_eq!(
        egui::text_edit::TextEditState::load(&c, focus)
            .unwrap()
            .cursor
            .char_range(),
        selection
    );
    frame(&mut a, &c, vec![]);
    assert_eq!(
        c.memory(|m| m.focused()),
        Some(focus),
        "focus survives the next native repaint"
    );
    frame(&mut a, &c, vec![egui::Event::Text("f".into())]);
    frame(&mut a, &c, vec![]);
    assert_eq!(a.mml_input.buffer, format!("{before}f"));
    open(&mut a, &c);
    query(&mut a, &c, "確定");
    press(&mut a, &c, Key::Enter);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    assert!(!a.mml_input.open);
    assert!(a.undo.available());
}

#[test]
fn ime_enter_does_not_execute_and_repeat_or_layout_repass_does_not_duplicate() {
    let mut a = app();
    let c = egui::Context::default();
    open(&mut a, &c);
    query(&mut a, &c, "Notes / Raw");
    frame(
        &mut a,
        &c,
        vec![
            egui::Event::Ime(egui::ImeEvent::Preedit("あ".into())),
            key(Key::Enter, Modifiers::NONE),
        ],
    );
    assert!(a.palette.is_open());
    assert!(a.note_mode);
    frame(
        &mut a,
        &c,
        vec![
            egui::Event::Ime(egui::ImeEvent::Commit(String::new())),
            key(Key::Enter, Modifiers::NONE),
        ],
    );
    assert!(a.palette.is_open());
    // Return to an exact query after the IME has finished.
    a.palette.query = "Notes / Raw".into();
    let _ = c.run(
        egui::RawInput {
            events: vec![key(Key::Enter, Modifiers::NONE)],
            ..Default::default()
        },
        |c| {
            super::super::show(&mut a, c);
            c.request_discard("palette execution test");
        },
    );
    assert!(!a.palette.is_open());
    assert!(!a.note_mode);
}

#[test]
fn playback_keeps_advancing_while_input_and_confirmation_remain_modal() {
    let mut a = app();
    let c = egui::Context::default();
    crate::play::playback::start(&mut a, crate::play::playback::Mode::AllTracks);
    crate::play::transport::advance(&mut a);
    a.audition.set_test_seconds(0.1);
    open(&mut a, &c);
    assert!(a.playback.active.is_some());
    a.audition.set_test_seconds(0.2);
    frame(&mut a, &c, vec![key(Key::Space, Modifiers::SHIFT)]);
    assert!(a.playback.active.is_some());
    press(&mut a, &c, Key::Escape);
    a.config.navigation.departure = Some(super::super::files::Action::New);
    open(&mut a, &c);
    assert!(a
        .palette
        .commands
        .iter()
        .all(|cmd| cmd.screen == Screen::Confirm));
    assert!(!a.playback.mixer_open);
    press(&mut a, &c, Key::Escape);
    assert!(a.config.navigation.departure.is_some());
}

#[test]
fn application_viewport_palette_keeps_its_close_target() {
    let mut a = app();
    super::super::analysis::toggle(&mut a);
    let c = egui::Context::default();
    let viewport = egui::ViewportId::from_hash_of("palette-test-child");
    let mut run = |events| {
        let mut raw = egui::RawInput {
            viewport_id: viewport,
            events,
            ..Default::default()
        };
        raw.viewports.insert(viewport, Default::default());
        let _ = c.run(raw, |c| {
            let input = begin(&mut a, c);
            commands::route(&mut a, c);
            end(&mut a, c, input);
        });
    };
    run(vec![key(Key::P, Modifiers::CTRL | Modifiers::SHIFT)]);
    run(vec![egui::Event::Text("閉じる".into())]);
    run(vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(!a.palette.is_open());
    assert!(!a.analysis.is_open());
    assert!(a.loaded.is_some());
}
