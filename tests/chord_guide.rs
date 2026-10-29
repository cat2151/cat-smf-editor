use cat_smf_editor::{
    edit::{self, meta_input::Kind},
    score::chord_guide as guide,
};

#[test]
fn renamed_guide_and_chords_round_trip_without_touching_other_tracks() {
    let mut music = edit::tracks::new_song();
    let before = cat_smf_editor::smf_io::encode(&music).unwrap();
    guide::add(&mut music).unwrap();
    assert!(guide::add(&mut music).is_err());
    edit::tracks::rename(&mut music.tracks[1], "Harmony").unwrap();
    for event in [
        Kind::KeySignature.event(0, "C major").unwrap(),
        guide::event(0, "Isus4").unwrap(),
        guide::event(1920, "I").unwrap(),
    ] {
        edit::insert(&mut music.tracks[1], event);
    }
    edit::refresh_deltas(&mut music.tracks[1]).unwrap();
    let bytes = cat_smf_editor::smf_io::encode(&music).unwrap();
    let restored = cat_smf_editor::ffi::parse(&bytes).unwrap();
    assert_eq!(guide::track(&restored), Some(1));
    assert_eq!(
        guide::chords(&restored)
            .map(|e| (e.tick, guide::chord(e).unwrap()))
            .collect::<Vec<_>>(),
        [(0, "Isus4"), (1920, "I")]
    );
    music.tracks.pop();
    assert_eq!(cat_smf_editor::smf_io::encode(&music).unwrap(), before);
}

#[test]
fn out_of_scope_formats_are_not_reinterpreted() {
    for (format, division) in [(0, 480), (2, 480), (1, 0xe728)] {
        let mut music = edit::tracks::new_song();
        music.format = format;
        music.division = division;
        assert!(guide::add(&mut music).is_err());
        assert!(guide::track(&music).is_none());
    }
    for invalid in ["", "  ", "I\nV", "I\rV", "I\tV"] {
        assert!(guide::event(0, invalid).is_err());
    }
}
