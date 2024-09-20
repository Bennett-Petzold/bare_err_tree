/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{
    cell::RefCell,
    fmt::{self, Debug, Display, Formatter},
    str,
};

use crate::ErrTree;

/// Simple single-linked list, tracking from children.
///
/// The node with no parent is treated as empty.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FmtDepthNode<'a> {
    pub val: bool,
    pub parent: Option<&'a FmtDepthNode<'a>>,
}

impl<'a> FmtDepthNode<'a> {
    pub fn new(val: bool, parent: Option<&'a FmtDepthNode<'a>>) -> Self {
        Self { val, parent }
    }
}

/// Iterates nodes from bottom to top (backwards).
impl Iterator for &FmtDepthNode<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(parent) = self.parent {
            let ret = self.val;
            *self = parent;
            Some(ret)
        } else {
            None
        }
    }
}

pub(crate) struct ErrTreeFmt<'a, const FRONT_MAX: usize> {
    pub tree: ErrTree<'a>,
    pub node: FmtDepthNode<'a>,
    /// Most be initialized large enough to fit 6 x (max depth) bytes
    #[cfg(not(feature = "heap_buffer"))]
    pub front_lines: &'a RefCell<[u8]>,
    #[cfg(feature = "heap_buffer")]
    pub front_lines: &'a RefCell<alloc::string::String>,
}

/// Workaround for lack of `const` in [`core::cmp::max`].
const fn max_const(lhs: usize, rhs: usize) -> usize {
    if lhs >= rhs {
        lhs
    } else {
        rhs
    }
}

const CONTINUING: &str = "│   ";
const DANGLING: &str = "    ";
const MAX_CELL_LEN: usize = max_const(CONTINUING.len(), DANGLING.len());

impl<const FRONT_MAX: usize> ErrTreeFmt<'_, FRONT_MAX> {
    /// Preamble arrow connections
    #[inline]
    fn write_front_lines(
        &self,
        f: &mut Formatter<'_>,
        #[cfg(not(feature = "heap_buffer"))] scratch_fill: usize,
    ) -> fmt::Result {
        let front_lines = self.front_lines.borrow();

        #[cfg(not(feature = "heap_buffer"))]
        let front_lines = str::from_utf8(&front_lines[..scratch_fill])
            .expect("All characters are static and guaranteed to be valid UTF-8");

        write!(f, "\n{0}", front_lines)
    }

    /// Push in the correct fill characters
    #[inline]
    fn add_front_line(&self, last: bool, #[cfg(not(feature = "heap_buffer"))] scratch_fill: usize) {
        let chars: &str = if last { DANGLING } else { CONTINUING };

        #[cfg(not(feature = "heap_buffer"))]
        self.front_lines.borrow_mut()[scratch_fill..scratch_fill + chars.len()]
            .copy_from_slice(chars.as_bytes());

        #[cfg(feature = "heap_buffer")]
        self.front_lines.borrow_mut().push_str(chars);
    }
}

impl<const FRONT_MAX: usize> Display for ErrTreeFmt<'_, FRONT_MAX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.tree.inner, f)?;

        // Any bytes after this are uninitialized
        // Handles the two sequences being different lengths
        let scratch_fill: usize = self
            .node
            .into_iter()
            .map(|x| if x { CONTINUING.len() } else { DANGLING.len() })
            .sum();

        #[cfg(feature = "source_line")]
        if let Some(location) = self.tree.location {
            self.write_front_lines(
                f,
                #[cfg(not(feature = "heap_buffer"))]
                scratch_fill,
            )?;

            if self.tree.sources.is_empty() {
                write!(f, "╰─ ")?;
            } else {
                write!(f, "├─ ")?;
            }
            write!(f, "at \x1b[3m{}\x1b[0m", location)?;
        }

        let mut source_fmt = |source: ErrTree, last: bool| {
            self.write_front_lines(
                f,
                #[cfg(not(feature = "heap_buffer"))]
                scratch_fill,
            )?;
            write!(f, "│")?;
            self.write_front_lines(
                f,
                #[cfg(not(feature = "heap_buffer"))]
                scratch_fill,
            )?;

            if !last {
                write!(f, "├─▶ ")?;
            } else {
                write!(f, "╰─▶ ")?;
            }

            let next_node = FmtDepthNode::new(!last, Some(&self.node));
            ErrTreeFmt::<FRONT_MAX> {
                tree: source,
                node: next_node,
                front_lines: self.front_lines,
            }
            .fmt(f)
        };

        let sources_len: usize = self.tree.sources.iter().map(|x| x.len()).sum();
        let mut sources_iter = self.tree.sources.iter().flat_map(|x| x.iter());

        if scratch_fill + MAX_CELL_LEN >= FRONT_MAX {
            // Stop printing deeper in the stack past this point
            writeln!(f, "{:.<1$}", "", MAX_CELL_LEN)?;
        } else {
            // Normal operation

            for _ in 0..sources_len.saturating_sub(1) {
                self.add_front_line(
                    false,
                    #[cfg(not(feature = "heap_buffer"))]
                    scratch_fill,
                );

                let mut res = Ok(());
                sources_iter
                .next()
                .expect(
                    "This is guaranteed to be within the iterator length by previous calculation",
                )
                .as_err_tree(&mut |source| {
                    res = source_fmt(source, false);
                });
                res?
            }

            if sources_len > 0 {
                self.add_front_line(
                    true,
                    #[cfg(not(feature = "heap_buffer"))]
                    scratch_fill,
                );

                let mut res = Ok(());
                sources_iter
                .next()
                .expect(
                    "This is guaranteed to be within the iterator length by previous calculation",
                )
                .as_err_tree(&mut |last_source| {
                    res = source_fmt(last_source, true);
                });
                res?
            }
        };

        Ok(())
    }
}
