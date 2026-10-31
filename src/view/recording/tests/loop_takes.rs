use super::*;
use cat_smf_editor::score::pattern::recording::{Pad, Range, Recorder, Settings};

#[test]
fn every_loop_take_is_created_by_one_edit_and_removed_by_one_undo() {
    let mut a = app();
    let original = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let mut settings = Settings::new(
        cat_smf_editor::score::pattern::recording::Key::from_signature(None),
        500_000,
    );
    settings.range = Range::Loop { measures: 1 };
    let mut r = Recorder::new(a.loaded.as_ref().unwrap().music.division, settings).unwrap();
    r.start(Duration::ZERO);
    for lap in 0..3u64 {
        let pad = Pad::Row {
            row: 0,
            column: lap as u8,
        };
        r.press(pad, Duration::from_millis(2000 * lap)).unwrap();
        r.release(pad, Duration::from_millis(2000 * lap + 500))
            .unwrap();
    }
    let mut ids = vec![];
    a.edit_tracks(|music| {
        ids = r.commit(music, None)?;
        Ok(())
    })
    .unwrap();
    assert_eq!(ids.len(), 3);
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert!(ids.iter().all(|id| c.patterns.contains_key(id)));
    a.undo_music().unwrap();
    assert_eq!(
        original,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!a.undo.available());
}
