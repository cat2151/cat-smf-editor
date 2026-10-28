use cat_smf_editor::model::{Event, Music, Track};

pub(crate) fn singing() -> crate::app::App {
    let meta = |tick, kind, text: &str| Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: kind,
        data: text.as_bytes().to_vec(),
    };
    let mut events = vec![
        meta(0, 3, "ずんだもん"),
        meta(0, 5, ""),
        meta(0, 5, "きゃ"),
        meta(0, 5, "duplicate"),
        meta(120, 5, "orphan"),
        meta(480, 5, "り"),
    ];
    for (tick, pitch) in [(0, 72), (0, 76), (480, 74)] {
        for (offset, status, velocity) in [(0, 0x90, 100), (240, 0x80, 0)] {
            events.push(Event {
                tick: tick + offset,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![pitch, velocity],
            });
        }
    }
    events.sort_by_key(|e| e.tick);
    let mut track = Track { events };
    cat_smf_editor::voicevox::make_singer(&mut track).unwrap();
    crate::view::tests::app(Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    })
}

#[test]
fn singing_lyrics_match_note_ticks_and_ignore_duplicates_and_orphans() {
    let mut app = singing();
    let read = |app: &crate::app::App| {
        let loaded = app.loaded.as_ref().unwrap();
        super::read(&loaded.music, 0, &crate::view::piano_roll::shown(loaded)[0])
    };
    assert_eq!(
        read(&app).into_iter().collect::<Vec<_>>(),
        [(0, "きゃ".into()), (480, "り".into())]
    );
    let loaded = app.loaded.as_mut().unwrap();
    cat_smf_editor::voicevox::Voice::write(&mut loaded.music.tracks[0], None).unwrap();
    assert!(read(&app).is_empty());
}
