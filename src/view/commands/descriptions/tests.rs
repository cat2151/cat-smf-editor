use super::*;
use crate::view::commands::{self, Screen};

pub(crate) enum Work {
    Locate,
    Read,
    Bundle,
}

thread_local! {
    static ACTIVITY: std::cell::Cell<[usize; 3]> = const { std::cell::Cell::new([0; 3]) };
}

pub(crate) fn activity() -> [usize; 3] {
    ACTIVITY.get()
}

pub(crate) fn record(work: Work) {
    let mut counts = activity();
    counts[work as usize] += 1;
    ACTIVITY.set(counts);
}

pub(crate) const PREVIEW: &str =
    "Patternを試聴します。配置先はSongカーソル位置、conductorのテンポに従います。";

/// Search/display assertions use synthetic prose so users can freely edit the real file.
pub(crate) fn document() -> String {
    let mut text = String::new();
    for (id, name, tooltip) in [
        ("file.open", "", "ファイルを開きます。"),
        ("file.new", "", "説明本文。"),
        ("pattern.preview", "name = \"Preview pattern\"\n", PREVIEW),
        (
            "pattern.record",
            "name = \"録音\"\n",
            "Patternを録音します。",
        ),
        (
            "song.open",
            "name = \"Pattern / Song\"\n",
            "Pattern・Scene・Songを配置します。",
        ),
    ] {
        text.push_str(&format!(
            "[[command]]\nids = [\"{id}\"]\n{name}tooltip = \"{tooltip}\"\n\n"
        ));
    }
    text
}

/// What the palette shows once the bundled wording is applied.
pub(crate) fn displayed(commands: Vec<Command>) -> Vec<Command> {
    let catalog = Catalog {
        entries: bundled(),
        ..Catalog::default()
    };
    catalog.apply(commands)
}

fn command(screen: Screen, id: &str) -> Command {
    Command::plain(screen, id, &[], |_, _| {})
}

fn shared(entries: &Entries, id: &str) -> Text {
    entries.commands[&(id.to_owned(), None)].clone()
}

#[test]
fn definitions_keep_tooltips_verbatim_and_tolerate_editor_encodings() {
    assert!(bundled().commands.len() > 300);
    let entries = parse(&document()).unwrap();
    assert_eq!(entries.commands.len(), 5);
    assert_eq!(
        shared(&entries, "file.open").tooltip.as_deref(),
        Some("ファイルを開きます。")
    );
    assert_eq!(shared(&entries, "file.open").name, None);
    let source = document().replace("ファイルを開きます。", "一行目 二行目\\n\\n次の段落");
    assert_eq!(
        shared(&parse(&source).unwrap(), "file.open")
            .tooltip
            .as_deref(),
        Some("一行目 二行目\n\n次の段落")
    );
    let edited = format!("\u{feff}{}", source.replace('\n', "\r\n"));
    assert_eq!(parse(&edited).unwrap(), parse(&source).unwrap());
}

#[test]
fn one_item_serves_every_id_and_a_screen_entry_wins_per_field() {
    let catalog = Catalog {
        entries: Arc::new(
            parse(
                r#"
[[command]]
ids = ["app.undo", "edit.undo"]
name = "元に戻す"
tooltip = "共有説明。"

[[command]]
ids = ["app.undo"]
screen = "Song"
name = "Undo"

[[command]]
ids = ["placement.play"]
screen = "Scene"
tooltip = "Scene説明。"
"#,
            )
            .unwrap(),
        ),
        ..Catalog::default()
    };
    let commands = catalog.apply(vec![
        command(Screen::Global, "app.undo"),
        command(Screen::Main, "edit.undo"),
        command(Screen::Song, "app.undo"),
        command(Screen::Scene, "placement.play"),
        command(Screen::Song, "placement.play"),
        command(Screen::Main, "unlisted"),
    ]);
    let shown: Vec<_> = commands
        .iter()
        .map(|c| (c.name.as_str(), c.description()))
        .collect();
    assert_eq!(
        shown,
        [
            ("元に戻す", "共有説明。"),
            ("元に戻す", "共有説明。"),
            ("Undo", "共有説明。"),
            ("placement.play", "Scene説明。"),
            ("placement.play", ""),
            ("unlisted", ""),
        ]
    );
}

#[test]
fn search_words_follow_the_same_inheritance_and_ignore_case() {
    let catalog = Catalog {
        entries: Arc::new(
            parse(
                r#"
[[command]]
ids = ["shared"]
search = "Shared WORD"

[[command]]
ids = ["shared"]
screen = "Song"
search = "song word"

[[command]]
ids = ["shared"]
screen = "Scene"
name = "Scene name"
"#,
            )
            .unwrap(),
        ),
        ..Catalog::default()
    };
    let commands = catalog.apply(vec![
        command(Screen::Main, "shared"),
        command(Screen::Song, "shared"),
        command(Screen::Scene, "shared"),
    ]);
    let searched: Vec<_> = commands.iter().map(|c| c.search.as_str()).collect();
    assert_eq!(searched, ["shared word", "song word", "shared word"]);
}

