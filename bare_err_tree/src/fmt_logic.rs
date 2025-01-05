/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{
    error::Error,
    fmt::{self, Display, Formatter},
    str,
};

use crate::{AsErrTree, ErrTree};

pub(crate) struct ErrTreeFmtWrap<const FRONT_MAX: usize, T> {
    pub tree: T,
}

impl<const FRONT_MAX: usize, T> Display for ErrTreeFmtWrap<FRONT_MAX, T>
where
    for<'a> &'a T: ErrTreeFormattable,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(not(feature = "heap_buffer"))]
        let mut front_lines = [0; FRONT_MAX];

        #[cfg(feature = "heap_buffer")]
        let mut front_lines = alloc::vec![0; FRONT_MAX].into_boxed_slice();

        #[cfg(all(not(feature = "heap_buffer"), feature = "tracing"))]
        let mut found_traces: [_; FRONT_MAX] = core::array::from_fn(|_| None);

        #[cfg(all(feature = "heap_buffer", feature = "tracing"))]
        let mut found_traces = core::iter::repeat_with(|| None)
            .collect::<alloc::vec::Vec<_>>()
            .into_boxed_slice();

        ErrTreeFmt::<FRONT_MAX, _> {
            tree: &self.tree,
            scratch_fill: 0,
            front_lines: &mut front_lines,

            #[cfg(feature = "tracing")]
            found_traces: &mut found_traces,
        }
        .fmt(f)
    }
}

#[cfg(feature = "tracing")]
pub(crate) struct TraceSpan<'a, T: Eq> {
    pub identifier: T,
    pub target: &'a str,
    pub name: &'a str,
    pub fields: &'a str,
    pub location: Option<(&'a str, u32)>,
}

