//! Installed creation resources and disk restoration, rendered without a device.
use super::*;
use cat_smf_editor::{
    edit,
    model::Event,
    score::{
        pattern::Catalog,
        scene::{
            self,
            arrangement::{self as grid, PatternOrScene},
        },
    },
};
use eframe::egui;

#[test]
#[ignore = "requires installed Surge XT, local catalog and sibling release player; offline only"]
fn initial_containers_save_restart_and_render_pattern_scene_song() {
    let root = std::path::PathBuf::from(
        std::env::var_os("PATTERN_NATIVE_WORK_DIR").expect("global work directory"),
    );
    let work = tempfile::tempdir_in(root).unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let plugin = cat_smf_editor::chain::default_instrument().unwrap();
    let table: cat_smf_editor::patch_roots::Table = serde_json::from_slice(
        &std::fs::read(
            dirs::config_local_dir()
                .unwrap()
                .join("cat-smf-editor/cache/patch-roots-v1.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let installed = table.plugin(&plugin).unwrap();
    crate::player::patch_roots::Assumed::set_root(std::path::Path::new(&installed.roots[0]));
    let player = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cat-plugin-player/target/release/cat-plugin-player.exe");
    assert!(player.is_file());
    let mut a = crate::view::tests::app(edit::tracks::new_song());
    a.config = crate::store::config::Store::open(work.path().join("status.json"));
    a.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
    a.player_selection.executable = Some(player);
    a.saved = crate::store::saved::Document::untitled(
        Some(work.path().into()),
        &a.loaded.as_ref().unwrap().music,
    )
    .unwrap();
    library::create(&mut a, None).unwrap();
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| crate::view::commands::route(&mut a, ctx),
    );
    assert!(!a.patterns.confirming_creation(), "{:?}", a.patterns.error);
    let saw = a.patterns.selected.unwrap();
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(c.sources.len(), 2);
    let drum = *c
        .sources
        .iter()
        .find(|&&t| a.loaded.as_ref().unwrap().patches[t].drums.is_some())
        .unwrap();
    let owner = c
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == drum).then_some(id))
        .unwrap();
    a.patterns.container = Some(owner);
    a.track = drum;
    library::create(&mut a, None).unwrap();
    let drums = a.patterns.selected.unwrap();
    let mut scene = 0;
    a.edit_tracks(|m| {
        let c = Catalog::read(m)?;
        for (id, pitches) in [
            (saw, vec![60]),
            (drums, cat_smf_editor::score::drums::NOTES.to_vec()),
        ] {
            let p = &c.patterns[&id];
            for (i, pitch) in pitches.into_iter().enumerate() {
                for (offset, status, velocity) in [(0, 0x99, 100), (120, 0x89, 35)] {
                    edit::insert(
                        &mut m.tracks[p.track],
                        Event {
                            tick: p.tick + i as u64 * 480 + offset,
                            delta: 0,
                            status,
                            meta_type: 0,
                            data: vec![pitch, velocity],
                        },
                    );
                }
            }
            edit::refresh_deltas(&mut m.tracks[p.track])?;
        }
        scene = grid::create_scene(m, "A", 1, 2)?;
        let lanes = scene::read(m)?.scenes[0].lanes.clone();
        grid::place(
            m,
            Some(scene),
            lanes[0],
            1,
            Some(PatternOrScene::Pattern(saw)),
        )?;
        grid::place(
            m,
            Some(scene),
            lanes[1],
            1,
            Some(PatternOrScene::Pattern(drums)),
        )?;
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 1, Some(PatternOrScene::Scene(scene)))
    })
    .unwrap();
    let chain = cat_smf_editor::chain::Chain::open(48000, &Default::default()).unwrap();
    let parameter = chain
        .parameters()
        .unwrap()
        .into_iter()
        .find(|p| p.eligible && p.name == "Global Volume")
        .unwrap();
    let m = &mut a.loaded.as_mut().unwrap().music;
    let owner = Catalog::read(m).unwrap().patterns[&saw].container;
    let key = cat_smf_editor::chain::default_instrument().unwrap();
    cat_smf_editor::score::pattern::automation::write(
        m,
        saw,
        cat_smf_editor::edit::parameter_curve::Curve {
            version: 1,
            target: cat_smf_editor::edit::parameter_curve::Target {
                group: owner,
                layer: 1,
                plugin_format: key.format,
                plugin_id: key.id,
                parameter_id: parameter.stable_id,
            },
            value_space: cat_smf_editor::edit::parameter_curve::ValueSpace::Plain,
            minimum: parameter.minimum,
            maximum: parameter.maximum,
            name: parameter.name,
            path: parameter.path,
            points: vec![cat_smf_editor::edit::parameter_curve::Point {
                tick: 0,
                value: 0.37,
            }],
            controls: vec![],
        },
        Some(parameter.current),
    )
    .unwrap();
    a.remember_loaded().unwrap();
    drop(chain);
    let original = a.loaded.as_ref().unwrap().patches.clone();
    let count = a.loaded.as_ref().unwrap().music.tracks.len();
    let mid = work.path().join("patterns.mid");
    a.saved.save_as(&mid).unwrap();
    a.sync_saved_path();
    a.state_cache.save().unwrap();
    let bytes = std::fs::read(&mid).unwrap();
    drop(a);
    for restart in 0..2 {
        let mut a = crate::view::tests::app(edit::tracks::new_song());
        a.loaded = None;
        a.config = crate::store::config::Store::open(work.path().join("status.json"));
        a.saved = crate::store::saved::Document::new(Some(work.path().into()));
        a.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
        a.player_selection.executable = Some("X:/missing/player.exe".into());
        a.patch_roots = crate::player::patch_roots::Cache::new(None);
        a.open_initial(None);
        assert!(a.load_error.is_none(), "{:?}", a.load_error);
        assert_eq!(a.loaded.as_ref().unwrap().music.tracks.len(), count);
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            bytes
        );
        for (before, after) in original.iter().zip(&a.loaded.as_ref().unwrap().patches) {
            assert_eq!(before.saved().unwrap(), after.saved().unwrap());
            for (b, s) in before.layers.iter().zip(&after.layers) {
                assert_eq!(
                    b.instrument.as_ref().map(|i| &i.state),
                    s.instrument.as_ref().map(|i| &i.state)
                );
            }
        }
        if restart == 0 {
            for id in [saw, drums] {
                render(preview::prepare(&a, id).unwrap());
            }
            render(crate::play::arrangement::prepare(&a, Some(scene)).unwrap());
            render(crate::play::arrangement::prepare(&a, None).unwrap());
            let flat = cat_smf_editor::score::performance::song(&a.loaded.as_ref().unwrap().music)
                .unwrap();
            let path = work.path().join("performance.mid");
            std::fs::write(&path, cat_smf_editor::smf_io::encode(&flat).unwrap()).unwrap();
            let mut restored = crate::view::tests::app(edit::tracks::new_song());
            restored.saved = crate::store::saved::Document::new(Some(work.path().into()));
            restored.state_cache =
                cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
            restored.load_mid_unchecked(path);
            assert!(restored.load_error.is_none(), "{:?}", restored.load_error);
            assert!(Catalog::read(&restored.loaded.as_ref().unwrap().music)
                .unwrap()
                .sources
                .is_empty());
            let prepared =
                crate::play::transport::prepare(&restored, crate::play::playback::Mode::AllTracks)
                    .unwrap();
            assert!(
                prepared.active.automation.error.is_none(),
                "{:?}",
                prepared.active.automation.error
            );
            assert_eq!(
                prepared
                    .parts
                    .iter()
                    .filter(|p| !p.2.notes.is_empty()
                        || prepared.active.automation.owner(p.0).is_some())
                    .count(),
                2
            );
            render(prepared);
        }
    }
}
fn render(p: crate::play::transport::Prepared) {
    crate::play::audition::offline::native_probe::verify(p);
}

