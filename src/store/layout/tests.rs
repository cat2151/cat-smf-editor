use super::*;
use std::time::{Duration, Instant, SystemTime};

fn view(kind: ViewKind, size: f32) -> (Node, f32) {
    (Node::View(kind), size)
}

/// A nested layout with a dock window of two views.
const NESTED: &str = r#"
[root]
row = [
  { view = "event_list", width = 2 },
  { width = 3, column = [
      { view = "piano_roll", height = 3 },
      { view = "mixer", height = 1 },
  ] },
]

[[window]]
title = "Sub 1"
rect = [1920, 0, 1600, 900]
column = [
  { view = "song" },
  { view = "keyboard" },
]
"#;

#[test]
fn nested_form_is_valid_toml_and_rejects_unknown_views() {
    let value: toml::Value = toml::from_str(NESTED).unwrap();
    let row = value["root"]["row"].as_array().unwrap();
    assert_eq!(row.len(), 2);
    let column = row[1]["column"].as_array().unwrap();
    assert_eq!(column[0]["view"].as_str(), Some("piano_roll"));
    assert_eq!(column[1]["height"].as_integer(), Some(1));
    let window = &value["window"].as_array().unwrap()[0];
    assert_eq!(window["column"].as_array().unwrap().len(), 2);

    let spec = parse(NESTED).unwrap();
    assert_eq!(
        spec.docks[0].node,
        Node::Column(vec![
            view(ViewKind::Song, 1.0),
            view(ViewKind::Keyboard, 1.0)
        ])
    );
    let error = parse(&NESTED.replace("keyboard", "lissajous")).unwrap_err();
    assert!(error.contains("unknown variant `lissajous`"), "{error}");
}

#[test]
fn nested_rows_and_columns_become_the_tree() {
    let spec = parse(
        r#"
[root]
row = [
  { view = "event_list", width = 2 },
  { width = 3, column = [
      { view = "score", height = 3 },
  ] },
]
"#,
    )
    .unwrap();
    assert_eq!(
        spec.root,
        Some(Node::Row(vec![
            view(ViewKind::EventList, 2.0),
            (Node::Column(vec![view(ViewKind::Score, 3.0)]), 3.0),
        ]))
    );
    assert!(spec.docks.is_empty());
}

#[test]
fn a_share_must_name_the_direction_of_its_parent() {
    for (text, expected) in [
        (
            "[root]\nrow = [{ view = \"score\", height = 2 }]",
            "in a `row` takes `width`",
        ),
        (
            "[root]\ncolumn = [{ view = \"score\", width = 2 }]",
            "in a `column` takes `height`",
        ),
    ] {
        let error = parse(text).unwrap_err();
        assert!(error.contains(expected), "{text}: {error}");
    }
}

#[test]
fn windows_hold_their_own_tree() {
    let spec = parse(
        r#"
[root]
view = "event_list"

[[window]]
title = "Sub 1"
rect = [1920, 0, 1600, 900]
column = [
  { view = "score" },
]
"#,
    )
    .unwrap();
    assert_eq!(spec.root, Some(Node::View(ViewKind::EventList)));
    assert_eq!(
        spec.docks,
        vec![Dock {
            title: "Sub 1".into(),
            placement: crate::store::layout::placement::Placement::Desktop([
                1920.0, 0.0, 1600.0, 900.0
            ]),
            node: Node::Column(vec![view(ViewKind::Score, 1.0)]),
        }]
    );
}

#[test]
fn without_a_root_tree_the_built_in_look_follows_the_mode() {
    let spec = parse("").unwrap();
    for (note_mode, split) in [(false, false), (true, false), (true, true)] {
        assert_eq!(
            spec.resolve(note_mode, split),
            Layout::default_for(note_mode, split)
        );
    }
    assert_eq!(parse("[root]\n").unwrap(), spec);
}

#[test]
fn invalid_files_are_errors() {
    for text in [
        // unknown keys
        "status = 1",
        "[root]\nview = \"score\"\nwidth = 2",
        "[root]\nrow = [{ view = \"score\", size = 2 }]",
        "[root]\nrow = [{ view = \"score\", weight = 2 }]",
        // unknown view
        "[root]\nview = \"organ\"",
        // the same view twice
        "[root]\nrow = [{ view = \"score\" }, { view = \"score\" }]",
        "[root]\nview = \"score\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 10, 10]\nview = \"score\"",
        // without a root tree the built-in root already holds these views
        "[[window]]\ntitle = \"a\"\nrect = [0, 0, 10, 10]\nview = \"event_list\"",
        // the event list stays in the main window
        "[root]\nview = \"score\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 10, 10]\nview = \"event_list\"",
        // more than one of view / row / column
        "[root]\nview = \"score\"\nrow = [{ view = \"event_list\" }]",
        "[root]\nrow = [{ view = \"score\", column = [{ view = \"event_list\" }] }]",
        "[root]\nrow = [{ view = \"score\" }]\ncolumn = [{ view = \"event_list\" }]",
        // a pane with nothing in it
        "[root]\nrow = [{ width = 2 }]",
        "[root]\nrow = []",
        // shares
        "[root]\nrow = [{ view = \"score\", width = 0 }]",
        "[root]\nrow = [{ view = \"score\", width = -1 }]",
        "[root]\nrow = [{ view = \"score\", width = nan }]",
        "[root]\ncolumn = [{ view = \"score\", height = 0 }]",
        // windows
        "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 0, 10]\nview = \"score\"",
        "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 10]\nview = \"score\"",
        "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 10, 10]",
    ] {
        assert!(parse(text).is_err(), "accepted: {text}");
    }
}