#[test]
fn every_screen_has_a_name_and_a_missing_one_shows_the_variant() {
    assert_eq!(bundled().screens.len(), 46);
    let shown = displayed(vec![command(Screen::Song, "x")]);
    let song = &bundled().screens[&Screen::Song];
    assert_eq!(shown[0].label(), format!("{song}: x"));
    let catalog = Catalog {
        entries: Arc::new(parse(&document()).unwrap()),
        ..Catalog::default()
    };
    let commands = catalog.apply(vec![
        command(Screen::Song, "x"),
        command(Screen::Global, "x"),
    ]);
    assert_eq!(commands[0].label(), "Song: x");
    assert_eq!(commands[1].label(), "x");
}

#[test]
fn text_id_binds_generated_rows_to_one_definition() {
    let catalog = Catalog {
        entries: Arc::new(parse("[[command]]\nids = [\"family\"]\nname = \"族\"\n").unwrap()),
        ..Catalog::default()
    };
    let commands = catalog.apply(vec![
        command(Screen::Main, "family-1").text_id("family"),
        command(Screen::Main, "family-2").text_id("family"),
    ]);
    assert_eq!(commands[0].name, "族");
    assert_eq!(commands[1].name, "族");
}

#[test]
fn broken_entries_fail_with_line_numbers_instead_of_silently_disappearing() {
    for (from, to, reason) in [
        ("ids = [\"file.new\"]", "ids = [\"file.open\"]", "重複"),
        (
            "ids = [\"file.open\"]",
            "ids = [\"file.open\", \"file.open\"]",
            "重複",
        ),
        ("ids = [\"file.open\"]", "ids = []", "command ID"),
        ("ids = [\"file.open\"]", "ids = [\"\"]", "command ID"),
        ("ids = [\"file.open\"]\n", "", "ids"),
        ("ファイルを開きます。", "", "空"),
        ("tooltip = \"ファイルを開きます。\"\n", "", "どれかが必要"),
        (
            "ids = [\"file.open\"]",
            "ids = [\"file.open\"]\nsearch = \"\"",
            "空",
        ),
        (
            "[[command]]\nids = [\"file.open\"]",
            "[screens]\nMain = \"\"\n\n[[command]]\nids = [\"file.open\"]",
            "空",
        ),
        (
            "[[command]]\nids = [\"file.open\"]",
            "[screens]\nTypo = \"x\"\n\n[[command]]\nids = [\"file.open\"]",
            "unknown variant",
        ),
        (
            "ids = [\"file.open\"]",
            "ids = [\"file.open\"]\nscreen = \"Typo\"",
            "unknown variant",
        ),
        (
            "ids = [\"file.open\"]",
            "ids = [\"file.open\"]\ntooltp = \"x\"",
            "tooltp",
        ),
        ("ids = [\"file.open\"]", "ids = [\"file.open\"", "]"),
    ] {
        let source = document().replacen(from, to, 1);
        assert_ne!(source, document(), "{from}");
        let error = parse(&source).unwrap_err();
        assert!(error.contains(reason), "{from}: {error}");
        assert!(
            error.split(':').next().unwrap().parse::<usize>().is_ok(),
            "{error}"
        );
    }
    assert!(parse("").is_err());
    let same_id_elsewhere = format!(
        "{}[[command]]\nids = [\"file.open\"]\nscreen = \"Song\"\nname = \"x\"\n",
        document()
    );
    assert!(parse(&same_id_elsewhere).is_ok());
}

#[test]
fn scoped_play_and_real_dynamic_rows_get_their_own_explanations() {
    let mut catalog = Catalog::default();
    let mut error = None;
    catalog.reload(&mut error);
    assert!(error.is_none(), "{error:?}");
    let key = |screen| ("placement.play".to_owned(), Some(screen));
    for screen in [Screen::Scene, Screen::Song] {
        let commands = catalog.apply(vec![command(screen, "placement.play")]);
        assert_eq!(
            Some(commands[0].description()),
            catalog.entries.commands[&key(screen)].tooltip.as_deref()
        );
    }
    assert!(
        catalog.apply(vec![command(Screen::Main, "placement.play")])[0]
            .description()
            .is_empty()
    );
    let mut app = crate::view::palette::tests::app();
    app.recent
        .history
        .entries
        .push(crate::store::recent::Entry {
            path: "X:/scores/example.mid".into(),
            opened: 0,
            title: String::new(),
            seconds: 0.0,
            tracks: 1,
        });
    for (screen, id, family) in [
        (Screen::Tracks, "track.entry-0", "track.entry"),
        (
            Screen::Recent,
            "recent.entry-X:/scores/example.mid",
            "recent.entry",
        ),
    ] {
        let commands = catalog.apply(commands::definitions(&app, screen));
        let command = commands.iter().find(|c| c.id == id).unwrap();
        assert_eq!(
            Some(command.description()),
            shared(&catalog.entries, family).tooltip.as_deref()
        );
    }
}

