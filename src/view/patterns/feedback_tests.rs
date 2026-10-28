use super::*;
use crate::view::palette::tests::app;
use cat_smf_editor::model::Event;
use eframe::egui;
/// Runs a palette command; an empty song first gets one sound container to confirm.
fn command(a: &mut App, c: &egui::Context, s: &str) {
    use crate::view::palette::tests as palette;
    if cat_smf_editor::score::pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .sources
        .is_empty()
    {
        a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
            .unwrap();
    }
    palette::open(a, c);
    palette::query(a, c, s);
    palette::press(a, c, eframe::egui::Key::Enter);
    if a.patterns.confirming_creation() {
        palette::press(a, c, eframe::egui::Key::Enter);
    }
}
#[test]
fn standalone_create_edits_only_body_and_preserves_normal_song() {
    let mut a = app();
    let c = egui::Context::default();
    let original = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    command(&mut a, &c, "create pattern");
    assert!(!a.palette.is_open());
    let edit = a.patterns.editing.as_ref().unwrap().clone();
    assert!(edit.standalone);
    let empty =
        crate::play::transport::prepare(&a, crate::play::playback::Mode::AllTracks).unwrap();
    assert_eq!(empty.parts.len(), 1);
    assert!(empty.parts[0].2.notes.is_empty());
    assert_eq!(empty.parts[0].1, Default::default());
    a.edit_music(|m| {
        for (tick, status) in [(120, 0x90), (600, 0x80)] {
            cat_smf_editor::edit::insert(
                &mut m.tracks[edit.track],
                Event {
                    tick,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![71, 99],
                },
            );
        }
        cat_smf_editor::edit::refresh_deltas(&mut m.tracks[edit.track])
    })
    .unwrap();
    let played =
        crate::play::transport::prepare(&a, crate::play::playback::Mode::AllTracks).unwrap();
    assert_eq!(played.parts.len(), 1);
    assert_eq!(played.parts[0].2.notes.len(), 1);
    assert_eq!(played.parts[0].2.notes[0].pitch, 71);
    let mut song = a.loaded.as_ref().unwrap().music.clone();
    song.tracks.truncate(3);
    assert_eq!(cat_smf_editor::smf_io::encode(&song).unwrap(), original);
}
#[test]
fn independent_copy_rebuilds_only_its_body_and_keeps_edit_context_on_undo() {
    let (mut a, _, id) = edit_tests::fixture();
    library::create(&mut a, Some(id)).unwrap();
    let track = a.track;
    let context = a.patterns.editing.clone().unwrap();
    assert_eq!(context.measure, 1);
    crate::play::playback::start(&mut a, crate::play::playback::Mode::AllTracks);
    let before = &a.playback.pending.as_ref().unwrap().parts;
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].2.notes[0].pitch, 67);
    a.edit_music(|m| cat_smf_editor::edit::note_value::set_pitch(&mut m.tracks[track], 0, 72))
        .unwrap();
    let after = &a.playback.pending.as_ref().unwrap().parts;
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].2.notes[0].pitch, 72);
    a.undo_music().unwrap();
    assert!(a.patterns.editing.as_ref().unwrap().standalone);
    assert_eq!(
        a.loaded.as_ref().unwrap().visible().tracks[track].events[0].data[0],
        67
    );
    assert_eq!(
        preview::prepare(&a, id).unwrap().parts[0].2.notes[0].pitch,
        67
    );
}
