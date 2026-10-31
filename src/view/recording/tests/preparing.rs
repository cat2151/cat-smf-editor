use super::*;
use cat_smf_editor::score::pattern::recording::{Layout, Mode, Range, Scale, Start};
use setup::Item;

fn press(a: &mut App, ctx: &egui::Context, keys: &[(Key, Modifiers)]) {
    for &(k, m) in keys {
        frame(a, ctx, vec![key(k, true, m), key(k, false, m)], true);
    }
}
fn ctrl(k: Key) -> (Key, Modifiers) {
    (k, Modifiers::CTRL)
}
fn back(k: Key) -> (Key, Modifiers) {
    (k, Modifiers::CTRL | Modifiers::SHIFT)
}
const DOWN: (Key, Modifiers) = (Key::ArrowDown, Modifiers::NONE);
const UP: (Key, Modifiers) = (Key::ArrowUp, Modifiers::NONE);
const RIGHT: (Key, Modifiers) = (Key::ArrowRight, Modifiers::NONE);
const LEFT: (Key, Modifiers) = (Key::ArrowLeft, Modifiers::NONE);
const BIG: (Key, Modifiers) = (Key::ArrowRight, Modifiers::SHIFT);

#[test]
fn every_item_changes_in_the_settings_window_while_the_pads_play() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert!(!session(&a).settings_open);
    press(&mut a, &ctx, &[ctrl(Key::Comma)]);
    assert!(session(&a).settings_open);
    let keys = |a: &mut App, _: &egui::Context, keys: &[(Key, Modifiers)]| settings_keys(a, keys);
    let press = keys;
    // One container: the container row is skipped, and Free skips the measure count.
    assert_eq!(session(&a).item, Item::Range);
    press(&mut a, &ctx, &[DOWN, DOWN]);
    assert_eq!(session(&a).item, Item::Bpm);
    press(&mut a, &ctx, &[UP, UP, UP]);
    assert_eq!(session(&a).item, Item::Range);
    press(&mut a, &ctx, &[DOWN, RIGHT]);
    assert_eq!(recorder(&a).settings().start, Start::Key);
    press(&mut a, &ctx, &[UP, RIGHT, DOWN, DOWN, RIGHT, BIG]);
    assert_eq!(recorder(&a).settings().range, Range::Loop { measures: 9 });
    press(&mut a, &ctx, &[DOWN, RIGHT, BIG]);
    assert_eq!(setup::bpm(recorder(&a).settings().tempo), 131);
    press(&mut a, &ctx, &[DOWN, RIGHT]);
    assert_eq!(recorder(&a).settings().layout, Layout::Letters);
    // Letters ignore every pad setting except the octave of the note base.
    press(&mut a, &ctx, &[DOWN]);
    assert_eq!(session(&a).item, Item::NoteBase);
    press(&mut a, &ctx, &[RIGHT]);
    assert_eq!(recorder(&a).settings().note_base, 72);
    press(&mut a, &ctx, &[LEFT, UP, LEFT]);
    assert_eq!(recorder(&a).settings().layout, Layout::Rows);
    press(
        &mut a,
        &ctx,
        &[
            DOWN, RIGHT, RIGHT, RIGHT, RIGHT, DOWN, RIGHT, DOWN, RIGHT, DOWN, RIGHT, DOWN, RIGHT,
            BIG, DOWN,
        ],
    );
    let t = *recorder(&a).settings();
    assert_eq!(session(&a).item, Item::ChordBase);
    assert_eq!(t.mode, Mode::SeventhInversions);
    assert_eq!((t.key().tonic, t.key().scale), (1, Scale::MinorPentatonic));
    // C# minor pentatonic above C#4, then the natural minor degree above C#3 and an octave.
    assert_eq!(t.note_base, 64);
    assert_eq!(t.chord_base, 51 + 12);
    press(&mut a, &ctx, &[(Key::Escape, Modifiers::NONE)]);
    assert!(!session(&a).settings_open && a.patterns.recording.is_open());
    let press = |a: &mut App, ctx: &egui::Context, keys: &[(Key, Modifiers)]| {
        for &(k, m) in keys {
            frame(a, ctx, vec![key(k, true, m), key(k, false, m)], true);
        }
    };
    press(&mut a, &ctx, &[(Key::Escape, Modifiers::NONE)]);
    assert!(!a.patterns.recording.is_open());
    assert!(!a.undo.available());
    // Reopening keeps the choices but rereads the key from the song.
    open_key(&mut a, &ctx);
    let kept = *recorder(&a).settings();
    assert_eq!(
        (kept.key().tonic, kept.key().scale),
        (0, Scale::MajorPentatonic)
    );
    assert_eq!(
        (kept.range, kept.start, kept.tempo, kept.mode),
        (t.range, Start::Key, t.tempo, t.mode)
    );
    assert_eq!((kept.note_base, kept.chord_base), (60, 60));
    assert_eq!(session(&a).measures, 9);
}

fn with_key(data: Option<[u8; 2]>) -> App {
    let mut a = app();
    let conductor = &mut a.loaded.as_mut().unwrap().music.tracks[0];
    conductor
        .events
        .retain(|e| !(e.status == 0xff && e.meta_type == 0x59));
    if let Some(data) = data {
        edit::insert(
            conductor,
            cat_smf_editor::model::Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x59,
                data: data.to_vec(),
            },
        );
    }
    edit::refresh_deltas(conductor).unwrap();
    a
}

