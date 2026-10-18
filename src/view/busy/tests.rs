use super::*;
use eframe::egui::{Key, Modifiers};

fn press(app: &mut App, ctx: &egui::Context, key: Key, modifiers: Modifiers) {
    let _ = ctx.run(
        egui::RawInput {
            modifiers,
            events: vec![egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
}

fn idle(app: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    ctx.run(Default::default(), |ctx| {
        crate::view::show(app, ctx);
        app.close(ctx);
    })
}

#[test]
fn ctrl_n_creates_the_song_one_pass_after_the_modal() {
    let mut app = App::empty();
    app.audition = crate::play::audition::Audition::disabled();
    let ctx = egui::Context::default();
    press(&mut app, &ctx, Key::N, Modifiers::CTRL);
    assert!(pending(&app) && app.loaded.is_none());
    idle(&mut app, &ctx);
    assert!(!pending(&app) && app.loaded.is_some());
}

#[test]
fn i_opens_mml_input_one_pass_after_the_modal() {
    let mut app = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }],
    });
    let ctx = egui::Context::default();
    press(&mut app, &ctx, Key::I, Modifiers::NONE);
    assert!(pending(&app) && !app.mml_input.open);
    idle(&mut app, &ctx);
    assert!(!pending(&app) && app.mml_input.open);
}

#[test]
fn approved_close_is_sent_one_pass_after_the_modal() {
    let mut app = App::empty();
    app.audition = crate::play::audition::Audition::disabled();
    let ctx = egui::Context::default();
    let commands = |output: egui::FullOutput| -> Vec<egui::ViewportCommand> {
        output
            .viewport_output
            .into_values()
            .flat_map(|v| v.commands)
            .collect()
    };
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let output = ctx.run(input, |ctx| {
        crate::view::show(&mut app, ctx);
        app.close(ctx);
    });
    assert!(commands(output).contains(&egui::ViewportCommand::CancelClose));
    assert!(app.config.navigation.close_approved);
    assert!(commands(idle(&mut app, &ctx)).contains(&egui::ViewportCommand::Close));
}
