/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{
    fmt::{self, Display, Formatter},
    str,
};

use crate::ErrTree;

pub(crate) struct ErrTreeFmtWrap<'a, const FRONT_MAX: usize> {
    pub tree: ErrTree<'a>,
}

impl<const FRONT_MAX: usize> Display for ErrTreeFmtWrap<'_, FRONT_MAX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(not(feature = "heap_buffer"))]
        let mut front_lines = [0; FRONT_MAX];

        #[cfg(feature = "heap_buffer")]
        let mut front_lines = alloc::string::String::new();

        ErrTreeFmt::<FRONT_MAX> {
            tree: self.tree.clone(),
            scratch_fill: 0,
            front_lines: &mut front_lines,
        }
        .fmt(f)
    }
}

pub(crate) struct ErrTreeFmt<'a, const FRONT_MAX: usize> {
    pub tree: ErrTree<'a>,
    pub scratch_fill: usize,
    /// Most be initialized large enough to fit 6 x (max depth) bytes
    #[cfg(not(feature = "heap_buffer"))]
    pub front_lines: &'a mut [u8],
    #[cfg(feature = "heap_buffer")]
    pub front_lines: &'a mut alloc::string::String,
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
        let front_lines = &self.front_lines;

        #[cfg(not(feature = "heap_buffer"))]
        let front_lines = str::from_utf8(&front_lines[..scratch_fill])
            .expect("All characters are static and guaranteed to be valid UTF-8");

        write!(f, "\n{0}", front_lines)
    }

    /// Push in the correct fill characters
    #[inline]
    fn add_front_line(
        &mut self,
        last: bool,
        #[cfg(not(feature = "heap_buffer"))] scratch_fill: usize,
    ) {
        let chars: &str = if last { DANGLING } else { CONTINUING };

        #[cfg(not(feature = "heap_buffer"))]
        self.front_lines[scratch_fill..scratch_fill + chars.len()]
            .copy_from_slice(chars.as_bytes());

        #[cfg(feature = "heap_buffer")]
        self.front_lines.push_str(chars);
    }
}

impl<const FRONT_MAX: usize> ErrTreeFmt<'_, FRONT_MAX> {
    fn fmt(mut self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.tree.inner, f)?;

        #[cfg(feature = "source_line")]
        if let Some(location) = self.tree.location {
            self.write_front_lines(
                f,
                #[cfg(not(feature = "heap_buffer"))]
                self.scratch_fill,
            )?;

            if self.tree.sources.is_empty() {
                write!(f, "╰─ ")?;
            } else {
                write!(f, "├─ ")?;
            }
            write!(f, "at \x1b[3m{}\x1b[0m", location)?;
        }

        let mut source_fmt = |this: &mut Self, source: ErrTree, last: bool| {
            this.write_front_lines(
                f,
                #[cfg(not(feature = "heap_buffer"))]
                this.scratch_fill,
            )?;
            write!(f, "│")?;
            this.write_front_lines(
                f,
                #[cfg(not(feature = "heap_buffer"))]
                this.scratch_fill,
            )?;

            if !last {
                write!(f, "├─▶ ")?;
            } else {
                write!(f, "╰─▶ ")?;
            }

            let additional_scratch = if last {
                DANGLING.len()
            } else {
                CONTINUING.len()
            };

            ErrTreeFmt::<FRONT_MAX> {
                tree: source,
                scratch_fill: this.scratch_fill + additional_scratch,
                front_lines: this.front_lines,
            }
            .fmt(f)
        };

        let sources_len: usize = self.tree.sources.iter().map(|x| x.len()).sum();
        let mut sources_iter = self.tree.sources.iter().flat_map(|x| x.iter());

        if self.scratch_fill + MAX_CELL_LEN >= FRONT_MAX {
            // Stop printing deeper in the stack past this point
            writeln!(f, "{:.<1$}", "", MAX_CELL_LEN)?;
        } else {
            // Normal operation

            for _ in 0..sources_len.saturating_sub(1) {
                self.add_front_line(
                    false,
                    #[cfg(not(feature = "heap_buffer"))]
                    self.scratch_fill,
                );

                let mut res = Ok(());
                sources_iter
                .next()
                .expect(
                    "This is guaranteed to be within the iterator length by previous calculation",
                )
                .as_err_tree(&mut |source| {
                    res = source_fmt(&mut self, source, false);
                });
                res?
            }

            if sources_len > 0 {
                self.add_front_line(
                    true,
                    #[cfg(not(feature = "heap_buffer"))]
                    self.scratch_fill,
                );

                let mut res = Ok(());
                sources_iter
                .next()
                .expect(
                    "This is guaranteed to be within the iterator length by previous calculation",
                )
                .as_err_tree(&mut |last_source| {
                    res = source_fmt(&mut self, last_source, true);
                });
                res?
            }
        };

        Ok(())
    }
}
