//! Detect the MML half without letting the permissive MML parser consume lyric text.
use cat_smf_editor::{edit::mml, model::Music};
use std::sync::LazyLock;

fn is_mml(text: &str) -> bool {
    static SYNTAX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"(?x)^(?:\s|[cdefgabCDEFGABrR][+-]?\d*\.*|o\d+|l\d+\.*|[tv@]\d+|(?:kt|KT)-?\d+|[<>;]|'(?:\s|[<>]|[cdefgabCDEFGAB][+-]?\d*\.*)+')+$",
        )
        .unwrap()
    });
    !text.is_empty()
        && SYNTAX.is_match(text)
        && text.chars().any(|c| "cdefgabrCDEFGABR".contains(c))
}

/// Prefer the longest complete MML half. Equally plausible orders require correction.
fn split(text: &str) -> Result<(&str, &str), String> {
    let mut candidates = Vec::new();
    for (at, _) in text.char_indices().skip(1) {
        let left = text[..at].trim();
        let right = text[at..].trim();
        if left.is_empty() || right.is_empty() {
            continue;
        }
        // A boundary is whitespace or the transition between MML and native lyric text.
        let before = text[..at].chars().next_back().unwrap();
        let after = text[at..].chars().next().unwrap();
        if !before.is_whitespace()
            && !after.is_whitespace()
            && before.is_ascii() == after.is_ascii()
        {
            continue;
        }
        if is_mml(left) {
            candidates.push((left, right));
        }
        if is_mml(right) {
            candidates.push((right, left));
        }
    }
    candidates.sort_by_key(|(mml, _)| std::cmp::Reverse(mml.len()));
    candidates.dedup();
    let Some(&(mml, lyrics)) = candidates.first() else {
        return Err("MMLと歌詞を入力してください（例: o5 l2 c d どん）".into());
    };
    if is_mml(lyrics) {
        return Err("MMLと歌詞の区別がつきません".into());
    }
    Ok((mml, lyrics))
}

pub(super) fn replace<'a>(
    music: &mut Music,
    track: usize,
    range: (u64, u64),
    text: &'a str,
) -> Result<&'a str, String> {
    let (input, lyrics) = split(text)?;
    let channel = mml::channel(music.tracks.get(track).ok_or("Invalid track")?, None);
    mml::replace_measure(music, track, range, channel, input)?;
    // Old lyrics on surviving ticks must not leak into a shorter replacement.
    music.tracks[track].events.retain(|e| {
        !(e.status == 0xff && e.meta_type == 5 && (range.0..range.1).contains(&e.tick))
    });
    Ok(lyrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_orders_and_spaces_inside_each_half() {
        for text in [
            "o5 l2 c d ど ん",
            "ど ん o5 l2 c d",
            "o5 l2 c dど ん",
            "ど んo5 l2 c d",
        ] {
            assert_eq!(split(text).unwrap(), ("o5 l2 c d", "ど ん"));
        }
        assert_eq!(split("c4 hello world").unwrap(), ("c4", "hello world"));
        assert!(split("o5 l2").is_err());
        assert!(split("c4 d4").is_err());
    }
}
