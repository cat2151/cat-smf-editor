use super::{App, Voice};
use crate::view::selection_history::{History, SINGER};
pub(super) fn identity(voice: &Voice) -> String {
    serde_json::to_string(&(&voice.speaker_uuid, voice.style_id)).unwrap()
}
pub(super) fn load(app: &mut App) {
    let previous = std::mem::take(&mut app.selection_history.singer);
    app.selection_history.singer = History::load(app, SINGER, previous);
}
pub(super) fn record(app: &mut App, voice: Option<&Voice>) {
    let (Some(picker), Some(voice)) = (&app.voicevox.singer, voice) else {
        return;
    };
    app.selection_history
        .singer
        .record(&picker.query, &identity(voice));
    if let Err(error) = app.selection_history.singer.save(app, SINGER) {
        app.config.error = Some(error);
    }
}
