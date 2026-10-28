use super::*;

#[test]
#[ignore = "requires installed Surge XT; offline native parameter delivery"]
fn surge_direct_parameter_reaches_processor_and_restores_saved_baseline() {
    let root = PathBuf::from(std::env::var_os("PATTERN_NATIVE_WORK_DIR").unwrap());
    let work = tempfile::tempdir_in(root).unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let patch = Patch::default();
    let mut native = Native::open(48000, &patch).unwrap();
    for _ in 0..10 {
        native.pump();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let parameter = native
        .parameters(1)
        .unwrap()
        .into_iter()
        .find(|p| p.eligible && p.name.to_lowercase().contains("volume"))
        .expect("continuous Surge volume parameter");
    let baseline = parameter.current;
    let value = parameter.minimum + (parameter.maximum - parameter.minimum) * 0.37;
    let key = cat_smf_editor::chain::default_instrument().unwrap();
    let curve = Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: key.format,
            plugin_id: key.id,
            parameter_id: parameter.stable_id.clone(),
        },
        value_space: if parameter.normalized {
            ValueSpace::Normalized
        } else {
            ValueSpace::Plain
        },
        minimum: parameter.minimum,
        maximum: parameter.maximum,
        name: parameter.name.clone(),
        path: parameter.path.clone(),
        points: vec![Point { tick: 0, value }],
        controls: vec![],
    };
    let owner = Arc::new(OwnerPlan {
        windows: Default::default(),
        midi: false,
        live: Default::default(),
        owner: 0,
        curves: vec![Arc::new(curve)],
        tempo: Arc::new(TempoMap::new(&tracks::new_song(), 0)),
        boundaries: vec![(0, 0.)],
    });
    let binding = native.prepare_automation(&owner).unwrap();
    let sequence = sequence();
    let timeline = crate::play::event_timeline::Timeline::new(&sequence, 0., 48000);
    let runtime = crate::play::automation::RuntimePlan::new(owner, &sequence, 0., 48000);
    {
        let mut render = unsafe { native.render() };
        render.set_automation(binding);
        assert!(render.process_scheduled(&timeline, Some(&runtime), 0, &mut [0.; 128], 2));
    }
    let current = native
        .parameters(1)
        .unwrap()
        .into_iter()
        .find(|p| p.stable_id == parameter.stable_id)
        .unwrap()
        .current;
    assert!(
        (current - value).abs() < 1e-4,
        "{} sent={value} actual={current}",
        parameter.name
    );
    let restored = native.restore_baseline(48000).unwrap();
    let current = restored
        .parameters(1)
        .unwrap()
        .into_iter()
        .find(|p| p.stable_id == parameter.stable_id)
        .unwrap()
        .current;
    assert!(
        (current - baseline).abs() < 1e-4,
        "{} baseline={baseline} actual={current}",
        parameter.name
    );
    assert_eq!(restored.patch(), &patch);
    eprintln!(
        "Surge {} native value {value}; baseline {baseline} restored",
        parameter.name
    );
}

#[test]
#[ignore = "requires explicit built parameter fixtures and global work directory; offline only"]
fn pattern_windows_and_midi_only_plans_restore_at_end_seek_and_native_reuse() {
    use cat_smf_editor::score::pattern::render::ParameterInterval;
    let root = PathBuf::from(std::env::var_os("PARAMETER_NATIVE_FIXTURE_DIR").unwrap())
        .canonicalize()
        .unwrap();
    let work = PathBuf::from(std::env::var_os("PARAMETER_NATIVE_WORK_DIR").unwrap());
    let _library = cat_smf_editor::native_library::load_at(&work).unwrap();
    let cancel = AtomicBool::new(false);
    for (format, file, id) in [
        ("CLAP", "parameter_clap_fixture.clap", "cat.test.parameters"),
        (
            "VST3",
            "parameter_vst3_fixture.vst3",
            "132132131234123456785678ABCDEF01",
        ),
    ] {
        let patch = patch(format, root.join(file), id);
        let mut plan = plans(format, id);
        let owner = Arc::make_mut(&mut Arc::make_mut(&mut plan.owners)[0]);
        let curve = Arc::make_mut(&mut owner.curves[0]);
        curve.points = vec![Point {
            tick: 0,
            value: 0.8,
        }];
        curve.controls.clear();
        let curve = curve.clone();
        owner.windows.insert(
            curve.target.clone(),
            Arc::new(vec![
                ParameterInterval {
                    start: 0,
                    end: 240,
                    baseline: 0.2,
                    curve: curve.clone(),
                },
                ParameterInterval {
                    start: 480,
                    end: 960,
                    baseline: 0.2,
                    curve,
                },
            ]),
        );
        owner.boundaries = vec![(0, 0.), (240, 0.25), (480, 0.5), (960, 1.)];
        let parts = vec![(0, patch.clone(), sequence())];
        let mut instruments =
            Instruments::open_automated(48000, &parts, &Default::default(), &plan).unwrap();
        let mut job = unsafe { instruments.job(48000, parts.clone(), &[1.], true, vec![]) };
        job.frames += 1; // Render the exact endpoint as well as the musical body.
        let audio = job.run(&cancel).unwrap().unwrap();
        for (sample, value) in [(0, 0.8), (16000, 0.2), (24000, 0.8)] {
            assert!(
                (audio[sample * 2] - value).abs() < 1e-6,
                "{format} sample {sample}"
            );
        }
        assert!(
            (instruments.natives[0].parameters(1).unwrap()[0].current - 0.2).abs() < 1e-6,
            "{format} endpoint baseline"
        );
        let mut seek = parts.clone();
        seek[0].2.skip(0.625);
        seek[0].2.truncate(0.125);
        let audio = unsafe { instruments.job_at(48000, seek, &[1.], true, vec![], 0.625) }
            .run(&cancel)
            .unwrap()
            .unwrap();
        assert!((audio[0] - 0.8).abs() < 1e-6, "{format} seek expression");
        let native = instruments
            .natives
            .remove(0)
            .restore_baseline(48000)
            .unwrap();
        assert!((native.parameters(1).unwrap()[0].current - 0.2).abs() < 1e-6);
        drop(native);
        drop(instruments);
        let mut midi = plans(format, id);
        let owner = Arc::make_mut(&mut Arc::make_mut(&mut midi.owners)[0]);
        owner.curves.clear();
        owner.midi = true;
        let mut s = Sequence::new(vec![], 0.25);
        s.events = vec![
            crate::play::audio_sequence::MidiEvent {
                time: 0.,
                word: 0x20b3_0740,
            },
            crate::play::audio_sequence::MidiEvent {
                time: 0.,
                word: 0x20e3_0060,
            },
        ];
        let parts = vec![(0, patch.clone(), s)];
        let mut instruments =
            Instruments::open_automated(48000, &parts, &Default::default(), &midi).unwrap();
        let audio = unsafe { instruments.job(48000, parts, &[1.], true, vec![]) }
            .run(&cancel)
            .unwrap()
            .unwrap();
        assert!((audio[0] - 64. / 127.).abs() < 1e-6 && (audio[1] - 12288. / 16383.).abs() < 1e-6);
        let native = instruments
            .natives
            .remove(0)
            .restore_baseline(48000)
            .unwrap();
        let values = native.parameters(1).unwrap();
        assert!(
            (values[0].current - 0.2).abs() < 1e-6 && (values[1].current - 0.3).abs() < 1e-6,
            "{format} MIDI reuse baseline"
        );
        assert_eq!(native.patch(), &patch);
    }
}
