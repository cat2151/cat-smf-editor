use super::*;
use crate::view::palette::tests::{frame, press};
use cat_smf_editor::score::note_view::note_on;
use eframe::egui::{self, Key};

/// Runs a palette command; an empty song first gets one sound container to confirm.
fn command(a: &mut App, c: &egui::Context, s: &str) {
    use crate::view::palette::tests as palette;
    if cat_smf_editor::score::pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .sources
        .is_empty()
    {
        a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
            .unwrap();
    }
    palette::open(a, c);
    palette::query(a, c, s);
    palette::press(a, c, eframe::egui::Key::Enter);
    if a.patterns.confirming_creation() {
        palette::press(a, c, eframe::egui::Key::Enter);
    }
}

fn text(a: &mut App, c: &egui::Context, s: &str) {
    frame(a, c, vec![]);
    frame(a, c, vec![egui::Event::Text(s.into())]);
    press(a, c, Key::Enter);
}
#[test]
fn root_pattern_input_cancel_invalid_and_ime_keep_context_and_song() {
    let mut a = crate::view::palette::tests::app();
    let c = egui::Context::default();
    command(&mut a, &c, "create pattern");
    let id = a.patterns.selected.unwrap();
    let saved = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    frame(&mut a, &c, vec![]);
    frame(
        &mut a,
        &c,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))],
    );
    press(&mut a, &c, Key::Enter);
    assert!(a.mml_input.open);
    frame(&mut a, &c, vec![egui::Event::Ime(egui::ImeEvent::Disabled)]);
    text(&mut a, &c, "r1 r1"); // No sounding notes: still invalid for Insert MML.
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    assert!(a.mml_input.open);
    assert_eq!(a.mml_input.buffer, "r1 r1");
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    press(&mut a, &c, Key::Escape);
    assert!(!a.mml_input.open);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        saved
    );
    assert!(mode::workspace_open(&a));
    a.select_track(0);
    frame(&mut a, &c, vec![]);
    assert!(!mode::workspace_open(&a));
}

fn labels(a: &mut App, c: &egui::Context) -> Vec<String> {
    fn visit(s: &egui::epaint::Shape, out: &mut Vec<String>) {
        match s {
            egui::epaint::Shape::Text(t) => out.push(t.galley.job.text.clone()),
            egui::epaint::Shape::Vec(shapes) => {
                for s in shapes {
                    visit(s, out);
                }
            }
            _ => {}
        }
    }
    let output = c.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            ..Default::default()
        },
        |c| crate::view::show(a, c),
    );
    let mut out = vec![];
    for shape in output.shapes {
        visit(&shape.shape, &mut out);
    }
    out
}
#[test]
fn focused_ordinary_editors_and_event_pane_share_live_pattern_identity() {
    let mut a = crate::view::palette::tests::app();
    let c = egui::Context::default();
    command(&mut a, &c, "create pattern");
    let name = presentation::name(&a, a.track).unwrap();
    frame(&mut a, &c, vec![]);
    let texts = labels(&mut a, &c);
    assert!(
        texts
            .iter()
            .filter(|t| *t == &format!("Pattern · {name}"))
            .count()
            >= 2,
        "{texts:?}"
    );
    assert!(c.memory(|m| m.focused()).is_some());
    text(&mut a, &c, "o5 l4 g");
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    press(&mut a, &c, Key::Home);
    press(&mut a, &c, Key::E);
    press(&mut a, &c, Key::P);
    assert!(a.note_edit.is_some());
    let texts = labels(&mut a, &c);
    assert!(
        texts
            .iter()
            .filter(|t| *t == &format!("Pattern · {name}"))
            .count()
            >= 2,
        "{texts:?}"
    );
    press(&mut a, &c, Key::Escape);
    a.select_track(0);
    let texts = labels(&mut a, &c);
    assert!(!texts.iter().any(|t| t.starts_with("Pattern · ")));
}

#[test]
fn full_measure_mml_keeps_independent_body_open_and_cursor_auditions_its_notes() {
    let mut a = crate::view::palette::tests::app();
    let c = egui::Context::default();
    command(&mut a, &c, "create pattern");
    let id = a.patterns.selected.unwrap();
    frame(&mut a, &c, vec![]);
    text(&mut a, &c, "o5 l1 g");
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    assert!(!a.mml_input.open);
    assert!(a.patterns.editing.as_ref().unwrap().standalone);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert!(presentation::name(&a, a.track).is_some());
    assert!(mode::workspace_open(&a));
    let p = preview::prepare(&a, id).unwrap();
    assert_eq!(p.parts[0].2.notes[0].duration, 2.0);
    let l = a.loaded.as_ref().unwrap();
    let on = l.visible().tracks[a.track]
        .events
        .iter()
        .position(note_on)
        .unwrap();
    let row = l.display_rows[a.track].event_row(on).unwrap();
    a.audition.auditioned.clear();
    crate::cursor::moves::select(&mut a, row);
    assert_eq!(a.audition.auditioned, vec![p.parts[0].2.notes[0].pitch]);
    command(&mut a, &c, "pattern song");
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    mode::close_workspace(&mut a);
    assert!(a.patterns.editing.is_none());
    assert!(!mode::workspace_open(&a));
}

