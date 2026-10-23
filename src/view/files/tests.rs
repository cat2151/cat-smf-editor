use super::*;
use cat_smf_editor::{
    model::{Event, Music, Track},
    smf_io,
};
mod auto_save;
mod auto_save_activation;
mod auto_save_departure;
mod clean_close;
#[path = "invalid_smf_tests.rs"]
mod invalid_smf;
mod native_selection;

struct Fixture {
    app: App,
    root: PathBuf,
    first: PathBuf,
    second: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fixture(name: &str) -> Fixture {
    let root = std::env::temp_dir().join(format!("smf-view-{name}-{}", std::process::id()));
    std::fs::create_dir_all(root.join("sources")).unwrap();
    let music = Music {
        format: 0,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    tick: 480,
                    delta: 480,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
                Event {
                    tick: 480,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    };
    let first = root.join("sources/first.mid");
    let second = root.join("sources/second.mid");
    for path in [&first, &second] {
        std::fs::write(path, smf_io::encode(&music).unwrap()).unwrap();
    }
    let mut app = crate::view::tests::app(music);
    app.loaded = None;
    app.saved = crate::store::saved::Document::new(Some(root.join("config")));
    app.config = crate::store::config::Store::open(root.join("status.json"));
    app.open_mid(first.clone());
    assert!(app.load_error.is_none());
    Fixture {
        app,
        root,
        first,
        second,
    }
}
fn edit(app: &mut App) {
    app.edit_music(|music| {
        music.tracks[0]
            .events
            .iter_mut()
            .find(|event| event.status == 0x90)
            .unwrap()
            .data[0] = 67;
        Ok(())
    })
    .unwrap();
}
fn press(app: &mut App, ctx: &egui::Context, key: Key) -> egui::FullOutput {
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: if key == Key::Comma {
                Modifiers::CTRL
            } else {
                Modifiers::NONE
            },
        }],
        ..Default::default()
    };
    ctx.run(input, |ctx| crate::view::show(app, ctx))
}

fn save(app: &mut App, ctx: &egui::Context) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::S,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn set_note(app: &mut App, note: u8) {
    app.edit_music(|music| {
        music.tracks[0].events[0].data[0] = note;
        Ok(())
    })
    .unwrap();
}

fn first_note(f: &Fixture) -> u8 {
    smf_io::load(&f.first).unwrap().tracks[0].events[0].data[0]
}

fn backups(f: &Fixture) -> usize {
    std::fs::read_dir(f.root.join("config/backup")).map_or(0, |dir| dir.count())
}

#[test]
fn backup_notice_is_daily_and_survives_restart_without_losing_navigation() {
    let mut f = fixture("daily-backup-tip");
    let ctx = egui::Context::default();
    set_note(&mut f.app, 67);
    save(&mut f.app, &ctx);
    assert!(f.app.config.navigation.backup_notice);
    let date = f.app.config.navigation.backup_tip_date.clone().unwrap();
    press(&mut f.app, &ctx, Key::Escape);
    assert!(!f.app.config.navigation.backup_notice);
    set_note(&mut f.app, 69);
    save(&mut f.app, &ctx);
    assert!(!f.app.config.navigation.backup_notice);
    assert_eq!(first_note(&f), 69);
    f.app.config = crate::store::config::Store::open(f.root.join("status.json"));
    assert_eq!(
        f.app.config.navigation.backup_tip_date.as_ref(),
        Some(&date)
    );
    assert_eq!(
        f.app.config.navigation.last_mid.as_ref(),
        Some(&std::fs::canonicalize(&f.first).unwrap())
    );
    f.app.config.navigation.backup_tip_date = Some("2000-01-01".into());
    set_note(&mut f.app, 71);
    save(&mut f.app, &ctx);
    assert!(f.app.config.navigation.backup_notice);
    assert_eq!(
        f.app.config.navigation.backup_tip_date.as_ref(),
        Some(&date)
    );
    press(&mut f.app, &ctx, Key::Enter);
    assert!(!f.app.config.navigation.backup_notice);
    assert_eq!(backups(&f), 3);
}

