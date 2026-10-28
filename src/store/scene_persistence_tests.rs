//! The verification score is entered through the actual three-window UI.
use crate::store::{config::Store, saved::Document};
use cat_smf_editor::score::{pattern::Catalog, scene};
#[test]
fn arrangement_save_load_two_startups_preserve_lanes_sound_and_shared_bodies() {
    let root = tempfile::tempdir().unwrap();
    let mid = root.path().join("arrangement.mid");
    let (mut a, c, first, _, _) = crate::view::patterns::workspace_tests::verification_song();
    let _ = c;
    a.config = Store::open(root.path().join("status.json"));
    let music = &a.loaded.as_ref().unwrap().music;
    let collection = scene::read(music).unwrap();
    let sound = Catalog::read(music).unwrap().patterns[&first].container;
    let expected = cat_smf_editor::smf_io::encode(music).unwrap();
    a.saved = Document::untitled(Some(root.path().to_owned()), music).unwrap();
    a.saved.save_as(&mid).unwrap();
    a.sync_saved_path();
    a.save_session();
    a.config.persist();
    let parsed = Document::new(None).load(&mid).unwrap();
    assert_eq!(scene::read(&parsed).unwrap(), collection);
    assert_eq!(cat_smf_editor::smf_io::encode(&parsed).unwrap(), expected);
    for _ in 0..2 {
        let mut restored = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        restored.loaded = None;
        restored.config = Store::open(root.path().join("status.json"));
        restored.saved = Document::new(Some(root.path().to_owned()));
        restored.open_initial(None);
        assert!(restored.load_error.is_none(), "{:?}", restored.load_error);
        let music = &restored.loaded.as_ref().unwrap().music;
        assert_eq!(cat_smf_editor::smf_io::encode(music).unwrap(), expected);
        assert_eq!(scene::read(music).unwrap(), collection);
        let c = Catalog::read(music).unwrap();
        assert_eq!(c.patterns[&first].container, sound);
        assert!(c.sources.contains(&c.tracks[&sound]));
        let p = crate::play::arrangement::prepare(&restored, None).unwrap();
        assert_eq!(p.active.end, 6.0);
        assert_eq!(p.parts.iter().map(|p| p.2.notes.len()).sum::<usize>(), 3);
        assert_eq!(
            crate::view::patterns::preview::sound(&restored, first)
                .unwrap()
                .1,
            restored.loaded.as_ref().unwrap().patches[c.tracks[&sound]]
        );
        restored.save_session();
    }
    assert_eq!(std::fs::read(mid).unwrap(), expected);
}
