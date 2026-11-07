use super::*;
use commands::descriptions::tests::{document, PREVIEW};
use commands::descriptions::Catalog;

#[test]
fn startup_and_closed_palette_do_no_description_io_or_bundle_parsing() {
    use commands::descriptions::tests::activity;
    let before = activity();
    let mut a = app();
    let ctx = egui::Context::default();
    assert_eq!(activity(), before);
    frame(&mut a, &ctx, vec![]);
    let _ = commands::candidates(&a, &ctx);
    assert_eq!(activity(), before);
    open(&mut a, &ctx);
    let opened = activity();
    assert!(
        opened[0] > before[0],
        "opening must resolve the external location"
    );
    assert_eq!(opened[1], before[1] + 1, "one read per opening");
    assert!(
        opened[2] > before[2],
        "bundled fallback is initialized lazily"
    );
    query(&mut a, &ctx, "preview pattern");
    press(&mut a, &ctx, Key::F1);
    draw(&mut a, &ctx, vec![]);
    assert_eq!(activity(), opened);
    press(&mut a, &ctx, Key::Escape);
    press(&mut a, &ctx, Key::Escape);
    assert_eq!(activity(), opened);
    open(&mut a, &ctx);
    assert_eq!(activity()[1], opened[1] + 1);
}

#[test]
fn reopening_reloads_search_and_tooltip_together_and_keeps_open_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    std::fs::write(
        &path,
        document().replace(PREVIEW, "再読込専用語の試聴です。"),
    )
    .unwrap();
    let mut a = app();
    a.palette.descriptions = Catalog::at(path.clone());
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "再読込専用語 shichou");
    assert_eq!(listed(&mut a), ["pattern.preview"]);
    press(&mut a, &ctx, Key::F1);
    assert!(has_text(
        &draw(&mut a, &ctx, vec![]).shapes,
        "再読込専用語の試聴です。"
    ));
    assert!(!a.help_open);
    assert!(a.playback.pending.is_none());

    // Even rebuilding candidates while open must retain this opening's prose.
    std::fs::write(
        &path,
        document().replace(PREVIEW, "別の説明に保存しました。"),
    )
    .unwrap();
    a.generation += 1;
    draw(&mut a, &ctx, vec![]);
    assert_eq!(listed(&mut a), ["pattern.preview"]);
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.palette.is_open());
    open(&mut a, &ctx);
    query(&mut a, &ctx, "別の説明");
    assert_eq!(listed(&mut a), ["pattern.preview"]);
    press(&mut a, &ctx, Key::F1);
    assert!(has_text(
        &draw(&mut a, &ctx, vec![]).shapes,
        "別の説明に保存しました。"
    ));
    press(&mut a, &ctx, Key::Escape);
    assert!(a.palette.is_open());
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.palette.is_open());

    // A fresh App reads the same external content without saved settings.
    let mut restarted = app();
    restarted.palette.descriptions = Catalog::at(path);
    open(&mut restarted, &ctx);
    query(&mut restarted, &ctx, "別の説明");
    assert_eq!(listed(&mut restarted), ["pattern.preview"]);
    press(&mut restarted, &ctx, Key::Enter);
    assert!(!restarted.palette.is_open());
    assert!(restarted.command_input.is_open());
    press(&mut restarted, &ctx, Key::Escape);
    assert!(!restarted.command_input.is_open());
}

