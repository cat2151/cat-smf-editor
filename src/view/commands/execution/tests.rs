use super::*;
use crate::view::palette::tests::{app, frame, press};
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
#[test]
fn public_input_starts_without_search_and_cancels_without_editing() {
    let mut a = app();
    let c = egui::Context::default();
    let before = bytes(&a);
    let request = Request::new(
        &a,
        Operation::InsertEvent {
            kind: Some(Kind::Tempo),
            text: None,
        },
    );
    assert_eq!(submit(&mut a, &c, request), Ok(Execution::Input));
    assert!(!a.palette.is_open());
    assert!(a.command_input.is_open());
    frame(&mut a, &c, vec![]);
    assert!(a.command_input.is_open());
    press(&mut a, &c, Key::Escape);
    assert!(!a.command_input.is_open());
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}
#[test]
fn saved_requests_and_running_input_reject_external_target_changes() {
    for change in 0..5 {
        let mut a = app();
        let c = egui::Context::default();
        let before = bytes(&a);
        let request = Request::new(
            &a,
            Operation::InsertEvent {
                kind: Some(Kind::Tempo),
                text: Some("140".into()),
            },
        );
        match change {
            0 => a.song_id += 1,
            1 => a.generation += 1,
            2 => a.track = 1,
            3 => a.selected_event = Some(1),
            _ => a.track_view.cursor_tick = Some(480),
        }
        assert!(submit(&mut a, &c, request).is_err());
        assert_eq!(bytes(&a), before);
        let request = Request::new(
            &a,
            Operation::InsertEvent {
                kind: Some(Kind::Tempo),
                text: None,
            },
        );
        submit(&mut a, &c, request).unwrap();
        frame(&mut a, &c, vec![]);
        a.generation += 1;
        frame(&mut a, &c, vec![egui::Event::Text("140".into())]);
        press(&mut a, &c, Key::Enter);
        assert!(!a.command_input.is_open());
        assert_eq!(bytes(&a), before);
        assert!(!a.undo.available());
    }
}

#[test]
fn every_registered_interactive_handler_runs_without_palette_special_cases() {
    for id in [
        "insert-event",
        "insert-tempo",
        "insert-key-signature",
        "insert-same-measure",
        "browse-views",
        "duplicate-pattern",
        "preview-pattern",
        "move-pattern-container",
        "pattern-structure",
        "pattern-audition-tempo",
        "pattern-midi-baseline",
    ] {
        let mut a = app();
        let c = egui::Context::default();
        let before = bytes(&a);
        let command = candidates(&a, &c).into_iter().find(|x| x.id == id).unwrap();
        assert!(execute(&mut a, &c, &command), "{id}");
        assert!(a.command_input.is_open(), "{id}");
        assert!(!a.palette.is_open());
        frame(&mut a, &c, vec![]);
        press(&mut a, &c, Key::Escape);
        assert!(!a.command_input.is_open(), "{id}");
        assert_eq!(bytes(&a), before, "{id}");
        assert!(!a.undo.available());
    }
}
#[test]
fn tempo_supplied_public_input_and_palette_have_identical_atomic_results() {
    use crate::view::palette::tests::{open, query};
    let mut results = vec![];
    for entry in 0..3 {
        let mut a = app();
        let c = egui::Context::default();
        let before = bytes(&a);
        if entry == 0 {
            let request = Request::new(
                &a,
                Operation::InsertEvent {
                    kind: Some(Kind::Tempo),
                    text: Some("140".into()),
                },
            );
            assert_eq!(submit(&mut a, &c, request), Ok(Execution::Completed));
        } else {
            if entry == 1 {
                let request = Request::new(
                    &a,
                    Operation::InsertEvent {
                        kind: Some(Kind::Tempo),
                        text: None,
                    },
                );
                submit(&mut a, &c, request).unwrap();
            } else {
                open(&mut a, &c);
                query(&mut a, &c, "insert tempo");
                press(&mut a, &c, Key::Enter);
            }
            frame(&mut a, &c, vec![]);
            frame(&mut a, &c, vec![egui::Event::Text("140".into())]);
            press(&mut a, &c, Key::Enter);
        }
        assert!(!a.command_input.is_open());
        let event = &a.loaded.as_ref().unwrap().music.tracks[0].events[0];
        assert_eq!(
            (event.meta_type, event.data.clone()),
            (0x51, vec![6, 138, 27])
        );
        results.push(bytes(&a));
        a.undo_music().unwrap();
        assert_eq!(bytes(&a), before);
        assert!(!a.undo.available());
    }
    assert!(results.windows(2).all(|p| p[0] == p[1]));
}
#[test]
fn public_pattern_arguments_reach_the_same_validation_and_undo() {
    use cat_smf_editor::score::pattern::{operations, Catalog};
    let mut music = app().loaded.unwrap().music;
    let track = operations::create_source(&mut music).unwrap();
    let id = operations::create(&mut music, track, None).unwrap();
    let mut a = crate::view::tests::app(music);
    let c = egui::Context::default();
    let before = bytes(&a);
    let request = Request::new(
        &a,
        Operation::Pattern {
            action: Action::Structure,
            pattern: Some(id),
            value: Some("3/4 5/8".into()),
            container: None,
        },
    );
    assert_eq!(submit(&mut a, &c, request), Ok(Execution::Completed));
    assert_eq!(
        Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns[&id]
            .meters
            .len(),
        2
    );
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
    let request = Request::new(
        &a,
        Operation::Pattern {
            action: Action::Structure,
            pattern: Some(id),
            value: Some("invalid".into()),
            container: None,
        },
    );
    assert!(submit(&mut a, &c, request).is_err());
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}

