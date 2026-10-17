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
