use super::App;
use cat_smf_editor::edit::parameter_curve::{self, Curve};

impl App {
    pub(crate) fn create_parameter_track(
        &mut self,
        owner: usize,
        curve: Curve,
    ) -> Result<usize, String> {
        let original = &self.loaded.as_ref().ok_or("No SMF is loaded")?.music;
        let mut next = original.clone();
        let track = parameter_curve::create(&mut next, owner, curve)?;
        if cat_smf_editor::smf_io::encode(original)? != cat_smf_editor::smf_io::encode(&next)? {
            self.edit_tracks(|music| {
                *music = next;
                Ok(())
            })?;
        }
        self.select_track(track);
        Ok(track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_smf_editor::edit::{
        parameter_curve::{Point, Target, ValueSpace},
        tracks,
    };
    #[test]
    fn parameter_creation_is_one_undo_and_duplicate_open_does_not_dirty() {
        let music = tracks::new_song();
        let before = cat_smf_editor::smf_io::encode(&music).unwrap();
        let mut app = crate::view::tests::app(music);
        let curve = Curve {
            version: 1,
            target: Target {
                group: 0,
                layer: 1,
                plugin_format: "CLAP".into(),
                plugin_id: "fixture".into(),
                parameter_id: "101".into(),
            },
            value_space: ValueSpace::Plain,
            minimum: 0.0,
            maximum: 1.0,
            name: "Cutoff".into(),
            path: "Filter".into(),
            points: vec![Point {
                tick: 120,
                value: 0.7,
            }],
            controls: vec![],
        };
        assert_eq!(app.create_parameter_track(0, curve.clone()).unwrap(), 1);
        let created = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
        assert_ne!(created, before);
        assert!(app.undo.available());
        assert_eq!(app.loaded.as_ref().unwrap().patches.len(), 2);
        let generation = app.generation;
        assert_eq!(app.create_parameter_track(0, curve).unwrap(), 1);
        assert_eq!(app.generation, generation);
        app.undo_music().unwrap();
        assert_eq!(
            cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
            before
        );
        assert!(!app.undo.available());
    }
}
