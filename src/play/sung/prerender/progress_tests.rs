use super::*;

#[test]
fn connecting_job_paints_engine_starting_at_top_left_and_clears_when_done() {
    let mut app = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 0,
        division: 480,
        tracks: vec![],
    });
    let (send, receive) = std::sync::mpsc::channel();
    app.voicevox.prerender.job = Some(std::thread::spawn(move || {
        let _ = receive.recv();
        Outcome {
            engine: None,
            sung: vec![],
            no_executable: false,
        }
    }));
    app.voicevox
        .prerender
        .connecting
        .store(true, Ordering::Relaxed);
    let ctx = eframe::egui::Context::default();
    let output = ctx.run(Default::default(), |ctx| {
        crate::view::voicevox::progress::show(&app, ctx)
    });
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        eframe::egui::epaint::Shape::Text(text)
            if text.galley.text() == "Starting VOICEVOX engine…"
                && text.pos.x < 100.0 && text.pos.y < 100.0)));
    app.voicevox
        .prerender
        .connecting
        .store(false, Ordering::Relaxed);
    assert_eq!(progress(&app), Some("Preparing VOICEVOX audio…"));
    send.send(()).unwrap();
    finish(&mut app);
    assert_eq!(progress(&app), None);
}
