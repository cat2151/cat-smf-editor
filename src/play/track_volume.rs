//! App-owned track output gain, independent of MIDI CC and instrument identity.
use cat_smf_editor::{
    app_meta::TRACK_VOLUME as PREFIX,
    model::{Event, Track},
};
pub const MIN_DB: i32 = -36;
pub const MAX_DB: i32 = 6;
pub const STEP_DB: i32 = 3;

pub fn db_to_gain(db: i32) -> f32 {
    10.0f32.powf(db as f32 / 20.0)
}

fn owned(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(PREFIX)
}

pub fn read(track: &Track) -> Result<f32, String> {
    let mut found = None;
    for event in track.events.iter().filter(|e| owned(e)) {
        if found.is_some()
            || event.tick != 0
            || event.data.len() != PREFIX.len() + 5
            || event.data[PREFIX.len()] != 1
        {
            return Err("Invalid or duplicate track volume metadata".into());
        }
        let gain = f32::from_le_bytes(event.data[PREFIX.len() + 1..].try_into().unwrap());
        if !gain.is_finite() || !(0.0..=db_to_gain(MAX_DB)).contains(&gain) {
            return Err("Invalid track volume".into());
        }
        found = Some(gain);
    }
    Ok(found.unwrap_or(1.0))
}

// Written into the in-memory SMF at the edit boundary.
pub fn write(track: &mut Track, gain: f32) {
    track.events.retain(|e| !owned(e));
    if gain == 1.0 {
        return;
    }
    let mut data = PREFIX.to_vec();
    data.push(1);
    data.extend(gain.to_le_bytes());
    track.events.insert(
        0,
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x7f,
            data,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_round_trips_with_patch_and_preserves_unrelated_events() {
        let event = Event {
            tick: 480,
            delta: 480,
            status: 0x90,
            meta_type: 0,
            data: vec![60, 90],
        };
        let mut track = Track {
            events: vec![event.clone()],
        };
        let patch = cat_smf_editor::score::instrument::Patch::default();
        let roots = &mut crate::player::patch_roots::Assumed;
        cat_smf_editor::score::instrument::write(&mut track, &patch, roots).unwrap();
        write(&mut track, 0.25);
        assert_eq!(read(&track).unwrap(), 0.25);
        assert_eq!(
            cat_smf_editor::score::instrument::read(&track, roots).unwrap(),
            patch
        );
        let music = cat_smf_editor::model::Music {
            format: 0,
            division: 480,
            tracks: vec![track],
        };
        let mut restored =
            cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(&music).unwrap()).unwrap();
        assert_eq!(read(&restored.tracks[0]).unwrap(), 0.25);
        write(&mut restored.tracks[0], 0.0);
        assert_eq!(read(&restored.tracks[0]).unwrap(), 0.0);
        assert_eq!(
            restored.tracks[0]
                .events
                .iter()
                .filter(|e| owned(e))
                .count(),
            1
        );
        write(&mut restored.tracks[0], 1.0);
        assert_eq!(read(&restored.tracks[0]).unwrap(), 1.0);
        assert!(restored.tracks[0]
            .events
            .iter()
            .any(|e| e.tick == event.tick && e.status == event.status && e.data == event.data));
    }
    #[test]
    fn malformed_volume_is_not_silently_accepted() {
        let mut track = Track { events: vec![] };
        write(&mut track, 0.5);
        track.events[0].data[PREFIX.len()] = 2;
        assert!(read(&track).is_err());
    }
    #[test]
    fn boosted_volume_round_trips_and_out_of_range_values_are_rejected() {
        let mut track = Track { events: vec![] };
        for db in [MIN_DB, -3, 0, MAX_DB] {
            let gain = db_to_gain(db);
            write(&mut track, gain);
            assert_eq!(read(&track).unwrap(), gain);
        }
        for gain in [db_to_gain(MAX_DB + STEP_DB), f32::NAN, f32::INFINITY, -1.0] {
            write(&mut track, gain);
            assert!(read(&track).is_err());
        }
    }
}
