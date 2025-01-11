/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::iter::FusedIterator;

/// Stores the most recent item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IterBuffer<I: Iterator> {
    iter: I,
    buffer: Option<I::Item>,
}

impl<I> From<I> for IterBuffer<I>
where
    I: Iterator,
{
    fn from(value: I) -> Self {
        Self {
            iter: value,
            buffer: None,
        }
    }
}

impl<I> Iterator for IterBuffer<I>
where
    I: Iterator<Item: Clone>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer = Some(self.iter.next()?);
        self.buffer.clone()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.buffer = self.iter.nth(n);
        self.buffer.clone()
    }
}

impl<I> FusedIterator for IterBuffer<I> where I: Iterator<Item: Clone> + FusedIterator {}

impl<I> ExactSizeIterator for IterBuffer<I>
where
    I: Iterator<Item: Clone> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        let underlying_len = self.iter.len();
        if underlying_len > 0 && self.buffer.is_some() {
            underlying_len + 1
        } else {
            underlying_len
        }
    }
}

impl<I> IterBuffer<I>
where
    I: Iterator,
{
    /// Returns the stored last value, if any, and voids it.
    pub fn take_stored(&mut self) -> Option<I::Item> {
        self.buffer.take()
    }

    #[allow(dead_code)]
    pub fn is_empty(&mut self) -> bool {
        if self.buffer.is_some() {
            false
        } else if let Some(val) = self.iter.next() {
            self.buffer = Some(val);
            false
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OneOrTwo<T> {
    One([T; 1]),
    Two([T; 2]),
}

impl<T> OneOrTwo<T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::One(x) => x.as_slice(),
            Self::Two(x) => x.as_slice(),
        }
    }
}

impl<I> IterBuffer<I>
where
    I: Iterator<Item: Clone>,
{
    /// Combines [`Iterator::next`] and [`Self::take_stored`].
    ///
    /// Only returns a value if there is a next value from the iterator.
    pub fn take_stored_and_next(&mut self) -> Option<OneOrTwo<I::Item>> {
        let next_val = self.iter.next()?;

        if let Some(buf_val) = self.buffer.replace(next_val.clone()) {
            Some(OneOrTwo::Two([buf_val, next_val]))
        } else {
            Some(OneOrTwo::One([next_val]))
        }
    }
}