/// The template's spelled-out example is the same as the built-in look.
#[test]
fn source_layout_and_commented_examples_are_valid() {
    parse(TEMPLATE).expect("the editable source layout must be valid");
    let example: String = TEMPLATE
        .lines()
        .skip_while(|line| *line != "# [root]")
        .take_while(|line| *line != "#")
        .map(|line| line.trim_start_matches('#').trim_start())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(example.starts_with("[root]"), "{example}");
    assert_eq!(
        parse(&example).unwrap().resolve(true, false),
        Layout::default_for(true, false)
    );
    let windows = windows();
    assert_eq!(windows.len(), 3, "{windows:?}");
    let docks = parse(&format!("[root]\nview = \"event_list\"\n{}", windows[0]))
        .unwrap()
        .docks;
    assert_eq!(docks.len(), 1, "{}", windows[0]);
    let demo = parse(&demo()).unwrap();
    assert_eq!(demo.root, None);
    let nodes: Vec<_> = demo.docks.into_iter().map(|d| d.node).collect();
    assert_eq!(
        nodes,
        vec![
            Node::Column(vec![
                view(ViewKind::Song, 2.0),
                view(ViewKind::Keyboard, 1.0)
            ]),
            Node::View(ViewKind::Analysis),
        ]
    );
}

/// Every view name the file accepts, as serde lists them when it rejects an unknown one.
pub(crate) fn view_names() -> Vec<String> {
    let error = parse("[root]\nview = \"?\"").unwrap_err();
    let (_, list) = error.split_once("expected one of ").expect(&error);
    let list = list.lines().next().unwrap_or_default();
    list.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The names listed under `画面名：` in the template.
fn template_names() -> Vec<String> {
    let text: String = TEMPLATE
        .lines()
        .skip_while(|line| !line.starts_with("# 画面名："))
        .take_while(|line| *line != "#")
        .map(|line| line.trim_start_matches('#').trim())
        .collect();
    text.trim_start_matches("画面名：")
        .split('、')
        .map(|entry| {
            entry
                .split('（')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn the_template_lists_every_view_name() {
    let names = view_names();
    assert!(names.contains(&"event_list".to_owned()), "{names:?}");
    assert!(names.contains(&"track_overview".to_owned()), "{names:?}");
    assert_eq!(template_names(), names);
}

/// The template's commented-out `[[window]]` examples, uncommented.
fn windows() -> Vec<String> {
    let lines: Vec<&str> = TEMPLATE.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| **line == "# [[window]]")
        .map(|(start, _)| {
            lines[start..]
                .iter()
                .take_while(|line| **line != "#")
                .map(|line| line.trim_start_matches('#').trim_start())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

/// The template's example of the Song and the keyboard in one window and the analysis in another.
pub(crate) fn demo() -> String {
    // UI fixtures exercise pane ownership without depending on the host's actual monitors.
    windows()[1..]
        .iter()
        .map(|text| {
            let panes = text.split("[window.monitor_placement]").next().unwrap();
            format!("{panes}\nrect = [0, 0, 900, 700]")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn write(path: &Path, text: &str, modified: SystemTime) {
    std::fs::write(path, text).unwrap();
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

#[test]
fn polling_picks_up_saved_changes_and_keeps_the_last_good_layout_on_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("layout.toml");
    let mut watch = Watch::open(directory.path());
    assert!(!path.exists(), "missing layout must not be auto-generated");
    assert_eq!(watch.spec, Spec::default());
    assert!(watch.error.is_none());

    let base = SystemTime::now() + Duration::from_secs(10);
    let start = Instant::now();
    let column = "[root]\ncolumn = [{ view = \"event_list\" }, { view = \"score\" }]\n";
    write(&path, column, base);
    watch.poll(start);
    let expected = parse(column).unwrap();
    assert_eq!(watch.spec, expected);

    // A save within the poll interval waits for the next poll.
    let single = "[root]\nview = \"event_list\"\n";
    write(&path, single, base + Duration::from_secs(1));
    watch.poll(start + Duration::from_millis(100));
    assert_eq!(watch.spec, expected);
    watch.poll(start + POLL);
    assert_eq!(watch.spec, parse(single).unwrap());

    // A broken save keeps the previous layout and raises the error.
    write(
        &path,
        "[root]\nview = \"organ\"\n",
        base + Duration::from_secs(2),
    );
    watch.poll(start + POLL * 2);
    assert_eq!(watch.spec, parse(single).unwrap());
    let error = watch.error.clone().unwrap();
    assert!(error.contains("layout.toml"), "{error}");

    write(&path, column, base + Duration::from_secs(3));
    watch.poll(start + POLL * 3);
    assert_eq!(watch.spec, expected);
    assert!(watch.error.is_none());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), column);

    // Reopening keeps the user's file.
    assert_eq!(Watch::open(directory.path()).spec, expected);

    std::fs::remove_file(&path).unwrap();
    watch.poll(start + POLL * 4);
    assert_eq!(watch.spec, Spec::default());
}

#[test]
fn a_watch_without_a_file_does_nothing() {
    let mut watch = Watch::default();
    assert!(!watch.watching());
    watch.poll(Instant::now());
    assert_eq!(watch.spec, Spec::default());
    assert!(watch.error.is_none());
}
