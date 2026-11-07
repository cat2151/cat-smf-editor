//! どの画面・どの窓でも Ctrl+Shift+P でパレットが開く。
use crate::app::App;
use crate::view::commands::Screen;
use crate::view::patterns::workspace::Pane;
use crate::view::patterns::workspace_tests::{button, context, focus, key, verification_song};
use cat_smf_editor::edit::tracks::new_song;
use eframe::egui::{self, Key, Modifiers};

pub(crate) const ALL: [Screen; 46] = [
    Screen::Global,
    Screen::Main,
    Screen::EditStyle,
    Screen::TrackRename,
    Screen::Settings,
    Screen::Program,
    Screen::Edit,
    Screen::Go,
    Screen::Tracks,
    Screen::Split,
    Screen::Play,
    Screen::Mixer,
    Screen::Layers,
    Screen::Drums,
    Screen::DrumInput,
    Screen::DrumCreate,
    Screen::LayerInput,
    Screen::Effects,
    Screen::Selection,
    Screen::SelectionMenu,
    Screen::ChordTiming,
    Screen::Mml,
    Screen::Note,
    Screen::Lyrics,
    Screen::Trill,
    Screen::Timeline,
    Screen::Overview,
    Screen::Help,
    Screen::Recent,
    Screen::Confirm,
    Screen::Backup,
    Screen::Invalid,
    Screen::Missing,
    Screen::Waiting,
    Screen::Browser,
    Screen::Analysis,
    Screen::Voicevox,
    Screen::Singer,
    Screen::MacroEditor,
    Screen::ParameterPicker,
    Screen::ParameterInput,
    Screen::Patterns,
    Screen::Scene,
    Screen::Song,
    Screen::ArrangementInput,
    Screen::Recording,
];

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

fn root(a: App) -> (App, egui::Context, Vec<egui::ViewportId>) {
    (a, context(), vec![ROOT])
}
fn song() -> App {
    crate::view::tests::app(new_song())
}
/// `f` で画面を開いた `song()` を、root 窓で試す。
fn on_root(f: impl FnOnce(&mut App)) -> (App, egui::Context, Vec<egui::ViewportId>) {
    let mut a = song();
    f(&mut a);
    root(a)
}
fn note(a: &App) -> usize {
    a.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .position(|e| e.status & 0xf0 == 0x90)
        .unwrap()
}

