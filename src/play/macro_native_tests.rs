//! Read-only real-patch probe; audio is rendered offline without an output device.
use super::audio_sequence::Sequence;
use cat_smf_editor::{
    edit::parameter_curve::{Curve, Point, Target, ValueSpace},
    score::instrument::Patch,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
#[test]
#[ignore = "requires MACRO_NATIVE_CACHE pointing to an existing Surge XT patch cache entry"]
fn surge_macro_native_live_values_and_restoration() {
    use crate::play::{audition_native::Native, automation::OwnerPlan};
    use cat_smf_editor::{
        chain_settings::{PluginState, Settings},
        patch_reference::PatchReference,
        score::audition_timing::TempoMap,
    };
    let path = std::env::var_os("MACRO_NATIVE_CACHE").expect("explicit patch cache entry");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let reference: PatchReference = serde_json::from_value(json["reference"].clone()).unwrap();
    let state: Vec<u8> = serde_json::from_value(json["state"].clone()).unwrap();
    let controls =
        cat_smf_editor::surge_macros::parse(&state, reference.plugin.format == "VST3").unwrap();
    let routed = controls.routed;
    assert!(routed.iter().any(|v| *v));
    let patch = Patch::from(Settings {
        instrument: Some(PluginState {
            reference: Some(reference.clone()),
            plugin: reference.plugin.clone(),
            state,
            preset_name: Some(reference.display.clone()),
            bypass: false,
        }),
        ..Default::default()
    });
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let mut native = Native::open(48000, &patch).unwrap();
    let catalog = native.parameters(1).unwrap();
    let mut curves = vec![];
    let mut live = BTreeMap::new();
    let mut originals = vec![];
    for (i, active) in routed.into_iter().enumerate() {
        if !active {
            continue;
        }
        let p = catalog
            .iter()
            .find(|p| p.name.starts_with(&format!("M{}:", i + 1)))
            .unwrap_or_else(|| {
                panic!(
                    "Macro {} missing in {:?}",
                    i + 1,
                    catalog
                        .iter()
                        .filter(|p| p.name.starts_with('M'))
                        .collect::<Vec<_>>()
                )
            });
        assert!(p.eligible);
        assert_eq!((p.minimum, p.maximum), (0.0, 1.0));
        let target = Target {
            group: 1,
            layer: 1,
            plugin_format: reference.plugin.format.clone(),
            plugin_id: reference.plugin.id.clone(),
            parameter_id: p.stable_id.clone(),
        };
        eprintln!(
            "macro {} current={} range={}..{} normalized={} name={}",
            i + 1,
            p.current,
            p.minimum,
            p.maximum,
            p.normalized,
            p.name
        );
        originals.push((p.stable_id.clone(), p.current, controls.bipolar[i]));
        live.insert(target.clone(), Arc::new(AtomicU64::new(0.2f64.to_bits())));
        curves.push(Arc::new(Curve {
            version: 1,
            target,
            value_space: if p.normalized {
                ValueSpace::Normalized
            } else {
                ValueSpace::Plain
            },
            minimum: 0.0,
            maximum: 1.0,
            name: p.name.clone(),
            path: p.path.clone(),
            points: vec![Point {
                tick: 0,
                value: controls.positions[i].unwrap(),
            }],
            controls: vec![],
        }));
    }
    let music = cat_smf_editor::edit::tracks::new_song();
    let owner = Arc::new(OwnerPlan {
        owner: 0,
        curves,
        live: live.clone(),
        tempo: Arc::new(TempoMap::new(&music, 0)),
        boundaries: vec![],
    });
    let bindings = native.prepare_automation(&owner).unwrap();
    let runtime =
        crate::play::automation::RuntimePlan::new(owner, &Sequence::new(vec![], 1.0), 0.0, 48000);
    {
        let mut render = unsafe { native.render() };
        render.set_automation(bindings);
        let mut out = [0.0; 128];
        for (frame, expected) in [(0, 0.2_f64), (64, 0.85_f64)] {
            for value in live.values() {
                value.store(expected.to_bits(), Ordering::Relaxed);
            }
            assert!(render.automate(Some(&runtime), frame));
            assert!(render.process(&[0; 128], &[0; 128], 0, &mut out, 2));
            let catalog = native.parameters(1).unwrap();
            for (id, _, bipolar) in &originals {
                let current = catalog.iter().find(|p| &p.stable_id == id).unwrap().current;
                let current = if *bipolar {
                    (current + 1.0) / 2.0
                } else {
                    current
                };
                assert!(
                    (current - expected).abs() < 0.002,
                    "{id}: {current} != {expected}"
                );
            }
        }
    }
    let restored = native.restore_baseline(48000).unwrap();
    let catalog = restored.parameters(1).unwrap();
    for (id, value, _) in &originals {
        assert!(
            (catalog.iter().find(|p| &p.stable_id == id).unwrap().current - value).abs() < 0.002
        );
    }
    if let Some(source) = std::env::var_os("MACRO_NATIVE_SMF") {
        use cat_smf_editor::score::macro_values::{self, Values};
        let original_bytes = std::fs::read(&source).unwrap();
        let original = cat_smf_editor::ffi::parse(&original_bytes).unwrap();
        let mut music = original.clone();
        let values = Values {
            version: 1,
            variations: vec![],
            parameters: runtime
                .owner
                .curves
                .iter()
                .map(|c| {
                    let mut c = c.as_ref().clone();
                    c.points[0].value = 0.65;
                    c
                })
                .collect(),
        };
        macro_values::write(&mut music.tracks[0], &values).unwrap();
        let copy = work.path().join("macro-roundtrip.mid");
        std::fs::write(&copy, cat_smf_editor::smf_io::encode(&music).unwrap()).unwrap();
        let reopened = cat_smf_editor::ffi::parse(&std::fs::read(&copy).unwrap()).unwrap();
        assert_eq!(macro_values::read(&reopened.tracks[0]).unwrap(), values);
        let mut stripped = reopened.clone();
        macro_values::write(
            &mut stripped.tracks[0],
            &Values {
                version: 1,
                variations: vec![],
                parameters: vec![],
            },
        )
        .unwrap();
        assert_eq!(
            cat_smf_editor::smf_io::encode(&stripped).unwrap(),
            cat_smf_editor::smf_io::encode(&original).unwrap()
        );
        let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        let loaded = app.loaded.as_mut().unwrap();
        loaded.patches = vec![Patch::default(); reopened.tracks.len()];
        loaded.patches[0] = patch.clone();
        loaded.music = reopened;
        loaded.refresh();
        let prepared = super::transport::prepare(&app, super::playback::Mode::AllTracks).unwrap();
        assert!(
            prepared.active.automation.error.is_none(),
            "{:?}",
            prepared.active.automation.error
        );
        let owner = prepared.active.automation.owner(0).unwrap();
        drop(restored);
        let mut restarted = Native::open(48000, &patch).unwrap();
        let bindings = restarted.prepare_automation(&owner).unwrap();
        let plan = super::automation::RuntimePlan::new(owner, &prepared.parts[0].2, 0.0, 48000);
        {
            let mut render = unsafe { restarted.render() };
            render.set_automation(bindings);
            assert!(render.automate(Some(&plan), 0));
            let mut out = [0.0; 128];
            assert!(render.process(&[0; 128], &[0; 128], 0, &mut out, 2));
        }
        let current = restarted.parameters(1).unwrap();
        for (id, _, bipolar) in &originals {
            let v = current.iter().find(|p| &p.stable_id == id).unwrap().current;
            let p = cat_smf_editor::surge_macros::Polarity::from_bipolar(*bipolar)
                .position(v)
                .unwrap();
            assert!((p - 0.65).abs() < 0.002);
        }
        assert_eq!(std::fs::read(source).unwrap(), original_bytes);
    }
}
