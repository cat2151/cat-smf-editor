//! The track's explicit SMF program name, or its first GM Program Change name.
use crate::{model::Track, score::gm_programs};

pub fn read(track: &Track) -> Option<String> {
    track
        .events
        .iter()
        .find_map(|event| {
            (event.status == 0xff && event.meta_type == 8)
                .then(|| String::from_utf8_lossy(&event.data).trim().to_lowercase())
                .filter(|name| !name.is_empty())
        })
        .or_else(|| {
            track.events.iter().find_map(|event| {
                match (event.status & 0xf0, event.data.as_slice()) {
                    (0xc0, [program]) if *program < 128 => {
                        Some(gm_programs::NAMES[usize::from(*program)].to_lowercase())
                    }
                    _ => None,
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Event;

    #[test]
    fn explicit_name_wins_over_gm_and_missing_program_stays_unfiltered() {
        let mut track = Track { events: vec![] };
        assert_eq!(read(&track), None);
        track.events.push(Event {
            delta: 0,
            tick: 0,
            status: 0xc3,
            meta_type: 0,
            data: vec![19],
        });
        assert_eq!(read(&track).as_deref(), Some("church organ"));
        track.events.push(Event {
            delta: 0,
            tick: 0,
            status: 0xff,
            meta_type: 8,
            data: b" Custom Organ ".to_vec(),
        });
        assert_eq!(read(&track).as_deref(), Some("custom organ"));
    }
}
