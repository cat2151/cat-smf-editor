use crate::{player::selection::bulk_tests::identical, view::tests::app};
use eframe::egui;

/// One frame at `time` seconds. `focused: None` leaves the viewport focus unknown, as the
/// first frames of eframe do.
fn frame(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    focused: Option<bool>,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut input = egui::RawInput {
        time: Some(time),
        events,
        ..Default::default()
    };
    input
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .focused = focused;
    ctx.run(input, |ctx| crate::view::show(app, ctx))
}

fn key(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

/// `egui::Spinner` paints one open 20-point line; nothing else in the wizard does.
fn spinner_drawn(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::epaint::Shape::Path(path)
            if !path.closed && path.points.len() == 20)
    })
}

/// Opens the wizard and lets its window settle, so the next frame paints it fully.
fn wizard(app: &mut crate::app::App, ctx: &egui::Context) -> egui::FullOutput {
    super::open(app);
    let mut output = frame(app, ctx, 0.0, Some(true), vec![]);
    for step in 1..4 {
        output = frame(app, ctx, f64::from(step) * 0.1, Some(true), vec![]);
    }
    output
}

#[test]
fn pressing_o_draws_the_spinner_in_the_same_frame_and_losing_focus_hides_it() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    assert!(!spinner_drawn(&wizard(&mut app, &ctx)));

    let pressed = frame(
        &mut app,
        &ctx,
        1.0,
        Some(true),
        vec![key(egui::Key::O, true)],
    );
    assert_eq!(app.voicevox.pages_opened, 1);
    assert!(spinner_drawn(&pressed));
    let released = frame(
        &mut app,
        &ctx,
        1.1,
        Some(true),
        vec![key(egui::Key::O, false)],
    );
    assert!(spinner_drawn(&released));
    // Unknown focus is not a loss.
    assert!(spinner_drawn(&frame(&mut app, &ctx, 1.2, None, vec![])));

    frame(&mut app, &ctx, 1.3, Some(false), vec![]);
    let after = frame(&mut app, &ctx, 1.4, Some(false), vec![]);
    assert!(!spinner_drawn(&after));
    assert!(app.voicevox.wizard);
    // Coming back does not bring the spinner back.
    assert!(!spinner_drawn(&frame(
        &mut app,
        &ctx,
        1.5,
        Some(true),
        vec![]
    )));
}

#[test]
fn without_losing_focus_the_spinner_goes_away_after_fifteen_seconds() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    wizard(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        1.0,
        Some(true),
        vec![key(egui::Key::O, true)],
    );
    frame(
        &mut app,
        &ctx,
        1.1,
        Some(true),
        vec![key(egui::Key::O, false)],
    );
    assert!(spinner_drawn(&frame(
        &mut app,
        &ctx,
        15.9,
        Some(true),
        vec![]
    )));
    frame(&mut app, &ctx, 16.0, Some(true), vec![]);
    assert!(!spinner_drawn(&frame(
        &mut app,
        &ctx,
        16.1,
        Some(true),
        vec![]
    )));
}

#[test]
fn clicking_the_page_button_draws_the_spinner_in_the_same_frame() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    let output = wizard(&mut app, &ctx);
    let button = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == "o: 公式ページ" => {
                Some(t.visual_bounding_rect().center())
            }
            _ => None,
        })
        .expect("the page button is painted");
    let pointer = |pressed| egui::Event::PointerButton {
        pos: button,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        &mut app,
        &ctx,
        1.0,
        Some(true),
        vec![egui::Event::PointerMoved(button)],
    );
    frame(&mut app, &ctx, 1.1, Some(true), vec![pointer(true)]);
    assert!(!spinner_drawn(&frame(
        &mut app,
        &ctx,
        1.15,
        Some(true),
        vec![]
    )));
    let released = frame(&mut app, &ctx, 1.2, Some(true), vec![pointer(false)]);
    assert_eq!(app.voicevox.pages_opened, 1);
    assert!(spinner_drawn(&released));

    // Closing the wizard drops the spinner with it.
    frame(
        &mut app,
        &ctx,
        1.3,
        Some(true),
        vec![key(egui::Key::Escape, true)],
    );
    super::open(&mut app);
    assert!(!spinner_drawn(&frame(
        &mut app,
        &ctx,
        1.4,
        Some(true),
        vec![]
    )));
}
