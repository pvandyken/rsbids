use std::{collections::HashSet, fmt, ops::Range};

use itertools::Itertools;
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiRange<I> {
    ranges: Vec<Range<I>>,
}

impl<I> MultiRange<I> {
    pub fn new() -> Self {
        MultiRange { ranges: Vec::new() }
    }
}

impl<I> From<Range<I>> for MultiRange<I> {
    fn from(item: Range<I>) -> Self {
        MultiRange { ranges: vec![item] }
    }
}

impl Into<HashSet<usize>> for MultiRange<usize> {
    fn into(self) -> HashSet<usize> {
        self.ranges.into_iter().flat_map(|range| range).collect()
    }
}

impl Into<HashSet<usize>> for &MultiRange<usize> {
    fn into(self) -> HashSet<usize> {
        self.ranges
            .iter()
            .cloned()
            .flat_map(|range| range)
            .collect()
    }
}

// impl IntoIterator for MultiRange<usize> {
//     type Item = usize;
//     type IntoIter = std::vec::IntoIter<Self::Item>;
//     fn into_iter(self) -> Self::IntoIter {
//         self.ranges.into_iter().flat_map(|range| range)
//     }
// }

impl<I: Ord + Default + Copy> MultiRange<I> {
    pub fn insert(&mut self, value: Range<I>) -> bool {
        match self.ranges.last_mut() {
            Some(prev) => {
                if prev.end == value.start {
                    prev.end = value.end
                } else if prev.end < value.start {
                    self.ranges.push(value);
                } else {
                    self.ranges = self.merge(&Self::from(value)).ranges;
                }
            }
            None => self.ranges.push(value),
        }
        true
    }

    pub fn extend(&mut self, values: &MultiRange<I>) {
        self.ranges = self.merge(values).ranges
    }

    pub fn contains(&self, value: &I) -> bool {
        for range in &self.ranges {
            if value >= &range.start && value < &range.end {
                return true;
            }
        }
        false
    }
    pub fn merge(&self, other: &MultiRange<I>) -> Self {
        let mut new_ranges = Vec::new();
        let mut curr_range: Range<I> = Range::default();
        let mut ours = self.ranges.iter();
        let mut theirs = other.ranges.iter();
        let mut next_right = theirs.find(|i| i.end > curr_range.end);
        let mut next_left = ours.find(|i| i.end > curr_range.end);
        loop {
            let (start, end) = if let Some(right) = next_right {
                if let Some(left) = next_left {
                    if left.start < right.start {
                        if left.end < right.start {
                            next_left = ours.next();
                            (left.start, left.end)
                        } else if left.end < right.end {
                            next_left = ours.find(|i| i.end > right.end);
                            next_right = theirs.next();
                            (left.start, right.end)
                        } else {
                            next_right = theirs.find(|i| i.end > left.end);
                            next_left = ours.next();
                            (left.start, left.end)
                        }
                    } else {
                        if right.end < left.start {
                            next_right = theirs.next();
                            (right.start, right.end)
                        } else if right.end < left.end {
                            next_right = theirs.find(|i| i.end > left.end);
                            next_left = ours.next();
                            (right.start, left.end)
                        } else {
                            next_left = ours.find(|i| i.end > right.end);
                            next_right = theirs.next();
                            (right.start, left.end)
                        }
                    }
                } else {
                    next_right = theirs.next();
                    (right.start, right.end)
                }
            } else {
                if let Some(left) = next_left {
                    next_left = ours.next();
                    (left.start, left.end)
                } else {
                    break;
                }
            };
            if start <= curr_range.end {
                curr_range.end = end
            } else {
                new_ranges.push(curr_range);
                curr_range = start..end;
            }
        }
        new_ranges.push(curr_range);
        Self { ranges: new_ranges }
    }
}

impl MultiRange<usize> {
    pub fn len(&self) {
        let mut len: usize = 0;
        for range in &self.ranges {
            len = (range.start - range.end) + len
        }
    }
}

impl<I: fmt::Display> fmt::Display for MultiRange<I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let formatted = self
            .ranges
            .iter()
            .map(|range| format!("{}..{}", range.start, range.end))
            .join(", ");
        write!(f, "{}", formatted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyVal {
    pub slice: Range<usize>,
    pub delimiter: usize,
}

impl KeyVal {
    pub fn new(slice: Range<usize>, delim: usize) -> KeyVal {
        KeyVal {
            slice,
            delimiter: delim,
        }
    }

    pub fn key_range(&self) -> Range<usize> {
        self.slice.start..self.delimiter
    }

    pub fn val_range(&self) -> Range<usize> {
        self.delimiter + 1..self.slice.end
    }

    pub fn get<'a>(&self, template: &'a str) -> (&'a str, &'a str) {
        (self.get_key(template), self.value(template))
    }

    pub fn get_key<'a>(&self, template: &'a str) -> &'a str {
        &template[self.key_range()]
    }

    pub fn value<'a>(&self, template: &'a str) -> &'a str {
        &template[self.val_range()]
    }
}

#[derive(Debug)]
pub enum ComponentType {
    ZeroType(Range<usize>),
    OneType(KeyVal),
    TwoType(Vec<Elements>),
}

#[derive(Debug)]
pub enum Elements {
    KeyVal(KeyVal),
    Suffix(Range<usize>),
    Part(Range<usize>),
}
#[derive(Debug)]
pub enum Primitive {
    Value(usize, usize),
    Key(usize, usize),
    Suffix(usize, usize),
    Part(usize, usize),
}

#[derive(Debug)]
pub enum PrePrimitive {
    Prefix,
    KeyLike(usize),
    ValueLike(usize),
}
