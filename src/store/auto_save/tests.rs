use super::*;
use cat_smf_editor::{model::Track, smf_io};

#[test]
fn opt_in_round_trips_without_changing_notes_or_foreign_metadata() {
    for format in [0, 1, 2] {
        let mut music = Music {
            format,
            division: 480,
            tracks: vec![Track {
                events: vec![
                    Event {
                        delta: 0,
                        tick: 0,
                        status: 0xff,
                        meta_type: 0x7f,
                        data: b"foreign".to_vec(),
                    },
                    Event {
                        delta: 10,
                        tick: 10,
                        status: 0x90,
                        meta_type: 0,
                        data: vec![60, 80],
                    },
                    Event {
                        delta: 10,
                        tick: 20,
                        status: 0x80,
                        meta_type: 0,
                        data: vec![60, 0],
                    },
                ],
            }],
        };
        let original = smf_io::encode(&music).unwrap();
        assert!(!enabled(&music));
        write(&mut music, true).unwrap();
        let bytes = smf_io::encode(&music).unwrap();
        let mut reopened = cat_smf_editor::ffi::parse(&bytes).unwrap();
        assert!(enabled(&reopened));
        write(&mut reopened, true).unwrap();
        assert_eq!(smf_io::encode(&reopened).unwrap(), bytes);
        write(&mut reopened, false).unwrap();
        assert!(!enabled(&reopened));
        assert_eq!(smf_io::encode(&reopened).unwrap(), original);
    }
}

#[test]
fn malformed_duplicate_or_misplaced_records_never_enable_overwriting() {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }, Track { events: vec![] }],
    };
    write(&mut music, true).unwrap();
    let record = music.tracks[0].events[0].clone();
    music.tracks[1].events.push(record.clone());
    assert!(!enabled(&music));
    music.tracks[0].events.clear();
    assert!(!enabled(&music));
    music.tracks[1].events.clear();
    music.tracks[0].events.push(record);
    music.tracks[0].events[0].tick = 1;
    assert!(!enabled(&music));
    music.tracks[0].events[0].tick = 0;
    music.tracks[0].events[0].data.push(0);
    assert!(!enabled(&music));
}

#[test]
#[ignore = "reads actual local recent MIDI files; verifies opt-in is absent without modifying them"]
fn auto_save_existing_local_files_stay_off_without_migration() {
    let root = crate::store::config::directory().unwrap();
    let status = std::fs::read(root.join("status.json")).unwrap();
    let recent = std::fs::read(root.join("recent-files.json")).unwrap();
    let list: serde_json::Value = serde_json::from_slice(&recent).unwrap();
    let mut paths: Vec<std::path::PathBuf> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["path"].as_str().unwrap().into())
        .collect();
    paths.push(
        crate::store::config::Store::local()
            .navigation
            .last_mid
            .unwrap(),
    );
    paths.sort();
    paths.dedup();
    assert!(!paths.is_empty());
    for path in paths {
        let original = std::fs::read(&path).unwrap();
        let mut document = crate::store::saved::Document::new(None);
        let music = document.load(&path).unwrap();
        assert!(!enabled(&music), "{}", path.display());
        document.remember(&path, &music).unwrap();
        assert!(!document.dirty());
        assert!(document.auto_save_deadline().is_none());
        document.save().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    assert_eq!(std::fs::read(root.join("status.json")).unwrap(), status);
    assert_eq!(
        std::fs::read(root.join("recent-files.json")).unwrap(),
        recent
    );
}
