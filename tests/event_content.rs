use cat_smf_editor::{model::Event, score::event_content::decoded};

fn event(status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick: 0,
        status,
        meta_type,
        data: data.into(),
    }
}
#[test]
fn arrangement_baselines_and_performance_meta_are_readable() {
    use cat_smf_editor::score::{
        pattern::automation::{self, Baseline},
        performance,
        scene::arrangement,
    };
    let data = [
        automation::BASELINES,
        serde_json::to_vec(&vec![Baseline::Midi {
            channel: 1,
            controller: Some(64),
            value: 0,
        }])
        .unwrap()
        .as_slice(),
    ]
    .concat();
    assert_eq!(
        decoded(&event(0xff, 0x7f, &data)),
        "Container baselines: Channel 2 / CC 64: 0"
    );
    assert_eq!(
        decoded(&event(0xff, 0x7f, &[performance::OWNER, &[1]].concat())),
        "Performance owner: Container"
    );
    let mut m = cat_smf_editor::edit::tracks::new_song();
    arrangement::add_lane(&mut m, None).unwrap();
    let lane = m
        .tracks
        .iter()
        .flat_map(|t| &t.events)
        .find(|e| cat_smf_editor::score::scene::owned(e))
        .unwrap();
    let text = decoded(lane);
    assert!(
        text.starts_with("Scenes: ") && text.contains("Song 1 lanes"),
        "{text}"
    );
}

#[test]
fn programs_use_zero_based_gm_names_and_pedals_use_states() {
    assert_eq!(
        decoded(&event(0xc0, 0, &[0])),
        "Program Acoustic Grand Piano"
    );
    assert_eq!(decoded(&event(0xc5, 0, &[19])), "Program Church Organ");
    assert_eq!(decoded(&event(0xc0, 0, &[127])), "Program Gunshot");
    for program in 0..128 {
        assert!(decoded(&event(0xc0, 0, &[program])).starts_with("Program "));
    }
    for (value, state) in [(0, "off"), (63, "off"), (64, "on"), (127, "on")] {
        assert_eq!(
            decoded(&event(0xb0, 0, &[64, value])),
            format!("Sustain {state}")
        );
    }
    assert_eq!(decoded(&event(0xb0, 0, &[7, 100])), "Volume 100");
    assert_eq!(decoded(&event(0xb0, 0, &[119, 42])), "CC 119 = 42");
    assert_eq!(decoded(&event(0xe0, 0, &[0, 64])), "Pitch Bend 0");
    assert_eq!(decoded(&event(0xe0, 0, &[0, 0])), "Pitch Bend -8192");
    assert_eq!(decoded(&event(0xe0, 0, &[127, 127])), "Pitch Bend 8191");
}

#[test]
fn meta_contents_are_readable_and_invalid_payloads_are_not_fabricated() {
    assert_eq!(
        decoded(&event(0xff, 0x51, &[7, 0xa1, 0x20])),
        "Tempo = 120.00 BPM (500000 µs/quarter)"
    );
    assert_eq!(
        decoded(&event(0xff, 0x58, &[6, 3, 24, 8])),
        "Time signature = 6/8 · 24 clocks/click · 8 32nds/quarter"
    );
    assert_eq!(
        decoded(&event(0xff, 0x59, &[0xff, 0])),
        "Key signature = F major"
    );
    assert_eq!(
        decoded(&event(0xff, 0x59, &[0, 1])),
        "Key signature = A minor"
    );
    assert_eq!(
        decoded(&event(0xff, 3, "Organ\nオルガン".as_bytes())),
        "Track name: Organ\\nオルガン"
    );
    assert_eq!(decoded(&event(0xff, 0x2f, &[])), "End of Track");
    for (status, kind, data) in [
        (0xff, 0x51, vec![0, 0, 0]),
        (0xff, 0x51, vec![7, 0xa1]),
        (0xff, 0x59, vec![8, 0]),
        (0xc0, 0, vec![]),
        (0xc0, 0, vec![128]),
        (0xb0, 0, vec![64]),
    ] {
        assert!(decoded(&event(status, kind, &data)).contains("undecoded"));
    }
    assert!(decoded(&event(0xf0, 0, &[0x41, 0x10, 0xf7])).contains("41 10 F7"));
    let long_text = decoded(&event(0xff, 1, &vec![b'a'; 100_000]));
    assert!(long_text.ends_with('…'));
    assert!(long_text.len() < 150);
}

