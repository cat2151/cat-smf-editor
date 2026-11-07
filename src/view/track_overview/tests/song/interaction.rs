use super::*;

#[test]
fn clicks_on_a_split_row_play_and_jump_to_its_placement() {
    let (mut a, _, _, _) = fixture();
    a.config.piano_roll_maximized = true;
    let (mut a, ctx) = opened_from(a);
    // At 0.2 pixels/tick the third Song measure begins 768 pixels into the pane.
    press(&mut a, &ctx, &[egui::Key::L; 2]);
    let rect = pane(&ctx, 2);
    let x = rect.left() + 3840.0 * 0.2 + 10.0;
    let pos = egui::pos2(x, rect.center().y);
    let expected = Song::read(&a).unwrap().at(2, 3840).unwrap().pattern;
    click(&mut a, &ctx, pos, egui::PointerButton::Primary);
    assert_eq!(a.track_overview.cell, (2, 3840));
    assert_eq!(
        a.playback.pending.as_ref().unwrap().active.start_seconds,
        4.0
    );
    click(&mut a, &ctx, pos, egui::PointerButton::Secondary);
    let output = render(&mut a, &ctx, vec![]);
    let button = text_pos(&output, "g — jump");
    click(&mut a, &ctx, button, egui::PointerButton::Primary);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, expected);
    assert!(!a.track_overview.open);
}

#[test]
fn changed_song_meter_drives_navigation_and_body_coordinates() {
    let (mut a, first) = crate::view::tests::unplaced::fixture();
    let mut second = 0;
    a.edit_tracks(|m| {
        second = *pattern::Catalog::read(m)?
            .patterns
            .keys()
            .find(|&&id| id != first)
            .unwrap();
        operations::resize(
            m,
            second,
            vec![scene::Meter {
                numerator: 3,
                exponent: 2,
                ticks: 1440,
            }],
        )?;
        for (tick, numerator) in [(1920, 3), (3360, 4)] {
            cat_smf_editor::edit::insert(
                &mut m.tracks[0],
                event(tick, 0xff, 0x58, vec![numerator, 2, 24, 8]),
            );
        }
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 1, Some(Pattern(first)))?;
        grid::place(m, None, lane, 2, Some(Pattern(second)))?;
        grid::place(m, None, lane, 3, Some(Pattern(first)))
    })
    .unwrap();
    let (mut a, ctx) = opened_from(a);
    assert_eq!(
        notes(&Panes::read(&a, 0).unwrap().overview),
        vec![(0, 60), (1920, 72), (3360, 60)]
    );
    press(&mut a, &ctx, &[egui::Key::L]);
    assert_eq!(a.track_overview.cell.1, 1920);
    press(&mut a, &ctx, &[egui::Key::L]);
    assert_eq!(a.track_overview.cell.1, 3360);
    press(&mut a, &ctx, &[egui::Key::G]);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, first);
    assert_eq!(
        crate::view::patterns::timing::pattern_tick(&a, first).unwrap(),
        3360
    );
}

#[test]
fn container_solo_survives_track_reordering_by_identity() {
    let (mut a, _, _, _) = fixture();
    open(&mut a);
    play_track(&mut a, 3, 0);
    assert_eq!(playing_tracks(&a), vec![2]);
    let saved = crate::play::transport::timeline::capture(&a).unwrap();
    a.edit_tracks(|m| {
        m.tracks.swap(1, 2);
        Ok(())
    })
    .unwrap();
    // Track replacement deliberately stops transport; resuming must rebind the saved IDs.
    crate::play::transport::timeline::resume(&mut a, saved);
    assert_eq!(playing_tracks(&a), vec![1]);
    assert!(a.playback.pending.as_ref().unwrap().parts.iter().all(|p| p
        .2
        .notes
        .iter()
        .all(|n| n.pitch == 67)));
}
