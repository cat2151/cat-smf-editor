use super::*;

/// Every repeating Loop pattern, as an ordinary pattern of the same body, each on its own lane
/// after the end of the Song; the phrases stop and those measures of the Song repeat.
pub(in crate::view::recording) fn to_song(a: &mut App, s: &mut Session) -> Result<(), String> {
    use cat_smf_editor::score::{
        pattern::operations,
        scene::{self, arrangement},
    };
    let loops: Vec<u64> = s
        .sounds
        .iter()
        .filter_map(|x| match x.looper {
            Looper::Looping { pattern, .. } => Some(pattern),
            _ => None,
        })
        .collect();
    if loops.is_empty() {
        return Ok(());
    }
    let mut added = (0, 0, None);
    a.edit_tracks(|music| {
        let mut parts = vec![];
        for &pattern in &loops {
            parts.push(PatternOrScene::Pattern(operations::placeable(
                music, pattern,
            )?));
        }
        let c = scene::read(music)?;
        let mut at = 1;
        for p in &c.song {
            at = at.max(p.measure + arrangement::length(&c, p.part)?);
        }
        let mut length = 0;
        for (i, &part) in parts.iter().enumerate() {
            let lane = match c.song_lanes.get(i) {
                Some(&lane) => lane,
                None => arrangement::add_lane(music, None)?,
            };
            arrangement::place(music, None, lane, at, Some(part))?;
            length = length.max(arrangement::length(&scene::read(music)?, part)?);
        }
        added = (
            arrangement::measure_tick(music, None, at)?,
            arrangement::measure_tick(music, None, at + length)?,
            Some((at, parts[0])),
        );
        Ok(())
    })?;
    for i in 0..s.sounds.len() {
        silence(s, i);
        s.sounds[i].looper = Looper::Off;
    }
    a.config.select_phrase_loop(false);
    if let Some((measure, part)) = added.2 {
        let song = a.patterns.workspace.selection_mut(Pane::Song);
        song.row = 0;
        song.measure = measure;
        song.choice = Some(part);
        song.reveal_choice = true;
    }
    crate::play::transport::repeat::start_song(a, (added.0, added.1));
    Ok(())
}
