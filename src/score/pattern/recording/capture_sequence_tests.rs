use super::*;

fn phrase(start: u64, pitch: u8) -> Phrase {
    Phrase {
        start,
        container: None,
        notes: vec![Note {
            start: 0,
            length: 250,
            pitch,
            velocity: 80,
        }],
    }
}

#[test]
fn capture_order_spans_containers_and_continues_after_reopening_past_nine() {
    let mut music = edit::tracks::new_song();
    music.division = 480;
    let tracks = [
        operations::create_source(&mut music).unwrap(),
        operations::create_source(&mut music).unwrap(),
    ];
    for n in 1..=12 {
        // Session clocks can restart between captures, independently of capture order.
        append(
            &mut music,
            Some(tracks[n % 2]),
            500_000,
            &phrase(0, 60),
            &[],
        )
        .unwrap();
    }
    let bytes = crate::smf_io::encode(&music).unwrap();
    let mut reopened = crate::ffi::parse(&bytes).unwrap();
    let before = captured(&reopened).unwrap();
    assert_eq!(before, captured(&music).unwrap());
    assert_eq!(
        before.iter().map(|p| p.sequence).collect::<Vec<_>>(),
        (1..=12).rev().collect::<Vec<_>>()
    );
    append(&mut reopened, Some(tracks[0]), 500_000, &phrase(0, 64), &[]).unwrap();
    let after = captured(&reopened).unwrap();
    assert_eq!(after[0].sequence, 13);
    assert_eq!(after[0].notes[0].pitch, 64);
    assert_eq!(after[1..], before);
}

#[test]
fn session_start_and_time_between_phrases_are_not_written_to_smf() {
    let mut base = edit::tracks::new_song();
    base.division = 480;
    let track = operations::create_source(&mut base).unwrap();
    append(&mut base, Some(track), 500_000, &phrase(0, 60), &[]).unwrap();
    let song = |starts: [u64; 2]| {
        // Share existing random pattern/container identities between the two versions.
        let mut music = base.clone();
        for start in starts {
            append(&mut music, Some(track), 500_000, &phrase(start, 60), &[]).unwrap();
        }
        crate::smf_io::encode(&music).unwrap()
    };
    assert_eq!(
        song([0, 1_000]),
        song([1_790_000_000_000, 1_890_000_000_000])
    );
}

#[test]
fn sequence_overflow_keeps_the_document_unchanged() {
    let mut music = edit::tracks::new_song();
    music.division = 480;
    let track = operations::create_source(&mut music).unwrap();
    append(&mut music, Some(track), 500_000, &phrase(0, 60), &[]).unwrap();
    let marker = music.tracks[track]
        .events
        .iter_mut()
        .find(|e| marker(e))
        .unwrap();
    marker.data = format!("Capture {}", u64::MAX).into_bytes();
    let before = crate::smf_io::encode(&music).unwrap();
    assert!(append(&mut music, Some(track), 500_000, &phrase(0, 64), &[]).is_err());
    assert_eq!(crate::smf_io::encode(&music).unwrap(), before);
}
