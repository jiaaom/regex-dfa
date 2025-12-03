// Copyright 2015 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Vendored copy of the old `refinery` crate's Partition implementation.
//!
//! This crate implements a simple and fast algorithm for building and refining a partition of the set
//! `[0usize, n)`. The most important type is `Partition` and its most important function is `refine`,
//! which refines the `Partition` according to some subset.

use std::collections::HashSet;
use std::fmt::{Debug, Error, Formatter};
use std::usize;

/// A partition of a set of `usize`s.
#[derive(Clone)]
pub struct Partition {
    // Contains the numbers 0..n in some order.
    elts: Vec<usize>,
    // A list of non-overlapping ranges. If `partition[i] = (j, k)` then the `i`th set in our
    // partition is `elts[j..k]`.
    partition: Vec<(usize, usize)>,
    // If `rev_partition[i] == j` then `i` is in the `j`th set of our partition.
    rev_partition: Vec<usize>,
    // If `rev_elts[i] == j` then `elts[j] = i`.
    rev_elts: Vec<usize>,

    // Some extra space that we allocate up front so that we don't need to allocate in our inner
    // loop. These are all modified during refinement.

    // This stores indices of all parts that intersect with the set we are currently refining with.
    active_sets: HashSet<usize>,
    // For every index in `active_sets`, this stores an index into elts. It points to the first
    // element in the difference between the refining set and the part.
    diff_start: Vec<usize>,
}

/// An iterator over the sets in a `Partition`.
#[derive(Clone, Debug)]
pub struct PartitionIter<'a> {
    next_set_idx: usize,
    partition: &'a Partition,
}

impl Partition {
    /// Constructs a new `Partition`.
    ///
    /// `sets` is an iterator over the sets of the partition. The sets must be non-overlapping, and
    /// they must all be subsets of the half-open range `[0, size)`.
    ///
    /// # Panics
    ///  - if any sets overlap
    ///  - if any sets contain an element larger than or equal to `size`.
    pub fn new<I, J>(sets: I, size: usize) -> Partition
    where I: Iterator<Item=J>, J: Iterator<Item=usize> {
        let mut ret = Partition {
            elts: Vec::with_capacity(size),
            partition: Vec::with_capacity(size),
            rev_partition: vec![usize::MAX; size],
            rev_elts: vec![usize::MAX; size],
            active_sets: HashSet::with_capacity(size),
            diff_start: vec![usize::MAX; size],
        };

        for set in sets {
            let set_idx = ret.partition.len();
            let set_start = ret.elts.len();
            for item in set {
                if item >= size {
                    panic!("all set elements must be smaller than `size`");
                } else if ret.rev_partition[item] != usize::MAX {
                    panic!("the element {} was repeated", item);
                } else {
                    ret.elts.push(item);
                    ret.rev_partition[item] = set_idx;
                    ret.rev_elts[item] = ret.elts.len() - 1;
                }
            }
            ret.partition.push((set_start, ret.elts.len()));
        }

        ret
    }

    /// Iterates over the sets in this partition, each of which is realized by a `&[usize]`.
    pub fn iter(&self) -> PartitionIter<'_> {
        PartitionIter {
            next_set_idx: 0,
            partition: self,
        }
    }

    /// Refines the partition with the given set.
    ///
    /// Every set `s` in the partition is replaced by `s.intersection(refiner)` and
    /// `s.difference(refiner)`.
    ///
    /// This function runs in time that is O(n log n) in the size of `refiner`.
    ///
    /// # Panics
    ///  - if `refiner` contains any elements that are not in the partition
    pub fn refine(&mut self, refiner: &[usize]) {
        self.refine_with_callback(refiner, |_, _, _| {});
    }

    /// Returns one part of this partition.
    pub fn part(&self, part_idx: usize) -> &[usize] {
        let (start, end) = self.partition[part_idx];
        &self.elts[start..end]
    }

    /// The number of parts in this partition.
    pub fn num_parts(&self) -> usize {
        self.partition.len()
    }

    /// Refines the partition with the given set.
    ///
    /// Every set `s` in the partition is replaced by `s.intersection(refiner)` and
    /// `s.difference(refiner)`. Every time we make such a non-trivial (meaning that both the
    /// intersection and the difference are non-empty) replacement, we call the supplied callback
    /// function with arguments `self`, `original_set_idx` and `difference_idx`.
    ///
    /// This function runs in time that is O(n log n) in the size of `refiner`.
    ///
    /// # Panics
    ///  - if `refiner` contains any elements that are not in the partition
    pub fn refine_with_callback<F>(&mut self, refiner: &[usize], mut split_callback: F)
    where F: FnMut(&Partition, usize, usize) {
        self.active_sets.clear();
        for &x in refiner.iter() {
            if x >= self.rev_partition.len() {
                panic!("`refiner` went out of bounds: {:?}", refiner);
            } else if self.rev_partition[x] == usize::MAX {
                panic!("`refiner contained an element ({:?}) that wasn't in the initial partition", x);
            }

            let part_idx = self.rev_partition[x];
            if self.active_sets.insert(part_idx) {
                // We start out saying that everything is in the difference (part \ refiner).
                self.diff_start[part_idx] = self.partition[part_idx].0;
            }

            let old_x_idx = self.rev_elts[x];
            let old_y_idx = self.diff_start[part_idx];
            let y = self.elts[old_y_idx];
            self.elts.swap(old_x_idx, old_y_idx);
            self.diff_start[part_idx] += 1;

            self.rev_elts[x] = old_y_idx;
            self.rev_elts[y] = old_x_idx;
        }

        // Go over the active sets and see which of them were actually split non-trivially.
        for &part_idx in &self.active_sets {
            let (start, end) = self.partition[part_idx];
            let diff_start = self.diff_start[part_idx];

            if diff_start != start && diff_start != end {
                // There was a non-trivial split. The intersection is in [start, diff_start). The
                // difference is in [diff_start, end).
                self.partition[part_idx] = (start, diff_start);
                self.partition.push((diff_start, end));
                for elt_idx in diff_start..end {
                    let elt = self.elts[elt_idx];
                    self.rev_partition[elt] = self.partition.len() - 1;
                    self.rev_elts[elt] = elt_idx;
                }

                split_callback(self, part_idx, self.partition.len() - 1);
            }
        }
    }
}

// This only exists so we can give a `Debug` impl that displays a slice like a set.
struct SliceSet<'a>(&'a [usize]);

impl<'a> Debug for SliceSet<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        fmt.debug_set().entries(self.0.iter()).finish()
    }
}

impl Debug for Partition {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        fmt.debug_set()
            .entries(self.partition.iter()
                     .map(|&(start, end)| { SliceSet(&self.elts[start..end]) }))
            .finish()
    }
}

impl<'a> Iterator for PartitionIter<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        if self.next_set_idx < self.partition.partition.len() {
            let (start, end) = self.partition.partition[self.next_set_idx];
            self.next_set_idx += 1;
            Some(&self.partition.elts[start..end])
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a Partition {
    type Item = &'a [usize];
    type IntoIter = PartitionIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
