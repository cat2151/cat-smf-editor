use super::*;

#[test]
fn layer_selection_changes_only_named_layer_and_rejects_deleted_or_reloaded_targets() {
    let mut app = crate::view::tests::app(music());
    app.select_track(1);
    let patch = &mut app.loaded.as_mut().unwrap().patches[1];
    crate::view::layers::add(patch).unwrap();
    let id = patch.layers[1].settings.id;
    let target = Open::Layer {
        track: 1,
        layer: id,
        song: app.song_id,
    };
    let chosen = slot("layer-synth", vec![5]);
    apply(&mut app, target, chosen.clone()).unwrap();
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    assert!(patch.layers[0].instrument.is_none());
    assert_eq!(patch.layers[1].instrument.as_ref(), Some(&chosen));
    let selected = Selected {
        kit: None,
        target,
        plugin: chosen.plugin.clone(),
        preset_name: chosen.preset_name.clone(),
        reference: chosen.reference.clone().map(Box::new),
        state: resident_ipc::StateRef {
            name: "unused".into(),
            len: 0,
        },
    };
    let requested = Target {
        open: target,
        bulk: false,
        original: app.loaded.as_ref().unwrap().original.clone(),
    };
    assert_eq!(check(&app, &selected, Some(&requested)), Ok(false));
    app.track = 2;
    assert!(check(&app, &selected, Some(&requested)).is_err());
    app.track = 1;
    app.loaded.as_mut().unwrap().patches[1]
        .layers
        .retain(|l| l.settings.id != id);
    assert!(check(&app, &selected, Some(&requested)).is_err());
    assert!(apply(&mut app, target, chosen.clone()).is_err());
    app.song_id += 1;
    assert!(apply(&mut app, target, chosen).is_err());
}

#[test]
fn kit_request_target_and_note_metadata_are_validated_before_state_is_read() {
    let mut app = crate::view::tests::app(music());
    app.select_track(1);
    let target = Open::Kit {
        track: 1,
        layer: None,
        song: app.song_id,
        request: 7,
    };
    let requested = Target {
        open: target,
        bulk: false,
        original: app.loaded.as_ref().unwrap().original.clone(),
    };
    let chosen = slot("kit-plugin", vec![5]);
    let mut selected = Selected {
        kit: Some(cat_smf_editor::drum_kit::Kit {
            layer: 1,
            rows: None,
            muted: vec![],
            swing: Default::default(),
            notes: vec![
                cat_smf_editor::drum_kit::Note {
                    note: 0,
                    name: "Original".into(),
                    velocity: 100,
                },
                cat_smf_editor::drum_kit::Note {
                    note: 127,
                    name: String::new(),
                    velocity: 100,
                },
            ],
        }),
        target,
        plugin: chosen.plugin,
        preset_name: chosen.preset_name,
        reference: chosen.reference.map(Box::new),
        state: resident_ipc::StateRef {
            name: "never-read".into(),
            len: 0,
        },
    };
    assert_eq!(check(&app, &selected, Some(&requested)), Ok(false));
    selected.target = Open::Kit {
        track: 1,
        layer: None,
        song: app.song_id,
        request: 6,
    };
    assert!(check(&app, &selected, Some(&requested)).is_err());
    selected.target = target;
    selected.kit.as_mut().unwrap().notes.clear();
    assert!(check(&app, &selected, Some(&requested)).is_err());
}
