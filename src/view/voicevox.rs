//! VOICEVOX engine の `run.exe` を選ぶ wizard と、editor が繋いだ engine。
pub(crate) mod progress;
pub(crate) mod singer;

use crate::app::App;
use crate::view::commands::{Command, Screen};
use cat_smf_editor::voicevox::engine::{self, Engine};
use eframe::egui::{self, Color32, Key};
use std::path::PathBuf;

pub const OFFICIAL_PAGE: &str = "https://voicevox.hiroshiba.jp/";
/// 公式ページを開いた後、editor の window が focus を失わなくてもスピナーを消すまでの秒数。
const PAGE_SPINNER_SECONDS: f64 = 15.0;

#[derive(Default)]
pub struct State {
    pub wizard: bool,
    pub singer: Option<singer::Picker>,
    pub catalog: singer::Catalog,
    /// 歌わせる途中で開いた。選ばずに閉じたら、歌うのを諦めてエラーにする。
    pub(crate) for_prerender: bool,
    pub(crate) pick_requested: bool,
    pub error: Option<String>,
    /// editor が繋いだ engine。editor が起動したものは editor の終了時に落ちる。
    pub engine: Option<Engine>,
    pub(crate) prerender: crate::play::sung::prerender::State,
    /// パレットの「1小節の歌詞を差し替え」の入力欄。
    pub(crate) lyrics: Option<lyrics::Editing>,
    /// 公式ページを開いた時刻（`egui::InputState::time`）。スピナーを出している間だけ `Some`。
    page_opened_at: Option<f64>,
    #[cfg(test)]
    pub(crate) pages_opened: usize,
}

/// `Ctrl+,` → `v`。
pub(crate) fn open(app: &mut App) {
    app.comma_menu.open = false;
    app.voicevox.wizard = true;
    app.voicevox.for_prerender = false;
}

/// 歌わせたいのに engine も `run.exe` も無いとき。
pub(crate) fn request_executable(app: &mut App) {
    app.voicevox.wizard = true;
    app.voicevox.for_prerender = true;
}

pub(crate) fn close(app: &mut App) {
    app.voicevox.wizard = false;
    if std::mem::take(&mut app.voicevox.for_prerender) {
        app.voicevox.error = Some("VOICEVOX engine is not set".into());
    }
}

fn open_page(app: &mut App, ctx: &egui::Context) {
    app.voicevox.page_opened_at = Some(ctx.input(|i| i.time));
    #[cfg(test)]
    let result: std::io::Result<()> = {
        app.voicevox.pages_opened += 1;
        Ok(())
    };
    // explorer は URL を既定のブラウザへ渡す。成功しても exit code 1 を返すので見ない。
    #[cfg(not(test))]
    let result = std::process::Command::new("explorer")
        .arg(OFFICIAL_PAGE)
        .spawn()
        .map(drop);
    app.voicevox.error = result
        .err()
        .map(|e| format!("Cannot open {OFFICIAL_PAGE}: {e}"));
}

/// ファイル選択 dialog は editor の window を親にするので、eframe の frame を持つ側で開く。
pub fn pick_dialog(app: &mut App, ctx: &egui::Context, frame: &eframe::Frame) {
    if !std::mem::take(&mut app.voicevox.pick_requested) {
        return;
    }
    crate::play::transport::stop(app);
    let mut dialog = rfd::FileDialog::new()
        .set_parent(frame)
        .set_title("VOICEVOX engine")
        .add_filter("run.exe", &["exe"]);
    if let Some(directory) = app
        .config
        .voicevox_engine
        .as_deref()
        .and_then(|path| path.parent())
        .filter(|directory| directory.is_dir())
    {
        dialog = dialog.set_directory(directory);
    }
    complete(app, dialog.pick_file());
    super::comma_menu::swallow(ctx);
    ctx.request_repaint();
}

pub(crate) fn complete(app: &mut App, selected: Option<PathBuf>) {
    if let Some(path) = selected {
        app.config.select_voicevox_engine(path);
        app.voicevox.wizard = false;
        app.voicevox.for_prerender = false;
        app.voicevox.error = None;
        app.voicevox.catalog.error = None;
    }
}

