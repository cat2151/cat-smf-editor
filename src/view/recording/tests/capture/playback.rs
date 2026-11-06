use super::*;
use crate::view::recording::capture;

fn interleaved() -> (App, egui::Context) {
    use cat_smf_editor::score::pattern::{
        operations,
        recording::capture::{append, Note, Phrase},
    };
    let mut a = app();
    a.edit_tracks(|m| {
        let tracks = [operations::create_source(m)?, operations::create_source(m)?];
        for (i, source) in [tracks[0], tracks[1], tracks[0]].into_iter().enumerate() {
            append(
                m,
                Some(source),
                500_000,
                &Phrase {
                    start: 0,
                    container: None,
                    notes: vec![Note {
                        start: 0,
                        length: 500,
                        pitch: 60 + i as u8,
                        velocity: 80,
                    }],
                },
                &[],
            )?;
        }
        Ok(())
    })
    .unwrap();
    let ctx = egui::Context::default();
    open_key(&mut a, &ctx);
    ctrl_j(&mut a, &ctx);
    frame(&mut a, &ctx, vec![], true);
    (a, ctx)
}

fn advance_after(a: &mut App, seconds: f64) {
    crate::play::transport::advance(a);
    a.audition.set_test_seconds(seconds);
    crate::play::transport::advance(a);
}

#[test]
fn capture_plays_every_number_across_containers_and_stops_after_the_newest() {
    let (mut a, ctx) = interleaved();
    let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    click_at(&mut a, &ctx, row(&ctx, 2).0.center());
    let first_pattern = a.patterns.preview;
    for (index, sequence) in [(2, 1), (1, 2), (0, 3)] {
        assert_eq!(capture::position(&a, session(&a)), Some((index, 0)));
        assert!(capture::title(&a, session(&a)).ends_with(&format!(" · #{sequence}")));
        let active = a
            .playback
            .pending
            .as_ref()
            .map(|p| &p.active)
            .or(a.playback.active.as_ref())
            .unwrap();
        // Only this phrase is scheduled, even when a later number shares its pattern.
        let pitches: Vec<_> = active
            .sources
            .iter()
            .flat_map(|p| p.1.notes.iter().map(|n| n.pitch))
            .collect();
        assert_eq!(pitches, vec![59 + sequence as u8]);
        if sequence < 3 {
            advance_after(&mut a, 2.1);
            assert!(capture::title(&a, session(&a))
                .ends_with(&format!("#{sequence} to #{} wait 2", sequence + 1)));
            advance_after(&mut a, 3.1);
            assert!(capture::title(&a, session(&a)).ends_with("wait 1"));
            advance_after(&mut a, 4.0);
            assert_eq!(a.patterns.preview == first_pattern, sequence == 2);
        }
    }
    advance_after(&mut a, 2.0);
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(capture::title(&a, session(&a)), capture::TITLE);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        bytes
    );
}

#[test]
fn capture_can_start_at_the_middle_number_and_stop_during_the_gap() {
    let (mut a, ctx) = interleaved();
    click_at(&mut a, &ctx, row(&ctx, 1).0.center());
    advance_after(&mut a, 2.1);
    assert!(capture::title(&a, session(&a)).ends_with("#2 to #3 wait 2"));
    capture_keys(&mut a, &[plain(Key::Space)]);
    crate::play::transport::advance(&mut a);
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(capture::title(&a, session(&a)), capture::TITLE);
    capture_keys(&mut a, &[plain(Key::Space)]);
    advance_after(&mut a, 4.0);
    assert_eq!(capture::position(&a, session(&a)), Some((0, 0)));
}

fn choose_container(a: &mut App, ctx: &egui::Context, container: Option<u64>) {
    let rect: egui::Rect = ctx
        .data(|d| d.get_temp(egui::Id::new("capture-filter")))
        .unwrap();
    click_at(a, ctx, rect.center());
    frame(a, ctx, vec![], true);
    let id = container.map_or_else(
        || egui::Id::new("capture-filter-all"),
        |id| egui::Id::new(("capture-filter-option", id)),
    );
    let rect: egui::Rect = ctx.data(|d| d.get_temp(id)).unwrap();
    click_at(a, ctx, rect.center());
    frame(a, ctx, vec![], true);
}

