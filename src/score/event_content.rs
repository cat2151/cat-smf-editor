//! Human-readable MIDI content, without modifying source payloads.
// Meta layouts: https://mido.readthedocs.io/en/stable/meta_message_types.html
use crate::{
    model::Event,
    score::gm_programs,
    score::note_view::{note_name, note_off},
};

mod sequencer;

/// A piece of the decoded text; the view colors values and notes.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Label(String),
    Value(String),
    Note(u8),
}

impl Part {
    pub fn text(&self) -> String {
        match self {
            Self::Label(text) | Self::Value(text) => text.clone(),
            Self::Note(note) => note_name(*note),
        }
    }
}

pub fn decoded(event: &Event) -> String {
    parts(event).iter().map(Part::text).collect()
}

pub fn parts(event: &Event) -> Vec<Part> {
    let data = event.data.as_slice();
    if event.status == 0xff {
        return meta(event);
    }
    let label = |text: String| vec![Part::Label(text)];
    match (event.status & 0xf0, data) {
        (0x80 | 0x90, [note, velocity]) => vec![
            Part::Label(
                if note_off(event) {
                    "Note Off "
                } else {
                    "Note On "
                }
                .into(),
            ),
            Part::Note(*note),
            Part::Label(format!(" · velocity {velocity}")),
        ],
        (0xa0, [note, pressure]) => vec![
            Part::Label("Poly pressure ".into()),
            Part::Note(*note),
            Part::Label(format!(" = {pressure}")),
        ],
        (0xb0, [controller, value]) => label({
            let name = cc_name(*controller);
            if (64..=69).contains(controller) || *controller == 122 {
                format!(
                    "{} {}",
                    name.unwrap_or("Switch"),
                    if *value >= 64 { "on" } else { "off" }
                )
            } else if let Some(name) = name {
                if matches!(controller, 120 | 123 | 124 | 125 | 127) {
                    name.into()
                } else {
                    format!("{name} {value}")
                }
            } else {
                format!("CC {controller} = {value}")
            }
        }),
        (0xc0, [program]) if *program < 128 => field(
            "Program",
            " ",
            gm_programs::NAMES[usize::from(*program)].into(),
        ),
        (0xd0, [pressure]) => label(format!("Channel pressure = {pressure}")),
        (0xe0, [lsb, msb]) => {
            let value = u16::from(*lsb) | (u16::from(*msb) << 7);
            label(format!("Pitch Bend {}", i32::from(value) - 8192))
        }
        _ => raw(event),
    }
}

/// The kind keeps the label color; only the content is a value.
fn field(kind: &str, separator: &str, value: String) -> Vec<Part> {
    vec![
        Part::Label(format!("{kind}{separator}")),
        Part::Value(value),
    ]
}

// Bound work for large payloads, and keep line breaks inside a single table row.
fn text_preview(data: &[u8]) -> String {
    let prefix = &data[..data.len().min(512)];
    let text = String::from_utf8_lossy(prefix);
    let preview: String = text
        .chars()
        .take(120)
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect();
    let suffix = if data.len() > prefix.len() || text.chars().count() > 120 {
        " …"
    } else {
        ""
    };
    format!("{preview}{suffix}")
}

/// Other tools' payloads that are mostly text read better as text than as hex.
fn readable(data: &[u8]) -> Option<Vec<Part>> {
    let text = String::from_utf8_lossy(&data[..data.len().min(512)]);
    let printable = text
        .chars()
        .filter(|c| !c.is_control() && *c != char::REPLACEMENT_CHARACTER)
        .count();
    (printable * 2 >= text.chars().count() && printable > 0)
        .then(|| field("Sequencer-specific", ": ", text_preview(data)))
}

