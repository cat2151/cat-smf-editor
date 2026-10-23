//! Exercise the saved document through the same native scheduler used by p → a.
use super::{
    audition_native::Native, automation::RuntimePlan, event_timeline::Timeline, playback, transport,
};
use cat_smf_editor::{
    edit::tracks,
    host::PluginKey,
    patch_roots::Roots,
    score::{instrument::Patch, state_cache::Cache},
};

#[test]
#[ignore = "requires MACRO_CURVE_TEST_SMF and the existing local plugin/cache; silent offline render"]
fn saved_macro_curve_plays_to_the_end_and_restarts_without_parameter_rejection() {
    let path = std::env::var_os("MACRO_CURVE_TEST_SMF").expect("explicit existing SMF");
    let bytes = std::fs::read(&path).unwrap();
    let music = cat_smf_editor::ffi::parse(&bytes).unwrap();
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let table: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    for plugin in &table.plugins {
        crate::player::patch_roots::Assumed
            .plugin(
                &PluginKey {
                    format: plugin.format.clone(),
                    id: plugin.id.clone(),
                    name: plugin.id.clone(),
                    vendor: String::new(),
                    bundle_path: plugin.bundle_path.clone(),
                },
                false,
            )
            .unwrap();
    }
    let surge = table
        .plugins
        .iter()
        .find(|p| p.id == "org.surge-synth-team.surge-xt")
        .unwrap();
    crate::player::patch_roots::Assumed::set_root(std::path::Path::new(&surge.roots[0]));
    let mut cache = Cache::new(Some(root));
    let patches = music
        .tracks
        .iter()
        .map(|track| {
            let settings = cat_smf_editor::instrument_layers::read(
                track,
                &mut crate::player::patch_roots::Assumed,
            )?;
            Patch::resolve(settings, |r| {
                cache.resolve(r, |_| Err("Read-only test requires cached state".into()))
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .unwrap();
    let mut app = crate::view::tests::app(tracks::new_song());
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music = music;
    loaded.patches = patches;
    loaded.refresh();
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let prepared = transport::prepare(&app, playback::Mode::AllTracks).unwrap();
    assert!(prepared.active.end > 1.0);
    assert!(
        prepared.active.automation.error.is_none(),
        "{:?}",
        prepared.active.automation.error
    );
    let mut checked = 0;
    for (track, patch, sequence) in &prepared.parts {
        let Some(owner) = prepared.active.automation.owner(*track) else {
            continue;
        };
        assert!(!cat_smf_editor::score::macro_curve::read(
            &app.loaded.as_ref().unwrap().music.tracks[*track]
        )
        .unwrap()
        .is_empty());
        let mut native = Native::open(48000, patch).unwrap();
        let runtime = RuntimePlan::new(owner, sequence, 0.0, 48000);
        let timeline = Timeline::new(sequence, 0.0, 48000);
        let bindings = native.prepare_automation(&runtime.owner).unwrap();
        let mut render = unsafe { native.render() };
        render.set_automation(bindings);
        let mut output = [0.0; 128];
        for _ in 0..2 {
            render.reset();
            let mut frame = 0;
            while frame < timeline.end {
                let count = runtime.next(frame, 64).min((timeline.end - frame) as usize);
                assert!(count > 0);
                let mut success = render.process_scheduled(
                    &timeline,
                    Some(&runtime),
                    frame,
                    &mut output[..count * 2],
                    2,
                );
                if !success && render.parameter_error == Some(-6) {
                    // Voice::refresh_automation normally performs this on the UI
                    // thread with the callback excluded, on these same instances.
                    native.pump();
                    render.set_automation(native.prepare_automation(&runtime.owner).unwrap());
                    success = render.process_scheduled(
                        &timeline,
                        Some(&runtime),
                        frame,
                        &mut output[..count * 2],
                        2,
                    );
                }
                assert!(
                    success,
                    "track {} / frame {} / tick {} / error {:?}",
                    track + 1,
                    frame,
                    runtime.sample_tick(frame),
                    render.parameter_error
                );
                assert!(output[..count * 2].iter().all(|v| v.is_finite()));
                frame += count as u64;
            }
        }
        checked += 1;
    }
    assert!(checked > 0);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
