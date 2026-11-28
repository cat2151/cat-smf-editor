use std::{ffi::OsString, path::PathBuf};

pub const USAGE: &str = "Usage: cat-smf-editor [<mid>] [--player <path>]\n       cat-smf-editor <mid> --render-wav <out.wav> [--track <n>] [--player <path>]\nWithout --player, search beside the editor, then sibling cat-plugin-player/target/release, then PATH.\n--render-wav writes stereo 16-bit 48000 Hz without a window; --track <n> (1-based) renders what track playback sounds.";

#[derive(Debug, PartialEq, Eq)]
pub enum Arguments {
    Run {
        player: Option<PathBuf>,
        mid: Option<PathBuf>,
    },
    Render(Render),
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Render {
    pub player: Option<PathBuf>,
    pub mid: PathBuf,
    pub wav: PathBuf,
    /// 0-based; None renders all tracks.
    pub track: Option<usize>,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Arguments, String> {
    let mut args = args.into_iter();
    let mut player = None;
    let mut mid = None;
    let mut wav = None;
    let mut track = None;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return Ok(Arguments::Help);
        }
        let slot = match arg.to_str() {
            Some("--player") => &mut player,
            Some("--render-wav") => &mut wav,
            Some("--track") => &mut track,
            _ => {
                if arg.is_empty() || arg.to_string_lossy().starts_with('-') {
                    return Err(format!("Unknown argument: {}", arg.to_string_lossy()));
                }
                if mid.is_some() {
                    return Err("Only one MIDI file may be specified".into());
                }
                mid = Some(PathBuf::from(arg));
                continue;
            }
        };
        let name = arg.to_string_lossy().into_owned();
        if slot.is_some() {
            return Err(format!("{name} may only be specified once"));
        }
        let value = args
            .next()
            .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with('-'))
            .ok_or_else(|| format!("{name} requires a value"))?;
        *slot = Some(value);
    }
    let player = player.map(PathBuf::from);
    let Some(wav) = wav else {
        if track.is_some() {
            return Err("--track requires --render-wav".into());
        }
        return Ok(Arguments::Run { player, mid });
    };
    let number = |value: Option<OsString>, name: &str| {
        value
            .map(|v| {
                v.to_string_lossy()
                    .parse::<u32>()
                    .map_err(|_| format!("{name} requires a nonnegative integer"))
            })
            .transpose()
    };
    let track = match number(track, "--track")? {
        Some(0) => return Err("--track starts at 1".into()),
        track => track.map(|t| t as usize - 1),
    };
    Ok(Arguments::Render(Render {
        player,
        mid: mid.ok_or("--render-wav requires a MIDI file")?,
        wav: wav.into(),
        track,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(args: &[&str]) -> Result<Arguments, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn optional_player_path_and_help() {
        assert_eq!(
            arguments(&[]).unwrap(),
            Arguments::Run {
                player: None,
                mid: None
            }
        );
        assert_eq!(
            arguments(&["--player", "X:/player with spaces/player.exe"]).unwrap(),
            Arguments::Run {
                player: Some("X:/player with spaces/player.exe".into()),
                mid: None
            }
        );
        assert_eq!(arguments(&["--help"]).unwrap(), Arguments::Help);
    }

    #[test]
    fn rejects_invalid_arguments() {
        for args in [
            vec!["--player"],
            vec!["--player", "--help"],
            vec!["--player", ""],
            vec!["--player", "a", "--player", "b"],
            vec!["--unknown"],
            vec!["one.mid", "two.mid"],
            vec!["--render-wav", "out.wav"],
            vec!["a.mid", "--track", "2"],
            vec!["a.mid", "--render-wav", "out.wav", "--track", "0"],
            vec!["a.mid", "--render-wav", "out.wav", "--track", "1.5"],
        ] {
            assert!(arguments(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn render_wav_takes_a_one_based_track() {
        assert_eq!(
            arguments(&["a.mid", "--render-wav", "o.wav"]).unwrap(),
            Arguments::Render(Render {
                player: None,
                mid: "a.mid".into(),
                wav: "o.wav".into(),
                track: None
            })
        );
        assert_eq!(
            arguments(&[
                "--render-wav",
                "o.wav",
                "--track",
                "10",
                "a.mid"
            ])
            .unwrap(),
            Arguments::Render(Render {
                player: None,
                mid: "a.mid".into(),
                wav: "o.wav".into(),
                track: Some(9)
            })
        );
    }
}

#[cfg(test)]
#[test]
fn midi_argument_supports_spaces_unicode_and_both_option_orders() {
    for args in [
        vec!["X:/music/日本語 song.mid", "--player", "X:/player.exe"],
        vec!["--player", "X:/player.exe", "X:/music/日本語 song.mid"],
    ] {
        assert_eq!(
            parse(args.into_iter().map(OsString::from)).unwrap(),
            Arguments::Run {
                mid: Some("X:/music/日本語 song.mid".into()),
                player: Some("X:/player.exe".into())
            }
        );
    }
}
