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
        };
        let measure = |tick| map.map(|m| m.position(tick).measure);
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
                result.push_gap(run, first, end, occupied);
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
            result.push_gap(run, first, count, [previous, None]);
            run += 1;
            first = 0;
        }
        result
    }
    /// Beats `first..end`; a measure that holds no source row keeps only its first beat.
    fn push_gap(&mut self, run: usize, first: u128, end: u128, occupied: [Option<u128>; 2]) {
        let r = &self.runs[run];
        let per = r.beats_per_measure();
        let base = r.position(0).measure;
        let occupied: Vec<u128> = occupied
            .into_iter()
            .flatten()
            .filter_map(|m| m.checked_sub(base))
            .collect();
        let mut beat = first;
        while beat < end {
            let measure = beat / per;
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
        self.count.min(usize::MAX as u128) as usize
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    fn block(&self, index: usize) -> &(u128, Block) {
        let block = self
            .blocks
            .partition_point(|(start, _)| *start <= index as u128)
            - 1;
        &self.blocks[block]
    }
    pub fn get(&self, index: usize) -> DisplayRow {
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
    /// A row standing for a whole measure that holds no source row.
    pub fn is_measure(&self, index: usize) -> bool {
        matches!(self.block(index).1, Block::Measures { .. })
    }
    pub fn event_row(&self, source: usize) -> Option<usize> {
        self.blocks.iter().find_map(|(row, b)| match b {
            Block::Event(i) if *i == source => usize::try_from(*row).ok(),
            _ => None,
        })
    }
}
