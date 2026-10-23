//! Explicit opt-in bundles, no device/catalog scan/user patch data.
use super::*;
use crate::play::{
    audio_sequence::{Note, Sequence},
    automation::{Automation, OwnerPlan},
};
use cat_smf_editor::{
    chain_settings::PluginState,
    edit::{
        parameter_curve::{Curve, Point, Target, ValueSpace},
        tracks,
    },
    host::PluginKey,
    score::{audition_timing::TempoMap, instrument::Patch},
};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

fn patch(format: &str, path: PathBuf, id: &str) -> Patch {
    let slot = crate::test_support::referenced(PluginState {
        reference: None,
        plugin: PluginKey {
            format: format.into(),
            id: id.into(),
            name: "Parameter fixture".into(),
            vendor: "cat test".into(),
            bundle_path: path.to_string_lossy().into_owned(),
        },
        state: [0.2f64.to_le_bytes(), 0.3f64.to_le_bytes()].concat(),
        preset_name: None,
        bypass: false,
    });
    Patch::from(cat_smf_editor::chain_settings::Settings {
        instrument: Some(slot),
        ..Default::default()
    })
}
fn plans(format: &str, id: &str) -> Automation {
    let curves = [("101", "Cutoff", 0.2, 0.8), ("118", "Resonance", 0.3, 0.6)]
        .into_iter()
        .map(|(parameter_id, name, start, end)| {
            Arc::new(Curve {
                version: 1,
                target: Target {
                    group: 1,
                    layer: 1,
                    plugin_format: format.into(),
                    plugin_id: id.into(),
                    parameter_id: parameter_id.into(),
                },
                value_space: if format == "CLAP" {
                    ValueSpace::Plain
                } else {
                    ValueSpace::Normalized
                },
                minimum: 0.0,
                maximum: 1.0,
                name: name.into(),
                path: "Fixture".into(),
                points: vec![
                    Point {
                        tick: 0,
                        value: start,
                    },
                    Point {
                        tick: 960,
                        value: end,
                    },
                ],
                controls: vec![Point {
                    tick: 480,
                    value: (start + end) / 2.0,
                }],
            })
        })
        .collect();
    Automation {
        owners: Arc::new(vec![Arc::new(OwnerPlan {
            live: Default::default(),
            owner: 0,
            curves,
            tempo: Arc::new(TempoMap::new(&tracks::new_song(), 0)),
            boundaries: vec![(0, 0.0), (960, 1.0)],
        })]),
        error: None,
    }
}
fn sequence() -> Sequence {
    Sequence::new(
        vec![Note {
            start: 0.0,
            duration: 1.0,
            pitch: 62,
            velocity: 100,
        }],
        1.0,
    )
}

