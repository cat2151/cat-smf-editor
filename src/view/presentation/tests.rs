use super::*;
use crate::view::palette::tests::windows::{fixture, ALL};
use crate::view::patterns::workspace_tests::{context, frame, key};
use egui::{Key, Modifiers, ViewportId};
const ROOT: ViewportId = ViewportId::ROOT;

/// Floating windows registered so far in the last frame of window `id`.
pub(crate) fn registered(c: &egui::Context, id: ViewportId) -> usize {
    c.data(|d| d.get_temp::<Layers>(egui::Id::new(("presentation-layers", id))))
        .map_or(0, |l| l.registered.len())
}
const FAMILIES: [Native; 5] = [
    Native::Analysis,
    Native::Workspace,
    Native::Keyboard,
    Native::Help,
    Native::Dock,
];

#[test]
fn every_root_owner_controls_all_native_families() {
    for screen in ALL {
        let Some((mut a, _, hosts)) = fixture(screen) else {
            continue;
        };
        if hosts != [ROOT] {
            continue;
        }
        let c = context();
        let expected = matches!(
            screen,
            Screen::Main
                | Screen::Score
                | Screen::Timeline
                | Screen::Mixer
                | Screen::Drums
                | Screen::Selection
        );
        for native in FAMILIES {
            let id = ViewportId::from_hash_of(format!("{native:?}"));
            let mut ran = false;
            let _ = c.run(Default::default(), |c| {
                native.show(&mut a, c, id, Default::default(), |_, _, _| ran = true);
            });
            assert_eq!(ran, expected, "{screen:?} / {native:?}");
            let visible = c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))));
            assert_eq!(visible, Some(expected), "{screen:?} / {native:?}");
        }
    }
}

#[test]
fn native_transition_skips_hidden_input_and_restores_existing_geometry() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let id = ViewportId::from_hash_of("companion");
    let at = egui::pos2(30.0, 40.0);
    let mut calls = 0;
    for blocked in [false, true, true, false] {
        a.edit_menu.open = blocked;
        let _ = c.run(Default::default(), |c| {
            Native::Analysis.show(
                &mut a,
                c,
                id,
                egui::ViewportBuilder::default().with_position(at),
                |_, _, _| calls += 1,
            );
        });
        let builder = c
            .data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
            .unwrap();
        assert_eq!(builder.visible, Some(!blocked));
        assert_eq!(builder.position, Some(at));
    }
    assert_eq!(calls, 2);
}

#[test]
fn selection_menu_hides_keyboard_family_ignores_stale_escape_and_restores() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let keyboard = crate::view::recording::window::viewport();
    key(&mut a, &c, ROOT, Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    crate::view::event_selection::tests::open_menu(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    let visible = |id| {
        c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
            .unwrap()
    };
    assert!(!visible(keyboard));
    assert!(!visible(crate::view::recording::piano_viewport()));
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open());
    assert_eq!(commands::root_owner(&a), Screen::SelectionMenu);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(visible(keyboard));
    assert!(a.patterns.recording.is_open());
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
}

#[test]
fn floating_input_is_above_workspace_and_owns_pointer() {
    let mut a = crate::view::palette::tests::app();
    let c = egui::Context::default();
    a.edit_menu.open = true;
    let operation = egui::Id::new("operation");
    let background = egui::Id::new("workspace");
    for _ in 0..3 {
        let _ = c.run(Default::default(), |c| {
            prepare(&a, c);
            window(c, Screen::Edit, operation, "operation")
                .fixed_pos([30.0, 30.0])
                .show(c, |ui| {
                    ui.label("edit");
                });
            // Deliberately drawn later and deliberately covering the operation.
            window(c, Screen::Timeline, background, "workspace")
                .fixed_pos([30.0, 30.0])
                .fixed_size([300.0, 300.0])
                .show(c, |ui| {
                    assert!(!ui.is_enabled());
                });
            finish(&a, c);
        });
    }
    assert_eq!(
        c.layer_id_at(egui::pos2(50.0, 50.0)),
        Some(egui::LayerId::new(egui::Order::Foreground, operation))
    );
}

#[test]
fn explicitly_summoned_help_is_allowed_only_for_current_operation() {
    let mut a = crate::view::palette::tests::app();
    let c = egui::Context::default();
    let id = crate::view::help::viewport();
    a.edit_menu.open = true;
    assert!(!Native::Help.visible(&a, &c, id));
    super::super::help::summon(&mut a);
    assert!(Native::Help.visible(&a, &c, id));
    a.help.raise = false;
    a.edit_menu.open = false;
    a.mml_input.open = true;
    assert!(!Native::Help.visible(&a, &c, id));
}

#[test]
fn later_child_modal_hides_earlier_native_windows_and_restores_on_close() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let analysis = ViewportId::from_hash_of("analysis-test");
    let host = ViewportId::from_hash_of("dialog-host-test");
    let keyboard = ViewportId::from_hash_of("keyboard-test");
    for blocked in [true, true, false, false, false] {
        let out = c.run(Default::default(), |c| {
            Native::Analysis.show(&mut a, c, analysis, Default::default(), |_, _, _| {});
            Native::Workspace.show(&mut a, c, host, Default::default(), |_, c, _| {
                if blocked {
                    super::modal(c, egui::Id::new("child-modal")).show(c, |ui| {
                        ui.label("input");
                    });
                }
            });
            Native::Keyboard.show(&mut a, c, keyboard, Default::default(), |_, _, _| {});
            finish(&a, c);
        });
        if blocked {
            for id in [analysis, keyboard] {
                assert_eq!(
                    out.viewport_output[&id].commands.last(),
                    Some(&egui::ViewportCommand::Visible(false))
                );
            }
        }
    }
    assert!(Native::Analysis.visible(&a, &c, analysis));
    assert!(Native::Keyboard.visible(&a, &c, keyboard));
}

