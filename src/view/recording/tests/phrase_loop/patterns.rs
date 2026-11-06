use super::*;

#[test]
fn the_repeating_measure_is_the_loop_pattern_shown_behind_the_keyboard() {
    use cat_smf_editor::score::{
        pattern::Catalog, pattern::Kind, scene::arrangement::PatternOrScene,
    };
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    a.edit_tracks(|m| {
        cat_smf_editor::score::pattern::operations::create(m, track, None).map(|_| ())
    })
    .unwrap();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        (c.patterns[&id].kind, c.patterns[&id].track),
        (Kind::Loop, track)
    );
    assert_eq!(a.patterns.editing.as_ref().map(|e| e.pattern), Some(id));
    assert_eq!(a.track, track);
    let song = a.patterns.workspace.selection(Pane::Song).choice;
    assert_eq!(song, Some(PatternOrScene::Pattern(id)));
    // The Loop leads its container's patterns in the Song window's list.
    let listed = crate::view::patterns::workspace::choices(&a, Pane::Song);
    assert_eq!(listed[0].0, PatternOrScene::Pattern(id));
    assert_eq!(listed.len(), 2);
}

#[test]
fn an_edit_of_the_loop_pattern_sounds_from_the_next_lap() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let start = start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    edit_loop(&mut a, 64);
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(looping_pattern(&a), id);
    let lap = measure(&a);
    assert!(due(&a, start + lap * 3 - Duration::from_millis(1)).is_empty());
    assert_eq!(
        due(&a, start + lap * 3 + Duration::from_millis(1)),
        vec![word(64, 100)]
    );
}

#[test]
fn deleting_the_loop_pattern_stops_the_phrase_and_the_switch_waits() {
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let start = start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    edit_music(&mut a, |m| {
        cat_smf_editor::score::pattern::operations::delete(m, id).unwrap()
    });
    frame(&mut a, &ctx, vec![], true);
    assert!(matches!(session(&a).sound().looper, Looper::Waiting));
    assert!(due(&a, start + measure(&a) * 9).is_empty());
}

#[test]
fn deleting_another_pattern_keeps_the_keyboard_and_its_phrase() {
    use cat_smf_editor::score::pattern::{operations, Catalog};
    let (mut a, track) = container();
    let mut other = 0;
    a.edit_tracks(|m| {
        other = operations::create(m, track, None)?;
        Ok(())
    })
    .unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let Looper::Looping { pattern, .. } = session(&a).sound().looper else {
        unreachable!()
    };
    crate::view::patterns::workspace::mutate(&mut a, |m| operations::delete(m, other)).unwrap();
    frame(&mut a, &ctx, vec![], true);
    assert!(a.patterns.recording.is_open());
    assert!(phrase::looping(session(&a)));
    let music = &a.loaded.as_ref().unwrap().music;
    assert!(Catalog::read(music)
        .unwrap()
        .patterns
        .contains_key(&pattern));
}

#[test]
fn the_keyboard_follows_its_container_to_its_new_track() {
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    a.edit_tracks(|m| {
        m.tracks
            .insert(track, cat_smf_editor::model::Track { events: vec![] });
        Ok(())
    })
    .unwrap();
    frame(&mut a, &ctx, vec![], true);
    assert!(phrase::looping(session(&a)));
    assert_eq!(session(&a).sound().track, track + 1);
}

#[test]
fn removing_the_playing_container_closes_the_keyboard() {
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    a.edit_tracks(|m| {
        m.tracks.remove(track);
        Ok(())
    })
    .unwrap();
    frame(&mut a, &ctx, vec![], true);
    assert!(!a.patterns.recording.is_open());
}