pub(crate) trait ErrTreeFormattable {
    fn apply_msg(&self, f: &mut Formatter<'_>) -> fmt::Result;

    type Source<'a>: ErrTreeFormattable<TraceSpanType = Self::TraceSpanType>;
    fn sources_len(&self) -> usize;
    fn apply_to_leading_sources<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result;
    fn apply_to_last_source<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result;

    #[cfg(feature = "source_line")]
    fn has_source_line(&self) -> bool;
    #[cfg(feature = "source_line")]
    fn apply_source_line(&self, f: &mut Formatter<'_>) -> fmt::Result;

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool;

    type TraceSpanType: Eq;

    #[cfg(feature = "tracing")]
    fn trace_unique(&self, found_traces: &[Option<Self::TraceSpanType>]) -> bool;

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(TraceSpan<'_, Self::TraceSpanType>) -> fmt::Result;
}

impl<T> ErrTreeFormattable for &T
where
    T: ErrTreeFormattable,
{
    fn apply_msg(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::apply_msg(self, f)
    }

    type Source<'a> = T::Source<'a>;
    fn sources_len(&self) -> usize {
        T::sources_len(self)
    }
    fn apply_to_leading_sources<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        T::apply_to_leading_sources(self, func)
    }
    fn apply_to_last_source<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        T::apply_to_last_source(self, func)
    }

    #[cfg(feature = "source_line")]
    fn has_source_line(&self) -> bool {
        T::has_source_line(self)
    }
    #[cfg(feature = "source_line")]
    fn apply_source_line(&self, f: &mut Formatter<'_>) -> fmt::Result {
        T::apply_source_line(self, f)
    }

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool {
        T::trace_empty(self)
    }

    type TraceSpanType = T::TraceSpanType;

    #[cfg(feature = "tracing")]
    fn trace_unique(&self, found_traces: &[Option<Self::TraceSpanType>]) -> bool {
        T::trace_unique(self, found_traces)
    }

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(TraceSpan<'_, Self::TraceSpanType>) -> fmt::Result,
    {
        T::apply_trace(self, func)
    }
}

impl ErrTreeFormattable for ErrTree<'_> {
    fn apply_msg(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <dyn Error as Display>::fmt(self.inner, f)
    }

    type Source<'a> = ErrTree<'a>;
    fn sources_len(&self) -> usize {
        self.sources.iter().map(|line| line.len()).sum()
    }
    fn apply_to_leading_sources<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        for source in self
            .sources
            .iter()
            .flat_map(|line| line.iter())
            .take(self.sources_len().saturating_sub(1))
        {
            let mut res = Ok(());
            source.as_err_tree(&mut |tree| res = (func)(tree));
            res?;
        }
        Ok(())
    }
    fn apply_to_last_source<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        if let Some(source) = self.sources.last().and_then(|line| line.last()) {
            let mut res = Ok(());
            source.as_err_tree(&mut |tree| res = (func)(tree));
            res?;
        }
        Ok(())
    }

    #[cfg(feature = "source_line")]
    fn has_source_line(&self) -> bool {
        self.location.is_some()
    }

    #[cfg(feature = "source_line")]
    fn apply_source_line(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(loc) = self.location {
            write!(f, "{}", loc)?;
        }
        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool {
        let mut empty = true;
        if let Some(trace) = &self.trace {
            trace.with_spans(|_, _| {
                empty = false;
                true
            });
        }
        empty
    }

    #[cfg(not(feature = "tracing"))]
    type TraceSpanType = ();

    #[cfg(feature = "tracing")]
    type TraceSpanType = tracing_core::callsite::Identifier;

    #[cfg(feature = "tracing")]
    fn trace_unique(&self, found_traces: &[Option<Self::TraceSpanType>]) -> bool {
        let mut unique = false;
        if let Some(trace) = &self.trace {
            trace.with_spans(|metadata, _| {
                unique = found_traces
                    .iter()
                    .take_while(|x| x.is_some())
                    .flatten()
                    .any(|x| x == &metadata.callsite());
                !unique
            })
        }
        unique
    }

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(TraceSpan<'_, Self::TraceSpanType>) -> fmt::Result,
    {
        if let Some(trace) = &self.trace {
            let mut res = Ok(());
            trace.with_spans(|metadata, fields| {
                res = (func)(TraceSpan {
                    identifier: metadata.callsite(),
                    target: metadata.target(),
                    name: metadata.name(),
                    fields,
                    location: metadata
                        .file()
                        .and_then(|file| metadata.line().map(|line| (file, line))),
                });
                res.is_ok()
            });
            res
        } else {
            Ok(())
        }
    }
}

pub(crate) struct ErrTreeFmt<'a, const FRONT_MAX: usize, T: ErrTreeFormattable> {
    pub tree: T,
    pub scratch_fill: usize,
    /// Most be initialized large enough to fit 6 x (max depth) bytes
    pub front_lines: &'a mut [u8],

    #[cfg(feature = "tracing")]
    pub found_traces: &'a mut [Option<T::TraceSpanType>],
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

impl<const FRONT_MAX: usize, T: ErrTreeFormattable> ErrTreeFmt<'_, FRONT_MAX, T> {
    /// Preamble arrow connections
    #[inline]
    fn write_front_lines(
        front_lines: &[u8],
        f: &mut Formatter<'_>,
        scratch_fill: usize,
    ) -> fmt::Result {
        let front_lines = str::from_utf8(&front_lines[..scratch_fill])
            .expect("All characters are static and guaranteed to be valid UTF-8");

        write!(f, "\n{0}", front_lines)
    }

    /// Push in the correct fill characters
    #[inline]
    fn add_front_line(front_lines: &mut [u8], last: bool, scratch_fill: usize) {
        let chars: &str = if last { DANGLING } else { CONTINUING };

        front_lines[scratch_fill..scratch_fill + chars.len()].copy_from_slice(chars.as_bytes());
    }
    #[cfg(feature = "tracing")]
    /// Check for a unique trace by searching found traces
    fn tracing_after(&self) -> bool {
        self.tree.trace_unique(self.found_traces)
    }

    #[cfg(not(feature = "tracing"))]
    fn tracing_after(&self) -> bool {
        false
    }

    #[cfg(feature = "source_line")]
    fn source_line(&self, f: &mut Formatter<'_>, tracing_after: bool) -> fmt::Result {
        if self.tree.has_source_line() {
            Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;

            if !tracing_after && self.tree.sources_len() == 0 {
                write!(f, "╰─ ")?;
            } else {
                write!(f, "├─ ")?;
            }
            if cfg!(feature = "unix_color") {
                write!(f, "at \x1b[3m")?;
                self.tree.apply_source_line(f)?;
                write!(f, "\x1b[0m")?;
            } else {
                write!(f, "at ")?;
                self.tree.apply_source_line(f)?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "tracing")]
    /// Simple implementation of pretty formatting
    fn tracing_field_fmt(
        f: &mut Formatter<'_>,
        front_lines: &[u8],
        mut fields: &str,
        scratch_fill: usize,
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
            let _ = Self::write_front_lines(front_lines, f, scratch_fill);
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
        if !self.tree.trace_empty() {
            Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;
            write!(f, "│")?;

            let mut repeated = alloc::vec::Vec::<usize>::new();

            self.tree.apply_trace(|trace_span| {
                let pos_dup = self
                    .found_traces
                    .iter()
                    .take_while(|x| x.is_some())
                    .flatten()
                    .position(|c| *c == trace_span.identifier);

                if let Some(pos_dup) = pos_dup {
                    repeated.push(pos_dup);
                } else {
                    let depth = self.found_traces.partition_point(|x| x.is_some());
                    if depth < self.found_traces.len() {
                        self.found_traces[depth] = Some(trace_span.identifier);
                    }

                    let _ = Self::write_front_lines(self.front_lines, f, self.scratch_fill);
                    let _ = write!(
                        f,
                        "├─ tracing frame {} => {}::{}",
                        depth, trace_span.target, trace_span.name
                    );

                    if !trace_span.fields.is_empty() {
                        let _ = write!(f, " with");
                        Self::tracing_field_fmt(
                            f,
                            self.front_lines,
                            trace_span.fields,
                            self.scratch_fill,
                        );
                    }

                    if let Some((file, line)) = trace_span.location {
                        let _ = Self::write_front_lines(self.front_lines, f, self.scratch_fill);
                        let _ = write!(f, "│        at {}:{}", file, line);
                    };
                };

                Ok(())
            })?;

            if !repeated.is_empty() {
                Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;
                write!(
                    f,
                    "├─ {} duplicate tracing frame(s): {:?}",
                    repeated.len(),
                    repeated
                )?;
            }
        }
        Ok(())
    }

    #[allow(unused_mut)]
    fn fmt(mut self, f: &mut Formatter<'_>) -> fmt::Result {
        self.tree.apply_msg(f)?;

        #[cfg_attr(
            not(any(feature = "source_line", feature = "tracing")),
            expect(unused_variables, reason = "only used to track for a tracing line")
        )]
        let tracing_after = self.tracing_after();

        #[cfg(feature = "source_line")]
        self.source_line(f, tracing_after)?;

        #[cfg(feature = "tracing")]
        self.tracing(f)?;

        let mut source_fmt =
            |front_lines: &mut [u8],
             scratch_fill: usize,
             #[cfg(feature = "tracing")] found_traces: &mut [Option<T::TraceSpanType>],
             source: T::Source<'_>,
             last: bool| {
                Self::write_front_lines(front_lines, f, scratch_fill)?;
                write!(f, "│")?;
                Self::write_front_lines(front_lines, f, scratch_fill)?;

                if last {
                    write!(f, "╰─▶ ")?;
                } else {
                    write!(f, "├─▶ ")?;
                }

                let additional_scratch = if last {
                    DANGLING.len()
                } else {
                    CONTINUING.len()
                };

                ErrTreeFmt::<FRONT_MAX, _> {
                    tree: source,
                    scratch_fill: scratch_fill + additional_scratch,
                    front_lines,

                    #[cfg(feature = "tracing")]
                    found_traces,
                }
                .fmt(f)
            };

        if self.scratch_fill + MAX_CELL_LEN >= FRONT_MAX {
            // Stop printing deeper in the stack past this point
            writeln!(f, "{:.<1$}", "", MAX_CELL_LEN)?;
        } else {
            // Normal operation

            Self::add_front_line(self.front_lines, false, self.scratch_fill);
            self.tree.apply_to_leading_sources(|source| {
                source_fmt(
                    self.front_lines,
                    self.scratch_fill,
                    #[cfg(feature = "tracing")]
                    self.found_traces,
                    source,
                    false,
                )
            })?;

            self.tree.apply_to_last_source(|source| {
                Self::add_front_line(self.front_lines, true, self.scratch_fill);
                source_fmt(
                    self.front_lines,
                    self.scratch_fill,
                    #[cfg(feature = "tracing")]
                    self.found_traces,
                    source,
                    true,
                )
            })?;
        };

        Ok(())
    }
}
