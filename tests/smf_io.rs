use cat_smf_editor::{score::catalog, smf_io};

#[test]
fn corpus_write_read_preserves_all_tracks_ticks_and_event_payloads() {
    for entry in catalog::discover(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cat-music-patterns"),
    )
    .unwrap()
    {
        let source = smf_io::load(&entry.path).unwrap();
        let bytes = smf_io::encode(&source).unwrap();
        let saved = cat_smf_editor::ffi::parse(&bytes).unwrap();
        assert_eq!(
            (source.format, source.division, source.tracks.len()),
            (saved.format, saved.division, saved.tracks.len())
        );
        for (before, after) in source.tracks.iter().zip(&saved.tracks) {
            // The writer only adds an End of Track if the source omits it.
            assert!(after.events.len() >= before.events.len());
            for (before, after) in before.events.iter().zip(&after.events) {
                assert_eq!(
                    (
                        before.tick,
                        before.delta,
                        before.status,
                        before.meta_type,
                        &before.data
                    ),
                    (
                        after.tick,
                        after.delta,
                        after.status,
                        after.meta_type,
                        &after.data
                    ),
                    "{}",
                    entry.path.display()
                );
            }
        }
    }
}
