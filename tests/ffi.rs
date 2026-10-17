use cat_smf_editor::{ffi, smf_io};

fn smf(track: &[u8]) -> Vec<u8> {
    let mut bytes = b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk".to_vec();
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(track);
    bytes
}

#[test]
fn preserves_raw_events_and_running_status() {
    let track = [
        0, 0x90, 60, 100, 10, 61, 0, 0, 0x80, 60, 64, 0, 0xa2, 60, 12, 0, 0xb3, 0, 7, 0, 0xc4, 127,
        0, 0xd5, 33, 0, 0xe6, 1, 64, 0, 0xff, 3, 3, b'a', b'b', b'c', 0, 0xff, 0x51, 3, 7, 0xa1,
        0x20, 0, 0xff, 0x7e, 2, 0xab, 0xcd, 0, 0xf0, 3, 1, 2, 0xf7, 0, 0xf7, 2, 3, 4, 0, 0xff,
        0x2f, 0,
    ];
    let music = ffi::parse(&smf(&track)).unwrap();
    assert_eq!(
        (music.format, music.division, music.tracks.len()),
        (0, 480, 1)
    );
    let events = &music.tracks[0].events;
    assert_eq!(events.len(), 14);
    assert_eq!(
        events.iter().map(|e| e.status).collect::<Vec<_>>(),
        [0x90, 0x90, 0x80, 0xa2, 0xb3, 0xc4, 0xd5, 0xe6, 0xff, 0xff, 0xff, 0xf0, 0xf7, 0xff]
    );
    assert_eq!(events[1].data, [61, 0]);
    for (event, expected) in events[..8].iter().zip([
        &[60, 100][..],
        &[61, 0],
        &[60, 64],
        &[60, 12],
        &[0, 7],
        &[127],
        &[33],
        &[1, 64],
    ]) {
        assert_eq!(event.data, expected);
    }
    assert_eq!((events[1].delta, events[1].tick), (10, 10));
    assert_eq!(events[10].meta_type, 0x7e);
    assert_eq!(events[10].data, [0xab, 0xcd]);
    assert_eq!(events[11].data, [1, 2, 0xf7]);
    assert_eq!(events[12].data, [3, 4]);
    assert_eq!(music.tracks[0].name().as_deref(), Some("abc"));
    assert!(events[13].data.is_empty());
    for n in 0..smf(&track).len() {
        assert!(ffi::parse(&smf(&track)[..n]).is_err(), "prefix {n}");
    }
    let path = std::env::temp_dir().join(format!("cat-smf-{}-日本語.mid", std::process::id()));
    std::fs::write(&path, smf(&track)).unwrap();
    assert_eq!(smf_io::load(&path).unwrap().tracks[0].events.len(), 14);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn rejects_invalid_lengths_and_status() {
    for track in [
        &[0, 60, 64][..],
        &[0, 0x90, 0x80, 0][..],
        &[0, 0xff, 1, 0xff, 0xff, 0xff, 0x7f][..],
        &[0xff, 0xff, 0xff, 0xff][..],
    ] {
        assert!(ffi::parse(&smf(track)).is_err());
    }
    let mut bytes = smf(&[]);
    bytes[18..22].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(ffi::parse(&bytes).is_err());
}

#[test]
fn keeps_track_boundaries_and_rejects_bad_headers() {
    let mut bytes = smf(&[10, 0x90, 60, 1, 0, 0xff, 0x2f, 0]);
    bytes[9] = 1;
    bytes[11] = 2;
    let second = smf(&[20, 0x91, 61, 2, 0, 0xff, 0x2f, 0]);
    bytes.extend_from_slice(&second[14..]);
    let music = ffi::parse(&bytes).unwrap();
    assert_eq!(music.tracks.len(), 2);
    assert_eq!(music.tracks[0].events[0].tick, 10);
    assert_eq!(music.tracks[1].events[0].tick, 20);
    assert_eq!(music.tracks[1].events[0].status, 0x91);
    for (offset, value) in [(8, 1), (9, 3), (10, 0x80), (12, 0), (13, 0)] {
        let mut bad = bytes.clone();
        if offset >= 12 {
            bad[12] = 0;
            bad[13] = 0;
        }
        bad[offset] = value;
        assert!(ffi::parse(&bad).is_err());
    }
}
