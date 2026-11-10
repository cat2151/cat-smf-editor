//! Other windows are read and commanded only while the backend has them.
use super::super::{finish, input_for, send_to, Native};
use crate::view::patterns::workspace_tests::{context, harness};
use eframe::egui::{ViewportCommand, ViewportId};

#[test]
fn a_window_not_shown_is_neither_read_nor_commanded_into_existence() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let host = ViewportId::from_hash_of("windows-test-host");
    let absent = ViewportId::from_hash_of("windows-test-absent");
    for frame in 0..3 {
        let _ = c.run(Default::default(), |c| {
            assert_eq!(input_for(c, absent, |i| i.focused), None);
            send_to(c, absent, ViewportCommand::Focus);
            Native::Keyboard.show(&mut a, c, host, Default::default(), |_, _, _| {});
            finish(&a, c);
        });
        assert_eq!(
            harness::unshown(&c),
            Vec::<ViewportId>::new(),
            "frame {frame}"
        );
    }
}

#[test]
fn a_command_before_a_window_first_shows_reaches_it_at_the_end_of_the_frame() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let window = ViewportId::from_hash_of("windows-test-new");
    let out = c.run(Default::default(), |c| {
        send_to(c, window, ViewportCommand::Focus);
        Native::Keyboard.show(&mut a, c, window, Default::default(), |_, _, _| {});
        finish(&a, c);
    });
    let commands = &out.viewport_output[&window].commands;
    assert!(commands.contains(&ViewportCommand::Focus), "{commands:?}");
}

#[test]
fn closing_a_bound_chain_focuses_its_restored_window_once_instead_of_root() {
    use crate::view::{dock::ViewKind, effects};
    use eframe::egui::{self, Key, Modifiers};
    for view in [ViewKind::Keyboard, ViewKind::StepSequencer] {
        let mut a = crate::view::palette::tests::app();
        let c = context();
        let host = ViewportId::from_hash_of("bound-chain-origin");
        let track = a.track;
        effects::entrances::bound(&mut a, track, view, host);
        let mut returned = 0;
        for frame in 0..6 {
            let out = c.run(
                egui::RawInput {
                    events: if frame == 2 {
                        [true, false]
                            .into_iter()
                            .map(|pressed| egui::Event::Key {
                                key: Key::Escape,
                                physical_key: None,
                                pressed,
                                repeat: false,
                                modifiers: Modifiers::NONE,
                            })
                            .collect()
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
                |c| {
                    effects::show(&mut a, c);
                    Native::Keyboard.show(&mut a, c, host, Default::default(), |_, _, _| {});
                    finish(&a, c);
                },
            );
            if out
                .viewport_output
                .get(&host)
                .is_some_and(|v| v.commands.contains(&ViewportCommand::Focus))
            {
                assert!(!a.effect_editor.is_open());
                assert!(!out.viewport_output[&ViewportId::ROOT]
                    .commands
                    .contains(&ViewportCommand::Focus));
                returned += 1;
            }
        }
        assert_eq!(returned, 1);
        assert!(harness::unshown(&c).is_empty());
    }
}

#[test]
fn an_unchanged_app_frame_sends_no_visible_command_to_any_window() {
    use crate::view::patterns::workspace_tests::{frame, key};
    use eframe::egui::{self, Key, Modifiers};
    let mut a = crate::view::palette::tests::app();
    let c = context();
    key(&mut a, &c, ViewportId::ROOT, Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    for _ in 0..3 {
        frame(&mut a, &c, ViewportId::ROOT, vec![]);
    }
    let windows = c.data(|d| {
        d.get_temp::<Vec<ViewportId>>(egui::Id::new("workspace-rendered"))
            .unwrap_or_default()
    });
    assert!(windows.contains(&crate::view::recording::window::viewport()));
    for id in windows.into_iter().chain([ViewportId::ROOT]) {
        let commands = c
            .data(|d| d.get_temp::<Vec<ViewportCommand>>(egui::Id::new(("workspace-commands", id))))
            .unwrap_or_default();
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, ViewportCommand::Visible(_))),
            "{id:?}: {commands:?}"
        );
    }
}
