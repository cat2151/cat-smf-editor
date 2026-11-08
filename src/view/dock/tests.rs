use super::*;
use eframe::egui::{pos2, Rect};

fn view(kind: ViewKind, size: f32) -> (Node, f32) {
    (Node::View(kind), size)
}

fn root(root: Node) -> Layout {
    Layout {
        root,
        docks: Vec::new(),
    }
}

#[test]
fn split_follows_weights_and_gaps() {
    let rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 70.0));
    let parts = split(rect, Axis::Row, &[2.0, 3.0], 0.0);
    assert_eq!(
        parts[0],
        Rect::from_min_max(pos2(10.0, 20.0), pos2(50.0, 70.0))
    );
    assert_eq!(
        parts[1],
        Rect::from_min_max(pos2(50.0, 20.0), pos2(110.0, 70.0))
    );

    let parts = split(rect, Axis::Column, &[1.0, 1.0, 3.0], 5.0);
    let heights: Vec<f32> = parts.iter().map(|r| r.height()).collect();
    assert_eq!(heights, vec![8.0, 8.0, 24.0]);
    assert_eq!(parts[1].top(), parts[0].bottom() + 5.0);
    assert_eq!(parts[2].bottom(), rect.bottom());
    assert!(parts.iter().all(|r| r.left() == 10.0 && r.right() == 110.0));
}

/// The default two-pane row lands where `ui.columns(2)` put the event list and the score.
#[test]
fn equal_row_matches_ui_columns() {
    let ctx = egui::Context::default();
    let mut expected = Vec::new();
    let mut actual = Vec::new();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            actual = split(rect, Axis::Row, &[1.0, 1.0], ui.spacing().item_spacing.x);
            ui.columns(2, |panes| {
                expected = panes.iter().map(|p| p.max_rect()).collect();
            });
        });
    });
    assert_eq!(expected.len(), 2);
    for (a, e) in actual.iter().zip(&expected) {
        assert!((a.min - e.min).length() < 1e-3, "{a:?} vs {e:?}");
        assert!((a.max - e.max).length() < 1e-3, "{a:?} vs {e:?}");
    }
}

#[test]
fn zero_or_negative_size_is_an_error() {
    for size in [0.0, -1.0, f32::NAN] {
        let layout = root(Node::Row(vec![
            view(ViewKind::EventList, 1.0),
            view(ViewKind::Score, size),
        ]));
        assert!(matches!(layout.validate(), Err(Error::Size(_))), "{size}");
    }
    assert_eq!(root(Node::Column(Vec::new())).validate(), Err(Error::Empty));
}

#[test]
fn one_view_lives_in_one_place() {
    let twice = root(Node::Row(vec![
        view(ViewKind::Score, 1.0),
        view(ViewKind::Score, 1.0),
    ]));
    assert_eq!(twice.validate(), Err(Error::Duplicate(ViewKind::Score)));

    let across_hosts = Layout {
        root: Node::View(ViewKind::EventList),
        docks: vec![Dock {
            title: "Sub".into(),
            placement: crate::store::layout::placement::Placement::Desktop([
                0.0, 0.0, 800.0, 600.0,
            ]),
            node: Node::View(ViewKind::EventList),
        }],
    };
    assert_eq!(
        across_hosts.validate(),
        Err(Error::Duplicate(ViewKind::EventList))
    );

    let list_in_a_dock = Layout {
        root: Node::View(ViewKind::Score),
        docks: vec![Dock {
            title: "Sub".into(),
            placement: crate::store::layout::placement::Placement::Desktop([
                0.0, 0.0, 800.0, 600.0,
            ]),
            node: Node::View(ViewKind::EventList),
        }],
    };
    assert_eq!(
        list_in_a_dock.validate(),
        Err(Error::MainOnly(ViewKind::EventList))
    );
}

#[test]
fn views_are_found_by_host() {
    let layout = Layout {
        root: Node::View(ViewKind::EventList),
        docks: vec![Dock {
            title: "Sub".into(),
            placement: crate::store::layout::placement::Placement::Desktop([
                0.0, 0.0, 800.0, 600.0,
            ]),
            node: Node::View(ViewKind::Score),
        }],
    };
    assert_eq!(layout.validate(), Ok(()));
    assert_eq!(layout.host_of(ViewKind::EventList), Some(Host::Root));
    assert_eq!(layout.host_of(ViewKind::Score), Some(Host::Dock(0)));
    assert!(!root(Node::View(ViewKind::EventList)).contains(ViewKind::Score));
}

#[test]
fn focus_moves_depth_first_and_wraps() {
    // Score sits deeper than EventList but comes first in the tree.
    let tree = Node::Row(vec![
        (Node::Column(vec![view(ViewKind::Score, 1.0)]), 1.0),
        view(ViewKind::EventList, 1.0),
    ]);
    assert_eq!(tree.views(), vec![ViewKind::Score, ViewKind::EventList]);
    let step = |current, forward| focus_step(&tree, current, forward);
    assert_eq!(step(None, true), Some(ViewKind::Score));
    assert_eq!(step(None, false), Some(ViewKind::EventList));
    assert_eq!(step(Some(ViewKind::Score), true), Some(ViewKind::EventList));
    assert_eq!(step(Some(ViewKind::EventList), true), Some(ViewKind::Score));
    assert_eq!(
        step(Some(ViewKind::Score), false),
        Some(ViewKind::EventList)
    );
}

#[test]
fn default_matches_the_mode() {
    let note = Layout::default_for(true, false);
    assert_eq!(note.validate(), Ok(()));
    assert_eq!(
        note.root.views(),
        vec![ViewKind::EventList, ViewKind::Score]
    );
    // Two tracks already fill the list, as before.
    assert_eq!(
        Layout::default_for(true, true).root,
        Node::View(ViewKind::EventList)
    );
    assert_eq!(
        Layout::default_for(false, false).root,
        Node::View(ViewKind::EventList)
    );
}