#[test]
fn bad_file_retains_previous_prose_and_fixed_file_clears_its_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    std::fs::write(&path, document()).unwrap();
    let mut a = app();
    a.palette.descriptions = Catalog::at(path.clone());
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    press(&mut a, &ctx, Key::Escape);
    std::fs::write(
        &path,
        document().replace("ids = [\"file.new\"]", "ids = [\"file.open\"]"),
    )
    .unwrap();
    open(&mut a, &ctx);
    let error = a.load_error.as_ref().unwrap();
    assert!(error.contains(&path.display().to_string()));
    assert!(error.contains("重複"));
    query(&mut a, &ctx, "preview pattern");
    press(&mut a, &ctx, Key::F1);
    assert!(has_text(
        &draw(&mut a, &ctx, vec![]).shapes,
        "Songカーソル位置"
    ));
    press(&mut a, &ctx, Key::Escape);
    press(&mut a, &ctx, Key::Escape);
    std::fs::write(&path, document()).unwrap();
    open(&mut a, &ctx);
    assert!(a.load_error.is_none());
    press(&mut a, &ctx, Key::Escape);
    // A release without an external file silently has bundled tooltips/search.
    let mut fresh = app();
    fresh.palette.descriptions = Catalog::at(dir.path().join("missing.toml"));
    open(&mut fresh, &ctx);
    assert!(fresh.load_error.is_none());
    choose(&mut fresh, &ctx, "pattern.preview");
    press(&mut fresh, &ctx, Key::F1);
    assert!(fresh.palette.description_open);
    press(&mut fresh, &ctx, Key::Enter);
    assert!(fresh.command_input.is_open());
}

#[test]
fn reopening_applies_an_edited_name_to_display_and_search() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    let named = |name: &str| {
        format!(
            r#"
[screens]
Main = "Event list"

[[command]]
ids = ["pattern.preview"]
name = "{name}"

[[command]]
ids = ["app.help"]
screen = "Main"
name = "画面別の名前"
"#
        )
    };
    let search = |a: &mut App, ctx: &egui::Context, text: &str| {
        open(a, ctx);
        query(a, ctx, text);
        let found = labels(a);
        press(a, ctx, Key::Escape);
        found
    };
    std::fs::write(&path, named("改名前")).unwrap();
    let mut a = app();
    a.palette.descriptions = Catalog::at(path.clone());
    let ctx = egui::Context::default();
    assert_eq!(search(&mut a, &ctx, "改名前"), ["改名前"]);
    assert_eq!(
        search(&mut a, &ctx, "画面別の名前"),
        ["Event list: 画面別の名前"]
    );

    std::fs::write(&path, named("改名後")).unwrap();
    assert!(search(&mut a, &ctx, "改名前").is_empty());
    open(&mut a, &ctx);
    query(&mut a, &ctx, "改名後");
    assert_eq!(labels(&mut a), ["改名後"]);
    assert!(has_text(&draw(&mut a, &ctx, vec![]).shapes, "改名後"));
    press(&mut a, &ctx, Key::Escape);

    // The last good names stay until the file is valid again.
    std::fs::write(&path, "broken").unwrap();
    assert_eq!(search(&mut a, &ctx, "改名後"), ["改名後"]);
    assert!(a.load_error.is_some());
}

#[test]
fn reopening_applies_edited_screen_names_and_search_words() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    let definitions = |screen: &str, search: &str| {
        format!(
            "[screens]\nMain = \"{screen}\"\n\n\
             [[command]]\nids = [\"app.help\"]\nname = \"手引き\"\nsearch = \"{search}\"\n"
        )
    };
    std::fs::write(&path, definitions("第一画面", "最初の語")).unwrap();
    let mut a = app();
    a.palette.descriptions = Catalog::at(path.clone());
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "最初の語");
    assert_eq!(labels(&mut a), ["第一画面: 手引き"]);
    press(&mut a, &ctx, Key::Escape);

    std::fs::write(&path, definitions("第二画面", "次の語")).unwrap();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "次の語");
    assert_eq!(labels(&mut a), ["第二画面: 手引き"]);
    press(&mut a, &ctx, Key::Escape);
    open(&mut a, &ctx);
    query(&mut a, &ctx, "最初の語");
    assert!(listed(&mut a).is_empty());
}

#[test]
fn a_search_word_outranks_an_explanation_only_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("command-palette.toml");
    std::fs::write(
        &path,
        "[[command]]\nids = [\"file.new\"]\ntooltip = \"順序語\"\n\n\
         [[command]]\nids = [\"file.save\"]\nname = \"保存\"\nsearch = \"順序語\"\n",
    )
    .unwrap();
    let mut a = app();
    a.palette.descriptions = Catalog::at(path);
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "順序語");
    assert_eq!(listed(&mut a), ["file.save", "file.new"]);
}
