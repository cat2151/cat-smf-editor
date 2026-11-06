use super::*;

#[test]
fn ctrl_enter_takes_the_repeating_measure_to_the_start_of_an_empty_song() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let before = song(&a);
    assert!(before.1.is_empty());
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let (patterns, placed) = song(&a);
    assert_eq!(patterns.len(), before.0.len() + 1);
    let id = *patterns.iter().find(|p| !before.0.contains(p)).unwrap();
    let quarter = u64::from(a.loaded.as_ref().unwrap().music.division);
    // The key was held a quarter of wall-clock time, plus the frames' own time.
    let (bars, notes) = bar(&a, id);
    assert_eq!((bars, notes.len(), notes[0].0, notes[0].2), (1, 1, 0, 60));
    assert!(
        (quarter..quarter + quarter / 4).contains(&notes[0].1),
        "{notes:?}"
    );
    assert_eq!(placed, vec![(1, id)]);
    // The keyboard stays open with the phrase stopped, and now plays the new container.
    assert!(!phrase::any_looping(session(&a)));
    assert!(!a.config.phrase_loop);
    assert!(session(&a).sounds.iter().all(|s| s.container.is_some()));
    a.undo_music().unwrap();
    assert_eq!(song(&a), before);
}

#[test]
fn to_song_appends_after_the_song_and_repeats_what_it_added() {
    use cat_smf_editor::score::scene::{arrangement, arrangement::PatternOrScene};
    let (mut a, track) = container();
    let ctx = egui::Context::default();
    a.edit_tracks(|m| {
        let id = cat_smf_editor::score::pattern::operations::create(m, track, None)?;
        let lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 1, Some(PatternOrScene::Pattern(id)))
    })
    .unwrap();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let before = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let (patterns, placed) = song(&a);
    assert_eq!(patterns.len(), before.0.len() + 1);
    let id = *patterns.iter().find(|p| !before.0.contains(p)).unwrap();
    assert_eq!(placed.len(), 2);
    assert!(placed.contains(&(2, id)));
    let m = &a.loaded.as_ref().unwrap().music;
    let range = (
        arrangement::measure_tick(m, None, 2).unwrap(),
        arrangement::measure_tick(m, None, 3).unwrap(),
    );
    assert_eq!(a.playback.repeat, Some(range));
    assert!(a.playback.pending.is_some());
    assert!(!phrase::any_looping(session(&a)));
    let cell = a.patterns.workspace.selection(Pane::Song);
    assert_eq!(
        (cell.measure, cell.choice),
        (2, Some(PatternOrScene::Pattern(id)))
    );
}

#[test]
fn ctrl_enter_again_after_to_song_changes_nothing() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let first = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    assert_eq!(song(&a), first);
}

#[test]
fn to_song_of_an_unchanged_loop_places_its_earlier_copy() {
    use cat_smf_editor::score::scene::arrangement::PatternOrScene;
    let (mut a, _) = container();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    let mut copy = 0;
    edit_music(&mut a, |m| {
        let track = cat_smf_editor::score::pattern::Catalog::read(m)
            .unwrap()
            .patterns[&id]
            .track;
        copy = cat_smf_editor::score::pattern::operations::create(m, track, Some(id)).unwrap();
    });
    let before = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    let (patterns, placed) = song(&a);
    assert_eq!(patterns, before.0);
    assert_eq!(placed, vec![(1, copy)]);
    assert_eq!(
        a.patterns.workspace.selection(Pane::Song).choice,
        Some(PatternOrScene::Pattern(copy))
    );
}

#[test]
fn ctrl_enter_does_nothing_until_the_phrase_repeats() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let before = song(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    press(&mut a, &ctx, Key::E, Modifiers::CTRL);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    assert_eq!(song(&a), before);
}

#[test]
fn to_song_places_an_ordinary_copy_and_the_loop_pattern_stays_shown() {
    use cat_smf_editor::score::pattern::{Catalog, Kind};
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    start_loop(&mut a, &ctx);
    let id = looping_pattern(&a);
    press(&mut a, &ctx, Key::Enter, Modifiers::CTRL);
    frame(&mut a, &ctx, vec![], true);
    assert!(a.patterns.recording.is_open());
    let (_, placed) = song(&a);
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(placed.len(), 1);
    assert_ne!(placed[0].1, id);
    assert_eq!(c.patterns[&placed[0].1].kind, Kind::Normal);
    assert_eq!(bar(&a, placed[0].1), bar(&a, id));
    assert_eq!(a.patterns.editing.as_ref().map(|e| e.pattern), Some(id));
}
