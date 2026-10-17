use cat_smf_editor::{
    edit::tempo_curve::{self, Curve, Point},
    model::{Event, Music, Track},
};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

fn tempo_ticks(music: &Music) -> Vec<u64> {
    music.tracks[0]
        .events
        .iter()
        .filter(|e| e.status == 0xff && e.meta_type == 0x51)
        .map(|e| e.tick)
        .collect()
}

#[test]
fn editing_notes_regenerates_tempo_events_at_the_new_note_ticks() {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 0xff, 0x2f, &[])],
            },
            Track {
                events: vec![
                    event(0, 0x90, 0, &[60, 100]),
                    event(480, 0x80, 0, &[60, 0]),
                    event(960, 0x90, 0, &[62, 100]),
                    event(1440, 0x80, 0, &[62, 0]),
                ],
            },
        ],
    };
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    let curve = Curve {
        version: 1,
        points: vec![
            Point { tick: 0, bpm: 60.0 },
            Point {
                tick: 2000,
                bpm: 200.0,
            },
        ],
        controls: vec![Point {
            tick: 1000,
            bpm: 130.0,
        }],
    };
    tempo_curve::write(&mut music, Some(&curve)).unwrap();
    tempo_curve::regenerate(&mut music, &curve).unwrap();
    assert_eq!(tempo_ticks(&music), vec![0, 480, 960, 1440]);
    let mut app = crate::view::tests::app(music);
    app.edit_music(|music| {
        cat_smf_editor::edit::note_move::move_note(&mut music.tracks[1], 2, 120)
    })
    .unwrap();
    let music = &app.loaded.as_ref().unwrap().music;
    assert_eq!(tempo_ticks(music), vec![0, 480, 1080, 1560]);
    assert_eq!(tempo_curve::read(music).unwrap(), Some(curve));
}
