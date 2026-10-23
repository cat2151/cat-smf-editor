//! Opt-in silent regression using a real document and its installed instruments.
use super::*;

#[test]
#[ignore = "requires CAT_SMF_PREVIEW_MID; installed plugins and muted audio device"]
fn local_picker_switches_and_loops_without_stale_bindings() {
    use std::time::{Duration, Instant};
    let path = std::path::PathBuf::from(std::env::var_os("CAT_SMF_PREVIEW_MID").unwrap());
    let original = std::fs::read(&path).unwrap();
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let root = crate::store::config::directory().unwrap();
    let mut roots: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    let mut cache = cat_smf_editor::score::state_cache::Cache::new(Some(root));
    let music = cat_smf_editor::smf_io::load(&path).unwrap();
    let patches = music
        .tracks
        .iter()
        .map(|track| {
            Patch::resolve(
                cat_smf_editor::instrument_layers::read(track, &mut roots).unwrap(),
                &mut |r: &cat_smf_editor::patch_reference::PatchReference| {
                    cache.resolve(r, |_| {
                        Err("Local regression requires a cached patch state".into())
                    })
                },
            )
            .unwrap()
        })
        .collect();
    app.install_resolved(
        path.clone(),
        "local.mid".into(),
        music,
        Default::default(),
        crate::app::sound::Sound {
            patches,
            master: Default::default(),
            volumes: vec![],
        },
    );
    app.track = 0;
    let patch = app.loaded.as_ref().unwrap().patches[0].clone();
    let native = cat_smf_editor::chain::Chain::open(
        48000,
        &cat_smf_editor::chain_settings::Settings {
            instrument: patch.layers[0].instrument.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let mut picker = crate::view::parameter_editor::tests::picker();
    picker.song = app.song_id;
    picker.generation = app.generation;
    picker.plugin = plugin(&patch, 1).unwrap();
    picker.patch = patch;
    picker.parameters = native
        .parameters()
        .unwrap()
        .into_iter()
        .filter(|p| p.eligible)
        .collect();
    drop(native);
    picker.parameters.truncate(6);
    assert!(picker.parameters.len() >= 5);
    app.parameter_editor.picker = Some(picker);
    app.audition = Default::default();
    for track in 0..app.loaded.as_ref().unwrap().music.tracks.len() {
        app.audition.set_gain(track, 0.0);
    }
    for selected in [0, 1, 2, 3, 4, 1, 0, 4] {
        app.parameter_editor.picker.as_mut().unwrap().selected = selected;
        start(&mut app);
        let range = app.playback.repeat.unwrap();
        let duration = app.playback.pending.as_ref().unwrap().active.end;
        let deadline = Instant::now() + Duration::from_secs_f64(duration * 2.5 + 0.2);
        let mut loops = 0;
        while Instant::now() < deadline {
            let was_active = app.playback.active.is_some();
            app.audition.pump();
            transport::advance(&mut app);
            assert!(
                app.audition.error().is_none(),
                "candidate {selected}: {:?}",
                app.audition.error()
            );
            assert_eq!(app.playback.repeat, Some(range));
            assert!(app.playback.parameter_preview.is_some());
            assert!(app.playback.active.is_some() || app.playback.pending.is_some());
            if was_active && app.playback.pending.is_some() {
                loops += 1;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(loops >= 2, "candidate {selected}: only {loops} loops");
        eprintln!("candidate {selected}: {loops} loops OK");
    }
    stop(&mut app);
    assert_eq!(std::fs::read(path).unwrap(), original);
}
