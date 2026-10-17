use cat_smf_editor::{
    model::{Event, Track},
    score::instrument::{self, Patch},
};
#[test]
fn plain_smf_uses_default_and_legacy_names_are_not_restoration_keys() {
    let mut track = Track {
        events: vec![Event {
            delta: 0,
            tick: 0,
            status: 0xff,
            meta_type: 9,
            data: b"other device".to_vec(),
        }],
    };
    let roots = &mut cat_smf_editor::patch_roots::Table::default();
    assert_eq!(instrument::read(&track, roots).unwrap(), Patch::default());
    instrument::write(&mut track, &Patch::default(), roots).unwrap();
    assert_eq!(track.events[1].data, b"other device");
    assert_eq!(instrument::read(&track, roots).unwrap(), Patch::default());
}
#[test]
fn invalid_write_does_not_mutate_track() {
    let mut track = Track { events: vec![] };
    assert!(instrument::write(
        &mut track,
        &Patch {
            layers: vec![cat_smf_editor::score::instrument::ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    volume_db: f32::NAN,
                    ..Default::default()
                },
                instrument: None
            }],
            ..Patch::default()
        },
        &mut cat_smf_editor::patch_roots::Table::default(),
    )
    .is_err());
    assert!(track.events.is_empty());
}
