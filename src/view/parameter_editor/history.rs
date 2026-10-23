use super::{App, Parameter, PluginKey};
use crate::view::selection_history::{History, AUTOMATION};
pub(super) fn identity(plugin: &PluginKey, parameter: &Parameter) -> String {
    serde_json::to_string(&(&plugin.format, &plugin.id, &parameter.stable_id)).unwrap()
}
pub(super) fn load(app: &mut App) -> History {
    let previous = std::mem::take(&mut app.selection_history.automation);
    let history = History::load(app, AUTOMATION, previous);
    app.selection_history.automation = history.clone();
    history
}
pub(super) fn record(app: &mut App, query: &str, identity: &str) {
    app.selection_history.automation.record(query, identity);
    if let Err(error) = app.selection_history.automation.save(app, AUTOMATION) {
        app.config.error = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::parameter_editor::{filtered, tests::picker};
    #[test]
    fn short_query_history_overrides_recency_after_restart_and_is_plugin_scoped() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("status.json");
        let mut a = crate::view::parameter_editor::tests::app();
        a.config = crate::store::config::Store::open(config.clone());
        let mut p = picker();
        let mut second = p.parameters[0].clone();
        second.stable_id = "102".into();
        p.parameters.push(second);
        let first = identity(&p.plugin, &p.parameters[0]);
        let second = identity(&p.plugin, &p.parameters[1]);
        record(&mut a, "on", &first);
        record(&mut a, "音色", &second);
        let mut b = crate::view::parameter_editor::tests::app();
        b.config = crate::store::config::Store::open(config);
        p.history = load(&mut b);
        assert_eq!(filtered(&mut p), vec![1, 0]);
        p.query = "on".into();
        assert_eq!(filtered(&mut p), vec![0, 1]);
        p.query = "音色 フィルター".into();
        assert_eq!(filtered(&mut p), vec![1, 0]);
        p.plugin.id = "another-plugin".into();
        assert_eq!(filtered(&mut p), vec![0, 1]);
        p.query = "no matches".into();
        assert!(filtered(&mut p).is_empty());
    }
    #[test]
    fn failed_creation_does_not_record_history() {
        let mut a = crate::view::parameter_editor::tests::app();
        a.parameter_editor.picker = Some(picker());
        assert!(crate::view::parameter_editor::create(&mut a).is_err());
        let p = a.parameter_editor.picker.as_ref().unwrap();
        assert_eq!(
            a.selection_history
                .automation
                .rank("", &identity(&p.plugin, &p.parameters[0])),
            (2, 0)
        );
    }
}
