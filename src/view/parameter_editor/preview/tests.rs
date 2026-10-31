use super::*;
use cat_smf_editor::model::{Event, Music, Track};

fn fixture() -> App {
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 1800,
                    delta: 1800,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![62, 80],
                },
                Event {
                    tick: 2400,
                    delta: 600,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![62, 0],
                },
                Event {
                    tick: 4000,
                    delta: 1600,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    });
    let mut p = super::super::tests::picker();
    p.song = app.song_id;
    p.tick = 2200;
    p.patch = app.loaded.as_ref().unwrap().patches[0].clone();
    let mut second = p.parameters[0].clone();
    second.stable_id = "102".into();
    second.minimum = -50.0;
    second.maximum = 150.0;
    p.parameters.push(second);
    app.parameter_editor.picker = Some(p);
    app
}

fn key(app: &mut App, key: egui::Key) {
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| crate::view::commands::route(app, ctx),
    );
}

#[test]
fn keyboard_switch_repeats_cursor_measure_sweeps_native_range_and_cancel_is_clean() {
    let mut app = fixture();
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let patches = app.loaded.as_ref().unwrap().patches.clone();
    let generation = app.generation;
    start(&mut app);
    assert_eq!(app.playback.repeat, Some((1920, 3840)));
    let first = &app.playback.parameter_preview.as_ref().unwrap().curve;
    first.validate().unwrap();
    assert_eq!(first.value_at(1920.0), Some(0.0));
    assert!((first.value_at(2880.0).unwrap() - 0.5).abs() < 1e-10);
    assert_eq!(first.value_at(3840.0), Some(1.0));
    let pending = app.playback.pending.as_ref().unwrap();
    assert_eq!(
        pending.parts[0].2.notes[0].start, 0.0,
        "held note crosses measure start"
    );
    key(&mut app, egui::Key::ArrowDown);
    let p = &app.playback.parameter_preview.as_ref().unwrap().curve;
    assert_eq!(p.target.parameter_id, "102");
    assert_eq!(p.value_at(1920.0), Some(-50.0));
    assert!((p.value_at(2880.0).unwrap() - 50.0).abs() < 1e-10);
    assert_eq!(p.value_at(3840.0), Some(150.0));
    let repeat = transport::repeat::prepare(&app, app.playback.repeat.unwrap()).unwrap();
    assert_eq!(
        repeat.active.automation.owner(0).unwrap().curves[0]
            .target
            .parameter_id,
        "102"
    );
    key(&mut app, egui::Key::ArrowUp);
    assert_eq!(
        app.playback
            .parameter_preview
            .as_ref()
            .unwrap()
            .curve
            .target
            .parameter_id,
        "101"
    );
    key(&mut app, egui::Key::Escape);
    assert!(app.playback.parameter_preview.is_none());
    assert!(
        app.playback.pending.is_none()
            && app.playback.active.is_none()
            && app.playback.repeat.is_none()
    );
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
    );
    assert_eq!(app.loaded.as_ref().unwrap().patches, patches);
    assert_eq!(app.generation, generation);
    assert!(!app.undo.available());
}

#[test]
fn empty_filter_stop_and_closed_picker_discard_preview() {
    let mut app = fixture();
    start(&mut app);
    app.parameter_editor.picker.as_mut().unwrap().query = "no match".into();
    start(&mut app);
    assert!(app.playback.parameter_preview.is_none());
    assert!(app.playback.repeat.is_none());
    app.parameter_editor.picker.as_mut().unwrap().query.clear();
    start(&mut app);
    transport::stop(&mut app);
    assert!(app.playback.parameter_preview.is_none());
    start(&mut app);
    app.parameter_editor.close();
    validate(&mut app);
    assert!(app.playback.repeat.is_none());
    assert!(app.playback.pending.is_none());
}