#[test]
fn save_writes_at_once_and_skips_unchanged_documents() {
    let mut f = fixture("save-at-once");
    let ctx = egui::Context::default();
    let original = std::fs::read(&f.first).unwrap();
    save(&mut f.app, &ctx);
    assert!(f.app.config.navigation.departure.is_none());
    assert!(!f.app.config.navigation.backup_notice);
    assert_eq!(std::fs::read(&f.first).unwrap(), original);
    assert_eq!(backups(&f), 0);
    edit(&mut f.app);
    save(&mut f.app, &ctx);
    assert!(f.app.config.navigation.departure.is_none());
    assert_eq!(first_note(&f), 67);
    assert!(!f.app.saved.dirty());
    assert_eq!(backups(&f), 1);
}

#[test]
fn dropping_a_file_confirms_dirty_switch_and_escape_keeps_current_edits() {
    let mut f = fixture("drop");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    let first = f.app.loaded.as_ref().unwrap().original.clone();
    let _ = ctx.run(
        egui::RawInput {
            dropped_files: vec![egui::DroppedFile {
                path: Some(f.second.clone()),
                ..Default::default()
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(&mut f.app, ctx),
    );
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::LoadWithProgress(_))
    ));
    press(&mut f.app, &ctx, Key::Escape);
    assert_eq!(f.app.loaded.as_ref().unwrap().original, first);
    assert!(f.app.saved.dirty());
    f.app.open_mid(f.second.clone());
    press(&mut f.app, &ctx, Key::N);
    assert_eq!(
        f.app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&f.second).unwrap()
    );
    assert_eq!(
        smf_io::load(&f.first).unwrap().tracks[0].events[0].data[0],
        60
    );
    f.app.open_mid(f.first.clone());
    edit(&mut f.app);
    f.app.open_mid(f.second.clone());
    press(&mut f.app, &ctx, Key::Y);
    assert_eq!(
        smf_io::load(&f.first).unwrap().tracks[0].events[0].data[0],
        67
    );
}

#[test]
fn edited_exit_confirms_and_each_key_has_the_requested_effect() {
    for key in [Key::Y, Key::N, Key::Escape] {
        let mut f = fixture(&format!("exit-{key:?}"));
        let ctx = egui::Context::default();
        assert!(f.app.allow_departure(Action::Close));
        assert!(f.app.config.navigation.departure.is_none());
        assert!(!f.app.config.navigation.close_approved);
        edit(&mut f.app);
        assert!(!f.app.allow_departure(Action::Close));
        let output = press(&mut f.app, &ctx, key);
        assert_eq!(f.app.config.navigation.close_approved, key != Key::Escape);
        assert_eq!(
            smf_io::load(&f.first).unwrap().tracks[0].events[0].data[0],
            if key == Key::Y { 67 } else { 60 }
        );
        assert_eq!(
            output
                .viewport_output
                .values()
                .any(|v| v.commands.contains(&egui::ViewportCommand::Close)),
            key != Key::Escape
        );
    }
}

#[test]
fn failed_save_stays_in_confirmation_and_can_cancel_or_retry() {
    let mut f = fixture("failed-save");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    let original = std::fs::read(&f.first).unwrap();
    std::fs::write(&f.first, b"external").unwrap();
    save(&mut f.app, &ctx);
    assert!(f.app.config.navigation.departure_error.is_some());
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::Save)
    ));
    assert!(f.app.saved.dirty());
    std::fs::write(&f.first, original).unwrap();
    press(&mut f.app, &ctx, Key::Y);
    assert!(f.app.config.navigation.departure.is_none());
    assert!(!f.app.saved.dirty());
}