/// `screen` の状態の app と、その画面を出している窓。画面を足すと、この match が教える。
pub(crate) fn fixture(screen: Screen) -> Option<(App, egui::Context, Vec<egui::ViewportId>)> {
    use crate::view::*;
    Some(match screen {
        Screen::Global => return None,
        Screen::Main => on_root(|_| {}),
        Screen::EditStyle => on_root(edit_style::open),
        Screen::TrackRename => on_root(|a| track_rename::begin(a, 0)),
        Screen::Settings => on_root(comma_menu::open),
        Screen::Program => on_root(|a| {
            comma_menu::open(a);
            a.comma_menu.program = true;
        }),
        Screen::Edit => on_root(|a| a.edit_menu.open = true),
        Screen::Go => on_root(|a| a.track_view.go_open = true),
        Screen::Tracks => on_root(|a| {
            a.track_view.go_open = true;
            a.track_view.track_list = true;
        }),
        Screen::Split => on_root(|a| a.track_view.split_open = true),
        Screen::Play => on_root(|a| a.playback.open = true),
        Screen::Mixer => on_root(crate::play::mixer::open),
        Screen::Layers => on_root(layers::open),
        Screen::LayerInput => on_root(layers::palette_tests::begin_input),
        Screen::Drums => root(drums::tests::fixture()),
        Screen::DrumInput => {
            let mut a = drums::tests::fixture();
            a.drum_editor.input = Some(drums::Input::Note {
                text: "36".into(),
                focus: false,
            });
            root(a)
        }
        Screen::DrumCreate => on_root(drums::open),
        Screen::Effects => on_root(|a| crate::play::mixer::open_effects(a, false)),
        Screen::Selection => on_root(event_selection::begin),
        Screen::SelectionMenu => on_root(event_selection::tests::open_menu),
        Screen::ChordTiming => root(piano_roll::chord_timing::tests::open(
            &egui::Context::default(),
        )),
        Screen::Mml => on_root(|a| a.mml_input.open = true),
        Screen::Note => on_root(|a| {
            let event = note(a);
            note_edit::begin(
                a,
                note_edit::Field::Pitch,
                0,
                event,
                egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(80.0, 20.0)),
                "C4",
            );
        }),
        Screen::Lyrics => on_root(voicevox::lyrics::begin),
        Screen::Trill => {
            let mut a = crate::view::tests::app(trill_edit::tests::song());
            let event = a.loaded.as_ref().unwrap().music.tracks[1]
                .events
                .iter()
                .position(cat_smf_editor::edit::trill::owned)
                .unwrap();
            trill_edit::begin(&mut a, 1, event).unwrap();
            root(a)
        }
        Screen::Timeline => on_root(piano_roll::open),
        Screen::Overview => on_root(track_overview::open),
        Screen::Help => {
            let (mut a, c, _) = on_root(|_| {});
            a.help_open = true;
            (a, c, vec![help::viewport()])
        }
        Screen::Recent => on_root(recent::commands::open),
        Screen::Confirm => on_root(|a| a.config.navigation.departure = Some(files::Action::New)),
        Screen::Backup => on_root(|a| a.config.navigation.backup_notice = true),
        Screen::Invalid => on_root(|a| a.invalid_smf_name = Some("invalid.mid".into())),
        Screen::Missing => on_root(|a| a.missing_last_mid = Some("missing.mid".into())),
        Screen::Waiting => {
            let (mut a, c, ids) = on_root(|_| {});
            busy::request(&mut a, &c, busy::Task::AddTrack);
            (a, c, ids)
        }
        Screen::Browser => on_root(|a| a.player_selection.open = true),
        Screen::Analysis => {
            let (a, c, _) = on_root(analysis::toggle);
            (a, c, vec![analysis::viewport(0)])
        }
        Screen::Voicevox => on_root(voicevox::open),
        Screen::Singer => on_root(voicevox::singer::open),
        Screen::MacroEditor => on_root(macro_editor::tests::open),
        Screen::ParameterPicker => {
            let mut a = parameter_editor::tests::app();
            parameter_editor::tests::open_picker(&mut a);
            root(a)
        }
        Screen::ParameterInput => {
            let mut a = parameter_editor::tests::app();
            let c = context();
            key(&mut a, &c, ROOT, Key::J, Modifiers::NONE);
            key(&mut a, &c, ROOT, Key::Enter, Modifiers::NONE);
            (a, c, vec![ROOT])
        }
        Screen::Patterns => {
            let (mut a, c, _, _, _) = verification_song();
            a.edit_tracks(|m| {
                cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ())
            })
            .unwrap();
            patterns::library::add(&mut a, &c);
            (a, c, vec![ROOT])
        }
        Screen::Scene => {
            let (a, c, _, _, _) = verification_song();
            (a, c, vec![Pane::Scene.viewport()])
        }
        Screen::Song => {
            let (a, c, _, _, _) = verification_song();
            (a, c, vec![Pane::Song.viewport()])
        }
        Screen::ArrangementInput => {
            let (mut a, c, _, _, _) = verification_song();
            button(&mut a, &c, Pane::Scene, "create");
            (a, c, vec![Pane::Scene.viewport()])
        }
        Screen::Recording => {
            let (mut a, c, _, _, _) = verification_song();
            key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
            let keyboard = recording::window::viewport();
            key(&mut a, &c, keyboard, Key::Comma, Modifiers::CTRL);
            key(&mut a, &c, keyboard, Key::J, Modifiers::CTRL);
            let ids = vec![
                keyboard,
                recording::settings_viewport(),
                recording::piano_viewport(),
                recording::capture_viewport(),
            ];
            (a, c, ids)
        }
    })
}

#[test]
fn every_screen_opens_the_palette_in_its_own_window() {
    for screen in ALL {
        let Some((mut a, c, windows)) = fixture(screen) else {
            continue;
        };
        for id in windows {
            focus(&c, id);
            key(&mut a, &c, id, Key::P, Modifiers::CTRL | Modifiers::SHIFT);
            assert_eq!(
                a.palette.origin(),
                Some((id, screen)),
                "{screen:?} {id:?}: {:?}",
                a.load_error
            );
            key(&mut a, &c, id, Key::Escape, Modifiers::NONE);
            assert!(!a.palette.is_open(), "{screen:?} {id:?}");
        }
    }
}

/// New native windows must use the presentation gateway and get a palette fixture.
#[test]
fn every_window_is_tried() {
    fn sources(dir: &std::path::Path, out: &mut Vec<(std::path::PathBuf, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|x| x == "rs")
                && !path.to_string_lossy().contains("test")
            {
                out.push((path.clone(), std::fs::read_to_string(path).unwrap()));
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![];
    sources(&src, &mut files);
    let native = regex::Regex::new(r"presentation::Native::\w+\.show\(").unwrap();
    let mut registered = 0;
    for (path, source) in files {
        if path.ends_with("presentation.rs") {
            continue;
        }
        assert!(
            !source.contains(".show_viewport_immediate("),
            "native gateway bypass: {}",
            path.display()
        );
        registered += native.find_iter(&source).count();
    }
    // Analysis, help, Scene/Song, keyboard, Capture, settings, piano.
    assert_eq!(registered, 7, "add a fixture for each new native window");
}
