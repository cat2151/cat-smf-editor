use std::{ffi::OsString, path::PathBuf};

pub const USAGE: &str = "Usage: cat-smf-editor [<mid>] [--player <path>]\nWithout --player, search beside the editor, then sibling cat-plugin-player/target/release, then PATH.";

#[derive(Debug, PartialEq, Eq)]
pub enum Arguments {
    Run {
        player: Option<PathBuf>,
        mid: Option<PathBuf>,
    },
    Help,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Arguments, String> {
    let mut args = args.into_iter();
    let mut player = None;
    let mut mid = None;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return Ok(Arguments::Help);
        }
        if arg != "--player" {
            if arg.is_empty() || arg.to_string_lossy().starts_with('-') {
                return Err(format!("Unknown argument: {}", arg.to_string_lossy()));
            }
            if mid.is_some() {
                return Err("Only one MIDI file may be specified".into());
            }
            mid = Some(PathBuf::from(arg));
            continue;
        }
        if player.is_some() {
            return Err("--player may only be specified once".into());
        }
        let path = args.next().ok_or("--player requires a path")?;
        if path.is_empty() || path.to_string_lossy().starts_with('-') {
            return Err("--player requires a path".into());
        }
        player = Some(PathBuf::from(path));
    }
    Ok(Arguments::Run { player, mid })
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
        ] {
            assert!(arguments(&args).is_err(), "{args:?}");
        }
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
