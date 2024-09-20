use std::cmp;

pub mod config;
pub mod directory;
pub mod macros;
pub mod style;

struct RangeCkecker {
    range: (isize, isize),
}

impl RangeCkecker {
    fn new(range: (isize, isize)) -> Self {
        Self { range }
    }

    /// Return range checked candidate with its overflows
    fn check(&self, candidate: isize) -> (isize, isize, isize) {
        let mut overflow_low = 0;
        let mut overflow_high = 0;
        let mut checked = candidate;
        if candidate < self.range.0 {
            overflow_high = self.range.0 - candidate;
            checked = self.range.0;
        }
        if candidate > self.range.1 {
            overflow_low = candidate - self.range.1;
            checked = self.range.1;
        }
        (overflow_high, checked, overflow_low)
    }
}

/// Surely there must be a better way.
pub fn get_borders(
    lines_size: isize,
    slice_size: isize,
    line_index: isize,
) -> (isize, isize, isize) {
    let checker = RangeCkecker::new((0, lines_size - 1));

    let slice_size_half = slice_size / 2;
    let odd = slice_size % 2;

    let candidate_high = line_index - slice_size_half;
    let candidate_low = line_index + slice_size_half - 2 + odd;

    let (overflow_high, mut checked_high, _) = checker.check(candidate_high);
    let (_, mut checked_low, mut overflow_low) = checker.check(candidate_low);

    if lines_size > slice_size {
        overflow_low += cmp::min(lines_size - slice_size_half + 1, 0);
    } else {
        overflow_low = 0;
    }

    let lines_index_checked = slice_size_half + overflow_low - overflow_high;

    checked_high -= overflow_low;
    checked_high = cmp::max(checked_high, 0);
    checked_low += overflow_high;
    checked_low = cmp::min(checked_low, lines_size - 1) + 1;

    (checked_high, lines_index_checked, checked_low)
}
