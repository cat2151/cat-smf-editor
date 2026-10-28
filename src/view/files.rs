//! File save, switch and exit confirmations.
use crate::app::App;
use eframe::egui;
#[cfg(test)]
use egui::{Key, Modifiers};
use std::path::PathBuf;
mod backup_tip;
mod invalid_smf;
mod missing_last;
mod open;
pub(crate) mod performance;
pub(crate) use backup_tip::close_notice as close_backup_notice;
pub(super) use backup_tip::notice_dialog as backup_notice_dialog;
pub(super) use invalid_smf::show as invalid_smf_dialog;
pub(crate) use missing_last::create_new_song;
pub(super) use missing_last::show as missing_last_dialog;
pub use open::open_dialog;
#[cfg(test)]
#[path = "files/local_migration_test.rs"]
mod local_migration_test;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub enum Action {
    Save,
    EnableAutoSave,
    New,
    Load(PathBuf),
    LoadWithProgress(PathBuf),
    LoadAndPlay(PathBuf),
    Close,
}

impl App {
    pub fn allow_departure(&mut self, action: Action) -> bool {
        if self.config.navigation.departure.is_some() {
            return false;
        }
        if let Err(error) = self.remember_loaded() {
            self.load_error = Some(error);
            return false;
        }
        if matches!(action, Action::Save) {
            self.save_now();
            return false;
        }
        let enabling = matches!(action, Action::EnableAutoSave);
        let mut error = None;
        if !enabling && self.saved.auto_save_enabled() {
            match self.saved.save() {
                Ok(()) => return true,
                Err(failure) => {
                    self.saved.auto_save_failed(failure.clone());
                    error = Some(failure);
                }
            }
        }
        if self.loaded.is_some()
            && (enabling || matches!(action, Action::Save) || self.saved.dirty())
        {
            self.config.navigation.backup_tip_visible = false;
            self.config.navigation.departure = Some(action);
            self.config.navigation.departure_error = error;
            return false;
        }
        true
    }

    fn save_now(&mut self) {
        if self.saved.is_untitled() {
            crate::play::transport::stop(self);
        }
        match self.saved.save_confirmed() {
            Ok(replaced) => {
                self.sync_saved_path();
                if replaced {
                    backup_tip::notice(self);
                }
            }
            Err(error) if error == crate::store::saved::SAVE_CANCELLED => {}
            Err(error) => {
                if self.saved.auto_save_enabled() {
                    self.saved.auto_save_failed(error.clone());
                } else {
                    self.saved.error = Some(error.clone());
                }
                self.config.navigation.departure = Some(Action::Save);
                self.config.navigation.departure_error = Some(error);
            }
        }
    }

