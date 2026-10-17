//! Regression coverage through overlay input, file departure, saving and undo.
use super::*;
use crate::view::files::Action;
use cat_smf_editor::{
    edit::{reference_tempo, trill},
    smf_io,
};

struct Document {
    app: crate::app::App,
    root: std::path::PathBuf,
    first: std::path::PathBuf,
    second: std::path::PathBuf,
    original: Vec<u8>,
}
impl Document {
    fn new(name: &str, music: Music) -> Self {
        let root = std::env::temp_dir().join(format!("smf-deferred-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("first.mid");
        let second = root.join("second.mid");
        let original = smf_io::encode(&music).unwrap();
        std::fs::write(&first, &original).unwrap();
        std::fs::write(&second, smf_io::encode(&super::music()).unwrap()).unwrap();
        let mut app = app(music);
        app.config = crate::store::config::Store::open(root.join("status.json"));
        app.saved = crate::store::saved::Document::new(Some(root.join("config")));
        app.saved.load(&first).unwrap();
        app.loaded.as_mut().unwrap().original = first.clone();
        Self {
            app,
            root,
            first,
            second,
            original,
        }
    }
    fn unchanged(&mut self) {
        self.app.remember_loaded().unwrap();
        assert!(!self.app.saved.dirty());
        assert!(!self.app.undo.available());
        assert_eq!(
            smf_io::encode(&self.app.loaded.as_ref().unwrap().music).unwrap(),
            self.original
        );
        assert_eq!(std::fs::read(&self.first).unwrap(), self.original);
        assert!(!self.root.join("config/backup").exists());
    }
    fn drop_second(&mut self, ctx: &egui::Context) {
        let input = egui::RawInput {
            dropped_files: vec![egui::DroppedFile {
                path: Some(self.second.clone()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| crate::view::show(&mut self.app, ctx));
    }
}
impl Drop for Document {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn trill_song() -> Music {
    let mut music = music();
    music.tracks[1] = notes(
        &(0..8)
            .map(|i| (100 + i * 24, 24, 60 + (i % 2) as u8))
            .collect::<Vec<_>>(),
    );
    music
}
fn text(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing {label}"))
}
fn click(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    pos: egui::Pos2,
) -> egui::FullOutput {
    super::edit::click(app, ctx, time, pos, egui::PointerButton::Primary)
}

#[test]
fn browsing_closing_and_dropping_preserve_original_with_or_without_saved_metadata() {
    for existing in [false, true] {
        let mut song = trill_song();
        if existing {
            reference_tempo::initialize(&mut song).unwrap();
            trill::initialize(&mut song).unwrap();
            // Deliberately not regenerated: browsing must preserve these notes too.
            tempo_curve::write(
                &mut song,
                Some(&Curve {
                    version: 1,
                    points: vec![Point { tick: 0, bpm: 80.0 }],
                    controls: vec![],
                }),
            )
            .unwrap();
        }
        let mut d = Document::new(&format!("browse-{existing}"), song);
        let generation = d.app.generation;
        let (ctx, _) = opened_in(&mut d.app, 0);
        d.unchanged();
        press(&mut d.app, &ctx, egui::Key::Escape);
        press(&mut d.app, &ctx, egui::Key::R);
        assert_eq!(d.app.generation, generation);
        d.unchanged();
        assert!(d.app.allow_departure(Action::Close));
        d.drop_second(&ctx);
        assert!(d.app.config.navigation.departure.is_none());
        assert_eq!(
            d.app.loaded.as_ref().unwrap().original,
            std::fs::canonicalize(&d.second).unwrap()
        );
        assert!(!d.app.saved.dirty());
        assert_eq!(std::fs::read(&d.first).unwrap(), d.original);
    }
}

#[test]
fn first_curve_edit_captures_original_references_and_trills_and_undo_removes_initialization() {
    let mut d = Document::new("curve", trill_song());
    let (ctx, output) = opened_in(&mut d.app, 0);
    let pos = circles(&output, TEMPO)[0].0 + egui::vec2(120.0, -20.0);
    click(&mut d.app, &ctx, 11.0, pos);
    let music = &d.app.loaded.as_ref().unwrap().music;
    assert!(tempo_curve::read(music).unwrap().is_some());
    assert_eq!(
        reference_tempo::read(music)
            .unwrap()
            .iter()
            .map(|p| p.bpm)
            .collect::<Vec<_>>(),
        [120.0, 150.0]
    );
    assert!((trill::read(music).unwrap()[0].metadata.interval_seconds - 0.025).abs() < 1e-12);
    assert!(d.app.saved.dirty());
    d.drop_second(&ctx);
    assert!(d.app.config.navigation.departure.is_some());
    press(&mut d.app, &ctx, egui::Key::Escape);
    d.app.undo_music().unwrap();
    at(&mut d.app, &ctx, 12.0, vec![]);
    d.unchanged();
}

#[test]
fn reference_edit_initializes_once_round_trips_and_undo_returns_to_original() {
    let mut d = Document::new("reference", trill_song());
    let (ctx, output) = opened_in(&mut d.app, 0);
    let from = circles(&output, TEMPO)[0].0 + egui::vec2(96.0, 0.0);
    let to = from + egui::vec2(0.0, -20.0);
    super::edit::hold(&mut d.app, &ctx, 11.0, from, to);
    d.unchanged();
    super::edit::release(&mut d.app, &ctx, 11.1, to);
    let edited = smf_io::encode(&d.app.loaded.as_ref().unwrap().music).unwrap();
    assert!(d.app.saved.dirty());
    assert_eq!(
        trill::read(&d.app.loaded.as_ref().unwrap().music)
            .unwrap()
            .len(),
        1
    );
    d.app.saved.save().unwrap();
    assert!(!d.app.saved.dirty());
    let restored = d.app.saved.load(&d.first).unwrap();
    assert_eq!(smf_io::encode(&restored).unwrap(), edited);
    // Save is not a history entry: undo still removes the first edit and metadata.
    d.app.undo_music().unwrap();
    assert!(!d.app.undo.available());
    assert_eq!(
        smf_io::encode(&d.app.loaded.as_ref().unwrap().music).unwrap(),
        d.original
    );
    assert!(d.app.saved.dirty());
}

#[test]
fn trill_dialog_noop_and_cancel_leave_no_initialization_or_undo_then_apply_is_one_edit() {
    let mut d = Document::new("trill", trill_song());
    let (ctx, output) = opened_in(&mut d.app, 0);
    let start = text(&output, "Trill Start");
    click(&mut d.app, &ctx, 11.0, start);
    let output = at(&mut d.app, &ctx, 12.0, vec![]);
    click(&mut d.app, &ctx, 13.0, text(&output, "Apply"));
    d.unchanged();
    let output = at(&mut d.app, &ctx, 14.0, vec![]);
    click(&mut d.app, &ctx, 15.0, text(&output, "Trill Start"));
    let output = at(&mut d.app, &ctx, 16.0, vec![]);
    click(&mut d.app, &ctx, 17.0, text(&output, "Upper"));
    assert!(d.app.saved.dirty());
    assert!(!d.app.undo.available());
    press(&mut d.app, &ctx, egui::Key::Escape);
    d.unchanged();
    let output = at(&mut d.app, &ctx, 18.0, vec![]);
    click(&mut d.app, &ctx, 19.0, text(&output, "Trill Start"));
    let output = at(&mut d.app, &ctx, 20.0, vec![]);
    click(&mut d.app, &ctx, 21.0, text(&output, "Upper"));
    let output = at(&mut d.app, &ctx, 22.0, vec![]);
    click(&mut d.app, &ctx, 23.0, text(&output, "Lower"));
    d.unchanged();
    let output = at(&mut d.app, &ctx, 24.0, vec![]);
    click(&mut d.app, &ctx, 25.0, text(&output, "Upper"));
    let output = at(&mut d.app, &ctx, 26.0, vec![]);
    click(&mut d.app, &ctx, 27.0, text(&output, "Apply"));
    assert!(d.app.trill_edit.is_none());
    assert!(d.app.saved.dirty());
    assert_eq!(
        reference_tempo::read(&d.app.loaded.as_ref().unwrap().music)
            .unwrap()
            .len(),
        2
    );
    d.app.undo_music().unwrap();
    at(&mut d.app, &ctx, 28.0, vec![]);
    d.unchanged();
}