#[test]
fn invalid_switch_does_not_drop_current_document_or_its_pending_edits() {
    let mut f = fixture("invalid");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    std::fs::write(&f.second, b"invalid MIDI").unwrap();
    let first = f.app.loaded.as_ref().unwrap().original.clone();
    f.app.open_mid(f.second.clone());
    press(&mut f.app, &ctx, Key::N);
    assert!(f.app.load_error.is_some());
    assert_eq!(f.app.loaded.as_ref().unwrap().original, first);
    assert!(f.app.saved.dirty());
}

#[test]
fn startup_argument_takes_priority_and_no_argument_restores_last_mid() {
    let mut f = fixture("startup");
    f.app.loaded = None;
    f.app.config.navigation.last_mid = Some(f.first.clone());
    f.app.open_initial(Some(f.second.clone()));
    assert!(!f.app.file_open_requested);
    assert_eq!(
        f.app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&f.second).unwrap()
    );
    f.app.save_session();
    f.app.loaded = None;
    f.app.config = crate::store::config::Store::open(f.root.join("status.json"));
    f.app.open_initial(None);
    assert!(!f.app.file_open_requested);
    assert_eq!(
        f.app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&f.second).unwrap()
    );
}

#[test]
fn native_close_request_opens_confirmation_and_cancel_keeps_the_app_open() {
    let mut f = fixture("native-close");
    edit(&mut f.app);
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let output = ctx.run(input, |ctx| {
        crate::view::show(&mut f.app, ctx);
        f.app.close(ctx);
    });
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::Close)
    ));
    assert!(output
        .viewport_output
        .values()
        .any(|v| v.commands.contains(&egui::ViewportCommand::CancelClose)));
    press(&mut f.app, &ctx, Key::Escape);
    assert!(f.app.config.navigation.departure.is_none());
    assert!(!f.app.config.navigation.close_approved);
}

#[test]
fn mixer_changes_are_confirmed_saved_and_restored_with_the_same_gain() {
    let mut f = fixture("mixer");
    let ctx = egui::Context::default();
    crate::play::mixer::volume::set(&mut f.app, 0, 0.5).unwrap();
    assert!(!f.app.allow_departure(Action::Close));
    assert!(f.app.saved.dirty());
    press(&mut f.app, &ctx, Key::Y);
    let stored = smf_io::load(&f.first).unwrap();
    assert_eq!(
        crate::play::track_volume::read(&stored.tracks[0]).unwrap(),
        0.5
    );
    f.app.loaded = None;
    f.app.open_initial(None);
    assert_eq!(f.app.audition.gain(0), 0.5);
}

#[test]
fn drop_same_file_confirms_edits_and_reloads_when_clean() {
    let mut f = fixture("same-file");
    let ctx = egui::Context::default();
    edit(&mut f.app);
    f.app.open_mid(f.first.clone());
    assert!(matches!(
        f.app.config.navigation.departure,
        Some(Action::Load(_))
    ));
    press(&mut f.app, &ctx, Key::N);
    assert_eq!(
        f.app.loaded.as_ref().unwrap().music.tracks[0].events[0].data[0],
        60
    );
    let mut external = smf_io::load(&f.first).unwrap();
    external.tracks[0].events[0].data[0] = 71;
    std::fs::write(&f.first, smf_io::encode(&external).unwrap()).unwrap();
    f.app.open_mid(f.first.clone());
    assert!(f.app.config.navigation.departure.is_none());
    assert_eq!(
        f.app.loaded.as_ref().unwrap().music.tracks[0].events[0].data[0],
        71
    );
}

#[test]
fn untitled_first_save_shows_no_backup_tip() {
    let mut f = fixture("backup-tip-untitled");
    let ctx = egui::Context::default();
    f.app.new_smf_unchecked();
    assert!(!f.app.allow_departure(Action::Close));
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut f.app, ctx));
    assert!(!f.app.config.navigation.backup_tip_visible);
    assert!(f.app.config.navigation.backup_tip_date.is_none());
    press(&mut f.app, &ctx, Key::Escape);
    let path = f.root.join("sources/new.mid");
    assert_eq!(f.app.saved.save_as(&path), Ok(false));
    assert_eq!(backups(&f), 0);
}
