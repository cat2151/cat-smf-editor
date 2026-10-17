use super::navigation::click;
use super::*;
use crate::play::{
    playback::{self, Mode},
    transport,
};
use cat_smf_editor::score::audition_timing::TempoMap;

#[test]
fn playing_piano_and_minimap_clicks_restart_at_the_clicked_position() {
    for minimap in [false, true] {
        let mut app = arpeggio();
        app.playback.follow = false;
        let (ctx, output) = opened_in(&mut app);
        playback::start(&mut app, Mode::AllTracks);
        transport::advance(&mut app);
        let pos = if minimap {
            let rect = app.piano_roll.viewport.minimap_state.rect.unwrap();
            egui::pos2(rect.left() + rect.width() * 0.7, rect.center().y)
        } else {
            all_notes(&output)[2].center()
        };
        click(&mut app, &ctx, 11.0, pos, false);
        let tick = playback::location(&app).0;
        assert!(tick > 0);
        if !minimap {
            assert_ne!(tick % 480, 0, "playing clicks keep their exact tick");
        }
        let map = TempoMap::new(&app.loaded.as_ref().unwrap().music, 0);
        let active = app.playback.active.as_ref().unwrap();
        assert!((active.start_seconds - map.duration(0, tick)).abs() < 1e-8);
        assert_eq!(app.playback.last, Mode::AllTracks);
    }
}