#[test]
fn reload_is_atomic_preserves_last_good_data_and_recovers_from_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    let mut catalog = Catalog::at(path.clone());
    let mut error = None;
    catalog.reload(&mut error);
    assert!(error.is_none());
    assert_eq!(catalog.entries, bundled());
    assert!(!path.exists());
    std::fs::write(&path, document()).unwrap();
    catalog.reload(&mut error);
    assert!(error.is_none());
    let original = catalog.entries.clone();
    std::fs::write(
        &path,
        document().replace("ids = [\"file.new\"]", "ids = [\"file.open\"]"),
    )
    .unwrap();
    catalog.reload(&mut error);
    assert!(error.as_ref().unwrap().contains("重複"));
    assert_eq!(catalog.entries, original);
    std::fs::write(
        &path,
        document().replace("ファイルを開きます。", "変更後の説明。"),
    )
    .unwrap();
    // A different error must not be cleared by repairing this file.
    error = Some("別の処理のエラー".into());
    catalog.reload(&mut error);
    assert_eq!(error.as_deref(), Some("別の処理のエラー"));
    assert_eq!(
        shared(&catalog.entries, "file.open").tooltip.as_deref(),
        Some("変更後の説明。")
    );
    std::fs::remove_file(&path).unwrap();
    catalog.reload(&mut error);
    assert_eq!(error.as_deref(), Some("別の処理のエラー"));
    assert_eq!(catalog.entries, bundled());
    std::fs::write(&path, document()).unwrap();
    catalog.reload(&mut error);
    assert_eq!(error.as_deref(), Some("別の処理のエラー"));
    assert_eq!(catalog.entries, original);
}

#[test]
fn release_without_external_file_uses_bundled_prose_for_search_and_display() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    let mut catalog = Catalog::at(path.clone());
    let mut error = Some("無関係なエラー".into());
    catalog.reload(&mut error);
    assert_eq!(error.as_deref(), Some("無関係なエラー"));
    assert_eq!(catalog.entries, bundled());
    assert!(!path.exists());
    let commands = catalog.apply(vec![command(Screen::Global, "pattern.preview")]);
    let bundle = bundled();
    let prose = shared(&bundle, "pattern.preview").tooltip.unwrap();
    assert_eq!(commands[0].description(), prose);
    let mut search = commands::SearchQuery::default();
    search.update(&prose);
    assert!(commands[0].matches_query(&search));
    std::fs::write(&path, "broken").unwrap();
    catalog.reload(&mut error);
    assert!(error
        .as_ref()
        .unwrap()
        .contains(&path.display().to_string()));
    assert_eq!(catalog.entries, bundled());
    std::fs::remove_file(&path).unwrap();
    catalog.reload(&mut error);
    assert!(error.is_none());
    assert_eq!(catalog.entries, bundled());
}

#[test]
fn external_location_exists_only_for_executables_under_the_project_target() {
    let project = Path::new("X:/source/editor");
    for executable in [
        "X:/source/editor/target/debug/editor.exe",
        "X:/source/editor/target/release/editor.exe",
        "X:/source/editor/target/debug/deps/editor-test.exe",
    ] {
        assert_eq!(
            source_path(Path::new(executable), project),
            Some(project.join("config/command-palette.toml"))
        );
    }
    for executable in [
        "X:/portable/editor.exe",
        "X:/source/editor/editor.exe",
        "X:/source/editor/target-other/editor.exe",
    ] {
        assert_eq!(source_path(Path::new(executable), project), None);
    }
}

#[test]
fn a_definition_file_beside_a_portable_executable_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let portable = dir.path().join("portable");
    std::fs::create_dir_all(&portable).unwrap();
    let executable = portable.join("editor.exe");
    let neighbor = portable.join("command-palette.toml");
    let reads = activity()[Work::Read as usize];
    for text in [
        "broken",
        "[[command]]\nids = [\"file.open\"]\nname = \"隣\"\n",
    ] {
        std::fs::write(&neighbor, text).unwrap();
        let mut catalog = Catalog::default();
        let mut error = None;
        catalog.load(source_path(&executable, &project), &mut error);
        assert!(error.is_none(), "{error:?}");
        assert_eq!(catalog.entries, bundled());
    }
    assert_eq!(activity()[Work::Read as usize], reads);
}