#[test]
fn typed_chord_keeps_pattern_input_alive_during_preview() {
    for note_mode in [true, false] {
        for count in [1, 2] {
            let mut a = crate::view::palette::tests::app();
            a.note_mode = note_mode;
            let c = egui::Context::default();
            for n in 0..count {
                if n > 0 {
                    press(&mut a, &c, Key::Escape);
                }
                command(&mut a, &c, "create pattern");
                frame(&mut a, &c, vec![]);
                assert!(a.mml_input.open);
            }
            let edit = a.patterns.editing.clone().unwrap();
            let name = presentation::name(&a, a.track).unwrap();
            let saved = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
            // Physical key plus text, followed by idle preview frames, like typing.
            frame(
                &mut a,
                &c,
                vec![
                    egui::Event::Key {
                        key: Key::C,
                        physical_key: Some(Key::C),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::SHIFT,
                    },
                    egui::Event::Text("C".into()),
                ],
            );
            for n in 0..8 {
                frame(&mut a, &c, vec![]);
                assert!(a.mml_input.open, "input closed during preview frame {n}");
                assert_eq!(a.patterns.editing.as_ref(), Some(&edit));
                assert_eq!(presentation::name(&a, a.track), Some(name.clone()));
                assert!(mode::workspace_open(&a));
            }
            let expected = preview::prepare(&a, edit.pattern).unwrap().parts[0]
                .2
                .notes
                .clone();
            assert_eq!(
                expected.len(),
                3,
                "C is a chord, not the single-note fixture"
            );
            // Cancel after a live preview must restore the empty body and retain editing.
            press(&mut a, &c, Key::Escape);
            assert_eq!(
                cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
                saved
            );
            assert_eq!(a.patterns.editing.as_ref(), Some(&edit));
            press(&mut a, &c, Key::I);
            frame(&mut a, &c, vec![]);
            frame(&mut a, &c, vec![egui::Event::Text("C".into())]);
            for _ in 0..4 {
                frame(&mut a, &c, vec![]);
            }
            press(&mut a, &c, Key::Enter);
            for _ in 0..4 {
                frame(&mut a, &c, vec![]);
            }
            assert!(!a.mml_input.open);
            assert_eq!(a.patterns.editing.as_ref(), Some(&edit));
            assert!(labels(&mut a, &c).contains(&format!("Pattern · {name}")));
            a.audition.auditioned.clear();
            press(&mut a, &c, Key::Home);
            press(&mut a, &c, Key::ArrowDown);
            press(&mut a, &c, Key::ArrowUp);
            assert!(
                !a.audition.auditioned.is_empty(),
                "cursor must audition the body"
            );
            press(&mut a, &c, Key::P);
            press(&mut a, &c, Key::A);
            let p = a.playback.pending.as_ref().unwrap();
            assert_eq!(p.parts.len(), 1);
            assert_eq!(p.parts[0].2.notes.len(), 3);
            assert_eq!(p.parts[0].1, Default::default());
            crate::play::transport::stop(&mut a);
            a.undo_music().unwrap();
            assert_eq!(
                cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
                saved
            );
            assert_eq!(a.patterns.editing.as_ref(), Some(&edit));
        }
    }
}

#[test]
fn independent_multimeasure_mml_and_raw_coordinates_preserve_adjacent_body() {
    let (mut a, source, id) = edit_tests::fixture();
    let other = cat_smf_editor::score::pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .values()
        .find(|p| p.id != id)
        .unwrap()
        .id;
    a.edit_tracks(|m| {
        cat_smf_editor::score::pattern::operations::relocate(
            m,
            other,
            cat_smf_editor::score::pattern::Catalog::read(m)?.patterns[&id].container,
            20000,
        )
    })
    .unwrap();
    a.edit_tracks(|m| {
        cat_smf_editor::score::pattern::operations::resize(
            m,
            other,
            vec![
                cat_smf_editor::score::scene::Meter {
                    numerator: 4,
                    exponent: 2,
                    ticks: 1920
                };
                4
            ],
        )
    })
    .unwrap();
    library::edit(&mut a, other).unwrap();
    let snapshot = |a: &App| {
        preview::prepare(a, id).unwrap().parts[0]
            .2
            .notes
            .iter()
            .map(|n| (n.start, n.duration, n.pitch, n.velocity))
            .collect::<Vec<_>>()
    };
    let original = snapshot(&a);
    let c = egui::Context::default();
    press(&mut a, &c, Key::I);
    text(&mut a, &c, "o4 l1 c d e");
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    assert!(!a.mml_input.open, "{:?}", a.load_error);
    assert_eq!(a.track, source);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, other);
    assert_eq!(snapshot(&a), original);
    let count = preview::prepare(&a, other).unwrap().parts[0].2.notes.len();
    assert!(count >= 3);
    crate::cursor::moves::at_tick(&mut a, 1920);
    press(&mut a, &c, Key::V);
    assert_eq!(crate::play::playback::location(&a).0, 21920);
    press(&mut a, &c, Key::V);
    assert_eq!(crate::play::playback::location(&a).0, 1920);
}
