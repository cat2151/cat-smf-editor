use crate::{player::selection::bulk_tests::identical, store::config::Store, view::tests::app};
use eframe::egui;

fn press(app: &mut crate::app::App, ctx: &egui::Context, key: egui::Key) -> egui::FullOutput {
    let mut output = None;
    for pressed in [true, false] {
        output = Some(ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: if key == egui::Key::Comma {
                        egui::Modifiers::CTRL | egui::Modifiers::COMMAND
                    } else {
                        egui::Modifiers::NONE
                    },
                }],
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        ));
    }
    output.unwrap()
}

fn shows(output: &egui::FullOutput, text: &str) -> bool {
    output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::epaint::Shape::Text(t) if t.galley.text() == text),
    )
}

#[test]
fn comma_v_opens_the_page_and_saves_the_chosen_run_exe_by_keyboard_only() {
    let root = tempfile::tempdir().unwrap();
    let status = root.path().join("status.json");
    let engine = root.path().join("run.exe");
    std::fs::write(&engine, b"").unwrap();
    let mut app = app(identical());
    app.config = Store::open(status.clone());
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::Comma);
    let output = press(&mut app, &ctx, egui::Key::V);
    assert!(app.voicevox.wizard);
    assert!(!app.comma_menu.open);
    assert!(shows(&output, "r: run.exe") && shows(&output, "o: 公式ページ"));

    press(&mut app, &ctx, egui::Key::O);
    assert_eq!(app.voicevox.pages_opened, 1);
    assert!(app.voicevox.wizard);
    // Menu letters must not reach the score's shortcuts while the wizard is open.
    assert!(!app.playback.open);
    press(&mut app, &ctx, egui::Key::P);
    assert!(!app.playback.open);

    press(&mut app, &ctx, egui::Key::R);
    assert!(app.voicevox.pick_requested);
    // The native dialog runs at the eframe boundary; its answer arrives here.
    app.voicevox.pick_requested = false;
    super::complete(&mut app, None);
    assert!(app.voicevox.wizard);
    super::complete(&mut app, Some(engine.clone()));
    assert!(!app.voicevox.wizard);
    assert_eq!(
        Store::open(status.clone()).voicevox_engine,
        Some(engine.clone())
    );

    // Choosing again shows the saved path and keeps it on Escape.
    press(&mut app, &ctx, egui::Key::Comma);
    let output = press(&mut app, &ctx, egui::Key::V);
    assert!(shows(&output, &engine.display().to_string()));
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.voicevox.wizard);
    assert!(app.voicevox.error.is_none());
    assert_eq!(Store::open(status).voicevox_engine, Some(engine));
}

#[test]
fn unset_or_missing_run_exe_needs_the_wizard() {
    let root = tempfile::tempdir().unwrap();
    let status = root.path().join("status.json");
    let mut store = Store::open(status.clone());
    assert_eq!(store.voicevox_engine, None);
    let missing = root.path().join("moved").join("run.exe");
    store.select_voicevox_engine(missing.clone());
    let saved = Store::open(status);
    assert_eq!(saved.voicevox_engine.as_deref(), Some(missing.as_path()));
    assert_eq!(
        cat_smf_editor::voicevox::engine::executable(saved.voicevox_engine.as_deref()),
        None
    );
}

#[test]
fn escape_from_a_prerender_request_gives_up_with_one_error() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    super::request_executable(&mut app);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.voicevox.wizard);
    let error = app.voicevox.error.clone().unwrap();
    let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
    let shown = output
        .shapes
        .iter()
        .filter(|shape| {
            matches!(&shape.shape, egui::epaint::Shape::Text(t) if t.galley.text() == error)
        })
        .count();
    assert_eq!(shown, 1);

    super::request_executable(&mut app);
    super::complete(&mut app, Some("run.exe".into()));
    assert!(app.voicevox.error.is_none());
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::V);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.voicevox.error.is_none());
}

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_engine_started_by_the_editor_stops_on_exit() {
    use cat_smf_editor::voicevox::engine;
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/.voicevox-engine-path");
    let path = std::path::PathBuf::from(std::fs::read_to_string(file).unwrap().trim());
    let port = 50124;
    let engine = engine::connect(port, Some(&path), engine::START_TIMEOUT).unwrap();
    let pid = engine.started_pid().expect("port 50124 was already in use");
    let mut app = app(identical());
    app.voicevox.engine = Some(engine);
    eframe::App::on_exit(&mut app, None);
    let client = cat_smf_editor::voicevox::client::Client::new(&format!("http://127.0.0.1:{port}"));
    assert!(client.version().is_err(), "engine still answers");
    let tasks = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .unwrap();
    let tasks = String::from_utf8_lossy(&tasks.stdout);
    assert!(!tasks.contains(&format!("\"{pid}\"")), "{tasks}");
}
