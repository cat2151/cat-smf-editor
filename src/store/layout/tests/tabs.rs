use super::*;

#[test]
fn tabs_list_their_views_in_order_and_take_the_share_of_their_parent() {
    let spec = parse(
        r#"
[root]
row = [
  { width = 2, tabs = [ { view = "event_list" }, { view = "track_overview" }, { view = "piano_roll" } ] },
  { view = "score" },
]

[[window]]
title = "Sub 1"
rect = [0, 0, 10, 10]
tabs = [ { view = "mixer" } ]
"#,
    )
    .unwrap();
    assert_eq!(
        spec.root,
        Some(Node::Row(vec![
            (
                Node::Tabs(vec![
                    ViewKind::EventList,
                    ViewKind::TrackOverview,
                    ViewKind::PianoRoll
                ]),
                2.0
            ),
            view(ViewKind::Score, 1.0),
        ]))
    );
    assert_eq!(spec.docks[0].node, Node::Tabs(vec![ViewKind::Mixer]));
    assert_eq!(
        parse("[root]\ncolumn = [{ height = 3, tabs = [{ view = \"score\" }] }]")
            .unwrap()
            .root,
        Some(Node::Column(vec![(Node::Tabs(vec![ViewKind::Score]), 3.0)]))
    );
}

#[test]
fn tabs_hold_views_only() {
    for (text, expected) in [
        ("[root]\ntabs = []", "`tabs` needs at least one view"),
        (
            "[root]\nrow = [{ tabs = [] }]",
            "`tabs` needs at least one view",
        ),
        (
            "[root]\ntabs = [{ row = [{ view = \"score\" }] }]",
            "unknown field `row`",
        ),
        (
            "[root]\ntabs = [{ column = [{ view = \"score\" }] }]",
            "unknown field `column`",
        ),
        (
            "[root]\ntabs = [{ tabs = [{ view = \"score\" }] }]",
            "unknown field `tabs`",
        ),
        (
            "[root]\ntabs = [{ view = \"score\", width = 2 }]",
            "unknown field `width`",
        ),
        (
            "[root]\ntabs = [{ view = \"score\", height = 2 }]",
            "unknown field `height`",
        ),
        ("[root]\ntabs = [{}]", "missing field `view`"),
        (
            "[root]\nview = \"score\"\ntabs = [{ view = \"event_list\" }]",
            "use only one of",
        ),
        (
            "[root]\nrow = [{ view = \"score\", tabs = [{ view = \"event_list\" }] }]",
            "use only one of",
        ),
        (
            "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 10, 10]\nview = \"score\"\ntabs = [{ view = \"mixer\" }]",
            "use only one of",
        ),
        (
            "[root]\ntabs = [{ view = \"score\" }, { view = \"score\" }]",
            "view placed twice",
        ),
        (
            "[root]\nview = \"score\"\n[[window]]\ntitle = \"a\"\nrect = [0, 0, 10, 10]\ntabs = [{ view = \"mixer\" }, { view = \"event_list\" }]",
            "view stays in the main window",
        ),
    ] {
        let error = parse(text).unwrap_err();
        assert!(error.contains(expected), "{text}: {error}");
    }
}

#[test]
fn a_window_of_tabs_is_titled_by_its_views() {
    let spec = parse(
        "[root]\nview = \"event_list\"\n[[window]]\nrect = [0, 0, 10, 10]\ntabs = [{ view = \"mixer\" }, { view = \"song\" }]",
    )
    .unwrap();
    assert_eq!(spec.docks[0].title, "cat-smf-editor · mixer · song");
}
