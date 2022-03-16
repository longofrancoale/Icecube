use core::cmp;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RangeSet {
    ranges: [Range; 32],

    in_use: u32,
}

impl RangeSet {
    pub const fn new() -> RangeSet {
        RangeSet {
            ranges: [Range { start: 0, end: 0 }; 32],
            in_use: 0,
        }
    }

    pub fn entries(&self) -> &[Range] {
        &self.ranges[..self.in_use as usize]
    }

    fn delete(&mut self, idx: usize) {
        assert!(idx < self.in_use as usize, "Index out of bounds");

        for ii in idx..self.in_use as usize - 1 {
            self.ranges.swap(ii, ii + 1);
        }

        self.in_use -= 1;
    }

    pub fn insert(&mut self, mut range: Range) {
        assert!(range.start <= range.end, "Invalid range shape");

        'try_merges: loop {
            for ii in 0..self.in_use as usize {
                let ent = self.ranges[ii];

                if !overlaps(
                    range.start,
                    range.end.saturating_add(1),
                    ent.start,
                    ent.end.saturating_add(1),
                ) {
                    continue;
                }

                range.start = cmp::min(range.start, ent.start);
                range.end = cmp::max(range.end, ent.end);

                self.delete(ii);

                continue 'try_merges;
            }

            break;
        }

        assert!(
            (self.in_use as usize) < self.ranges.len(),
            "Too many entries in RangeSet on insert"
        );

        self.ranges[self.in_use as usize] = range;
        self.in_use += 1;
    }

    pub fn remove(&mut self, range: Range) {
        assert!(range.start <= range.end, "Invalid range shape");

        'try_subtractions: loop {
            for ii in 0..self.in_use as usize {
                let ent = self.ranges[ii];

                if !overlaps(range.start, range.end, ent.start, ent.end) {
                    continue;
                }

                if contains(ent.start, ent.end, range.start, range.end) {
                    self.delete(ii);
                    continue 'try_subtractions;
                }

                if range.start <= ent.start {
                    self.ranges[ii].start = range.end.saturating_add(1);
                } else if range.end >= ent.end {
                    self.ranges[ii].end = range.start.saturating_sub(1);
                } else {
                    self.ranges[ii].start = range.end.saturating_add(1);

                    assert!(
                        (self.in_use as usize) < self.ranges.len(),
                        "Too many entries in RangeSet on split"
                    );

                    self.ranges[self.in_use as usize] = Range {
                        start: ent.start,
                        end: range.start.saturating_sub(1),
                    };
                    self.in_use += 1;
                    continue 'try_subtractions;
                }
            }

            break;
        }
    }

    pub fn subtract(&mut self, rs: &RangeSet) {
        for &ent in rs.entries() {
            self.remove(ent);
        }
    }

    pub fn sum(&self) -> Option<u64> {
        self.entries()
            .iter()
            .try_fold(0u64, |acc, x| Some(acc + (x.end - x.start).checked_add(1)?))
    }

    pub fn allocate(&mut self, size: u64, align: u64) -> Option<usize> {
        if size == 0 {
            return None;
        }

        if align.count_ones() != 1 {
            return None;
        }

        let alignmask = align - 1;

        let mut allocation = None;
        for ent in self.entries() {
            let align_fix = (align - (ent.start & alignmask)) & alignmask;

            let base = ent.start;
            let end = base.checked_add(size - 1)?.checked_add(align_fix)?;

            if base > core::usize::MAX as u64 || end > core::usize::MAX as u64 {
                continue;
            }

            if end > ent.end {
                continue;
            }

            let prev_size = allocation.map(|(base, end, _)| end - base);

            if allocation.is_none() || prev_size.unwrap() > end - base {
                allocation = Some((base, end, (base + align_fix) as usize));
            }
        }

        allocation.map(|(base, end, ptr)| {
            self.remove(Range {
                start: base,
                end: end,
            });

            ptr
        })
    }
}

fn overlaps(mut x1: u64, mut x2: u64, mut y1: u64, mut y2: u64) -> bool {
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }

    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }

    x1 <= y2 && y1 <= x2
}

fn contains(mut x1: u64, mut x2: u64, mut y1: u64, mut y2: u64) -> bool {
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }

    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }

    x1 >= y1 && x2 <= y2
}
