use super::*;

#[test]
fn pattern_macro_confirm_saves_container_settings_without_rewriting_any_body() {
    use cat_smf_editor::score::pattern::{body, Catalog};
    fn signature(events: Vec<cat_smf_editor::model::Event>) -> Vec<(u64, u32, u8, u8, Vec<u8>)> {
        events
            .into_iter()
            .map(|e| (e.tick, e.delta, e.status, e.meta_type, e.data))
            .collect()
    }
    let (mut app, source, id) = crate::view::patterns::edit_tests::fixture();
    crate::view::patterns::library::edit(&mut app, id).unwrap();
    let m = &app.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(m).unwrap();
    let bodies = catalog
        .patterns
        .iter()
        .map(|(&id, p)| {
            (
                id,
                signature(body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap()),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let before = cat_smf_editor::smf_io::encode(m).unwrap();
    let mut draft = session(&app);
    draft.owner = source;
    draft.patch = app.loaded.as_ref().unwrap().patches[source].clone();
    draft.rows[0].set(0.7);
    app.macro_editor.session = Some(draft);
    finish(&mut app, true);
    assert!(
        app.macro_editor.error.is_none(),
        "{:?}",
        app.macro_editor.error
    );
    assert!(!app.macro_editor.open());
    let m = &app.loaded.as_ref().unwrap().music;
    assert_eq!(
        macro_values::read(&m.tracks[source]).unwrap().parameters[0].points[0].value,
        0.7
    );
    assert_eq!(app.patterns.editing.as_ref().unwrap().pattern, id);
    for (&id, p) in &Catalog::read(m).unwrap().patterns {
        assert_eq!(
            signature(body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap()),
            bodies[&id]
        );
    }
    app.undo_music().unwrap();
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
