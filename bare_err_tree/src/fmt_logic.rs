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

        #[cfg(feature = "tracing")]
        let mut found_traces = alloc::vec::Vec::new();

        ErrTreeFmt::<FRONT_MAX> {
            tree: self.tree.clone(),
            scratch_fill: 0,
            front_lines: &mut front_lines,

            #[cfg(feature = "tracing")]
            found_traces: &mut found_traces,
        }
        .fmt(f)
    }
}

pub(crate) struct ErrTreeFmt<'a, const FRONT_MAX: usize> {
    tree: ErrTree<'a>,
    scratch_fill: usize,
    /// Most be initialized large enough to fit 6 x (max depth) bytes
    #[cfg(not(feature = "heap_buffer"))]
    front_lines: &'a mut [u8],
    #[cfg(feature = "heap_buffer")]
    front_lines: &'a mut alloc::string::String,

    #[cfg(feature = "tracing")]
    found_traces: &'a mut alloc::vec::Vec<tracing_core::callsite::Identifier>,
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
        #[cfg(not(feature = "heap_buffer"))] front_lines: &[u8],
        #[cfg(feature = "heap_buffer")] front_lines: &alloc::string::String,
        f: &mut Formatter<'_>,
        #[cfg(not(feature = "heap_buffer"))] scratch_fill: usize,
    ) -> fmt::Result {
        #[cfg(not(feature = "heap_buffer"))]
        let front_lines = str::from_utf8(&front_lines[..scratch_fill])
            .expect("All characters are static and guaranteed to be valid UTF-8");

        write!(f, "\n{0}", front_lines)
    }

    /// Push in the correct fill characters
    #[inline]
    fn add_front_line(
        #[cfg(not(feature = "heap_buffer"))] front_lines: &mut [u8],
        #[cfg(feature = "heap_buffer")] front_lines: &mut alloc::string::String,
        last: bool,
        #[cfg(not(feature = "heap_buffer"))] scratch_fill: usize,
    ) {
        let chars: &str = if last { DANGLING } else { CONTINUING };

        #[cfg(not(feature = "heap_buffer"))]
        front_lines[scratch_fill..scratch_fill + chars.len()].copy_from_slice(chars.as_bytes());

        #[cfg(feature = "heap_buffer")]
        front_lines.push_str(chars);
    }
}