#[test]
fn opening_reads_the_song_key_or_falls_back_to_c_major() {
    for (data, tonic, scale) in [
        (Some([0, 1]), 9, Scale::MinorPentatonic),
        (None, 0, Scale::MajorPentatonic),
    ] {
        let mut a = with_key(data);
        let ctx = egui::Context::default();
        open_key(&mut a, &ctx);
        let t = *recorder(&a).settings();
        assert_eq!((t.key().tonic, t.key().scale), (tonic, scale));
        assert_eq!((t.note_base, t.chord_base), (60 + tonic, 48 + tonic));
    }
}

/// A source container whose patch is the drum sequencer, as the current track.
fn drum_app() -> App {
    let mut a = app();
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let track = a.loaded.as_ref().unwrap().music.tracks.len() - 1;
    a.loaded.as_mut().unwrap().patches[track].drums = Some(Default::default());
    a.track = track;
    a
}

#[test]
fn a_drum_container_plays_one_lane_per_row_and_records_into_itself() {
    let mut a = drum_app();
    let track = a.track;
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert!(session(&a).error.is_none() && !session(&a).settings_open);
    assert!(recorder(&a).settings().drums);
    for item in [Item::Mode, Item::Tonic, Item::NoteBase, Item::ChordBase] {
        assert!(!setup::enabled(session(&a), item), "{item:?}");
    }
    let rows = [Key::Z, Key::X, Key::A, Key::Q, Key::Num1];
    let down = |pressed| {
        rows.iter()
            .map(|&k| key(k, pressed, Modifiers::NONE))
            .collect()
    };
    frame(
        &mut a,
        &ctx,
        [vec![key(Key::Space, true, Modifiers::NONE)], down(true)].concat(),
        true,
    );
    let sounding: Vec<u8> = (0..128).filter(|&p| recorder(&a).sounding(p)).collect();
    assert_eq!(sounding, [36, 38, 39, 42]);
    // Octave keys leave the lanes where they are.
    press(&mut a, &ctx, &[UP]);
    a.patterns.recording.session.as_mut().unwrap().clock -= Duration::from_millis(500);
    frame(&mut a, &ctx, down(false), true);
    press(&mut a, &ctx, &[(Key::Escape, Modifiers::NONE)]);
    assert!(!a.patterns.recording.is_open(), "{:?}", a.load_error);
    let music = &a.loaded.as_ref().unwrap().music;
    let c = Catalog::read(music).unwrap();
    let p = &c.patterns[&a.patterns.selected.unwrap()];
    assert_eq!(c.tracks[&p.container], track);
    let b = body::extract(&music.tracks[p.track].events, p.range().unwrap()).unwrap();
    let mut on: Vec<u8> = b
        .iter()
        .filter(|e| e.status == 0x90)
        .map(|e| e.data[0])
        .collect();
    on.sort_unstable();
    assert_eq!(on, [36, 38, 39, 42]);
}

#[test]
fn shortcuts_change_items_from_the_pads_until_a_take_fixes_them() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    press(
        &mut a,
        &ctx,
        &[
            (Key::Tab, Modifiers::SHIFT),
            (Key::Tab, Modifiers::SHIFT),
            back(Key::K),
            ctrl(Key::M),
            ctrl(Key::N),
            back(Key::H),
            ctrl(Key::R),
            ctrl(Key::D),
            back(Key::B),
            ctrl(Key::S),
            (Key::Tab, Modifiers::NONE),
        ],
    );
    let t = *recorder(&a).settings();
    assert_eq!((t.key().tonic, t.key().scale), (11, Scale::MinorPentatonic));
    assert_eq!(t.range, Range::Loop { measures: 5 });
    assert_eq!(setup::bpm(t.tempo), 119);
    // Shift+Tab twice steps back to Triad inversions, Tab forward to Seventh inversions.
    assert_eq!((t.start, t.mode), (Start::Key, Mode::SeventhInversions));
    // B minor pentatonic from B4 one step up; the natural minor degree below B3.
    assert_eq!((t.note_base, t.chord_base), (74, 57));
    // Armed, the tempo and range belong to the take.
    press(&mut a, &ctx, &[(Key::Space, Modifiers::NONE)]);
    press(
        &mut a,
        &ctx,
        &[ctrl(Key::B), ctrl(Key::R), (Key::Tab, Modifiers::NONE)],
    );
    let armed = *recorder(&a).settings();
    assert_eq!((armed.tempo, armed.range), (t.tempo, t.range));
    assert_eq!(armed.mode, Mode::Note);
    // Pads do not sound with Ctrl.
    assert!(!(0..128).any(|p| recorder(&a).sounding(p)));
}

#[test]
fn window_keys_toggle_the_companions_and_raise_help() {
    let mut a = app();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    assert!(session(&a).piano_open && !session(&a).settings_open);
    press(&mut a, &ctx, &[ctrl(Key::Period), ctrl(Key::Comma)]);
    assert!(!session(&a).piano_open && session(&a).settings_open);
    for k in [Key::F1, Key::Questionmark] {
        a.help_open = false;
        press(&mut a, &ctx, &[(k, Modifiers::NONE)]);
        assert!(a.help_open);
        // A second press keeps it open and lifts it again.
        press(&mut a, &ctx, &[(k, Modifiers::SHIFT)]);
        assert!(a.help_open && a.help.raise);
    }
    settings_keys(&mut a, &[(Key::F1, Modifiers::NONE), ctrl(Key::Comma)]);
    assert!(!session(&a).settings_open);
}
