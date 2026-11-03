use super::*;
use cat_smf_editor::score::pattern::{
    event, record, recording::capture::TickNote, Definition, Kind, Record,
};

/// `k` held for `held`, then silence for `rest`.
fn tap(a: &mut App, ctx: &egui::Context, k: Key, held: Duration, rest: Duration) {
    frame(a, ctx, vec![key(k, true, Modifiers::NONE)], true);
    rewind(a, held);
    frame(a, ctx, vec![key(k, false, Modifiers::NONE)], true);
    rewind(a, rest);
}
/// Keys pressed in the Capture window.
fn capture_keys(a: &mut App, keys: &[(Key, Modifiers)]) {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let events: Vec<_> = keys.iter().map(|&(k, m)| key(k, true, m)).collect();
    super::super::capture::keys(a, state.session.as_mut().unwrap(), &events);
    a.patterns.recording = state;
}
fn listed(a: &App) -> Vec<Vec<TickNote>> {
    let (phrases, _, _) = super::super::capture::phrases(a, session(a)).unwrap();
    phrases.iter().map(|p| p.notes.clone()).collect()
}
/// Pressed and released, so the next press is not a repeat.
fn ctrl_j(a: &mut App, ctx: &egui::Context) {
    let j = |pressed| key(Key::J, pressed, Modifiers::CTRL);
    frame(a, ctx, vec![j(true), j(false)], true);
}
fn plain(k: Key) -> (Key, Modifiers) {
    (k, Modifiers::NONE)
}
fn normal(c: &Catalog) -> Vec<&Definition> {
    c.patterns
        .values()
        .filter(|p| p.kind == Kind::Normal)
        .collect()
}

#[test]
fn keys_played_without_rec_are_kept_listed_newest_first_and_taken_as_a_pattern() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::Z, ms(500), ms(3000));
    tap(&mut a, &ctx, Key::C, ms(250), ms(0));
    assert!(!recorder(&a).started());
    ctrl_j(&mut a, &ctx);
    let phrases = listed(&a);
    assert_eq!(
        phrases
            .iter()
            .map(|p| p.iter().map(|n| n.pitch).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![64], vec![60]]
    );
    // 500 ms at the keyboard's 120 BPM is a quarter.
    let held = phrases[1][0].length;
    let quarter = u64::from(a.loaded.as_ref().unwrap().music.division);
    assert!((quarter..quarter * 6 / 5).contains(&held), "{held}");

    capture_keys(&mut a, &[plain(Key::ArrowDown), plain(Key::Space)]);
    let (_, selected, auditioning) = super::super::capture::phrases(&a, session(&a)).unwrap();
    assert_eq!((selected, auditioning), (1, true));
    frame(&mut a, &ctx, vec![], true);
    // Listening wrote nothing.
    assert!(!a.undo.available());
    capture_keys(&mut a, &[plain(Key::Enter)]);
    let music = &a.loaded.as_ref().unwrap().music;
    let c = Catalog::read(music).unwrap();
    assert_eq!(normal(&c).len(), 1, "{:?}", session(&a).error);
    let id = a.patterns.selected.unwrap();
    let p = &c.patterns[&id];
    let b = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        b.iter()
            .filter(|e| e.status != 0xb0)
            .map(|e| (e.tick - p.tick == 0, e.status, e.data[0]))
            .collect::<Vec<_>>(),
        vec![(true, 0x90, 60), (false, 0x80, 60)]
    );
    // The CC1 the keyboard played comes along, starting with the note.
    assert_eq!(b[0].status, 0xb0);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert!(a.undo.available());
    assert!(a.patterns.recording.is_open());

    capture_keys(&mut a, &[plain(Key::Escape)]);
    assert!(super::super::capture::phrases(&a, session(&a)).is_none());
    // Closing after a take keeps the taken pattern open.
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
}

