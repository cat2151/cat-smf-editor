use super::context::{observe, Target};
use crate::view::commands::{owner, Screen};
use eframe::egui::{self, Key, Modifiers};

fn app() -> crate::app::App {
    crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }],
    })
}

#[test]
fn focused_viewport_wins_and_help_focus_keeps_last_live_target() {
    let mut a = app();
    let c = egui::Context::default();
    a.piano_roll.open = true;
    let _ = c.run(Default::default(), |c| observe(&mut a, c));
    assert_eq!(a.help.target, Target::Piano);
    let song = crate::view::patterns::workspace::Pane::Song.viewport();
    a.patterns.workspace_open = true;
    let _ = c.run(viewport_input(song), |c| {
        observe(&mut a, c);
        assert_eq!(owner(&a, c), Screen::Song);
    });
    assert_eq!(a.help.target, Target::Song);
    let _ = c.run(viewport_input(super::viewport()), |c| {
        observe(&mut a, c);
        assert_eq!(owner(&a, c), Screen::Help);
    });
    assert_eq!(a.help.target, Target::Song);
    a.patterns.workspace_open = false;
    let _ = c.run(
        egui::RawInput {
            focused: false,
            ..Default::default()
        },
        |c| observe(&mut a, c),
    );
    assert_eq!(a.help.target, Target::Piano);
}

#[test]
fn timeline_front_window_selects_content_and_overview_takes_input() {
    let mut a = app();
    a.piano_roll.open = true;
    a.tempo_curve.open = true;
    let c = egui::Context::default();
    let _ = c.run(Default::default(), |c| {
        egui::Window::new("Tempo curve").show(c, |ui| {
            ui.label("curve");
        });
    });
    let _ = c.run(Default::default(), |c| observe(&mut a, c));
    assert_eq!(a.help.target, Target::Tempo);
    a.track_overview.open = true;
    let _ = c.run(Default::default(), |c| observe(&mut a, c));
    assert_eq!(a.help.target, Target::Overview);
}

#[test]
fn help_keys_never_dispatch_editing_or_spectrum_close() {
    let mut a = app();
    a.help_open = true;
    a.piano_roll.open = true;
    let c = egui::Context::default();
    for key in [Key::Escape, Key::F1, Key::Space, Key::R] {
        let _ = c.run(
            egui::RawInput {
                viewports: viewport_input(super::viewport()).viewports,
                viewport_id: super::viewport(),
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |c| crate::view::commands::route(&mut a, c),
        );
        assert!(a.piano_roll.open);
        assert!(a.playback.active.is_none() && a.playback.pending.is_none());
    }
}

fn viewport_input(id: egui::ViewportId) -> egui::RawInput {
    let mut input = egui::RawInput {
        viewport_id: id,
        ..Default::default()
    };
    input.viewports.insert(
        id,
        egui::ViewportInfo {
            focused: Some(true),
            ..Default::default()
        },
    );
    input
}
