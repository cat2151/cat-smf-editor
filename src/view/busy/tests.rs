use super::*;
use eframe::egui::{Key, Modifiers};

fn press(app: &mut App, ctx: &egui::Context, key: Key, modifiers: Modifiers) -> egui::FullOutput {
    ctx.run(
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
        |ctx| {
            crate::view::show(app, ctx);
            show(app, ctx);
        },
    )
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
fn ctrl_shift_n_adds_the_track_one_pass_after_the_spinner() {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let ctx = egui::Context::default();
    let output = press(&mut app, &ctx, Key::N, Modifiers::CTRL | Modifiers::SHIFT);
    assert!(pending(&app));
    assert_eq!(app.track, 0);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 1);
    assert!(output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::epaint::Shape::Path(path) if path.points.len() > 2)
    }));
    idle(&mut app, &ctx);
    assert!(!pending(&app));
    assert_eq!(app.track, 1);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 2);
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

fn waiting(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::epaint::Shape::Text(text)
            if text.galley.text() == "Preparing playback, please wait...")
    })
}

#[test]
fn mml_enter_paints_before_confirming_on_the_sixth_track() {
    let music = cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::edit::tracks::new_song().tracks.remove(0); 5],
    };
    let mut app = crate::view::tests::app(music);
    app.add_track().unwrap();
    crate::play::transport::stop(&mut app);
    assert_eq!(app.track, 5);
    app.undo.clear();
    let ctx = egui::Context::default();
    crate::view::mml_input::open(&mut app, &ctx);
    app.mml_input.buffer = "c1".into();
    press(&mut app, &ctx, Key::ArrowRight, Modifiers::NONE);
    let output = press(&mut app, &ctx, Key::Enter, Modifiers::NONE);
    assert!(waiting(&output));
    assert!(pending(&app));
    assert!(app.mml_input.open);
    assert!(!app.undo.available());
    assert!(app.playback.pending.is_none());

    let output = press(&mut app, &ctx, Key::Enter, Modifiers::NONE);
    assert!(!pending(&app));
    assert!(!app.mml_input.open);
    assert!(app.undo.available());
    assert!(app.playback.pending.is_some());
    assert!(waiting(&output));

    assert_eq!(app.playback.pending.as_ref().unwrap().parts.len(), 6);
    let output = press(&mut app, &ctx, Key::ArrowRight, Modifiers::NONE);
    assert!(app.playback.active.is_some());
    assert!(waiting(&output), "keep waiting until the first audio frame");
    app.audition.set_test_seconds(0.1);
    let output = press(&mut app, &ctx, Key::ArrowRight, Modifiers::NONE);
    assert!(!waiting(&output));
}

#[test]
fn failed_mml_confirmation_removes_waiting_overlay() {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let ctx = egui::Context::default();
    crate::view::mml_input::open(&mut app, &ctx);
    press(&mut app, &ctx, Key::ArrowRight, Modifiers::NONE);
    assert!(waiting(&press(&mut app, &ctx, Key::Enter, Modifiers::NONE)));
    let output = press(&mut app, &ctx, Key::ArrowRight, Modifiers::NONE);
    assert!(!waiting(&output));
    assert!(app.mml_input.open);
    assert!(!pending(&app));
    assert!(!app.undo.available());
}

#[test]
fn mml_confirmation_waits_for_a_presented_frame_despite_layout_retry() {
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let ctx = egui::Context::default();
    let mut passes = 0;
    let output = ctx.run(Default::default(), |ctx| {
        passes += 1;
        if passes == 1 {
            request(&mut app, ctx, Task::MmlConfirm);
            ctx.request_discard("exercise layout retry");
        }
        run(&mut app, ctx);
        show(&app, ctx);
    });
    assert!(passes > 1);
    assert!(pending(&app));
    assert!(waiting(&output));
}
