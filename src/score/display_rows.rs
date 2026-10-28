//! Merge source rows and missing beats without allocating one object per beat.
use crate::{
    model::Event,
    score::musical_time::{BeatRun, MeterMap, Position},
};

enum Block {
    Event(usize),
    Beats {
        run: usize,
        first: u128,
    },
    /// One row per measure without source rows; `first` is the beat index of its first measure.
    Measures {
        run: usize,
        first: u128,
    },
}
pub struct DisplayRows {
    runs: Vec<BeatRun>,
    blocks: Vec<(u128, Block)>,
    count: u128,
    first: usize,
}
pub enum DisplayRow {
    Event(usize),
    Beat(Position),
}
impl DisplayRows {
    pub fn new(events: &[Event], sources: &[usize], map: Option<&MeterMap>) -> Self {
        let runs = map.map_or_else(Vec::new, |m| {
            events.last().map_or_else(Vec::new, |e| m.beat_runs(e.tick))
        });
        let mut result = Self {
            runs,
            blocks: Vec::new(),
            count: 0,
            first: 0,
        };
        let measure = |tick| map.map(|m| m.position(tick).measure);
        let collapsed = events
            .iter()
            .filter(|e| crate::score::same_measure::owned(e))
            .filter_map(|e| measure(e.tick))
            .collect::<std::collections::BTreeSet<_>>();
        let mut run = 0;
        let mut first = 0;
        let mut previous = None;
        for &source in sources {
            let tick = events[source].tick;
            let occupied = [previous, measure(tick)];
            while run < result.runs.len() && result.runs[run].tick <= tick {
                let r = &result.runs[run];
                let units = u128::from(tick - r.tick) * r.denominator;
                let end = units.div_ceil(r.step).min(r.count);
                result.push_gap(run, first, end, occupied, &collapsed);
                let r = &result.runs[run];
                first = (units / r.step + 1).min(r.count);
                if first == r.count {
                    run += 1;
                    first = 0;
                } else {
                    break;
                }
            }
            result.push(Block::Event(source), 1);
            previous = measure(tick);
        }
        while run < result.runs.len() {
            let count = result.runs[run].count;
            result.push_gap(run, first, count, [previous, None], &collapsed);
            run += 1;
            first = 0;
        }
        result
    }
    /// Beats `first..end`; a measure that holds no source row keeps only its first beat.
    fn push_gap(
        &mut self,
        run: usize,
        first: u128,
        end: u128,
        occupied: [Option<u128>; 2],
        collapsed: &std::collections::BTreeSet<u128>,
    ) {
        let r = &self.runs[run];
        let per = r.beats_per_measure();
        let base = r.position(0).measure;
        let occupied: Vec<u128> = occupied
            .into_iter()
            .flatten()
            .chain(collapsed.iter().copied())
            .filter_map(|m| m.checked_sub(base))
            .collect();
        let mut beat = first;
        while beat < end {
            let measure = beat / per;
            if collapsed.contains(&(base + measure)) {
                beat = end.min((measure + 1) * per);
                continue;
            }
            if beat.is_multiple_of(per) && !occupied.contains(&measure) {
                let stop = occupied
                    .iter()
                    .copied()
                    .filter(|&m| m > measure)
                    .fold(end.div_ceil(per), u128::min);
                self.push(Block::Measures { run, first: beat }, stop - measure);
                beat = stop * per;
            } else {
                let stop = end.min((measure + 1) * per);
                self.push(Block::Beats { run, first: beat }, stop - beat);
                beat = stop;
            }
        }
    }
    fn push(&mut self, block: Block, count: u128) {
        self.blocks.push((self.count, block));
        self.count += count;
    }
    pub fn len(&self) -> usize {
        self.count
            .saturating_sub(self.first as u128)
            .min(usize::MAX as u128) as usize
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn block(&self, index: usize) -> &(u128, Block) {
        let block = self
            .blocks
            .partition_point(|(start, _)| *start <= index as u128)
            - 1;
        &self.blocks[block]
    }
    pub fn get(&self, index: usize) -> DisplayRow {
        let index = index + self.first;
        let (start, kind) = self.block(index);
        match kind {
            Block::Event(source) => DisplayRow::Event(*source),
            Block::Beats { run, first } => {
                DisplayRow::Beat(self.runs[*run].position(*first + index as u128 - start))
            }
            Block::Measures { run, first } => {
                let r = &self.runs[*run];
                DisplayRow::Beat(
                    r.position(*first + (index as u128 - start) * r.beats_per_measure()),
                )
            }
        }
    }
    /// Restrict the ordinary event list to an independently edited body, preserving
    /// source event indices and musical positions for all existing note tools.
    pub fn clip_to_measure(&mut self, events: &[Event], map: &MeterMap, range: (u64, u64)) {
        let lower = |tick: u64| {
            let mut lo = 0;
            let mut hi = self.len();
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let at = match self.get(mid) {
                    DisplayRow::Event(i) => events[i].tick,
                    DisplayRow::Beat(p) => map.tick_at(&p).unwrap_or(u64::MAX),
                };
                if at < tick {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            lo
        };
        let first = lower(range.0);
        let end = lower(range.1);
        self.count = (self.first + end) as u128;
        self.first += first;
    }
    /// A row standing for a whole measure that holds no source row.
    pub fn is_measure(&self, index: usize) -> bool {
        matches!(self.block(index + self.first).1, Block::Measures { .. })
    }
    /// Source events in a visible row range, without expanding empty beat runs.
    pub fn events_in_rows(
        &self,
        range: std::ops::Range<usize>,
    ) -> impl Iterator<Item = usize> + '_ {
        self.blocks
            .iter()
            .filter_map(move |(row, block)| match block {
                Block::Event(i)
                    if *row >= (range.start + self.first) as u128
                        && *row < (range.end + self.first) as u128
                        && *row < self.count =>
                {
                    Some(*i)
                }
                _ => None,
            })
    }
    pub fn event_row(&self, source: usize) -> Option<usize> {
        self.blocks.iter().find_map(|(row, b)| match b {
            Block::Event(i) if *i == source && *row >= self.first as u128 && *row < self.count => {
                usize::try_from(*row - self.first as u128).ok()
            }
            _ => None,
        })
    }
}
