use super::tests::contents;
use super::*;
use serde_json::Value;

fn json(archive: &[u8], name: &str) -> Value {
    serde_json::from_slice(&contents(archive.to_vec(), name)).unwrap()
}

#[test]
fn layers_merge_before_track_effects_and_master_keeps_its_own_chain() {
    let _library = unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() };
    let mut app = crate::view::palette::tests::app();
    let loaded = app.loaded.as_mut().unwrap();
    let patch = &mut loaded.patches[1];
    let mut second = patch.layers[0].clone();
    second.settings.id = 2;
    second.settings.octave = 1;
    second.settings.volume_db = -6.0206;
    patch.layers.push(second);
    patch.effects = ["compressor", "delay", "reverb"]
        .map(crate::view::effects::tests::effect)
        .into();
    patch.effects[1].bypass = true;
    crate::play::track_volume::write(&mut loaded.music.tracks[1], 0.4);
    let effects = ["limiter", "hall"].map(crate::view::effects::tests::effect);
    app.audition
        .set_master(cat_smf_editor::master_effects::Settings {
            effects: effects.into(),
            ..Default::default()
        });
    let archive = bytes(&app).unwrap();
    let project = json(&archive, "project.uapmd");
    assert_eq!(project["tracks"][1]["clips"][0]["file"], "clips/1.midi2");
    assert!((project["tracks"][1]["volume"].as_f64().unwrap() - 0.4).abs() < 1e-6);
    assert_eq!(
        project["master_track"]["graph"]["external_file"],
        "graphs/master.json"
    );
    let graph = json(&archive, "graphs/1.json");
    assert_eq!(graph["plugins"].as_array().unwrap().len(), 5);
    assert_eq!(graph["plugins"][0]["group_index"], 0);
    assert_eq!(graph["plugins"][1]["group_index"], 1);
    let edges = |graph: &Value| {
        graph["connections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| {
                (
                    edge["source"]["node_id"].as_str().unwrap().to_owned(),
                    edge["target"]["node_id"].as_str().unwrap().to_owned(),
                )
            })
            .collect::<Vec<_>>()
    };
    let connections = edges(&graph);
    for source in ["layer:0:gain", "layer:1:gain"] {
        assert!(connections.contains(&(source.into(), "effect:0".into())));
    }
    assert!(connections.contains(&("effect:0".into(), "effect:2".into())));
    assert!(!connections
        .iter()
        .any(|(a, b)| a == "effect:1" || b == "effect:1"));
    assert_eq!(contents(archive.clone(), "plugin_states/1-fx-1.state"), [1]);
    let master = json(&archive, "graphs/master.json");
    assert_eq!(
        edges(&master),
        [
            ("graph:input".into(), "effect:0".into()),
            ("effect:0".into(), "effect:1".into()),
            ("effect:1".into(), "graph:output".into())
        ]
    );
    // Inspect the writer's grouped clip independently of the export builder.
    let data = contents(archive, "clips/1.midi2");
    assert_eq!(&data[..8], b"SMF2CLIP");
    let words: Vec<_> = data[8..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_be_bytes(*b))
        .collect();
    let mut notes = vec![];
    let mut offset = 0;
    let mut tick = 0;
    while offset < words.len() {
        let word = words[offset];
        let kind = word >> 28;
        if word & 0xfff00000 == 0x00400000 {
            tick += word & 0xfffff;
        }
        if kind == 2 || kind == 4 {
            notes.push((tick, (word >> 24) & 15, (word >> 8) & 127));
        }
        offset += match kind {
            3 | 4 => 2,
            5 | 13 | 15 => 4,
            _ => 1,
        };
    }
    assert_eq!(notes, [(0, 0, 60), (0, 1, 72), (480, 0, 60), (480, 1, 72)]);
}