/// Body events but CC1 of the only Capture pattern as `(bar, status, first data byte)`.
fn captured(a: &App) -> Vec<(u64, u8, u8)> {
    let music = &a.loaded.as_ref().unwrap().music;
    let c = Catalog::read(music).unwrap();
    let mut captures = c.patterns.values().filter(|p| p.kind == Kind::Capture);
    let p = captures.next().expect("a Capture pattern");
    assert!(captures.next().is_none());
    let bar = u64::from(music.division) * 4;
    body::extract(&music.tracks[p.track].events, p.range().unwrap())
        .unwrap()
        .iter()
        .filter(|e| e.status != 0xb0)
        .map(|e| {
            let byte = if e.status == 0xff {
                e.meta_type
            } else {
                e.data[0]
            };
            ((e.tick - p.tick) / bar, e.status, byte)
        })
        .collect()
}

#[test]
fn two_phrases_go_to_one_capture_pattern_with_a_bar_between_and_no_undo_step() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::Z, ms(500), ms(3000));
    // The silence ends the first phrase before the next key.
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(captured(&a).len(), 3, "{:?}", session(&a).error);
    tap(&mut a, &ctx, Key::C, ms(250), ms(0));
    // Closing the keyboard puts the open phrase in the song.
    frame(
        &mut a,
        &ctx,
        vec![key(Key::Escape, true, Modifiers::SHIFT)],
        true,
    );
    assert!(!a.patterns.recording.is_open());
    assert_eq!(
        captured(&a),
        vec![
            (0, 0xff, 6),
            (0, 0x90, 60),
            (0, 0x80, 60),
            (2, 0xff, 6),
            (2, 0x90, 64),
            (2, 0x80, 64),
        ]
    );
    assert!(!a.undo.available());
    // The fallback sound's phrases share the container the first one created.
    assert_eq!(
        Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .sources
            .len(),
        1
    );
}

#[test]
fn a_renamed_capture_pattern_keeps_collecting_the_keys() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::Z, ms(100), ms(3000));
    frame(&mut a, &ctx, vec![], true);
    let music = &mut a.loaded.as_mut().unwrap().music;
    let c = Catalog::read(music).unwrap();
    let p = c.patterns.values().next().unwrap();
    let meta = music.tracks[p.track]
        .events
        .iter_mut()
        .find(|e| matches!(record(e), Ok(Some(Record::Pattern(_)))))
        .unwrap();
    let Ok(Some(Record::Pattern(mut r))) = record(meta) else {
        unreachable!()
    };
    r.name = "Ideas".into();
    *meta = event(meta.tick, &Record::Pattern(r));
    tap(&mut a, &ctx, Key::C, ms(100), ms(3000));
    frame(&mut a, &ctx, vec![], true);
    assert_eq!(
        captured(&a)
            .iter()
            .filter(|e| e.1 == 0x90)
            .map(|e| (e.0, e.2))
            .collect::<Vec<_>>(),
        vec![(0, 60), (2, 64)]
    );
}

/// Presses and releases the mouse at `at`.
fn click_at(a: &mut App, ctx: &egui::Context, at: egui::Pos2) {
    for pressed in [true, false] {
        let events = vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ];
        frame(a, ctx, events, true);
    }
}
/// The listening part and the pattern button of the row at `index`, as last drawn.
fn row(ctx: &egui::Context, index: usize) -> (egui::Rect, egui::Rect) {
    ctx.data(|d| d.get_temp(egui::Id::new(("capture-row", index))))
        .unwrap()
}

#[test]
fn a_click_auditions_its_row_and_the_rows_button_takes_it() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::Z, ms(500), ms(3000));
    tap(&mut a, &ctx, Key::C, ms(250), ms(0));
    ctrl_j(&mut a, &ctx);
    frame(&mut a, &ctx, vec![], true);

    click_at(&mut a, &ctx, row(&ctx, 1).0.center());
    let (_, selected, auditioning) = super::super::capture::phrases(&a, session(&a)).unwrap();
    assert_eq!((selected, auditioning), (1, true));
    assert!(!a.undo.available());

    click_at(&mut a, &ctx, row(&ctx, 0).1.center());
    let music = &a.loaded.as_ref().unwrap().music;
    let c = Catalog::read(music).unwrap();
    assert_eq!(normal(&c).len(), 1, "{:?}", session(&a).error);
    let p = &c.patterns[&a.patterns.selected.unwrap()];
    let b = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    // Row 0 is the newest phrase, the C key; its notes are the listed ones.
    let listed = &listed(&a)[0];
    assert_eq!(
        b.iter()
            .filter(|e| e.status != 0xb0)
            .map(|e| (e.tick - p.tick, e.status, e.data[0]))
            .collect::<Vec<_>>(),
        listed
            .iter()
            .flat_map(|n| [
                (n.start, 0x90, n.pitch),
                (n.start + n.length, 0x80, n.pitch)
            ])
            .collect::<Vec<_>>()
    );
    assert_eq!(listed.iter().map(|n| n.pitch).collect::<Vec<_>>(), vec![64]);
    // No marker is copied.
    assert!(b.iter().all(|e| e.status != 0xff));
}

