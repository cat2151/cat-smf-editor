//! Read-only real-patch probe; audio is rendered offline without an output device.
use super::audio_sequence::Sequence;
use cat_smf_editor::{
    edit::parameter_curve::{Curve, Point, Target, ValueSpace},
    host::parameters::domain::{Domain, Position},
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
#[ignore = "requires MACRO_NATIVE_CACHE pointing to an existing Surge XT, Vaporizer2, Six Sines or Floe patch cache entry"]
fn macro_native_live_values_and_restoration() {
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
    let instrument = cat_smf_editor::instrument_macros::Instrument::of(
        &reference.plugin.format,
        &reference.plugin.id,
    )
    .unwrap();
    let macros = instrument
        .movable(&reference.plugin.format, &state)
        .unwrap();
    assert!(!macros.is_empty());
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
    settle(&native);
    let catalog = native.parameters(1).unwrap();
    let mut curves = vec![];
    let mut live = BTreeMap::new();
    let mut originals = vec![];
    let mut senders = vec![];
    for m in &macros {
        let p = catalog
            .iter()
            .find(|p| instrument.parameter(m.number, &p.name))
            .unwrap_or_else(|| panic!("Macro {} missing", m.number));
        assert!(p.eligible);
        assert_eq!((p.minimum, p.maximum), instrument.range());
        let target = Target {
            group: 1,
            layer: 1,
            plugin_format: reference.plugin.format.clone(),
            plugin_id: reference.plugin.id.clone(),
            parameter_id: p.stable_id.clone(),
        };
        eprintln!(
            "macro {} saved={:?} current={} range={}..{} normalized={} name={}",
            m.number, m.position, p.current, p.minimum, p.maximum, p.normalized, p.name
        );
        // Surge reads a bipolar macro back as -1-1 although its host range is 0-1.
        let reads_signal =
            instrument == cat_smf_editor::instrument_macros::Instrument::SurgeXt && m.bipolar;
        let mut curve = Curve {
            version: 1,
            target: target.clone(),
            value_space: if p.normalized {
                ValueSpace::Normalized
            } else {
                ValueSpace::Plain
            },
            minimum: p.minimum,
            maximum: p.maximum,
            name: p.name.clone(),
            path: p.path.clone(),
            points: vec![Point {
                tick: 0,
                value: 0.0,
            }],
            controls: vec![],
        };
        let domain = curve.domain().unwrap();
        curve.points[0].value = api(domain, m.position.unwrap());
        let value = Arc::new(AtomicU64::new(api(domain, 0.2).to_bits()));
        live.insert(target, value.clone());
        senders.push((value, domain));
        originals.push((p.stable_id.clone(), p.current, reads_signal, domain));
        curves.push(Arc::new(curve));
    }
    let music = cat_smf_editor::edit::tracks::new_song();
    let owner = Arc::new(OwnerPlan {
        owner: 0,
        curves,
        live: live.clone(),
        tempo: Arc::new(TempoMap::new(&music, 0)),
        boundaries: vec![],
    });
    let runtime =
        crate::play::automation::RuntimePlan::new(owner, &Sequence::new(vec![], 1.0), 0.0, 48000);
    let mut frame = 0;
    for expected in [0.2_f64, 0.85_f64] {
        for (value, domain) in &senders {
            value.store(api(*domain, expected).to_bits(), Ordering::Relaxed);
        }
        drive(&mut native, &runtime, &mut frame);
        let catalog = native.parameters(1).unwrap();
        for (id, _, bipolar, domain) in &originals {
            let current = catalog.iter().find(|p| &p.stable_id == id).unwrap().current;
            let current = position(*domain, *bipolar, current);
            assert!(
                (current - expected).abs() < 0.002,
                "{id}: {current} != {expected}"
            );
        }
    }
    let restored = native.restore_baseline(48000).unwrap();
    let catalog = restored.parameters(1).unwrap();
    for (id, value, _, _) in &originals {
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
                    c.points[0].value = api(c.domain().unwrap(), 0.65);
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
        settle(&restarted);
        let plan = super::automation::RuntimePlan::new(owner, &prepared.parts[0].2, 0.0, 48000);
        drive(&mut restarted, &plan, &mut 0);
        let current = restarted.parameters(1).unwrap();
        for (id, _, bipolar, domain) in &originals {
            let v = current.iter().find(|p| &p.stable_id == id).unwrap().current;
            assert!((position(*domain, *bipolar, v) - 0.65).abs() < 0.002);
        }
        assert_eq!(std::fs::read(source).unwrap(), original_bytes);
    }
}
const SETTLE_BLOCKS: usize = 20;
/// Sends `plan` for `SETTLE_BLOCKS` blocks from `frame`, then pumps the main thread.
/// Six Sines reads back its smoothed level, which settles within a few blocks; Floe
/// reports a new value only after a pump.
fn drive(
    native: &mut crate::play::audition_native::Native,
    plan: &crate::play::automation::RuntimePlan,
    frame: &mut u64,
) {
    let mut out = [0.0; 128];
    let mut blocks = 0;
    let mut rebinds = 0;
    while blocks < SETTLE_BLOCKS {
        let bindings = native.prepare_automation(&plan.owner).unwrap();
        let mut render = unsafe { native.render() };
        render.set_automation(bindings);
        while blocks < SETTLE_BLOCKS {
            if !render.automate(Some(plan), *frame) {
                // Surge rescans its metadata once a macro moves, which stales the
                // bindings; the editor binds afresh, and so does this probe.
                assert_eq!(render.parameter_error, Some(-6));
                rebinds += 1;
                assert!(rebinds <= 3);
                break;
            }
            assert!(render.process(&[0; 128], &[0; 128], 0, &mut out, 2));
            *frame += 64;
            blocks += 1;
        }
    }
    native.pump();
}
/// Floe drops parameter events until real time has passed since its state loaded.
fn settle(native: &crate::play::audition_native::Native) {
    for _ in 0..10 {
        native.pump();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
fn api(domain: Domain, position: f64) -> f64 {
    domain.api(Position::new(position).unwrap()).get()
}
/// The 0-1 position of a read value; `signal` marks Surge's -1-1 read of a bipolar macro.
fn position(domain: Domain, signal: bool, current: f64) -> f64 {
    if signal {
        (current + 1.0) / 2.0
    } else {
        domain
            .position(domain.value(current).unwrap())
            .unwrap()
            .get()
    }
}
