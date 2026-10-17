//! The editor's one resident cat-plugin-player: started by the first browser
//! request, then only asked to show a browser. Dropping the link closes the
//! pipes, which ends the player.
use crate::app::App;
use crate::player::resident::{answer, Target};
use crate::player::selection::{resolve, Browser};
use cat_smf_editor::patch_identity::PatchIdentity;
use cat_smf_editor::resident_ipc::{self, Listener, Open, Selected, Sender, ToEditor, ToPlayer};
use cat_smf_editor::resident_log;
use eframe::egui;
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

fn log(message: impl std::fmt::Display) {
    resident_log::log("editor", message);
}

/// The pipe is read once per frame, so a long frame or a long idle delays every answer.
pub fn trace_frame(app: &mut App, started: Instant) {
    let Some(link) = &mut app.player_selection.resident else {
        return;
    };
    let now = Instant::now();
    let idle = link.last_frame.map_or(Duration::ZERO, |end| started - end);
    if now - started > Duration::from_millis(50) || idle > Duration::from_millis(200) {
        log(format_args!(
            "frame took {} ms after {} ms idle",
            (now - started).as_millis(),
            idle.as_millis()
        ));
    }
    link.last_frame = Some(now);
}

pub fn trace_slow(app: &App, part: &str, started: Instant) {
    let took = started.elapsed();
    if app.player_selection.resident.is_some() && took > Duration::from_millis(50) {
        log(format_args!("{part} took {} ms", took.as_millis()));
    }
}

/// Starts a player that connects to the link `name`; `None` when there is no
/// process to watch.
pub type Launch = fn(executable: &Path, name: &str) -> std::io::Result<Option<Child>>;

pub struct Request {
    pub browser: Browser,
    pub track: usize,
    pub slot: Option<usize>,
    pub bulk: bool,
}

pub struct Link {
    name: String,
    events: mpsc::Receiver<Event>,
    context: Arc<Mutex<Option<egui::Context>>>,
    child: Option<Child>,
    /// `None` until the player has connected.
    sender: Option<Sender>,
    target: Target,
    panes: [String; 2],
    query: String,
    cursor: Option<PatchIdentity>,
    last_frame: Option<Instant>,
    activity: super::browser_activity::Activity,
    loading: super::browser_loading::Loading,
}

enum Event {
    Connected(Sender),
    Selected(Selected),
    Cursor(String, Option<PatchIdentity>),
    Panes(String, [String; 2], String),
    Closed(Option<String>),
}

/// Shows the requested browser, starting the player if this editor has none.
pub fn request(app: &mut App, request: Request) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let open = match request.browser {
        Browser::Patch => Open::Layer {
            track: request.track,
            layer: app
                .player_selection
                .layer
                .or_else(|| {
                    loaded.patches[request.track]
                        .layers
                        .first()
                        .map(|l| l.settings.id)
                })
                .ok_or("Track has no instrument layer")?,
            song: app.song_id,
        },
        Browser::Effect => Open::Effect {
            track: request.track,
            slot: request.slot,
        },
        Browser::MasterEffect => Open::MasterEffect { slot: request.slot },
    };
    let target = Target {
        open,
        bulk: request.bulk,
        original: loaded.original.clone(),
    };
    let panes = app
        .config
        .browser_panes
        .get(&pane_context(&target))
        .cloned()
        .unwrap_or_default();
    let query = app
        .config
        .browser_queries
        .get(&pane_context(&target))
        .cloned()
        .unwrap_or_default();
    let cursor = app
        .config
        .patch_cursors
        .get(&pane_context(&target))
        .cloned();
    log(format_args!("request {target:?}"));
    app.audition.browser_used();
    if let Some(link) = &mut app.player_selection.resident {
        link.target = target;
        link.panes = panes;
        link.query = query;
        link.cursor = cursor;
        return link.show();
    }
    let executable = resolve(
        app.player_selection.executable.clone(),
        &std::env::current_exe().unwrap_or_default(),
        std::env::var_os("PATH"),
    )?;
    let launch = app.player_selection.launch.unwrap_or(spawn);
    app.player_selection.resident = Some(Link::start(
        &executable,
        launch,
        target,
        panes,
        query,
        cursor,
    )?);
    Ok(())
}

/// Answers the player's selections and notices when it has gone.
pub fn poll(app: &mut App, ctx: &egui::Context) {
    let Some(mut link) = app.player_selection.resident.take() else {
        return;
    };
    link.context
        .lock()
        .unwrap()
        .get_or_insert_with(|| ctx.clone());
    let closed = loop {
        let event = match link.events.try_recv() {
            Ok(event) => event,
            Err(mpsc::TryRecvError::Empty) => break link.exited_early(),
            Err(mpsc::TryRecvError::Disconnected) => break Some(None),
        };
        match event {
            Event::Connected(sender) => {
                log("player connected");
                link.sender = Some(sender);
                if let Err(error) = link.show() {
                    break Some(Some(error));
                }
            }
            Event::Selected(selected) => {
                app.audition.browser_used();
                log(format_args!(
                    "selected {:?} {} {:?} ({} bytes); target {:?}",
                    selected.target,
                    selected.plugin.id,
                    selected.preset_name,
                    selected.state.len,
                    link.target
                ));
                let Some(sender) = &mut link.sender else {
                    log("selection before connection; dropped");
                    continue;
                };
                if let Err(error) = answer(app, selected, Some(&link.target), sender) {
                    break Some(Some(format!("Could not answer the player: {error}")));
                }
            }
            Event::Panes(context, panes, query) => {
                app.config.select_browser_panes(context, panes, query)
            }
            Event::Cursor(context, patch) => app.config.select_patch_cursor(context, patch),
            Event::Closed(error) => break Some(error),
        }
    };
    match closed {
        None => {
            if link.sender.is_none() || link.loading.title().is_some() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            app.player_selection.resident = Some(link);
        }
        Some(error) => {
            log(format_args!("link closed: {error:?}"));
            app.player_selection.open = true;
            app.player_selection.error = Some(match error {
                Some(error) => format!("cat-plugin-player stopped: {error}"),
                None => "cat-plugin-player closed".into(),
            });
            ctx.request_repaint();
        }
    }
}