#[test]
#[ignore = "requires installed Surge XT; offline control delivery and pitch measurement"]
fn surge_pattern_value_cc_and_pitch_bend_reach_the_instrument() {
    use crate::play::{
        audio_sequence::{Note, Sequence},
        automation::{Automation, OwnerPlan},
    };
    use std::{collections::BTreeMap, sync::Arc};
    let root = std::path::PathBuf::from(std::env::var_os("PATTERN_NATIVE_WORK_DIR").unwrap());
    let work = tempfile::tempdir_in(root).unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let music = edit::tracks::new_song();
    let automation = Automation {
        owners: Arc::new(vec![Arc::new(OwnerPlan {
            windows: BTreeMap::new(),
            midi: true,
            live: BTreeMap::new(),
            owner: 0,
            curves: vec![],
            tempo: Arc::new(cat_smf_editor::score::audition_timing::TempoMap::new(
                &music, 0,
            )),
            boundaries: vec![(0, 0.)],
        })]),
        error: None,
    };
    let audio = |sustain: u8, bend: u16| {
        let mut sequence = Sequence::new(
            vec![Note {
                start: 0.,
                duration: 0.25,
                pitch: 60,
                velocity: 100,
            }],
            1.0,
        );
        let mut events = vec![
            crate::play::audio_sequence::MidiEvent {
                time: 0.,
                word: 0x20b0_4000 | u32::from(sustain),
            },
            crate::play::audio_sequence::MidiEvent {
                time: 0.,
                word: 0x20e0_0000 | u32::from(bend & 127) << 8 | u32::from(bend >> 7),
            },
        ];
        events.extend(sequence.events);
        sequence.events = events;
        let parts = vec![(0, Default::default(), sequence)];
        let native = crate::play::audition::offline::Instruments::open_automated(
            48000,
            &parts,
            &Default::default(),
            &automation,
        )
        .unwrap();
        unsafe { native.job(48000, parts, &[1.], true, vec![]) }
            .run(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .unwrap()
    };
    let normal = audio(0, 8192);
    let held = audio(127, 8192);
    let bent = audio(127, 16383);
    let rms = |v: &[f32]| {
        (v[24000..72000]
            .iter()
            .map(|v| f64::from(*v).powi(2))
            .sum::<f64>()
            / 48000.)
            .sqrt()
    };
    assert!(rms(&held) > 0.001);
    assert!(
        rms(&normal) < rms(&held) * 0.1,
        "CC64 released={} held={}",
        rms(&normal),
        rms(&held)
    );
    let crossings = |v: &[f32]| {
        (12000..36000)
            .filter(|&i| v[i * 2] <= 0. && v[(i + 1) * 2] > 0.)
            .count() as f64
            * 2.
    };
    let base = crossings(&held);
    let bend = crossings(&bent);
    assert!(
        (base - 261.63).abs() < 12. && bend > base * 1.08 && bend < base * 1.17,
        "pitch normal={base} bend={bend}"
    );
    eprintln!(
        "Surge XT CC64 RMS {} -> {}; pitch {} -> {} Hz",
        rms(&normal),
        rms(&held),
        base,
        bend
    );
}
