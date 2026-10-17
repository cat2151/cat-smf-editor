use crate::store::saved::Document;
use cat_smf_editor::{
    model::{Event, Music, Track},
    smf_io,
};

fn music(pitch: u8) -> Music {
    Music {
        format: 0,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 9,
                    data: b"Surge XT".to_vec(),
                },
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 8,
                    data: b"Init Saw".to_vec(),
                },
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x7f,
                    data: vec![0, 123, 255],
                },
                Event {
                    tick: 1,
                    delta: 1,
                    status: 0xf0,
                    meta_type: 0,
                    data: vec![0x7d, 1, 0xf7],
                },
                Event {
                    tick: 130,
                    delta: 129,
                    status: 0xc0,
                    meta_type: 0,
                    data: vec![42],
                },
                Event {
                    tick: 150,
                    delta: 20,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![pitch, 100],
                },
                Event {
                    tick: 480,
                    delta: 330,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![pitch, 0],
                },
                Event {
                    tick: 500,
                    delta: 20,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    }
}

#[test]
fn smf_round_trip_preserves_payloads_and_recomputes_edited_tick_deltas() {
    let mut original = music(60);
    original.tracks[0].events[5].tick = 200;
    let loaded = cat_smf_editor::ffi::parse(&smf_io::encode(&original).unwrap()).unwrap();
    for (expected, actual) in original.tracks[0]
        .events
        .iter()
        .zip(&loaded.tracks[0].events)
    {
        assert_eq!(
            (
                expected.tick,
                expected.status,
                expected.meta_type,
                &expected.data
            ),
            (actual.tick, actual.status, actual.meta_type, &actual.data)
        );
    }
    assert_eq!(loaded.tracks[0].events[5].delta, 70);
    original.tracks[0].events[5].tick = 0;
    assert!(smf_io::encode(&original).is_err());
}

fn root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("smf-direct-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn direct_save_preserves_payloads_backs_up_and_detects_external_changes() {
    let root = root("save");
    let path = root.join("日本語.mid");
    let original = smf_io::encode(&music(60)).unwrap();
    std::fs::write(&path, &original).unwrap();
    let mut document = Document::new(Some(root.clone()));
    let mut edited = document.load(&path).unwrap();
    document.remember(&path, &edited).unwrap();
    document.save().unwrap();
    assert!(!root.join("backup").exists());
    edited.tracks[0].events[5].data[0] = 67;
    document.remember(&path, &edited).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    document.save().unwrap();
    let backups: Vec<_> = std::fs::read_dir(root.join("backup"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(std::fs::read(&backups[0]).unwrap(), original);
    assert_eq!(
        smf_io::encode(&smf_io::load(&path).unwrap()).unwrap(),
        smf_io::encode(&edited).unwrap()
    );
    assert!(!document.dirty());
    document.save().unwrap();
    assert_eq!(std::fs::read_dir(root.join("backup")).unwrap().count(), 1);
    let expected = std::fs::read(&path).unwrap();
    edited.tracks[0].events[5].data[0] = 70;
    document.remember(&path, &edited).unwrap();
    std::fs::write(&path, b"external change").unwrap();
    assert!(document
        .save()
        .unwrap_err()
        .contains("changed since it was loaded"));
    assert!(document.dirty());
    assert_eq!(std::fs::read(&path).unwrap(), b"external change");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    std::fs::write(&path, expected).unwrap();
    document.save().unwrap();
    assert_eq!(
        smf_io::encode(&smf_io::load(&path).unwrap()).unwrap(),
        smf_io::encode(&edited).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_backup_preserves_document_and_can_be_retried() {
    let root = root("backup-failure");
    let path = root.join("song.mid");
    let original = smf_io::encode(&music(60)).unwrap();
    std::fs::write(&path, &original).unwrap();
    let mut document = Document::new(Some(root.clone()));
    document.load(&path).unwrap();
    document.remember(&path, &music(67)).unwrap();
    std::fs::write(root.join("backup"), b"blocked").unwrap();
    assert!(document.save().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(document.dirty());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    std::fs::remove_file(root.join("backup")).unwrap();
    document.save().unwrap();
    assert!(!document.dirty());
    std::fs::remove_dir_all(root).unwrap();
}
