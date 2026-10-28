use super::*;
#[test]
fn independent_duplicate_displays_only_its_measure_and_ordinary_note_rows_keep_source_indices() {
    let (mut a, _, id) = edit_tests::fixture();
    library::create(&mut a, Some(id)).unwrap();
    let edit = a.patterns.editing.as_ref().unwrap();
    assert_eq!(edit.measure, 1);
    let l = a.loaded.as_ref().unwrap();
    let events = &l.visible().tracks[a.track].events;
    let rows = &l.display_rows[a.track];
    assert!(!rows.is_empty());
    for i in 0..rows.len() {
        let p = match rows.get(i) {
            cat_smf_editor::score::display_rows::DisplayRow::Event(e) => {
                assert_eq!(rows.event_row(e), Some(i));
                l.time.map(a.track).unwrap().position(events[e].tick)
            }
            cat_smf_editor::score::display_rows::DisplayRow::Beat(p) => p,
        };
        assert_eq!(p.measure, edit.measure);
    }
    assert_eq!(rows.events_in_rows(0..rows.len()).count(), 1);
}

#[test]
fn independent_edit_audition_uses_container_without_touching_other_tracks() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    let normal = a.loaded.as_ref().unwrap().patches[0].clone();
    a.loaded.as_mut().unwrap().patches[source].layers[0]
        .settings
        .octave = 2;
    let (track, sound) = preview::edit_sound(&a, a.track).unwrap();
    assert_eq!(track, source);
    assert_eq!(sound.layers[0].settings.octave, 2);
    assert_eq!(a.loaded.as_ref().unwrap().patches[0], normal);
}
