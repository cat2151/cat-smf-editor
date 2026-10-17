//! Track-local dormant chains. Active chains never remain in this cache.
use cat_smf_editor::score::instrument::Patch;

pub(super) struct Cache<T> {
    entries: Vec<(usize, Patch, T)>,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> Cache<T> {
    pub fn put(&mut self, track: usize, patch: Patch, value: T) {
        self.entries.push((track, patch, value));
    }

    /// Exact state wins over a compatible instance needing a state reload.
    pub fn take(&mut self, track: usize, patch: &Patch) -> Option<(T, bool)> {
        let exact = self
            .entries
            .iter()
            .position(|(t, p, _)| *t == track && p == patch);
        let i = exact.or_else(|| {
            self.entries
                .iter()
                .position(|(t, p, _)| *t == track && p.same_plugins(patch))
        })?;
        let (_, _, value) = self.entries.swap_remove(i);
        Some((value, exact.is_none()))
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|(_, _, value)| value)
    }
}

#[cfg(test)]
#[path = "instance_cache_tests.rs"]
mod tests;
