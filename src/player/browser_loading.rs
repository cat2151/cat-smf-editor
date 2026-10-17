//! Initial browser loading lasts until the requested window actually becomes visible.
use cat_smf_editor::resident_ipc::Open;

#[derive(Default)]
pub(super) struct Loading {
    shown: [bool; 2],
    pending: Option<(usize, &'static str)>,
}

impl Loading {
    pub fn request(&mut self, open: Open) {
        let (index, title) = match open {
            Open::Patch { .. } | Open::Layer { .. } => (0, "Browse patches"),
            Open::Effect { .. } => (1, "Browse effects"),
            Open::MasterEffect { .. } => (1, "Browse master effects"),
        };
        self.pending = (!self.shown[index]).then_some((index, title));
    }

    pub fn title(&self) -> Option<&'static str> {
        self.pending.map(|(_, title)| title)
    }

    pub fn observe(&mut self, visible: bool) {
        if visible && let Some((index, _)) = self.pending.take() {
            self.shown[index] = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_browser_waits_for_visibility_only_on_its_first_open() {
        let mut loading = Loading::default();
        loading.request(Open::Patch { track: 1 });
        assert_eq!(loading.title(), Some("Browse patches"));
        loading.observe(false);
        assert!(loading.title().is_some());
        loading.observe(true);
        assert_eq!(loading.title(), None);
        loading.request(Open::Effect {
            track: 1,
            slot: None,
        });
        assert_eq!(loading.title(), Some("Browse effects"));
        loading.observe(true);
        loading.request(Open::Patch { track: 2 });
        assert_eq!(loading.title(), None);
        loading.request(Open::MasterEffect { slot: None });
        assert_eq!(loading.title(), None);
    }

    #[test]
    fn first_master_effect_open_waits_for_its_own_window_title() {
        let mut loading = Loading::default();
        loading.request(Open::MasterEffect { slot: None });
        assert_eq!(loading.title(), Some("Browse master effects"));
        loading.observe(true);
        loading.request(Open::Effect {
            track: 1,
            slot: None,
        });
        assert_eq!(loading.title(), None);
    }
}
