//! Real kit fixture exported by the player's production catalog reader; no device output.
use cat_smf_editor::{
    chain_settings::PluginState,
    drum_kit::Kit,
    patch_reference::PatchReference,
    score::{
        drum_kit,
        instrument::{Patch, ResolvedLayer},
        state_cache::Cache,
    },
};
#[test]
#[ignore = "requires CAT_KIT_CONTRACT_FIXTURE exported by player installed_kit_contract_fixture; offline"]
fn installed_kit_offline_save_restore_and_render_actual_notes() {
    let fixture = std::path::PathBuf::from(std::env::var_os("CAT_KIT_CONTRACT_FIXTURE").unwrap());
    let (reference, kit, state): (PatchReference, Kit, Vec<u8>) =
        serde_json::from_slice(&std::fs::read(&fixture).unwrap()).unwrap();
    let work = fixture.parent().unwrap().join("kit-offline");
    std::fs::create_dir_all(&work).unwrap();
    let _library = cat_smf_editor::native_library::load_at(&work).unwrap();
    let root = match &reference.source {
        cat_smf_editor::patch_reference::Source::Catalog { file, .. } => file.parent().unwrap(),
        _ => unreachable!(),
    };
    crate::player::patch_roots::Assumed::set_root(root);
    let mut cache = Cache::new(Some(work.clone()));
    cache.remember(&reference, state.clone()).unwrap();
    let patch = Patch {
        kit: Some(kit.clone()),
        layers: vec![ResolvedLayer {
            settings: Default::default(),
            instrument: Some(PluginState {
                plugin: reference.plugin.clone(),
                reference: Some(reference.clone()),
                state,
                preset_name: Some(reference.display.clone()),
                bypass: false,
            }),
        }],
        ..Default::default()
    };
    let mut music = cat_smf_editor::edit::tracks::new_song();
    let track = music.tracks.len() - 1;
    music.tracks[track].events.retain(|e| {
        !cat_smf_editor::score::note_view::note_on(e)
            && !cat_smf_editor::score::note_view::note_off(e)
    });
    cat_smf_editor::score::instrument::write(
        &mut music.tracks[track],
        &patch,
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    let notes: Vec<_> = kit.notes.iter().take(3).map(|n| n.note).collect();
    assert!(notes.len() >= 2);
    for (step, &note) in notes.iter().enumerate() {
        drum_kit::toggle(&mut music, track, note, 1, step).unwrap();
    }
    cache.save().unwrap();
    let file = work.join("kit.mid");
    std::fs::write(&file, cat_smf_editor::smf_io::encode(&music).unwrap()).unwrap();
    drop(cache);
    let restored = cat_smf_editor::smf_io::load(&file).unwrap();
    let mut fresh = Cache::new(Some(work.clone()));
    let saved = cat_smf_editor::instrument_layers::read(
        &restored.tracks[track],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    let patch = Patch::resolve(saved, |r| {
        fresh.resolve(r, |_| panic!("fresh cache should recover exact sound"))
    })
    .unwrap();
    assert_eq!(patch.kit, Some(kit));
    assert_eq!(patch.layers.len(), 1);
    for &pitch in &notes {
        let source = &restored.tracks[track].events;
        let selected: Vec<_> = source
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                (cat_smf_editor::score::note_view::note_on(e) && e.data[0] == pitch).then_some(i)
            })
            .collect();
        let tempo = cat_smf_editor::score::audition_timing::TempoMap::new(&restored, track);
        let sequence = crate::play::audio_sequence::Sequence::from_source(
            vec![crate::play::audio_sequence::Note {
                start: tempo.seconds_at(source[selected[0]].tick),
                duration: 0.25,
                pitch,
                velocity: 100,
            }],
            2.5,
            source,
            &selected,
            &tempo,
            0,
        );
        assert!(sequence
            .events
            .iter()
            .filter(|e| matches!((e.word >> 20) & 15, 8 | 9))
            .all(|e| (e.word >> 8) & 127 == u32::from(pitch)));
        let parts = vec![(track, patch.clone(), sequence)];
        let instruments =
            crate::play::audition::offline::Instruments::open(48000, &parts, &Default::default())
                .unwrap();
        let job = unsafe { instruments.job(48000, parts, &vec![1.0; track + 1], true, vec![]) };
        let audio = job
            .run(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert!(audio.iter().all(|v| v.is_finite()));
        let peak = audio.iter().fold(0.0_f32, |p, v| p.max(v.abs()));
        assert!(peak > 0.00001, "actual note {pitch} silent");
        eprintln!("kit actual note {pitch}: peak={peak}");
    }
    let actual: Vec<_> = restored.tracks[track]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .map(|e| e.data[0])
        .collect();
    assert_eq!(actual, notes);
}
