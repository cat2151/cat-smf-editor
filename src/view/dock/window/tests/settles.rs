//! Every view, in the main window, alone in a dock window or in its own window, settles on a
//! fixed set of native windows: each frame hands the backend only the windows it shows, and
//! neither the windows nor the commands sent to them grow from frame to frame.
use super::every_view::{kind, placing, showing};
use super::*;
use crate::view::patterns::workspace::Pane;
use crate::view::patterns::workspace_tests::harness;

const FRAMES: usize = 30;

/// Where the view goes: the main window, a dock window, or nowhere (its own window).
fn placements(name: &str) -> Vec<(&'static str, crate::store::layout::Spec)> {
    let mut out = vec![("own window", Default::default())];
    out.push((
        "root",
        crate::store::layout::parse(&format!("[root]\nview = \"{name}\"")).unwrap(),
    ));
    if let Ok(spec) = placing(name) {
        out.push(("dock", spec));
    }
    out
}

/// The windows a frame showed and the commands it sent, then forgets them. Panics on a window
/// the output carries without showing it.
fn settle(c: &egui::Context, context: &str) -> (usize, usize) {
    let (rendered, outputs) = c.data(|d| {
        (
            d.get_temp::<Vec<egui::ViewportId>>(egui::Id::new("workspace-rendered"))
                .unwrap_or_default(),
            d.get_temp::<Vec<egui::ViewportId>>(egui::Id::new("workspace-outputs"))
                .unwrap_or_default(),
        )
    });
    let commands = outputs
        .iter()
        .map(|id| {
            c.data(|d| {
                d.get_temp::<Vec<egui::ViewportCommand>>(egui::Id::new(("workspace-commands", *id)))
            })
            .map_or(0, |v| v.len())
        })
        .sum();
    let unshown = harness::unshown(c);
    assert!(unshown.is_empty(), "{context}: {unshown:?}");
    (rendered.len(), commands)
}

#[test]
fn every_view_in_every_place_settles_on_the_windows_it_shows() {
    let names = crate::store::layout::tests::view_names();
    assert_eq!(names.len(), 14, "{names:?}");
    for name in &names {
        let view = kind(name);
        for (place, spec) in placements(name) {
            let (mut a, c) = showing(view);
            // A window drawn after the others hands the backend whatever they left behind.
            a.help_open = true;
            a.layout.spec = spec;
            frame(&mut a, &c, ROOT, vec![]);
            if matches!(view, ViewKind::Keyboard | ViewKind::Piano) {
                key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
            }
            frame(&mut a, &c, ROOT, vec![]);
            harness::unshown(&c);
            let counts: Vec<(usize, usize)> = (0..FRAMES)
                .map(|i| {
                    frame(&mut a, &c, ROOT, vec![]);
                    settle(&c, &format!("{name} in {place}, frame {i}"))
                })
                .collect();
            let (early, late) = counts.split_at(FRAMES / 2);
            let most = |half: &[(usize, usize)], f: fn(&(usize, usize)) -> usize| {
                half.iter().map(f).max().unwrap_or_default()
            };
            assert!(
                most(late, |c| c.0) <= most(early, |c| c.0)
                    && most(late, |c| c.1) <= most(early, |c| c.1),
                "{name} in {place}: (windows, commands) per frame {counts:?}"
            );
        }
    }
}