fn meta(event: &Event) -> Vec<Part> {
    let data = event.data.as_slice();
    match (event.meta_type, data) {
        (1..=9, _) => {
            let name = [
                "Text",
                "Copyright",
                "Track name",
                "Instrument name",
                "Lyrics",
                "Marker",
                "Cue point",
                "Program name",
                "Device name",
            ][usize::from(event.meta_type - 1)];
            field(name, ": ", text_preview(data))
        }
        (0x00, [hi, lo]) => field(
            "Sequence number",
            " = ",
            u16::from_be_bytes([*hi, *lo]).to_string(),
        ),
        (0x20, [channel]) if *channel < 16 => {
            field("Channel prefix", " = ", (channel + 1).to_string())
        }
        (0x21, [port]) => field("MIDI port", " = ", port.to_string()),
        (0x2f, []) => vec![Part::Label("End of Track".into())],
        (0x51, [a, b, c]) => {
            let micros = u32::from_be_bytes([0, *a, *b, *c]);
            if micros == 0 {
                return raw(event);
            }
            field(
                "Tempo",
                " = ",
                format!(
                    "{:.2} BPM ({micros} µs/quarter)",
                    60_000_000.0 / f64::from(micros)
                ),
            )
        }
        (0x54, [hour, minute, second, frame, subframe]) if *hour < 0x80 => {
            let fps = ["24", "25", "29.97 drop-frame", "30"][usize::from((hour >> 5) & 3)];
            field(
                "SMPTE offset",
                " = ",
                format!(
                    "{:02}:{minute:02}:{second:02}:{frame:02}.{subframe:02} ({fps} fps)",
                    hour & 31
                ),
            )
        }
        (0x58, [numerator, exponent, clocks, thirty_seconds]) => {
            let denominator = 1u128
                .checked_shl(u32::from(*exponent))
                .map_or_else(|| format!("2^{exponent}"), |d| d.to_string());
            field(
                "Time signature",
                " = ",
                format!(
                    "{numerator}/{denominator} · {clocks} clocks/click · {thirty_seconds} 32nds/quarter"
                ),
            )
        }
        (0x59, [sf, mode]) if (-7..=7).contains(&(*sf as i8)) && *mode <= 1 => {
            let index = usize::try_from(i16::from(*sf as i8) + 7).unwrap();
            let names = if *mode == 0 {
                [
                    "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#",
                    "C#",
                ]
            } else {
                [
                    "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#",
                    "A#",
                ]
            };
            field(
                "Key signature",
                " = ",
                format!(
                    "{} {}",
                    names[index],
                    if *mode == 0 { "major" } else { "minor" }
                ),
            )
        }
        (0x7f, data) => sequencer::parts(data)
            .or_else(|| readable(data))
            .unwrap_or_else(|| raw(event)),
        _ => raw(event),
    }
}

fn cc_name(controller: u8) -> Option<&'static str> {
    Some(match controller {
        0 => "Bank Select MSB",
        1 => "Modulation",
        2 => "Breath",
        4 => "Foot",
        5 => "Portamento time",
        6 => "Data Entry MSB",
        7 => "Volume",
        8 => "Balance",
        10 => "Pan",
        11 => "Expression",
        32 => "Bank Select LSB",
        38 => "Data Entry LSB",
        64 => "Sustain",
        65 => "Portamento",
        66 => "Sostenuto",
        67 => "Soft pedal",
        68 => "Legato",
        69 => "Hold 2",
        71 => "Resonance",
        72 => "Release time",
        73 => "Attack time",
        74 => "Brightness",
        84 => "Portamento control",
        91 => "Reverb send",
        93 => "Chorus send",
        96 => "Data increment",
        97 => "Data decrement",
        98 => "NRPN LSB",
        99 => "NRPN MSB",
        100 => "RPN LSB",
        101 => "RPN MSB",
        120 => "All Sound Off",
        121 => "Reset All Controllers",
        122 => "Local Control",
        123 => "All Notes Off",
        124 => "Omni Off",
        125 => "Omni On",
        126 => "Mono",
        127 => "Poly",
        _ => return None,
    })
}

fn raw(event: &Event) -> Vec<Part> {
    let name = match event.status {
        0xff if event.meta_type == 0x7f => "Sequencer-specific".into(),
        0xff => format!("Meta {:02X} (undecoded)", event.meta_type),
        0xf0 => "SysEx F0 (undecoded)".into(),
        0xf7 => "SysEx F7 continuation/escape (undecoded)".into(),
        status => format!("Event {status:02X} (undecoded)"),
    };
    let preview = event
        .data
        .iter()
        .take(24)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let preview = format!("{preview}{}", if event.data.len() > 24 { " …" } else { "" });
    if event.status == 0xff {
        field(&name, ": ", preview)
    } else {
        vec![Part::Label(format!("{name}: {preview}"))]
    }
}
