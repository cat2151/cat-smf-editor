//! Typed requests bind the target when the initiating UI constructs them.
use super::*;
use crate::view::patterns::Action;
use cat_smf_editor::edit::meta_input::Kind;

#[derive(Clone)]
pub(crate) enum Operation {
    PatternWork,
    Pattern {
        action: Action,
        pattern: Option<u64>,
        value: Option<String>,
        container: Option<u64>,
    },
    InsertEvent {
        kind: Option<Kind>,
        text: Option<String>,
    },
    SameMeasure {
        batch: bool,
        number: Option<u64>,
    },
    Views,
    GuideEvent {
        moving: bool,
    },
}
impl Operation {
    pub(crate) fn pattern(action: Action) -> Self {
        Self::Pattern {
            action,
            pattern: None,
            value: None,
            container: None,
        }
    }
}
pub(crate) struct Request {
    target: Target,
    operation: Operation,
}
impl Request {
    pub(crate) fn new(app: &App, operation: Operation) -> Self {
        Self {
            target: Target::of(app),
            operation,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Execution {
    Completed,
    Input,
}

/// UI-independent execution; saved requests cannot silently move to another target.
pub(crate) fn submit(
    app: &mut App,
    ctx: &egui::Context,
    request: Request,
) -> Result<Execution, String> {
    if request.target != Target::of(app) {
        return Err("Command target changed".into());
    }
    if crate::view::modal::is_open(app) || app.patterns.confirming_creation() {
        return Err("Another command owns input".into());
    }
    if app.patterns.workspace.input_open()
        || matches!(
            owner(app, ctx),
            Screen::Waiting
                | Screen::Missing
                | Screen::Invalid
                | Screen::Confirm
                | Screen::Backup
                | Screen::EditStyle
                | Screen::TrackRename
                | Screen::Mml
                | Screen::Note
                | Screen::LayerInput
                | Screen::DrumInput
                | Screen::ParameterInput
                | Screen::ParameterPicker
                | Screen::Singer
                | Screen::Lyrics
        )
    {
        return Err("Another editor owns input".into());
    }
    if app.loaded.is_none() {
        return Err("No SMF is loaded".into());
    }
    super::super::command_input::start(app, ctx, request.operation)
}
pub(crate) fn invoke(app: &mut App, ctx: &egui::Context, operation: Operation) -> bool {
    match submit(app, ctx, Request::new(app, operation)) {
        Ok(_) => true,
        Err(error) => {
            app.load_error = Some(error);
            false
        }
    }
}
impl Command {
    pub(crate) fn operation(
        screen: Screen,
        id: impl Into<String>,
        name: impl Into<String>,
        operation: Operation,
    ) -> Self {
        Self {
            id: id.into(),
            screen,
            name: name.into(),
            keys: vec![],
            repeat: false,
            target: None,
            run: Box::new(move |a, c| invoke(a, c, operation.clone())),
        }
    }
}

#[cfg(test)]
mod tests;
