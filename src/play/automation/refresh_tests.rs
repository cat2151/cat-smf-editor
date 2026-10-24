//! Real metadata invalidation must refresh tokens without replacing the running command.
use super::*;
use crate::play::{
    audio_sequence::Sequence,
    audition::Audition,
    automation::{Automation, OwnerPlan},
};
use cat_smf_editor::{
    chain_settings::{PluginState, Settings},
    edit::parameter_curve::{Curve, Point, Target, ValueSpace},
    host::PluginKey,
    score::audition_timing::TempoMap,
};
use std::time::{Duration, Instant};

#[test]
#[ignore = "requires PARAMETER_NATIVE_FIXTURE_DIR; uses a muted audio device"]
fn metadata_rescan_refreshes_running_voice_and_rejects_missing_target() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let (patch, automation) = fixture();
    let mut audition = Audition::default();
    audition.set_gain(0, 0.0);
    assert!(audition.sequences_automated(
        vec![(0, patch, Sequence::new(vec![], 10.0))],
        vec![],
        &automation,
        0.0,
    ));
    let voice = audition.voice.as_ref().unwrap();
    let before = voice.natives[0].parameters(1).unwrap()[0].binding;
    let progress = audition.progress.clone().unwrap();
    let generation = voice.mailbox.lock().unwrap().generation;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let voice = audition.voice.as_ref().unwrap();
        voice.natives[0].pump();
        if voice.parameter_error() == Some(-6) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "fixture did not invalidate the binding"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    audition.pump();
    assert!(audition.error().is_none(), "{:?}", audition.error());
    assert!(Arc::ptr_eq(&progress, audition.progress.as_ref().unwrap()));
    let voice = audition.voice.as_ref().unwrap();
    assert_eq!(voice.mailbox.lock().unwrap().generation, generation);
    let after = voice.natives[0].parameters(1).unwrap()[0].binding;
    assert_ne!(before, after);
    let elapsed = audition.seconds().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    audition.pump();
    assert!(audition.error().is_none());
    assert!(audition.seconds().unwrap() > elapsed);
    // Re-resolution must still reject a target that actually disappeared.
    let voice = audition.voice.as_mut().unwrap();
    let mut command = voice.mailbox.lock().unwrap();
    let old = command.parts[0].automation.as_ref().unwrap();
    let mut owner = old.owner.as_ref().clone();
    Arc::make_mut(&mut owner.curves[0]).target.parameter_id = "missing".into();
    let sequence = command.parts[0].sequence.clone().unwrap();
    command.parts = Arc::new(vec![super::super::Part {
        automation: Some(Arc::new(crate::play::automation::RuntimePlan::new(
            Arc::new(owner),
            &sequence,
            old.origin,
            old.rate,
        ))),
        sequence: Some(sequence),
        ..Default::default()
    }]);
    voice.renders.lock().unwrap()[0].parameter_error = Some(-6);
    voice.failed.store(true, Ordering::Relaxed);
    drop(command);
    audition.pump();
    assert!(audition.voice.is_none());
    assert!(audition.error().unwrap().contains("Missing parameter"));
}

#[test]
#[ignore = "requires PARAMETER_NATIVE_FIXTURE_DIR; uses a muted audio device"]
fn metadata_pump_rebinds_all_tracks_before_a_long_gui_frame() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let (patch, mut automation) = fixture();
    let mut second = automation.owners[0].as_ref().clone();
    second.owner = 1;
    Arc::make_mut(&mut automation.owners).push(Arc::new(second));
    let mut audition = Audition::default();
    audition.set_gain(0, 0.0);
    audition.set_gain(1, 0.0);
    assert!(audition.sequences_automated(
        vec![
            (0, patch.clone(), Sequence::new(vec![], 10.0)),
            (1, patch, Sequence::new(vec![], 10.0)),
        ],
        vec![],
        &automation,
        0.0,
    ));
    let progress = audition.progress.clone().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while progress.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline, "callback did not start");
        std::thread::sleep(Duration::from_millis(1));
    }
    let voice = audition.voice.as_mut().unwrap();
    let generation = voice.mailbox.lock().unwrap().generation;
    let before: Vec<_> = voice
        .natives
        .iter()
        .map(|n| n.parameters(1).unwrap()[0].binding)
        .collect();
    let renders = voice.renders.clone();
    assert!(cat_smf_editor::host::Host::main_thread_work_pending());
    // A callback posted after an empty-queue snapshot must wait for the next
    // protected poll, even though it is now visible to the native event loop.
    voice.pump(false, || {}).unwrap();
    assert!(cat_smf_editor::host::Host::main_thread_work_pending());
    assert!(voice.natives.iter().all(|n| !n.automation_changed()));
    // Metadata rebinding must not clear an independent device failure flag.
    voice.failed.store(true, Ordering::Relaxed);
    voice
        .pump(true, || assert!(renders.try_lock().is_err()))
        .unwrap();
    assert!(voice.failed.load(Ordering::Relaxed));
    voice.failed.store(false, Ordering::Relaxed);
    for (native, before) in voice.natives.iter().zip(before) {
        assert_ne!(before, native.parameters(1).unwrap()[0].binding);
        assert!(!native.automation_changed());
    }
    // No UI pump during this interval: the former path kept returning -6.
    let frame = progress.load(Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(350));
    assert!(!voice.failed.load(Ordering::Relaxed));
    assert!(voice.parameter_error().is_none());
    assert!(progress.load(Ordering::Relaxed) > frame);
    assert_eq!(voice.mailbox.lock().unwrap().generation, generation);
    assert!(Arc::ptr_eq(&progress, audition.progress.as_ref().unwrap()));
}

fn fixture() -> (Patch, Automation) {
    let key = PluginKey {
        format: "CLAP".into(),
        id: "cat.test.parameters".into(),
        name: "Parameter fixture".into(),
        vendor: "cat test".into(),
        bundle_path: std::path::PathBuf::from(
            std::env::var_os("PARAMETER_NATIVE_FIXTURE_DIR").unwrap(),
        )
        .join("parameter_clap_fixture.clap")
        .to_string_lossy()
        .into_owned(),
    };
    let patch = Patch::from(Settings {
        instrument: Some(crate::test_support::referenced(PluginState {
            reference: None,
            plugin: key.clone(),
            state: [0.2f64.to_le_bytes(), 0.3f64.to_le_bytes()].concat(),
            preset_name: None,
            bypass: false,
        })),
        ..Default::default()
    });
    let automation = Automation {
        owners: Arc::new(vec![Arc::new(OwnerPlan {
            live: Default::default(),
            owner: 0,
            curves: vec![Arc::new(Curve {
                version: 1,
                target: Target {
                    group: 1,
                    layer: 1,
                    plugin_format: key.format,
                    plugin_id: key.id,
                    parameter_id: "101".into(),
                },
                value_space: ValueSpace::Plain,
                minimum: 0.0,
                maximum: 1.0,
                name: "Cutoff".into(),
                path: "Fixture".into(),
                // This fixture requests a real CLAP rescan on receiving 0.91.
                points: vec![Point {
                    tick: 0,
                    value: 0.91,
                }],
                controls: vec![],
            })],
            tempo: Arc::new(TempoMap::new(&cat_smf_editor::edit::tracks::new_song(), 0)),
            boundaries: vec![(0, 0.0)],
        })]),
        error: None,
    };
    (patch, automation)
}