#[test]
fn meta_kinds_and_contents_are_separate_parts_and_notes_keep_their_pitch() {
    use cat_smf_editor::score::event_content::{parts, Part};
    assert_eq!(
        parts(&event(0xff, 0x51, &[7, 0xa1, 0x20])),
        [
            Part::Label("Tempo = ".into()),
            Part::Value("120.00 BPM (500000 µs/quarter)".into())
        ]
    );
    assert_eq!(
        parts(&event(0xff, 3, b"Organ")),
        [
            Part::Label("Track name: ".into()),
            Part::Value("Organ".into())
        ]
    );
    assert_eq!(
        parts(&event(0xc5, 0, &[19])),
        [
            Part::Label("Program ".into()),
            Part::Value("Church Organ".into())
        ]
    );
    assert_eq!(
        parts(&event(0x80, 0, &[81, 0])),
        [
            Part::Label("Note Off ".into()),
            Part::Note(81),
            Part::Label(" · velocity 0".into())
        ]
    );
    assert_eq!(
        decoded(&event(0x80, 0, &[81, 0])),
        "Note Off A5 · velocity 0"
    );
}

#[test]
fn own_sequencer_specific_payloads_read_as_their_purpose() {
    use cat_smf_editor::{
        host::PluginKey,
        instrument_layers::{self, Layer, Settings},
        model::Track,
        patch_reference::{PatchReference, Source},
    };
    let reference = |display: &str| PatchReference {
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "test".into(),
            name: "test".into(),
            vendor: "test".into(),
            bundle_path: "X:/test.clap".into(),
        },
        source: Source::Catalog {
            path: "X:/presets/organ.preset".into(),
            file: "X:/presets/organ.preset".into(),
        },
        display: display.into(),
    };
    let settings = Settings {
        layers: vec![
            Layer::default(),
            Layer {
                id: 2,
                patch: Some(reference("Organ")),
                octave: 1,
                volume_db: -3.0,
                mute: true,
                solo: false,
            },
        ],
        effects: vec![
            cat_smf_editor::instrument_layers::Effect {
                patch: reference("Hall"),
                bypass: false,
            },
            cat_smf_editor::instrument_layers::Effect {
                patch: reference("Delay"),
                bypass: true,
            },
        ],
        ..Default::default()
    };
    let mut track = Track { events: vec![] };
    struct Present(cat_smf_editor::patch_roots::Table);
    impl cat_smf_editor::patch_roots::Roots for Present {
        fn plugin(
            &mut self,
            key: &PluginKey,
            _: bool,
        ) -> Result<cat_smf_editor::patch_roots::Plugin, String> {
            self.0.plugin(key)
        }
        fn exists(&self, _: &std::path::Path) -> bool {
            true
        }
    }
    let roots = &mut Present(cat_smf_editor::patch_roots::Table {
        plugins: vec![cat_smf_editor::patch_roots::Plugin {
            format: "CLAP".into(),
            id: "test".into(),
            bundle_path: "X:/test.clap".into(),
            roots: vec!["X:/presets".into()],
        }],
    });
    instrument_layers::write(&mut track, &settings, roots).unwrap();
    assert_eq!(
        decoded(&track.events[0]),
        "Instrument layers: Surge XT / Init Saw + Organ (+1 oct, -3.0 dB, Mute) · Effects Hall → Delay (Bypass)"
    );
    let unknown = event(0xff, 0x7f, &[0x7d, 1, 2]);
    assert_eq!(decoded(&unknown), "Sequencer-specific: 7D 01 02");
    let foreign = event(0xff, 0x7f, br#"ly2mid-tempo-v1:{"tempo": 300000}"#);
    assert_eq!(
        decoded(&foreign),
        r#"Sequencer-specific: ly2mid-tempo-v1:{"tempo": 300000}"#
    );
    let mut broken = instrument_layers::PREFIX.to_vec();
    broken.extend(b"{");
    assert!(decoded(&event(0xff, 0x7f, &broken)).starts_with("Instrument layers: invalid ("));
}

