//! Only an empty current sound container can be removed from the palette.
use super::*;
use cat_smf_editor::score::pattern::Catalog;

fn target(a: &App) -> Option<(u64, usize)> {
    let music = &a.loaded.as_ref()?.music;
    if music.tracks.len() <= 1 {
        return None;
    }
    let c = Catalog::read(music).ok()?;
    let track_container = || {
        c.tracks
            .iter()
            .find_map(|(&id, &track)| (track == a.track).then_some(id))
    };
    let id = if a.note_mode {
        a.patterns
            .container
            .filter(|id| c.tracks.contains_key(id))
            .or_else(track_container)
    } else {
        track_container()
    }?;
    let track = *c.tracks.get(&id)?;
    (!c.patterns.values().any(|p| p.container == id)).then_some((id, track))
}

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if s == Screen::Global && target(a).is_some() {
        out.push(Command::plain(
            s,
            "delete-current-container-track",
            "Delete current container track",
            &[],
            |a, _| {
                if let Err(error) = delete(a) {
                    a.load_error = Some(error);
                }
            },
        ));
    }
}

fn delete(a: &mut App) -> Result<(), String> {
    let (id, track) = target(a).ok_or("Current container must have no patterns")?;
    let current = a.track;
    a.edit_tracks(|music| {
        let c = Catalog::read(music)?;
        if c.patterns.values().any(|p| p.container == id) {
            return Err("Current container must have no patterns".into());
        }
        music.tracks.remove(track);
        Ok(())
    })?;
    a.patterns.container = None;
    a.track_view.split = None;
    let last = a.loaded.as_ref().unwrap().music.tracks.len() - 1;
    a.select_track(
        current
            .saturating_sub(usize::from(current > track))
            .min(last),
    );
    refresh(a);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_targets_the_current_track_instead_of_the_remembered_notes_container() {
        let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        let first = library::new_container_with(&mut a, None).unwrap();
        let second = library::new_container_with(&mut a, None).unwrap();
        assert_eq!(target(&a).map(|t| t.0), Some(second));
        let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
        a.note_mode = false;
        a.select_track(catalog.tracks[&first]);
        assert_eq!(target(&a).map(|t| t.0), Some(first));
        a.select_track(0);
        assert!(target(&a).is_none());
    }
    #[test]
    fn palette_deletes_only_current_empty_container_and_undo_restores_it() {
        let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        let id = library::new_container_with(&mut a, None).unwrap();
        let before = a.loaded.as_ref().unwrap().music.clone();
        let ctx = eframe::egui::Context::default();
        let command = crate::view::commands::candidates(&a, &ctx)
            .into_iter()
            .find(|c| c.id == "delete-current-container-track")
            .unwrap();
        assert!(crate::view::commands::execute(&mut a, &ctx, &command));
        assert!(!Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .tracks
            .contains_key(&id));
        a.undo_music().unwrap();
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            cat_smf_editor::smf_io::encode(&before).unwrap()
        );
        a.patterns.container = Some(id);
        library::create_for_container(&mut a, id).unwrap();
        assert!(target(&a).is_none());
        assert!(!crate::view::commands::execute(&mut a, &ctx, &command));
    }
}