pub(super) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    singer::collect(app, screen, out);
    match screen {
        Screen::Main | Screen::Timeline
            if screen == Screen::Main || app.piano_roll.open || app.tempo_curve.open =>
        {
            let singer = app
                .loaded
                .as_ref()
                .and_then(|loaded| loaded.music.tracks.get(app.track))
                .map(cat_smf_editor::voicevox::is_singer);
            if singer.is_some() {
                out.push(Command::plain(
                    screen,
                    "voicevox-singer",
                    "VOICEVOX: 現在のtrackの歌声を選ぶ",
                    &[],
                    |a, _| singer::open(a),
                ));
            }
            if screen == Screen::Main && singer == Some(true) {
                out.push(Command::plain(
                    screen,
                    "measure-mml-lyrics",
                    "VOICEVOX: 1小節のMMLと歌詞を同時に差し替え",
                    &[],
                    |a, _| {
                        lyrics::begin(a);
                        if let Some(editing) = &mut a.voicevox.lyrics {
                            editing.combined = true;
                        }
                    },
                ));
            }
            if screen == Screen::Main && app.loaded.is_some() {
                out.push(Command::plain(
                    screen,
                    "lyrics-measure",
                    "VOICEVOX: 1小節の歌詞を差し替え",
                    &[],
                    |a, _| lyrics::begin(a),
                ));
            }
        }
        Screen::Lyrics => {
            out.push(Command::plain(
                screen,
                "lyrics-confirm",
                "確定",
                &[Key::Enter],
                lyrics::confirm,
            ));
            out.push(Command::plain(
                screen,
                "lyrics-cancel",
                "取消",
                &[Key::Escape],
                |a, _| lyrics::cancel(a),
            ));
        }
        Screen::Settings => out.push(Command::plain(
            screen,
            "voicevox",
            "VOICEVOX engine",
            &[Key::V],
            |a, _| open(a),
        )),
        Screen::Voicevox => {
            out.push(Command::plain(
                screen,
                "voicevox-pick",
                "run.exe",
                &[Key::R],
                |a, _| a.voicevox.pick_requested = true,
            ));
            out.push(Command::plain(
                screen,
                "voicevox-page",
                "公式ページ",
                &[Key::O],
                open_page,
            ));
            out.push(Command::plain(
                screen,
                "voicevox-close",
                "閉じる",
                &[Key::Escape],
                |a, _| close(a),
            ));
        }
        _ => {}
    }
}

/// ブラウザが前へ出て editor の window が focus を失ったか、時間切れなら、スピナーを消す。
fn expire_page_spinner(app: &mut App, ctx: &egui::Context) {
    let Some(opened_at) = app.voicevox.page_opened_at else {
        return;
    };
    let (now, focused) = ctx.input(|i| (i.time, i.viewport().focused));
    if !app.voicevox.wizard || focused == Some(false) || now - opened_at >= PAGE_SPINNER_SECONDS {
        app.voicevox.page_opened_at = None;
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    expire_page_spinner(app, ctx);
    if !app.voicevox.wizard {
        return;
    }
    let mut open = true;
    egui::Window::new("VOICEVOX engine")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            if let Some(path) = &app.config.voicevox_engine {
                let text = path.display().to_string();
                if engine::executable(Some(path)).is_some() {
                    ui.label(text);
                } else {
                    ui.colored_label(Color32::LIGHT_RED, text);
                }
            }
            if ui.button("r: run.exe").clicked() {
                app.voicevox.pick_requested = true;
            }
            ui.horizontal(|ui| {
                if ui
                    .button("o: 公式ページ")
                    .on_hover_text(OFFICIAL_PAGE)
                    .clicked()
                {
                    open_page(app, ctx);
                }
                if app.voicevox.page_opened_at.is_some() {
                    ui.spinner();
                }
            });
        });
    if !open {
        close(app);
    }
}

#[cfg(test)]
mod audition_engine_tests;
#[cfg(test)]
mod audition_tests;
mod combined;
#[cfg(test)]
mod combined_tests;
#[cfg(test)]
mod flow_engine_tests;
#[cfg(test)]
mod flow_tests;
pub(crate) mod lyrics;
#[cfg(test)]
mod measure_lyrics_tests;
#[cfg(test)]
mod note_lyric_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod singer_tests;
#[cfg(test)]
mod tests;