/// Starts the prepared playback and reports where it sounds `seconds` in.
fn sounding_at(a: &mut App, seconds: f64) -> u64 {
    crate::play::transport::advance(a);
    a.audition.set_test_seconds(seconds);
    crate::play::transport::tick_for(a, a.track).expect("playing")
}

#[test]
fn a_clicked_phrase_shows_its_capture_pattern_read_only_and_jumps_play_from_the_bar() {
    let mut a = app();
    let ctx = egui::Context::default();
    let before = (a.track, a.note_mode, a.patterns.editing.clone());
    open_key(&mut a, &ctx);
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::Z, ms(500), ms(3000));
    // The first phrase makes the container the second one joins.
    frame(&mut a, &ctx, vec![], true);
    tap(&mut a, &ctx, Key::C, ms(250), ms(0));
    ctrl_j(&mut a, &ctx);
    frame(&mut a, &ctx, vec![], true);
    let encode = |a: &App| cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music);
    let song = encode(&a).unwrap();
    let bar = u64::from(a.loaded.as_ref().unwrap().music.division) * 4;

    // Row 0 is the newest phrase, two bars after the first.
    click_at(&mut a, &ctx, row(&ctx, 0).0.center());
    let (phrases, _, auditioning) = super::super::capture::phrases(&a, session(&a)).unwrap();
    let newest = phrases[0].clone();
    assert!(auditioning, "{:?}", session(&a).error);
    assert_eq!(newest.range.0, 2 * bar);
    let edit = a.patterns.editing.clone().unwrap();
    assert!(edit.standalone);
    assert_eq!(edit.pattern, newest.pattern);
    assert!(a.loaded.as_ref().unwrap().display_music.is_some());
    assert_eq!(crate::play::playback::location(&a).1.unwrap().measure, 3);
    assert_eq!(a.patterns.preview, Some(newest.pattern));
    let at = sounding_at(&mut a, 0.01);
    assert!((2 * bar..3 * bar).contains(&at), "{at}");

    // Read-only: the note editors refuse it and the song stays as it was.
    assert!(!a.can_edit_notes(a.track));
    let track = a.track;
    let edited = a.edit_music(|m| {
        m.tracks[track].events.retain(|e| e.status & 0xf0 != 0x90);
        Ok(())
    });
    assert!(edited.is_err());
    crate::view::mml_input::open(&mut a, &ctx, false);
    assert!(!a.mml_input.open);
    assert_eq!(encode(&a).unwrap(), song);

    // The older phrase from bar 1; L jumps the cursor and the play to bar 2.
    click_at(&mut a, &ctx, row(&ctx, 1).0.center());
    assert_eq!(crate::play::playback::location(&a).1.unwrap().measure, 1);
    let at = sounding_at(&mut a, 0.01);
    assert!((0..bar).contains(&at), "{at}");
    crate::cursor::moves::boundary(&mut a, true, true);
    assert_eq!(crate::play::playback::location(&a).1.unwrap().measure, 2);
    let at = sounding_at(&mut a, 0.01);
    assert!((bar..2 * bar).contains(&at), "{at}");

    // A staff click while the phrase plays seeks both there.
    click_at(&mut a, &ctx, row(&ctx, 1).0.center());
    sounding_at(&mut a, 0.01);
    crate::play::transport::timeline::seek(&mut a, track, 2 * bar + bar / 8);
    assert_eq!(crate::play::playback::location(&a).1.unwrap().measure, 3);
    let at = sounding_at(&mut a, 0.01);
    assert!((2 * bar + bar / 8..3 * bar).contains(&at), "{at}");

    // Closing the window stops the play and shows the song again.
    capture_keys(&mut a, &[plain(Key::Escape)]);
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!((a.track, a.note_mode, a.patterns.editing.clone()), before);
    assert_eq!(encode(&a).unwrap(), song);
    assert!(!a.undo.available());
}

