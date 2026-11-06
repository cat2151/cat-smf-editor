use super::*;
use cat_smf_editor::score::drum_kit;
pub(crate) fn fixture(count: u8) -> App {
    let mut a = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }; 2],
    });
    let patch = Patch {
        kit: Some(cat_smf_editor::drum_kit::Kit {
            muted: vec![],
            swing: Default::default(),
            layer: 1,
            rows: Some((0..count).map(Some).collect()),
            notes: (0..count)
                .map(|note| cat_smf_editor::drum_kit::Note {
                    note,
                    name: format!("Source {note}"),
                    velocity: 100,
                })
                .collect(),
        }),
        ..Default::default()
    };
    crate::player::selection::store_track(&mut a, 1, &patch).unwrap();
    a.select_track(1);
    a
}
#[test]
fn kit_edit_and_velocity_default_keep_existing_hits_and_undo() {
    let mut a = fixture(128);
    a.edit_music(|m| drum_kit::toggle(m, 1, 127, 1, 0)).unwrap();
    let before = a.loaded.as_ref().unwrap().music.clone();
    a.edit_music(|m| drum_kit::velocity(m, 1, 127, 1, 0, 64))
        .unwrap();
    let hit = |a: &App| {
        a.loaded.as_ref().unwrap().music.tracks[1]
            .events
            .iter()
            .find(|e| cat_smf_editor::score::note_view::note_on(e))
            .unwrap()
            .data[1]
    };
    assert_eq!(hit(&a), 64);
    a.undo_music().unwrap();
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        cat_smf_editor::smf_io::encode(&before).unwrap()
    );
    let mut patch = a.loaded.as_ref().unwrap().patches[1].clone();
    patch.kit.as_mut().unwrap().notes[127].velocity = 45;
    crate::player::selection::store_track(&mut a, 1, &patch).unwrap();
    assert_eq!(hit(&a), 100);
    a.edit_music(|m| drum_kit::toggle(m, 1, 127, 1, 1)).unwrap();
    let values: Vec<_> = a.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .map(|e| e.data[1])
        .collect();
    assert_eq!(values, [100, 45]);
}
