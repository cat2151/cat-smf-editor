use super::*;
use std::io::Write;

fn library() -> libloading::Library {
    unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() }
}

pub(crate) fn fixture(path: &Path, position: u64, traversal: bool) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let mut add = |name: &str, bytes: &[u8]| {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    };
    let mut clip = b"SMF2CLIP".to_vec();
    let words: [u32; 24] = [
        0x00400000, 0x003001e0, 0x00400000, 0xf0200000, 0, 0, 0, 0x00400000, 0x40903c00,
        0xba000000, 0x00400780, 0x40803c00, 0, 0x00400000, 0x40904000, 0xba000000, 0x00400780,
        0x40804000, 0, 0x00400000, 0xf0210000, 0, 0, 0,
    ];
    for word in &words {
        clip.extend(word.to_be_bytes());
    }
    add("clips/notes.midi2", &clip);
    let project = serde_json::json!({
        "tracks": [{"id":"track_1", "volume":1,
            "graph":{"graph_type":"urn:uapmd-graph:common/graph/dag/v1", "external_file":"graphs/track.json"},
            "clips":[{"id":"clip_1", "position_samples":position, "file":"clips/notes.midi2", "clip_type":"midi", "tick_resolution":480}]}],
        "master_track":{"id":"master_track", "volume":1, "clips":[]}
    });
    add("project.uapmd", &serde_json::to_vec(&project).unwrap());
    let graph = serde_json::json!({"plugins":[{"node_id":"plugin:1", "plugin_id":"test.synth", "format":"CLAP", "display_name":"Test Synth", "state_file":"plugin_states/synth.state"}],
    "connections":[
        {"bus_type":"event", "source":{"type":"graph_input","node_id":"graph:input","bus_index":0},"target":{"type":"plugin","node_id":"plugin:1","bus_index":0}},
        {"bus_type":"audio", "source":{"type":"plugin","node_id":"plugin:1","bus_index":0},"target":{"type":"graph_output","node_id":"graph:output","bus_index":0}}
    ]});
    add("graphs/track.json", &serde_json::to_vec(&graph).unwrap());
    add("plugin_states/synth.state", &[3, 1, 4]);
    if traversal {
        add("../escape", &[0]);
    }
    zip.finish().unwrap();
}

#[test]
fn midi2_notes_keep_long_ticks_and_state_references_survive_smf() {
    let _library = library();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("fixture.uapmdz");
    fixture(&path, 0, false);
    let imported = read_with(&path, |bytes| {
        imported_state::store_at(&temp.path().join("states"), bytes)
    })
    .unwrap();
    assert_eq!(imported.music.division, 480);
    let notes: Vec<_> = imported.music.tracks[1]
        .events
        .iter()
        .filter(|e| e.status < 0xf0)
        .map(|e| (e.tick, e.status, e.data.clone()))
        .collect();
    assert_eq!(
        notes,
        [
            (0, 0x90, vec![60, 93]),
            (1920, 0x80, vec![60, 0]),
            (1920, 0x90, vec![64, 93]),
            (3840, 0x80, vec![64, 0])
        ]
    );
    assert_eq!(
        imported.patches[0].source,
        Source::Imported {
            sha256: imported_state::digest(&[3, 1, 4])
        }
    );
    let mut roots = patch_roots::Table {
        plugins: vec![patch_roots::Plugin {
            format: "CLAP".into(),
            id: "test.synth".into(),
            bundle_path: "X:/plugins/synth.clap".into(),
            roots: vec![],
        }],
    };
    let music = imported.into_music(&mut roots).unwrap();
    let restored = crate::ffi::parse(&crate::smf_io::encode(&music).unwrap()).unwrap();
    let settings = instrument_layers::read(&restored.tracks[1], &mut roots).unwrap();
    assert_eq!(
        settings.layers[0]
            .patch
            .as_ref()
            .unwrap()
            .plugin
            .bundle_path,
        "X:/plugins/synth.clap"
    );
    assert_eq!(
        settings.layers[0].patch.as_ref().unwrap().source,
        Source::Imported {
            sha256: imported_state::digest(&[3, 1, 4])
        }
    );
    assert!(crate::smf_io::encode(&restored)
        .unwrap()
        .windows(3)
        .all(|v| v != [3, 1, 4]));
}

#[test]
fn unsupported_positions_and_archive_escape_fail_before_state_storage() {
    let _library = library();
    let temp = tempfile::tempdir().unwrap();
    for (position, traversal) in [(1, false), (0, true)] {
        let path = temp.path().join("fixture.uapmdz");
        fixture(&path, position, traversal);
        assert!(read_with(&path, |_| panic!("invalid import must not store a patch")).is_err());
    }
}

#[test]
#[ignore = "requires UAPMD_TEST_PROJECT; reads the user's project without modifying it"]
fn real_project_notes_and_patch_bytes() {
    let _library = library();
    let path = PathBuf::from(std::env::var_os("UAPMD_TEST_PROJECT").unwrap());
    let mut state_sizes = vec![];
    let imported = read_with(&path, |bytes| {
        state_sizes.push(bytes.len());
        Ok(imported_state::digest(bytes))
    })
    .unwrap();
    assert_eq!(state_sizes, [59868, 171333]);
    assert_eq!(imported.music.division, 480);
    assert_eq!(imported.music.tracks.len(), 3);
    for (track, count) in [(1, 6), (2, 2)] {
        assert_eq!(
            imported.music.tracks[track]
                .events
                .iter()
                .filter(|e| e.status & 0xf0 == 0x90)
                .count(),
            count
        );
        assert_eq!(
            imported.music.tracks[track].events.last().unwrap().tick,
            3840
        );
    }
    assert_eq!(
        imported
            .patches
            .iter()
            .map(|p| p.plugin.id.as_str())
            .collect::<Vec<_>>(),
        ["org.surge-synth-team.surge-xt", "com.vastdynamics.VAST2"]
    );
    assert_eq!(
        imported.music.tracks[0]
            .events
            .iter()
            .filter(|e| e.meta_type == 0x51)
            .count(),
        1
    );
}