#[test]
fn container_dropdown_filters_navigation_take_and_continuous_playback() {
    let (mut a, ctx) = interleaved();
    let container = capture::phrases(&a, session(&a)).unwrap().0[0].container;
    let original = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    choose_container(&mut a, &ctx, Some(container));
    let (phrases, selected, _) = capture::phrases(&a, session(&a)).unwrap();
    assert_eq!(
        phrases.iter().map(|p| p.sequence).collect::<Vec<_>>(),
        [3, 1]
    );
    assert_eq!(selected, 0);
    capture_keys(&mut a, &[plain(Key::ArrowDown), plain(Key::Space)]);
    assert_eq!(capture::position(&a, session(&a)), Some((1, 0)));
    advance_after(&mut a, 4.0);
    assert_eq!(capture::position(&a, session(&a)), Some((0, 0)));
    advance_after(&mut a, 2.0);
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
    capture_keys(&mut a, &[plain(Key::Enter)]);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(normal(&catalog)[0].container, container);
    choose_container(&mut a, &ctx, None);
    assert_eq!(capture::phrases(&a, session(&a)).unwrap().0.len(), 3);
}

#[test]
fn switching_container_stops_listening_and_refresh_keeps_the_filter() {
    let (mut a, ctx) = interleaved();
    let container = capture::phrases(&a, session(&a)).unwrap().0[1].container;
    click_at(&mut a, &ctx, row(&ctx, 0).0.center());
    choose_container(&mut a, &ctx, Some(container));
    assert!(!crate::play::transport::timeline::running(&a));
    assert_eq!(capture::phrases(&a, session(&a)).unwrap().0.len(), 1);
    a.edit_tracks(|m| edit::tracks::rename(&mut m.tracks[0], "Edited"))
        .unwrap();
    frame(&mut a, &ctx, vec![], true);
    let (phrases, selected, _) = capture::phrases(&a, session(&a)).unwrap();
    assert_eq!(phrases.len(), 1);
    assert_eq!(phrases[selected].container, container);
    capture_keys(&mut a, &[plain(Key::ArrowDown), plain(Key::ArrowUp)]);
    assert_eq!(capture::phrases(&a, session(&a)).unwrap().1, 0);
    capture_keys(&mut a, &[(Key::Delete, Modifiers::CTRL | Modifiers::SHIFT)]);
    frame(&mut a, &ctx, vec![], true);
    assert!(listed(&a).is_empty());
    capture_keys(&mut a, &[plain(Key::Space), plain(Key::Enter)]);
    assert!(normal(&Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap()).is_empty());
    choose_container(&mut a, &ctx, None);
    assert!(listed(&a).is_empty());
}

#[test]
fn dropdown_keyboard_events_do_not_audition_take_or_close_capture() {
    let (mut a, _) = interleaved();
    let ctx = egui::Context::default();
    let mut state = std::mem::take(&mut a.patterns.recording);
    let s = state.session.as_mut().unwrap();
    let mut step = |events: Vec<egui::Event>| {
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                capture::test_ui(&mut a, s, ctx);
            },
        );
    };
    step(vec![]);
    step(vec![key(Key::Tab, true, Modifiers::NONE)]);
    assert!(ctx.wants_keyboard_input());
    step(vec![key(Key::Enter, true, Modifiers::NONE)]);
    assert!(ctx.memory(|m| m.any_popup_open()));
    step(vec![key(Key::Escape, true, Modifiers::NONE)]);
    assert!(!ctx.memory(|m| m.any_popup_open()));
    assert!(s.capture.is_some());
    assert!(!crate::play::transport::timeline::running(&a));
    assert!(normal(&Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap()).is_empty());
    a.patterns.recording = state;
}
