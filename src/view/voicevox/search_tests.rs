//! Searching the palette for "voicevox" lists every lyric and singer command.
use crate::view::{
    palette::tests::{listed, open, query},
    tests::app,
};
use cat_smf_editor::model::{Event, Music, Track};
use cat_smf_editor::score::note_view::note_on;
use eframe::egui;

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type: 0,
        data: data.to_vec(),
    }
}

/// A plain track with one quarter note: not a singer yet.
fn song() -> Music {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                event(0, 0x90, &[60, 100]),
                event(480, 0x80, &[60, 0]),
                Event {
                    meta_type: 0x2f,
                    ..event(1920, 0xff, &[])
                },
            ],
        }],
    };
    cat_smf_editor::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
    music
}

#[test]
fn voicevox_lists_the_lyric_and_singer_commands() {
    let mut app = app(song());
    let ctx = egui::Context::default();
    let loaded = app.loaded.as_ref().unwrap();
    let on = loaded.music.tracks[0]
        .events
        .iter()
        .position(note_on)
        .unwrap();
    let row = loaded.display_rows[0].event_row(on).unwrap();
    crate::cursor::moves::place(&mut app, row);
    open(&mut app, &ctx);
    query(&mut app, &ctx, "voicevox");
    let listed = listed(&mut app);
    assert_eq!(
        listed
            .iter()
            .filter(|s| s.contains("現在のtrackの歌声を選ぶ"))
            .count(),
        1
    );
    eprintln!("{listed:#?}");
    for label in [
        "Event list: VOICEVOX: 現在のtrackの歌声を選ぶ",
        "Event list: VOICEVOX: 1小節の歌詞を差し替え",
        "Edit: Lyrics",
        "Settings: VOICEVOX engine",
    ] {
        assert!(listed.iter().any(|l| l == label), "{label}");
    }
}

#[test]
fn palette_offers_combined_input_only_for_singers_and_no_sequential_inputs() {
    for singer in [false, true] {
        let mut music = song();
        if singer {
            cat_smf_editor::voicevox::make_singer(&mut music.tracks[0]).unwrap();
        }
        let mut app = app(music);
        let ctx = egui::Context::default();
        open(&mut app, &ctx);
        query(&mut app, &ctx, "voicevox");
        let mut actual: Vec<_> = listed(&mut app)
            .into_iter()
            .filter(|label| label.starts_with("Event list: VOICEVOX:"))
            .collect();
        actual.sort();
        let mut expected = vec!["Event list: VOICEVOX: 1小節の歌詞を差し替え"];
        expected.push("Event list: VOICEVOX: 現在のtrackの歌声を選ぶ");
        if singer {
            expected.push("Event list: VOICEVOX: 1小節のMMLと歌詞を同時に差し替え");
        }
        expected.sort();
        assert_eq!(actual, expected, "singer={singer}");
    }
}
