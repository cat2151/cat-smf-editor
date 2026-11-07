use super::*;
use crate::{
    instrument_layers,
    score::{
        instrument::Patch,
        note_view::{note_off, note_on},
    },
};

#[test]
#[ignore = "read-only validation of CAT_EXISTING_KIT_DOCUMENT through the normal SMF reader"]
fn existing_kit_document_loads_and_reloads_with_default_controls() {
    let file = std::path::PathBuf::from(std::env::var_os("CAT_EXISTING_KIT_DOCUMENT").unwrap());
    let bytes = std::fs::read(&file).unwrap();
    let music = crate::smf_io::load(&file).unwrap();
    let kits = music
        .tracks
        .iter()
        .map(read)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(kits.iter().any(Option::is_some));
    for kit in kits.iter().flatten() {
        kit.validate().unwrap();
        assert!(kit.muted.is_empty());
        assert!(kit.swing.is_empty());
    }
    let restored = crate::ffi::parse(&crate::smf_io::encode(&music).unwrap()).unwrap();
    assert_eq!(
        restored
            .tracks
            .iter()
            .map(read)
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        kits
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}
fn kit() -> Kit {
    Kit {
        layer: 1,
        rows: None,
        muted: vec![],
        swing: Default::default(),
        notes: [0, 20, 36, 42, 60, 127]
            .into_iter()
            .map(|note| Note {
                note,
                name: format!("Source {note}"),
                velocity: 100,
            })
            .collect(),
    }
}
fn roots() -> crate::patch_roots::Fixture {
    crate::patch_roots::Fixture {
        table: Default::default(),
        files: vec![],
        refreshed: 0,
    }
}
fn music(n: u8, d: u8) -> Music {
    let mut source = Track {
        events: vec![Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![n, d, 24, 8],
        }],
    };
    instrument_layers::write(
        &mut source,
        &instrument_layers::Settings {
            kit: Some(kit()),
            ..Default::default()
        },
        &mut roots(),
    )
    .unwrap();
    Music {
        division: 480,
        format: 0,
        tracks: vec![source],
    }
}
#[test]
fn actual_notes_multiple_rows_and_meters_keep_independent_velocities_and_offs_after_reload() {
    for (n, d, length) in [(4, 2, 1920), (3, 2, 1440), (6, 3, 1440)] {
        let mut m = music(n, d);
        for note in [0, 20, 36, 42, 60, 127] {
            toggle(&mut m, 0, note, 1, 0).unwrap();
        }
        toggle(&mut m, 0, 127, 1, 4).unwrap();
        velocity(&mut m, 0, 127, 1, 0, 64).unwrap();
        let mut saved = instrument_layers::read(&m.tracks[0], &mut roots()).unwrap();
        saved.kit.as_mut().unwrap().notes[5].velocity = 45;
        instrument_layers::write(&mut m.tracks[0], &saved, &mut roots()).unwrap();
        toggle(&mut m, 0, 127, 2, 0).unwrap();
        let parsed = crate::ffi::parse(&crate::smf_io::encode(&m).unwrap()).unwrap();
        let t = &parsed.tracks[0];
        assert_eq!(t.events.iter().filter(|e| note_on(e)).count(), 8);
        let ons: Vec<_> = t
            .events
            .iter()
            .filter(|e| note_on(e) && e.data[0] == 127)
            .map(|e| (e.tick, e.data[1]))
            .collect();
        assert_eq!(ons, [(0, 64), (480, 100), (length, 45)]);
        let offs: Vec<_> = t
            .events
            .iter()
            .filter(|e| note_off(e) && e.data[0] == 127)
            .map(|e| e.tick)
            .collect();
        assert_eq!(offs, [480, length, 2 * length]);
        assert_eq!(rows(t, &read(t).unwrap().unwrap()).len(), 6);
        toggle(&mut m, 0, 0, 1, 0).unwrap();
        assert!(!m.tracks[0]
            .events
            .iter()
            .any(|e| (note_on(e) || note_off(e)) && e.data[0] == 0));
    }
}
#[test]
fn swapped_assignments_retain_authored_pitches_and_unknown_rows() {
    let mut m = music(4, 2);
    toggle(&mut m, 0, 127, 1, 0).unwrap();
    let mut saved = instrument_layers::read(&m.tracks[0], &mut roots()).unwrap();
    saved.kit.as_mut().unwrap().notes.retain(|n| n.note != 127);
    instrument_layers::write(&mut m.tracks[0], &saved, &mut roots()).unwrap();
    let rows = rows(&m.tracks[0], &read(&m.tracks[0]).unwrap().unwrap());
    assert_eq!(rows.last().unwrap().note, 127);
    assert!(rows.last().unwrap().name.is_empty());
    let mut patch = Patch::resolve(saved, |_| unreachable!()).unwrap();
    patch.drums = Some(Default::default());
    assert!(patch.validate().is_err());
}
#[test]
fn rows_take_pitches_from_normal_patterns_but_not_from_captures() {
    use crate::score::pattern::{self, Kind, PatternRecord, Record};
    let body = |tick: u64, kind: Kind, id: u64| {
        pattern::event(
            tick,
            &Record::Pattern(PatternRecord {
                version: pattern::VERSION,
                id,
                kind,
                name: kind.label().into(),
                meters: vec![pattern::Meter {
                    numerator: 4,
                    exponent: 2,
                    ticks: 1920,
                }],
            }),
        )
    };
    let hit = |tick: u64, note: u8| Event {
        tick,
        delta: 0,
        status: 0x90,
        meta_type: 0,
        data: vec![note, 100],
    };
    let track = Track {
        events: vec![
            body(0, Kind::Normal, 1),
            hit(0, 50),
            body(1920, Kind::Capture, 2),
            hit(1920, 70),
            hit(3839, 71),
            body(3840, Kind::Normal, 3),
            hit(3840, 52),
        ],
    };
    assert_eq!(
        layout::rows(&track, &kit()),
        [Some(52), Some(50)]
            .into_iter()
            .chain(layout::DEFAULT)
            .collect::<Vec<_>>()
    );
}