#[test]
#[ignore = "requires explicit built parameter fixtures; set PARAMETER_NATIVE_FIXTURE_DIR"]
fn automation_native_offline_two_ids_seek_and_baseline_restore() {
    let root = PathBuf::from(
        std::env::var_os("PARAMETER_NATIVE_FIXTURE_DIR").expect("explicit fixture directory"),
    );
    let root = root.canonicalize().unwrap();
    let work = PathBuf::from(
        std::env::var_os("PARAMETER_NATIVE_WORK_DIR").expect("explicit global work directory"),
    );
    let _library = cat_smf_editor::native_library::load_at(&work).unwrap();
    let cancelled = AtomicBool::new(false);
    for (format, file, id) in [
        ("CLAP", "parameter_clap_fixture.clap", "cat.test.parameters"),
        (
            "VST3",
            "parameter_vst3_fixture.vst3",
            "132132131234123456785678ABCDEF01",
        ),
    ] {
        let mut patch = patch(format, root.join(file), id);
        let mut other = patch.layers[0].clone();
        other.settings.id = 2;
        patch.layers.push(other);
        let baseline = patch.clone();
        let automation = plans(format, id);
        let ordinary = Instruments::open(
            48000,
            &[(0, patch.clone(), sequence())],
            &Default::default(),
        )
        .unwrap();
        let normal_started = Instant::now();
        let normal = unsafe {
            ordinary.job(
                48000,
                vec![(0, patch.clone(), sequence())],
                &[1.0],
                true,
                vec![],
            )
        }
        .run(&cancelled)
        .unwrap()
        .unwrap();
        let normal_millis = normal_started.elapsed().as_secs_f64() * 1000.0;
        assert!((normal[0] - 0.4).abs() < 1e-6 && (normal[1] - 0.6).abs() < 1e-6);
        drop(ordinary);
        let parts = vec![(0, patch.clone(), sequence())];
        let mut instruments =
            Instruments::open_automated(48000, &parts, &Default::default(), &automation).unwrap();
        assert_eq!(instruments.natives.len(), 1);
        let started = Instant::now();
        let audio = unsafe { instruments.job(48000, parts, &[1.0], true, vec![]) }
            .run(&cancelled)
            .unwrap()
            .unwrap();
        let millis = started.elapsed().as_secs_f64() * 1000.0;
        for frame in [0usize, 64, 1024, 12032, 24000, 36032, 47040] {
            let expected_left = 0.4 + 0.6 * frame as f64 / 48000.0;
            let expected_right = 0.6 + 0.3 * frame as f64 / 48000.0;
            assert!(
                (f64::from(audio[frame * 2]) - expected_left).abs() < 1e-5,
                "{format} sample {frame} left {} expected {expected_left}",
                audio[frame * 2]
            );
            assert!((f64::from(audio[frame * 2 + 1]) - expected_right).abs() < 1e-5);
        }
        let current = instruments.natives[0].patch().clone();
        assert_eq!(current, baseline);
        let untouched = instruments.natives[0].parameters(2).unwrap();
        assert!(
            (untouched[0].current - 0.2).abs() < 1e-6 && (untouched[1].current - 0.3).abs() < 1e-6
        );
        // A seek starts with the interpolated value before its restored held note.
        let parts = vec![(
            0,
            patch.clone(),
            Sequence::new(
                vec![Note {
                    start: 0.0,
                    duration: 0.25,
                    pitch: 62,
                    velocity: 100,
                }],
                0.25,
            ),
        )];
        let audio = unsafe { instruments.job_at(48000, parts, &[1.0], true, vec![], 0.5) }
            .run(&cancelled)
            .unwrap()
            .unwrap();
        assert!((audio[0] - 0.7).abs() < 1e-6 && (audio[1] - 0.75).abs() < 1e-6);
        let native = instruments
            .natives
            .remove(0)
            .restore_baseline(48000)
            .unwrap();
        let values = native.parameters(1).unwrap();
        assert!((values[0].current - 0.2).abs() < 1e-6 && (values[1].current - 0.3).abs() < 1e-6);
        assert_eq!(native.patch(), &baseline);
        drop(native);
        // Three distinct ticks round to one sample. The early note sees the
        // old values; the later note sees the new ones, without an extra frame.
        let mut collision = plans(format, id);
        let owner = Arc::make_mut(&mut Arc::make_mut(&mut collision.owners)[0]);
        let mut fast = tracks::new_song();
        fast.division = 32767;
        fast.tracks[0]
            .events
            .iter_mut()
            .find(|e| e.meta_type == 0x51)
            .unwrap()
            .data = vec![0, 0, 1];
        owner.tempo = Arc::new(TempoMap::new(&fast, 0));
        owner.boundaries = (0..=3)
            .map(|tick| (tick, owner.tempo.seconds_at(tick)))
            .collect();
        for lane in &mut owner.curves {
            let lane = Arc::make_mut(lane);
            let start = lane.points[0].value;
            let end = if lane.target.parameter_id == "101" {
                0.7
            } else {
                0.8
            };
            lane.points = vec![
                Point {
                    tick: 0,
                    value: start,
                },
                Point {
                    tick: 1,
                    value: start,
                },
                Point {
                    tick: 2,
                    value: end,
                },
            ];
            lane.controls = vec![
                Point {
                    tick: 0,
                    value: start,
                },
                Point {
                    tick: 1,
                    value: start,
                },
            ];
        }
        let tick = owner.tempo.seconds_at(1);
        let seq = Sequence::new(
            vec![
                Note {
                    start: tick,
                    duration: tick,
                    pitch: 61,
                    velocity: 100,
                },
                Note {
                    start: tick * 3.0,
                    duration: 0.1,
                    pitch: 60,
                    velocity: 100,
                },
            ],
            0.1,
        );
        let mut one = patch.clone();
        one.layers.truncate(1);
        let parts = vec![(0, one, seq)];
        let ordered =
            Instruments::open_automated(48000, &parts, &Default::default(), &collision).unwrap();
        let audio = unsafe { ordered.job(48000, parts, &[1.0], true, vec![]) }
            .run(&cancelled)
            .unwrap()
            .unwrap();
        assert!(
            (audio[0] - 0.7).abs() < 1e-6 && (audio[1] - 0.8).abs() < 1e-6,
            "{format} source-tick collision result {:?}",
            &audio[..2]
        );
        println!("{format}: 2 IDs, held note, seek, 1 owner Native/2 layers, untouched layer, baseline restoration PASS; one second/48000 Hz: normal {normal_millis:.3} ms, automation {millis:.3} ms ({:.3}% realtime); isolated fixture/debug build, single run",millis/10.0);
    }
}
