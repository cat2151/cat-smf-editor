use crate::model::{Event, Track};
use serde::{Deserialize, Serialize};

pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0voicevox\0";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Voice {
    pub speaker_uuid: String,
    pub name: String,
    pub style_id: u32,
    pub style_name: String,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            speaker_uuid: "388f246b-8c41-4ac1-8e2d-5d79f3ff56d9".into(),
            name: super::SINGER.into(),
            style_id: super::STYLE,
            style_name: "ノーマル".into(),
        }
    }
}

impl Voice {
    pub fn label(&self) -> String {
        format!("{} / {}", self.name, self.style_name)
    }

    pub fn read(track: &Track) -> Option<Self> {
        let mut records = track.events.iter().filter(|e| owned(e));
        let event = records.next()?;
        if event.tick != 0 || records.next().is_some() {
            return None;
        }
        let voice: Self = serde_json::from_slice(&event.data[PREFIX.len()..]).ok()?;
        (!voice.speaker_uuid.is_empty() && !voice.name.is_empty() && !voice.style_name.is_empty())
            .then_some(voice)
    }

    pub fn write(track: &mut Track, voice: Option<&Self>) -> Result<(), String> {
        track.events.retain(|e| !owned(e));
        if let Some(voice) = voice {
            let mut data = PREFIX.to_vec();
            data.extend(serde_json::to_vec(voice).map_err(|e| e.to_string())?);
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
        crate::edit::refresh_deltas(track)
    }

    pub fn same_style(&self, other: &Self) -> bool {
        self.speaker_uuid == other.speaker_uuid && self.style_id == other.style_id
    }
}

fn owned(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(PREFIX)
}

pub fn is_singer(track: &Track) -> bool {
    Voice::read(track).is_some()
}

pub fn make_singer(track: &mut Track) -> Result<(), String> {
    Voice::write(track, Some(&Voice::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_round_trips_without_changing_track_names() {
        let mut track = Track {
            events: vec![Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 3,
                data: b"Melody".to_vec(),
            }],
        };
        let original = format!("{track:?}");
        make_singer(&mut track).unwrap();
        assert_eq!(Voice::read(&track), Some(Voice::default()));
        assert_eq!(track.name().as_deref(), Some("Melody"));
        let next = Voice {
            style_id: 3001,
            ..Voice::default()
        };
        Voice::write(&mut track, Some(&next)).unwrap();
        assert_eq!(track.events.len(), 2);
        assert_eq!(Voice::read(&track), Some(next));
        Voice::write(&mut track, None).unwrap();
        assert_eq!(format!("{track:?}"), original);
        track.events[0].data = super::super::SINGER.as_bytes().to_vec();
        assert!(!is_singer(&track));
    }
}