#[test]
#[ignore = "requires explicit parameter fixture bundles; set PARAMETER_NATIVE_FIXTURE_DIR"]
fn native_preview_renders_sweep_at_measure_origin_for_clap_and_vst3() {
    use crate::play::audition::offline::Instruments;
    use cat_smf_editor::chain_settings::{PluginState, Settings};
    use std::{path::PathBuf, sync::atomic::AtomicBool};
    let root = PathBuf::from(std::env::var_os("PARAMETER_NATIVE_FIXTURE_DIR").unwrap());
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    for (format, file, id) in [
        ("CLAP", "parameter_clap_fixture.clap", "cat.test.parameters"),
        (
            "VST3",
            "parameter_vst3_fixture.vst3",
            "132132131234123456785678ABCDEF01",
        ),
    ] {
        let mut app = fixture();
        let p = app.parameter_editor.picker.as_mut().unwrap();
        p.plugin.format = format.into();
        p.plugin.id = id.into();
        p.plugin.bundle_path = root.join(file).to_string_lossy().into_owned();
        p.parameters.truncate(1);
        p.parameters[0].normalized = format == "VST3";
        p.patch = Patch::from(Settings {
            instrument: Some(crate::test_support::referenced(PluginState {
                reference: None,
                plugin: p.plugin.clone(),
                state: [0.2f64.to_le_bytes(), 0.3f64.to_le_bytes()].concat(),
                preset_name: None,
                bypass: false,
            })),
            ..Default::default()
        });
        app.loaded.as_mut().unwrap().patches[0] = p.patch.clone();
        start(&mut app);
        let prepared = app.playback.pending.take().unwrap();
        let instruments = Instruments::open_automated(
            48000,
            &prepared.parts,
            &Default::default(),
            &prepared.active.automation,
        )
        .unwrap();
        let audio = unsafe {
            instruments.job_at(
                48000,
                prepared.parts,
                &[1.0],
                true,
                vec![],
                prepared.active.start_seconds,
            )
        }
        .run(&AtomicBool::new(false))
        .unwrap()
        .unwrap();
        // Leave the offline renderer's final 10 ms fade outside the sample checks.
        for frame in [0usize, 64, 24000, 48000, 94016] {
            assert!(
                (f64::from(audio[frame * 2]) - frame as f64 / 96000.0).abs() < 1e-5,
                "{format} frame {frame}: {}",
                audio[frame * 2]
            );
            assert!((audio[frame * 2 + 1] - 0.3).abs() < 1e-5);
        }
        drop(instruments);
        stop(&mut app);
        // The ordinary sound still comes from the saved baseline, never the sweep.
        let ordinary = transport::repeat::prepare(&app, (1920, 3840)).unwrap();
        let instruments = Instruments::open(48000, &ordinary.parts, &Default::default()).unwrap();
        let audio = unsafe { instruments.job(48000, ordinary.parts, &[1.0], true, vec![]) }
            .run(&AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert!((audio[0] - 0.2).abs() < 1e-5);
        assert!((audio[1] - 0.3).abs() < 1e-5);
    }
}

#[test]
fn focused_search_and_help_keep_navigation_and_ime_keys_isolated() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let frame = |app: &mut App, events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                crate::view::commands::route(app, ctx);
                super::super::show(app, ctx);
            },
        );
    };
    let press = |key| {
        vec![
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    frame(&mut app, vec![]);
    // Install TextEdit's navigation focus filter after the initial focus request.
    frame(&mut app, vec![]);
    frame(&mut app, press(egui::Key::ArrowDown));
    assert_eq!(app.parameter_editor.picker.as_ref().unwrap().selected, 1);
    assert_eq!(
        app.playback
            .parameter_preview
            .as_ref()
            .unwrap()
            .curve
            .target
            .parameter_id,
        "102"
    );
    frame(&mut app, press(egui::Key::F1));
    assert!(app.help_open);
    frame(&mut app, press(egui::Key::F1));
    // Pressing again keeps help open and lifts it above the other windows.
    assert!(app.help_open && app.help.raise);
    frame(&mut app, vec![egui::Event::Text("onshoku".into())]);
    assert_eq!(
        app.parameter_editor.picker.as_ref().unwrap().query,
        "onshoku"
    );
    frame(
        &mut app,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("音".into()))],
    );
    let selected = app.parameter_editor.picker.as_ref().unwrap().selected;
    frame(&mut app, press(egui::Key::ArrowDown));
    assert_eq!(
        app.parameter_editor.picker.as_ref().unwrap().selected,
        selected
    );
    frame(&mut app, vec![egui::Event::Ime(egui::ImeEvent::Disabled)]);
    frame(&mut app, press(egui::Key::Escape));
    assert!(app.parameter_editor.picker.is_none());
    assert!(app.playback.parameter_preview.is_none());
}