/// Refresh before audition expiry, including after a long paused GUI frame.
pub fn refresh_browser_activity(app: &mut App) {
    if let Some(link) = &mut app.player_selection.resident
        && let Some(child) = &link.child
        && let Some(title) = link.loading.title()
    {
        link.loading
            .observe(super::browser_activity::visible(child.id(), title));
    }
    if let Some(link) = &mut app.player_selection.resident
        && let Some(child) = &link.child
        && link.activity.poll(child.id())
    {
        app.audition.browser_used();
    }
}

fn pane_context(target: &Target) -> String {
    if let Open::Layer { track, layer, .. } = target.open {
        return serde_json::to_string(&(&target.original, "patch", track, layer)).unwrap();
    }
    let (kind, track) = match target.open {
        Open::Patch { track } | Open::Layer { track, .. } => ("patch", Some(track)),
        Open::Effect { track, .. } => ("effect", Some(track)),
        Open::MasterEffect { .. } => ("master_effect", None),
    };
    serde_json::to_string(&(&target.original, kind, track)).unwrap()
}

impl Link {
    fn start(
        executable: &Path,
        launch: Launch,
        target: Target,
        panes: [String; 2],
        query: String,
        cursor: Option<PatchIdentity>,
    ) -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = format!(
            "editor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        resident_log::start("editor");
        log(format_args!(
            "start player {} on {name}",
            executable.display()
        ));
        let listener = Listener::create(&name).map_err(|e| format!("Cannot listen: {e}"))?;
        let child = launch(executable, &name).map_err(|e| format!("Cannot start player: {e}"))?;
        let (deliver, events) = mpsc::channel();
        let context = Arc::new(Mutex::new(None::<egui::Context>));
        let repaint = context.clone();
        // `accept` and `receive` block, so the pipe is read off the GUI thread.
        std::thread::spawn(move || {
            let send = |event| {
                let sent = deliver.send(event).is_ok();
                if let Some(context) = repaint.lock().unwrap().as_ref() {
                    context.request_repaint();
                }
                sent
            };
            let mut receiver = match listener.accept() {
                Ok((sender, receiver)) => {
                    if !send(Event::Connected(sender)) {
                        return;
                    }
                    receiver
                }
                Err(error) => {
                    send(Event::Closed(Some(error.to_string())));
                    return;
                }
            };
            loop {
                let event = match receiver.receive::<ToEditor>() {
                    Ok(Some(ToEditor::Selected(selected))) => {
                        log(format_args!("pipe selected {:?}", selected.preset_name));
                        Event::Selected(selected)
                    }
                    Ok(Some(ToEditor::Panes {
                        context,
                        panes,
                        query,
                    })) => Event::Panes(context, panes, query),
                    Ok(Some(ToEditor::Cursor { context, patch })) => Event::Cursor(context, patch),
                    Ok(None) => Event::Closed(None),
                    Err(error) => Event::Closed(Some(error.to_string())),
                };
                let closed = matches!(event, Event::Closed(_));
                if !send(event) || closed {
                    return;
                }
            }
        });
        let mut loading = super::browser_loading::Loading::default();
        loading.request(target.open);
        Ok(Self {
            name,
            events,
            context,
            child,
            sender: None,
            target,
            panes,
            query,
            cursor,
            last_frame: None,
            activity: Default::default(),
            loading,
        })
    }

    /// Before the player connects, the target is sent on connection instead.
    fn show(&mut self) -> Result<(), String> {
        self.loading.request(self.target.open);
        match &mut self.sender {
            Some(sender) => sender
                .send(&ToPlayer::OpenFiltered {
                    open: self.target.open,
                    context: pane_context(&self.target),
                    panes: self.panes.clone(),
                    query: self.query.clone(),
                    cursor: self.cursor.clone(),
                })
                .map_err(|e| format!("Cannot reach cat-plugin-player: {e}")),
            None => Ok(()),
        }
    }

    /// A player that exits before connecting would leave `accept` waiting
    /// forever; connecting from here releases that thread.
    fn exited_early(&mut self) -> Option<Option<String>> {
        if self.sender.is_some() {
            return None;
        }
        let status = self.child.as_mut()?.try_wait().ok()??;
        drop(resident_ipc::connect(&self.name));
        Some(Some(format!("exited before connecting ({status})")))
    }
}

pub fn loading(app: &App) -> bool {
    app.player_selection
        .resident
        .as_ref()
        .is_some_and(|link| link.loading.title().is_some())
}

fn spawn(executable: &Path, name: &str) -> std::io::Result<Option<Child>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new(executable)
        .arg("--resident")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(Some)
}

#[cfg(test)]
#[path = "resident_link_tests.rs"]
mod tests;