    pub(super) fn finish_departure(&mut self, save: bool, ctx: &egui::Context) {
        if save && self.saved.is_untitled() {
            crate::play::transport::stop(self);
        }
        let result = if !save {
            Ok(())
        } else if matches!(
            self.config.navigation.departure,
            Some(Action::EnableAutoSave)
        ) {
            self.enable_auto_save()
        } else {
            self.remember_loaded()
                .and_then(|_| self.saved.save_confirmed().map(|_| ()))
        };
        if let Err(error) = result {
            self.saved.error = Some(error.clone());
            self.config.navigation.departure_error = Some(error);
            return;
        }
        self.sync_saved_path();
        // Persist plugin caches before approving a close, including the discard path.
        if matches!(self.config.navigation.departure, Some(Action::Close))
            && let Err(error) = self.state_cache.save()
        {
            self.config.navigation.departure_error = Some(error);
            return;
        }
        let Some(action) = self.config.navigation.departure.take() else {
            return;
        };
        self.config.navigation.departure_error = None;
        self.saved.error = None;
        match action {
            Action::Save | Action::EnableAutoSave => {}
            Action::New => super::busy::request(self, ctx, super::busy::Task::New),
            Action::Load(path) => self.load_mid_unchecked(path),
            Action::LoadWithProgress(path) => super::opening::request(self, path, false),
            Action::LoadAndPlay(path) => self.load_and_play(path),
            Action::Close => {
                self.config.navigation.close_approved = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

pub fn inputs(app: &mut App, ctx: &egui::Context) {
    if app.config.navigation.departure.is_some() {
        return;
    }
    let dropped = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .find_map(|file| file.path.clone())
    });
    if let Some(path) = dropped {
        super::opening::request_drop(app, path);
        // A file drop owns this frame, including any simultaneously pressed keys.
        super::comma_menu::swallow(ctx);
        return;
    }
    if !super::recent::available(app, ctx) || app.recent.open {
        return;
    }
    crate::view::commands::route(app, ctx);
}

pub(crate) fn open_backup(app: &mut App, _ctx: &egui::Context) {
    if let Some(root) = &app.saved.directory
        && let Err(e) = backup_tip::open_directory(&root.join("backup"))
    {
        app.config.navigation.departure_error = Some(e);
    }
}

pub fn confirmation(app: &mut App, ctx: &egui::Context) {
    backup_tip::prepare(app);
    crate::view::commands::route(app, ctx);
    if app.config.navigation.departure.is_none() {
        return;
    }
    let closing = matches!(app.config.navigation.departure, Some(Action::Close));
    let saving = matches!(app.config.navigation.departure, Some(Action::Save));
    let creating = matches!(app.config.navigation.departure, Some(Action::New));
    let enabling = matches!(
        app.config.navigation.departure,
        Some(Action::EnableAutoSave)
    );
    let failed = app.config.navigation.departure_error.is_some();
    let mut save = false;
    let mut discard = false;
    let mut resume = false;
    egui::Modal::new(egui::Id::new("file-confirmation")).show(ctx, |ui| {
        ui.set_min_width(370.0);
        if let Some(loaded) = &app.loaded {
            ui.strong(&loaded.name);
        }
        backup_tip::show(app, ui);
        if let Some(error) = &app.config.navigation.departure_error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if enabling {
            ui.label("現在の編集内容を保存して、Auto saveを有効にしますか？");
            save |= ui.button("y: 保存してAuto saveを有効にする").clicked();
            resume |= ui.button("n / esc: 有効にせず編集を続ける").clicked();
            resume |= discard;
            discard = false;
        } else if saving {
            save |= ui
                .button(if failed {
                    "y: 保存を再試行"
                } else if app.saved.is_untitled() {
                    "y: 名前を付けて保存する"
                } else {
                    "y: 上書き保存する"
                })
                .clicked();
            resume |= ui.button("n / esc: 保存せず編集を続ける").clicked();
            resume |= discard;
            discard = false;
        } else if closing {
            save |= ui
                .button(if failed {
                    "y: 保存を再試行してアプリ終了"
                } else {
                    "y: 上書き保存してアプリ終了"
                })
                .clicked();
            discard |= ui.button("n: 編集を破棄してアプリ終了").clicked();
            resume |= ui.button("esc: 編集を続ける").clicked();
        } else if creating {
            save |= ui
                .button(if failed {
                    "y: 保存を再試行して新規作成"
                } else {
                    "y: 保存して新規作成"
                })
                .clicked();
            discard |= ui.button("n: 編集を破棄して新規作成").clicked();
            resume |= ui.button("esc: 編集を続ける").clicked();
        } else {
            save |= ui
                .button(if failed {
                    "y: 保存を再試行してファイルを開く"
                } else {
                    "y: 上書き保存してファイルを開く"
                })
                .clicked();
            discard |= ui.button("n: 編集を破棄してファイルを開く").clicked();
            resume |= ui.button("esc: 編集を続ける").clicked();
        }
    });
    super::comma_menu::swallow(ctx);
    if resume {
        app.config.navigation.departure = None;
        app.config.navigation.departure_error = None;
    } else if save {
        app.finish_departure(true, ctx);
    } else if discard {
        app.finish_departure(false, ctx);
    }
}