impl<const FRONT_MAX: usize> ErrTreeFmt<'_, FRONT_MAX> {
    #[cfg(feature = "tracing")]
    /// Check for a unique trace by searching found traces
    fn tracing_after(&self) -> bool {
        if let Some(trace) = &self.tree.trace {
            let mut unique = false;

            trace.with_spans(|metadata, _| {
                unique = self.found_traces.contains(&metadata.callsite());
                false
            });

            unique
        } else {
            false
        }
    }

    #[cfg(not(feature = "tracing"))]
    fn tracing_after(&self) -> bool {
        false
    }

    #[cfg(feature = "source_line")]
    fn source_line(&self, f: &mut Formatter<'_>, tracing_after: bool) -> fmt::Result {
        if let Some(location) = self.tree.location {
            Self::write_front_lines(
                self.front_lines,
                f,
                #[cfg(not(feature = "heap_buffer"))]
                self.scratch_fill,
            )?;

            if !tracing_after && self.tree.sources.is_empty() {
                write!(f, "╰─ ")?;
            } else {
                write!(f, "├─ ")?;
            }
            write!(f, "at \x1b[3m{}\x1b[0m", location)?;
        }

        Ok(())
    }

    #[cfg(feature = "tracing")]
    /// Simple implementation of pretty formatting
    fn tracing_field_fmt(
        f: &mut Formatter<'_>,
        #[cfg(not(feature = "heap_buffer"))] front_lines: &[u8],
        #[cfg(feature = "heap_buffer")] front_lines: &alloc::string::String,
        mut fields: &str,
        #[cfg(not(feature = "heap_buffer"))] scratch_fill: usize,
    ) {
        let mut next_slice = |search_chars: &mut alloc::vec::Vec<_>, depth: &mut usize| {
            fields = fields.trim();

            let next_idx = {
                let last_search_char = search_chars.last().and_then(|c| fields.find(*c));
                let brace = fields.find('{');
                let bracket = fields.find('[');
                let paren = fields.find('(');
                let comma = fields.find(',');
                let quote = fields.find('"');

                let before = |lhs: usize, rhs: Option<usize>| {
                    if let Some(rhs) = rhs {
                        lhs < rhs
                    } else {
                        true
                    }
                };

                // Default, no special chars left
                let mut idx = None;

                if let Some(last_search_char) = last_search_char {
                    if before(last_search_char, brace)
                        && before(last_search_char, paren)
                        && before(last_search_char, bracket)
                        && before(last_search_char, comma)
                        && before(last_search_char, quote)
                    {
                        if last_search_char == 0 {
                            let _ = search_chars.pop();
                            *depth = depth.saturating_sub(1);

                            if fields.chars().nth(1) == Some(',') {
                                idx = Some(2);
                            } else {
                                idx = Some(1);
                            }
                        } else {
                            idx = Some(last_search_char);
                        }
                    }
                }
                if let Some(brace) = brace {
                    if idx.is_none()
                        && before(brace, bracket)
                        && before(brace, paren)
                        && before(brace, comma)
                        && before(brace, quote)
                    {
                        search_chars.push('}');
                        *depth = depth.saturating_add(1);
                        idx = Some(brace + 1);
                    }
                }
                if let Some(bracket) = bracket {
                    if idx.is_none()
                        && before(bracket, paren)
                        && before(bracket, comma)
                        && before(bracket, quote)
                    {
                        search_chars.push(']');
                        *depth = depth.saturating_add(1);
                        idx = Some(bracket + 1);
                    }
                }
                if let Some(paren) = paren {
                    if idx.is_none() && before(paren, comma) && before(paren, quote) {
                        search_chars.push(')');
                        *depth = depth.saturating_add(1);
                        idx = Some(paren + 1);
                    }
                }
                if let Some(comma) = comma {
                    if idx.is_none() && before(comma, quote) {
                        idx = Some(comma + 1);
                    }
                }
                if let Some(quote) = quote {
                    if idx.is_none() {
                        idx = Some(
                            fields[quote + 1..]
                                .find('"')
                                .expect("If the quote opens, it must close")
                                + quote
                                + 2,
                        );
                    }
                }

                idx.unwrap_or(fields.len())
            };

            let cur = &fields[..next_idx];
            fields = &fields[next_idx..];
            cur
        };

        let mut search_chars = alloc::vec::Vec::new();
        let mut prev_depth = 0;
        let mut depth = 0;

        let mut next = next_slice(&mut search_chars, &mut depth);
        while !next.is_empty() {
            let _ = Self::write_front_lines(
                front_lines,
                f,
                #[cfg(not(feature = "heap_buffer"))]
                scratch_fill,
            );
            let _ = write!(f, "│    ");
            for _ in 0..core::cmp::min(prev_depth, depth) {
                let _ = write!(f, "  ");
            }
            let _ = write!(f, "{}", next);

            prev_depth = depth;
            next = next_slice(&mut search_chars, &mut depth);
        }
    }

    #[cfg(feature = "tracing")]
    fn tracing(&mut self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(trace) = &self.tree.trace {
            Self::write_front_lines(
                self.front_lines,
                f,
                #[cfg(not(feature = "heap_buffer"))]
                self.scratch_fill,
            )?;
            write!(f, "│")?;

            let mut repeated = alloc::vec::Vec::<usize>::new();

            trace.with_spans(|metadata, fields| {
                let pos_dup = self
                    .found_traces
                    .iter()
                    .position(|c| *c == metadata.callsite());

                if let Some(pos_dup) = pos_dup {
                    repeated.push(pos_dup);
                } else {
                    let depth = self.found_traces.len();
                    self.found_traces.push(metadata.callsite());

                    let _ = Self::write_front_lines(
                        self.front_lines,
                        f,
                        #[cfg(not(feature = "heap_buffer"))]
                        self.scratch_fill,
                    );
                    let _ = write!(
                        f,
                        "├─ tracing frame {} => {}::{}",
                        depth,
                        metadata.target(),
                        metadata.name()
                    );

                    if !fields.is_empty() {
                        let _ = write!(f, " with");
                        Self::tracing_field_fmt(
                            f,
                            self.front_lines,
                            fields,
                            #[cfg(not(feature = "heap_buffer"))]
                            self.scratch_fill,
                        );
                    }

                    if let Some((file, line)) = metadata
                        .file()
                        .and_then(|file| metadata.line().map(|line| (file, line)))
                    {
                        let _ = Self::write_front_lines(
                            self.front_lines,
                            f,
                            #[cfg(not(feature = "heap_buffer"))]
                            self.scratch_fill,
                        );
                        let _ = write!(f, "│        at {}:{}", file, line);
                    };
                };

                true
            });

            if !repeated.is_empty() {
                let _ = Self::write_front_lines(
                    self.front_lines,
                    f,
                    #[cfg(not(feature = "heap_buffer"))]
                    self.scratch_fill,
                );
                let _ = write!(
                    f,
                    "├─ {} duplicate tracing frame(s): {:?}",
                    repeated.len(),
                    repeated
                );
            }
        };

        Ok(())
    }

    fn fmt(mut self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.tree.inner, f)?;

        #[cfg_attr(
            not(any(feature = "source_line", feature = "tracing")),
            expect(unused_variables, reason = "only used to track for a tracing line")
        )]
        let tracing_after = self.tracing_after();

        #[cfg(feature = "source_line")]
        self.source_line(f, tracing_after)?;

        #[cfg(feature = "tracing")]
        self.tracing(f)?;

        let mut source_fmt = |this: &mut Self, source: ErrTree, last: bool| {
            Self::write_front_lines(
                this.front_lines,
                f,
                #[cfg(not(feature = "heap_buffer"))]
                this.scratch_fill,
            )?;
            write!(f, "│")?;
            Self::write_front_lines(
                this.front_lines,
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

                #[cfg(feature = "tracing")]
                found_traces: this.found_traces,
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

            Self::add_front_line(
                self.front_lines,
                false,
                #[cfg(not(feature = "heap_buffer"))]
                self.scratch_fill,
            );
            for _ in 0..sources_len.saturating_sub(1) {
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

            // Clean up previous work when pushing to a String
            #[cfg(feature = "heap_buffer")]
            self.front_lines.truncate(self.scratch_fill);

            if sources_len > 0 {
                Self::add_front_line(
                    self.front_lines,
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