// Namespace-only literals classify ownership; only concrete payload prefixes need decoders.
fn own_payload_prefixes(source: &str) -> Vec<Vec<u8>> {
    source
        .match_indices(r#"b"\x7dcat-"#)
        .filter_map(|(start, _)| {
            let literal = &source[start + 2..];
            let literal = &literal[..literal.find('"').unwrap()];
            if literal
                .split_once(r"\0")
                .is_some_and(|(_, kind)| kind.is_empty())
            {
                return None;
            }
            Some(
                literal
                    .replace(r"\x7d", "\x7d")
                    .replace(r"\0", "\0")
                    .into_bytes(),
            )
        })
        .collect()
}

#[test]
fn prefix_discovery_excludes_namespaces_but_keeps_unknown_payload_kinds() {
    let source = r#"
        data.starts_with(b"\x7dcat-smf-editor\0");
        data.starts_with(b"\x7dcat-plugin-player\0");
        const VOLUME: &[u8] = b"\x7dcat-smf-editor\0volume\0";
        const FUTURE: &[u8] = b"\x7dcat-smf-editor\0unknown-future-kind\0";
    "#;
    let prefixes = own_payload_prefixes(source);
    assert_eq!(
        prefixes,
        [
            cat_smf_editor::app_meta::TRACK_VOLUME.to_vec(),
            b"\x7dcat-smf-editor\0unknown-future-kind\0".to_vec(),
        ]
    );
    // An unimplemented kind still reaches and fails the decoder coverage check.
    assert!(!decoded(&event(0xff, 0x7f, &prefixes[0])).starts_with("Sequencer-specific"));
    assert!(decoded(&event(0xff, 0x7f, &prefixes[1])).starts_with("Sequencer-specific"));
}

/// Every concrete own payload prefix in the sources must have a named decoder.
#[test]
fn every_own_sequencer_specific_prefix_has_a_decoder() {
    fn sources(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, files);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    sources(&root.join("../cat-plugin-player/shared"), &mut files);
    let mut prefixes = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        prefixes.extend(
            own_payload_prefixes(&text)
                .into_iter()
                .map(|prefix| (file.clone(), prefix)),
        );
    }
    assert!(prefixes.len() >= 7, "{prefixes:?}");
    for (file, prefix) in prefixes {
        let text = decoded(&event(0xff, 0x7f, &prefix));
        assert!(
            !text.starts_with("Sequencer-specific"),
            "{}: {text}",
            file.display()
        );
    }
}

#[test]
fn app_owned_payloads_read_as_values() {
    use cat_smf_editor::{app_meta, edit::tempo_curve};
    let mut volume = app_meta::TRACK_VOLUME.to_vec();
    volume.push(1);
    volume.extend(0.5f32.to_le_bytes());
    assert_eq!(
        decoded(&event(0xff, 0x7f, &volume)),
        "Track volume: -6.0 dB"
    );
    let auto_save = [app_meta::AUTO_SAVE, b"\x01\x01"].concat();
    assert_eq!(decoded(&event(0xff, 0x7f, &auto_save)), "Auto save: On");
    let mut tempo = tempo_curve::PREFIX.to_vec();
    tempo.extend(
        br#"{"version":1,"points":[{"tick":0,"bpm":120.0},{"tick":480,"bpm":90.0}],"controls":[{"tick":240,"bpm":105.0}]}"#,
    );
    assert_eq!(
        decoded(&event(0xff, 0x7f, &tempo)),
        "Tempo curve: 120.00 → 90.00 BPM"
    );
}

#[test]
fn chord_guide_role_and_chord_are_readable() {
    use cat_smf_editor::score::chord_guide;
    assert_eq!(
        decoded(&event(0xff, 0x7f, &[chord_guide::ROLE, &[1]].concat())),
        "Track role: Chord guide"
    );
    assert_eq!(
        decoded(&chord_guide::event(1920, "Isus4").unwrap()),
        "Chord: Isus4"
    );
}
