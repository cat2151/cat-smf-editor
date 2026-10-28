use super::*;
use cat_smf_editor::score::pattern::Catalog;
#[test]
fn actual_tracks_keep_order_and_selection_in_both_modes_after_roundtrip() {
    let (a, source, id) = edit_tests::fixture();
    let original = a.loaded.as_ref().unwrap().music.clone();
    let bytes = cat_smf_editor::smf_io::encode(&original).unwrap();
    let music = cat_smf_editor::ffi::parse(&bytes).unwrap();
    assert_eq!(music.tracks[source].name().as_deref(), Some("Init Saw"));
    let c = Catalog::read(&music).unwrap();
    assert_eq!(c.patterns[&id].track, source);
    let mut a = crate::view::tests::app(music);
    a.loaded.as_mut().unwrap().refresh();
    for notes in [true, false] {
        a.note_mode = notes;
        for track in 0..original.tracks.len() {
            crate::cursor::split::go(&mut a, track, 0);
            assert_eq!(a.track, track);
            assert_eq!(
                cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
                bytes
            );
            assert!(a.loaded.as_ref().unwrap().track_titles[track]
                .starts_with(&format!("{} ", track + 1)));
        }
    }
}