fn track_name(a: &App, track: usize) -> Option<Vec<u8>> {
    a.loaded.as_ref().unwrap().music.tracks[track]
        .events
        .iter()
        .find(|e| e.status == 0xff && e.meta_type == 3)
        .map(|e| e.data.clone())
}
/// One phrase on Z, ended by the silence and the keyboard closed.
fn play_one_phrase(a: &mut App, ctx: &egui::Context) {
    open_key(a, ctx);
    let ms = Duration::from_millis;
    tap(a, ctx, Key::Z, ms(500), ms(3000));
    frame(a, ctx, vec![], true);
    frame(a, ctx, vec![key(Key::Escape, true, Modifiers::SHIFT)], true);
    assert!(!a.patterns.recording.is_open());
}

#[test]
fn undoing_an_edit_made_before_the_keys_keeps_their_capture_and_its_new_container() {
    let mut a = app();
    let ctx = egui::Context::default();
    a.edit_tracks(|m| edit::tracks::rename(&mut m.tracks[0], "Edited"))
        .unwrap();
    play_one_phrase(&mut a, &ctx);
    let played = captured(&a);
    assert_eq!(played.iter().filter(|e| e.1 == 0x90).count(), 1);

    frame(&mut a, &ctx, vec![key(Key::Z, true, Modifiers::CTRL)], true);
    assert_ne!(track_name(&a, 0), Some(b"Edited".to_vec()));
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(c.patterns.len(), 1);
    assert_eq!(c.sources.len(), 1);
    assert_eq!(captured(&a), played);
    assert!(!a.undo.available());
    // The Capture window still lists the phrase.
    open_key(&mut a, &ctx);
    ctrl_j(&mut a, &ctx);
    let phrases = listed(&a);
    assert_eq!(phrases.len(), 1);
    assert_eq!(
        phrases[0].iter().map(|n| n.pitch).collect::<Vec<_>>(),
        vec![60]
    );
}

#[test]
fn each_undo_before_the_keys_still_takes_one_step_and_keeps_the_capture() {
    let mut a = app();
    let ctx = egui::Context::default();
    a.edit_tracks(|m| {
        let source = cat_smf_editor::score::pattern::operations::create_source(m)?;
        cat_smf_editor::score::pattern::operations::create(m, source, None)?;
        Ok(())
    })
    .unwrap();
    let source = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .sources
        .into_iter()
        .next()
        .unwrap();
    a.edit_tracks(|m| edit::tracks::rename(&mut m.tracks[source], "Edited"))
        .unwrap();
    play_one_phrase(&mut a, &ctx);
    let played = captured(&a);

    a.undo_music().unwrap();
    assert_ne!(track_name(&a, source), Some(b"Edited".to_vec()));
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(normal(&c).len(), 1);
    assert_eq!(captured(&a), played);

    // Undoing the container's creation brings it back for the Capture alone.
    a.undo_music().unwrap();
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert!(normal(&c).is_empty());
    assert_eq!(c.sources.len(), 1);
    assert_eq!(captured(&a), played);
    assert!(!a.undo.available());
}

#[test]
fn a_phrase_added_to_an_existing_capture_keeps_the_song_and_is_listed() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    let ms = Duration::from_millis;
    tap(&mut a, &ctx, Key::Z, ms(500), ms(3000));
    frame(&mut a, &ctx, vec![], true);
    ctrl_j(&mut a, &ctx);
    assert_eq!(listed(&a).len(), 1);
    let song = a.song_id;
    tap(&mut a, &ctx, Key::C, ms(250), ms(3000));
    frame(&mut a, &ctx, vec![], true);
    frame(&mut a, &ctx, vec![], true);
    // A browser opened on a track before the phrase still applies there.
    assert_eq!(a.song_id, song);
    assert_eq!(listed(&a).len(), 2, "{:?}", session(&a).error);
}