#[test]
fn hidden_native_siblings_do_not_cover_a_palette_in_its_host() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let keyboard = crate::view::recording::window::viewport();
    key(&mut a, &c, ROOT, Key::R, Modifiers::SHIFT);
    key(
        &mut a,
        &c,
        keyboard,
        Key::P,
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    assert_eq!(a.palette.viewport(), Some(keyboard));
    assert!(Native::Keyboard.visible(&a, &c, keyboard));
    assert!(!Native::Keyboard.visible(&a, &c, crate::view::recording::piano_viewport()));
    assert!(!Native::Analysis.visible(&a, &c, ViewportId::from_hash_of("analysis")));
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!a.palette.is_open());
    assert!(Native::Keyboard.visible(&a, &c, crate::view::recording::piano_viewport()));
}

#[test]
fn menu_prevents_background_track_clicks() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    crate::view::event_selection::tests::open_menu(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    let rect = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("actual-track", 1))))
        .unwrap();
    let track = a.track;
    for pressed in [true, false] {
        frame(
            &mut a,
            &c,
            ROOT,
            vec![
                egui::Event::PointerMoved(rect.center()),
                egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(a.track, track);
    assert_eq!(commands::root_owner(&a), Screen::SelectionMenu);
}

#[test]
fn production_surfaces_use_the_shared_gateway() {
    fn check(dir: &std::path::Path) {
        for e in std::fs::read_dir(dir).unwrap() {
            let path = e.unwrap().path();
            if path.is_dir() {
                check(&path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") || path.to_string_lossy().contains("test")
            {
                continue;
            }
            let relative = path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative == "src/view/presentation.rs"
                || relative == "src/view/presentation/popups.rs"
                || relative == "src/view/presentation/windows.rs"
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for bypass in [".input_for(", "send_viewport_cmd_to(", ".viewport_for("] {
                assert!(!source.contains(bypass), "native window bypass: {relative}");
            }
            for bypass in [
                ".context_menu(",
                ".menu_button(",
                "egui::popup_below_widget(",
            ] {
                assert!(!source.contains(bypass), "popup bypass: {relative}");
            }
            assert!(
                !source.contains("egui::Modal::new("),
                "modal bypass: {relative}"
            );
            // Native fallbacks are already gated; preview is an intentionally inert rendering.
            let fallback = matches!(
                relative.as_str(),
                "src/view/recording/window.rs"
                    | "src/view/recording/capture.rs"
                    | "src/view/recording/companions.rs"
                    | "src/view/patterns/workspace.rs"
                    | "src/view/dock/window.rs"
                    | "src/view/command_input/views/preview.rs"
            );
            assert!(
                fallback || !source.contains("egui::Window::new("),
                "window bypass: {relative}"
            );
        }
    }
    check(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}

#[test]
fn root_context_menu_registers_its_host_before_native_windows_draw() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let popup = egui::Id::new("test-popup");
    let id = ViewportId::from_hash_of("native-popup-test");
    for _ in 0..2 {
        let _ = c.run(Default::default(), |c| {
            prepare(&a, c);
            egui::CentralPanel::default().show(c, |ui| {
                let response = ui.button("menu");
                ui.memory_mut(|m| m.open_popup(popup));
                super::popup_below_widget(
                    ui,
                    popup,
                    &response,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.label("input");
                    },
                );
            });
            assert!(!Native::Keyboard.visible(&a, c, id));
            Native::Keyboard.show(&mut a, c, id, Default::default(), |_, _, _| {
                panic!("covered popup received input")
            });
            finish(&a, c);
        });
    }
    let _ = c.run(Default::default(), |c| {
        prepare(&a, c);
        c.memory_mut(|m| m.close_popup());
        finish(&a, c);
    });
    assert!(Native::Keyboard.visible(&a, &c, id));
}

#[test]
fn restoration_finishes_at_the_previous_operation_host() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let id = ViewportId::from_hash_of("restored-native");
    for blocked in [true, false] {
        a.edit_menu.open = blocked;
        let output = c.run(Default::default(), |c| {
            Native::Analysis.show(&mut a, c, id, Default::default(), |_, _, _| {});
            finish(&a, c);
        });
        assert_eq!(
            output.viewport_output[&ROOT].commands.last(),
            Some(&egui::ViewportCommand::Focus)
        );
    }
}

#[test]
fn popup_owns_escape_instead_of_the_underlying_menu() {
    let mut a = crate::view::palette::tests::app();
    let c = egui::Context::default();
    a.edit_menu.open = true;
    c.memory_mut(|m| m.open_popup(egui::Id::new("menu-popup")));
    let _ = c.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |c| commands::route(&mut a, c),
    );
    assert!(a.edit_menu.open);
}

#[test]
fn menu_registration_owns_keys_before_next_frames_widgets_run() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let host = ViewportId::from_hash_of("menu-owner-test");
    for frame in 0..3 {
        let _ = c.run(Default::default(), |c| {
            Native::Keyboard.show(&mut a, c, host, Default::default(), |_, c, _| {
                assert!(!c.memory(|m| m.any_popup_open()));
                if frame == 0 {
                    super::claim(c, true);
                } else {
                    assert_eq!(popup_owns_input(c), frame == 1);
                }
            });
            finish(&a, c);
        });
    }
}
mod panes;
mod windows;