#[test]
fn projection_changes_invalidate_requests_even_with_unchanged_selected_pattern() {
    use cat_smf_editor::score::pattern::editing::Context;
    for change in 0..3 {
        let mut a = app();
        let c = egui::Context::default();
        let before = bytes(&a);
        let request = Request::new(
            &a,
            Operation::InsertEvent {
                kind: Some(Kind::Tempo),
                text: Some("140".into()),
            },
        );
        let context = Context {
            track: 0,
            measure: 1,
            pattern: 42,
            placement: None,
            scene: None,
            standalone: true,
        };
        match change {
            0 => a.note_mode = !a.note_mode,
            1 => a.patterns.editing = Some(context),
            _ => a.patterns.raw_context = Some(context),
        }
        assert!(submit(&mut a, &c, request).is_err());
        assert_eq!(bytes(&a), before);
        assert!(!a.undo.available());
    }
}
#[test]
fn event_value_can_precede_the_common_kind_picker() {
    let mut a = app();
    let c = egui::Context::default();
    let request = Request::new(
        &a,
        Operation::InsertEvent {
            kind: None,
            text: Some("140".into()),
        },
    );
    assert_eq!(submit(&mut a, &c, request), Ok(Execution::Input));
    frame(&mut a, &c, vec![]);
    press(&mut a, &c, Key::Enter);
    press(&mut a, &c, Key::Enter);
    assert!(!a.command_input.is_open());
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[0].events[0].meta_type,
        0x51
    );
}
#[test]
fn supplied_creation_container_and_common_picker_create_the_same_mml_target() {
    use cat_smf_editor::score::pattern::{operations, Catalog};
    let mut music = app().loaded.unwrap().music;
    operations::create_source(&mut music).unwrap();
    operations::create_source(&mut music).unwrap();
    let catalog = Catalog::read(&music).unwrap();
    let id = catalog
        .tracks
        .iter()
        .filter(|(_, t)| catalog.sources.contains(t))
        .min_by_key(|(_, t)| *t)
        .map(|(&id, _)| id)
        .unwrap();
    let mut results = vec![];
    for supplied in [true, false] {
        let mut a = crate::view::tests::app(music.clone());
        let c = egui::Context::default();
        let request = Request::new(
            &a,
            Operation::Pattern {
                action: Action::Create,
                pattern: None,
                value: None,
                container: supplied.then_some(id),
            },
        );
        submit(&mut a, &c, request).unwrap();
        if !supplied {
            frame(&mut a, &c, vec![]);
            press(&mut a, &c, Key::Enter);
        }
        for _ in 0..3 {
            frame(&mut a, &c, vec![]);
        }
        assert!(a.mml_input.open);
        let pattern = a.patterns.selected.unwrap();
        assert_eq!(
            Catalog::read(&a.loaded.as_ref().unwrap().music)
                .unwrap()
                .patterns[&pattern]
                .container,
            id
        );
        let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
        let definition = &catalog.patterns[&pattern];
        assert!(a
            .patterns
            .editing
            .as_ref()
            .is_some_and(|c| c.pattern == pattern && c.standalone));
        let mut normal_before = music.clone();
        normal_before.tracks.truncate(3);
        let mut normal_after = a.loaded.as_ref().unwrap().music.clone();
        normal_after.tracks.truncate(3);
        assert_eq!(
            cat_smf_editor::smf_io::encode(&normal_after).unwrap(),
            cat_smf_editor::smf_io::encode(&normal_before).unwrap()
        );
        results.push((
            definition.container,
            definition.name.clone(),
            definition.meters.clone(),
        ));
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn input_commit_owns_the_closing_frame_across_layout_retries() {
    let mut a = app();
    let c = egui::Context::default();
    let before = bytes(&a);
    let request = Request::new(
        &a,
        Operation::InsertEvent {
            kind: Some(Kind::Tempo),
            text: None,
        },
    );
    submit(&mut a, &c, request).unwrap();
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![egui::Event::Text("140".into())]);
    let _ = c.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events: vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |c| {
            crate::view::show(&mut a, c);
            c.request_discard("command input closing frame");
        },
    );
    assert!(!a.command_input.is_open());
    assert!(!a.mml_input.open);
    assert!(a.note_edit.is_none());
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[0]
            .events
            .iter()
            .filter(|e| e.meta_type == 0x51)
            .count(),
        1
    );
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}
