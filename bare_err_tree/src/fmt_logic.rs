/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::{
    fmt::{self, Display, Formatter, Write},
    str::{self, Chars},
};

use crate::{AsErrTree, ErrTree, ErrTreeDisplay};

impl<E: AsErrTree, const FRONT_MAX: usize> ErrTreeDisplay<E, FRONT_MAX> {
    pub fn new(tree: E) -> Self {
        Self(tree)
    }
}

impl<E: AsErrTree, const FRONT_MAX: usize> Display for ErrTreeDisplay<E, FRONT_MAX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut res = Ok(());
        self.0.as_err_tree(&mut |tree| {
            res = fmt_tree::<FRONT_MAX, _, _>(tree, f);
        });
        res
    }
}

pub(crate) fn fmt_tree<const FRONT_MAX: usize, T, W>(tree: T, f: &mut W) -> fmt::Result
where
    T: ErrTreeFormattable,
    W: fmt::Write + ?Sized,
{
    #[cfg(not(feature = "heap_buffer"))]
    let mut front_lines = [0; FRONT_MAX];

    #[cfg(feature = "heap_buffer")]
    let mut front_lines = alloc::vec![0; FRONT_MAX].into_boxed_slice();

    #[cfg(all(not(feature = "heap_buffer"), feature = "tracing"))]
    let mut found_traces: [_; FRONT_MAX] = core::array::from_fn(|_| None);

    #[cfg(all(feature = "heap_buffer", feature = "tracing"))]
    let mut found_traces = core::iter::repeat_with(|| None)
        .take(FRONT_MAX)
        .collect::<alloc::vec::Vec<_>>()
        .into_boxed_slice();

    ErrTreeFmt::<FRONT_MAX, _> {
        tree,
        scratch_fill: 0,
        front_lines: &mut front_lines,

        #[cfg(feature = "tracing")]
        found_traces: &mut found_traces,
    }
    .fmt(f)
}

#[cfg(feature = "tracing")]
pub(crate) struct TraceSpan<T: Eq, CharIter> {
    pub identifier: T,
    pub target: CharIter,
    pub name: CharIter,
    pub fields: CharIter,
    pub location: Option<(CharIter, u32)>,
}

pub(crate) trait ErrTreeFormattable {
    fn apply_msg<W: fmt::Write>(&self, f: W) -> fmt::Result;

    type Source<'a>: ErrTreeFormattable<TraceSpanId = Self::TraceSpanId>;

    #[allow(unused)]
    fn sources_empty(&mut self) -> bool;

    fn apply_to_leading_sources<F>(&mut self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result;
    fn apply_to_last_source<F>(&mut self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result;
    #[cfg(feature = "source_line")]
    fn has_source_line(&self) -> bool;
    #[cfg(feature = "source_line")]
    fn apply_source_line<W: fmt::Write>(&self, f: W) -> fmt::Result;

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool;

    type TraceSpanId: Eq;
    type TraceSpanIter<'a>: IntoIterator<Item = char>;

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(TraceSpan<Self::TraceSpanId, Self::TraceSpanIter<'_>>) -> fmt::Result;
}

impl<T> ErrTreeFormattable for &mut T
where
    T: ErrTreeFormattable,
{
    fn apply_msg<W: fmt::Write>(&self, f: W) -> fmt::Result {
        T::apply_msg(self, f)
    }

    type Source<'a> = T::Source<'a>;
    fn sources_empty(&mut self) -> bool {
        T::sources_empty(self)
    }
    fn apply_to_leading_sources<F>(&mut self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        T::apply_to_leading_sources(self, func)
    }
    fn apply_to_last_source<F>(&mut self, func: F) -> fmt::Result
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
    fn apply_source_line<W: fmt::Write>(&self, f: W) -> fmt::Result {
        T::apply_source_line(self, f)
    }

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool {
        T::trace_empty(self)
    }

    type TraceSpanId = T::TraceSpanId;
    type TraceSpanIter<'a> = T::TraceSpanIter<'a>;

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(TraceSpan<Self::TraceSpanId, Self::TraceSpanIter<'_>>) -> fmt::Result,
    {
        T::apply_trace(self, func)
    }
}

impl ErrTreeFormattable for ErrTree<'_> {
    fn apply_msg<W: fmt::Write>(&self, mut f: W) -> fmt::Result {
        write!(f, "{}", self.inner)
    }

    type Source<'a> = ErrTree<'a>;
    fn sources_empty(&mut self) -> bool {
        self.sources.is_empty()
    }
    fn apply_to_leading_sources<F>(&mut self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        if let Some(initial_slice) = self.sources.take_stored_and_next() {
            let mut initial_iter = initial_slice.as_slice().iter().cloned();
            if let Some(mut source) = initial_iter.next() {
                for next_source in initial_iter.chain(self.sources.by_ref()) {
                    let mut res = Ok(());
                    source.as_err_tree(&mut |tree| res = (func)(tree));
                    res?;
                    source = next_source
                }
            }
        }
        Ok(())
    }
    fn apply_to_last_source<F>(&mut self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        let _ = self.sources.by_ref().last();
        if let Some(source) = self.sources.take_stored() {
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
    fn apply_source_line<W: fmt::Write>(&self, mut f: W) -> fmt::Result {
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
    type TraceSpanId = ();

    #[cfg(feature = "tracing")]
    type TraceSpanId = tracing_core::callsite::Identifier;

    type TraceSpanIter<'a> = Chars<'a>;

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(TraceSpan<Self::TraceSpanId, Self::TraceSpanIter<'_>>) -> fmt::Result,
    {
        if let Some(trace) = &self.trace {
            let mut res = Ok(());
            trace.with_spans(|metadata, fields| {
                res = (func)(TraceSpan {
                    identifier: metadata.callsite(),
                    target: metadata.target().chars(),
                    name: metadata.name().chars(),
                    fields: fields.chars(),
                    location: metadata
                        .file()
                        .and_then(|file| metadata.line().map(|line| (file.chars(), line))),
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
    pub found_traces: &'a mut [Option<T::TraceSpanId>],
}

/// Workaround for lack of `const` in [`core::cmp::max`].
#[cfg_attr(coverage, coverage(off))]
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
    /// The front lines
    #[inline]
    fn front_lines_str(front_lines: &[u8], scratch_fill: usize) -> &str {
        str::from_utf8(&front_lines[..scratch_fill])
            .expect("All characters are static and guaranteed to be valid UTF-8")
    }

    /// Preamble arrow connections
    #[inline]
    fn write_front_lines<W>(front_lines: &[u8], f: &mut W, scratch_fill: usize) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
    {
        f.write_char('\n')?;
        f.write_str(Self::front_lines_str(front_lines, scratch_fill))
    }

    /// Push in the correct fill characters
    #[inline]
    fn add_front_line(front_lines: &mut [u8], last: bool, scratch_fill: usize) {
        let chars: &str = if last { DANGLING } else { CONTINUING };

        front_lines[scratch_fill..scratch_fill + chars.len()].copy_from_slice(chars.as_bytes());
    }
    #[cfg(feature = "tracing")]
    /// There is tracing after if the trace is nonempty
    fn tracing_after(&self) -> bool {
        !self.tree.trace_empty()
    }

    #[cfg(not(feature = "tracing"))]
    fn tracing_after(&self) -> bool {
        false
    }

    #[cfg(feature = "source_line")]
    fn source_line<W>(&mut self, f: &mut W, tracing_after: bool) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
    {
        if self.tree.has_source_line() {
            Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;

            if !tracing_after && self.tree.sources_empty() {
                f.write_str("╰─ ")?;
            } else {
                f.write_str("├─ ")?;
            }
            if cfg!(feature = "unix_color") {
                f.write_str("at \x1b[3m")?;
                self.tree.apply_source_line(&mut *f)?;
                f.write_str("\x1b[0m")?;
            } else {
                f.write_str("at ")?;
                self.tree.apply_source_line(f)?;
            }
        }

        Ok(())
    }

    /// Simple implementation of pretty formatting
    #[cfg(feature = "tracing")]
    fn tracing_field_fmt<I, W>(
        f: &mut W,
        front_lines: &[u8],
        fields: I,
        scratch_fill: usize,
    ) -> fmt::Result
    where
        I: IntoIterator<Item = char>,
        W: fmt::Write + ?Sized,
    {
        let mut depth = 0;
        let mut in_quote = false;

        const START_CHARS: [char; 3] = ['{', '[', '('];
        const END_CHARS: [char; 3] = ['}', ']', ')'];
        const ESC: char = '\\';

        let push_front = |f: &mut W, depth| {
            Self::write_front_lines(front_lines, f, scratch_fill)?;
            f.write_str("│    ")?;
            for _ in 0..depth {
                f.write_str("  ")?;
            }
            Ok(())
        };

        push_front(f, depth)?;
        let mut prev = '\0';
        for c in fields {
            let mut space_except = false;

            if in_quote {
                if prev == '"' {
                    in_quote = false;
                    if c == ' ' {
                        space_except = true;
                    }
                }
            } else {
                match prev {
                    x if START_CHARS.contains(&x) => {
                        depth += 1;
                        push_front(f, depth)?;
                        if c == ' ' {
                            space_except = true;
                        }
                    }
                    ',' => {
                        push_front(f, depth)?;
                        if c == ' ' {
                            space_except = true;
                        }
                    }
                    '"' => in_quote = true,
                    x => {
                        if END_CHARS.contains(&c) {
                            depth -= 1;
                            push_front(f, depth)?;
                        } else if c == ' ' && END_CHARS.contains(&x) {
                            space_except = true;
                            if depth == 0 {
                                push_front(f, depth)?;
                            }
                        }
                    }
                }
            }

            // Special case for escaping
            prev = if prev == ESC { '\0' } else { c };

            if !space_except {
                f.write_char(c)?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "tracing")]
    fn tracing<W>(&mut self, f: &mut W) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
    {
        if !self.tree.trace_empty() {
            Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;
            write!(f, "│")?;

            #[cfg(all(not(feature = "heap_buffer"), feature = "tracing"))]
            let mut repeated: [_; FRONT_MAX] = core::array::from_fn(|_| None);

            #[cfg(all(feature = "heap_buffer", feature = "tracing"))]
            let mut repeated = core::iter::repeat_with(|| None)
                .take(FRONT_MAX)
                .collect::<alloc::vec::Vec<_>>()
                .into_boxed_slice();

            let mut repeated_idx = 0;

            self.tree.apply_trace(|trace_span| {
                let pos_dup = self
                    .found_traces
                    .iter()
                    .take_while(|x| x.is_some())
                    .flatten()
                    .position(|c| *c == trace_span.identifier);

                if let Some(pos_dup) = pos_dup {
                    repeated[repeated_idx] = Some(pos_dup);
                    repeated_idx += 1;
                } else {
                    let depth = self.found_traces.partition_point(|x| x.is_some());
                    if depth < self.found_traces.len() {
                        self.found_traces[depth] = Some(trace_span.identifier);
                    }

                    Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;
                    write!(f, "├─ tracing frame {} => ", depth)?;
                    //depth, trace_span.target, trace_span.name
                    for c in trace_span.target {
                        f.write_char(c)?
                    }
                    f.write_str("::")?;
                    for c in trace_span.name {
                        f.write_char(c)?
                    }

                    let mut fields = trace_span.fields.into_iter().peekable();
                    if fields.peek().is_some() {
                        write!(f, " with")?;
                        Self::tracing_field_fmt(f, self.front_lines, fields, self.scratch_fill)?;
                    }

                    if let Some((file, line)) = trace_span.location {
                        Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;
                        f.write_str("│        at ")?;
                        for c in file {
                            f.write_char(c)?
                        }
                        f.write_char(':')?;
                        write!(f, "{line}")?;
                    };
                };

                Ok(())
            })?;

            if repeated_idx > 0 {
                Self::write_front_lines(self.front_lines, f, self.scratch_fill)?;
                if self.tree.sources_empty() {
                    f.write_str("╰─ ")?;
                } else {
                    f.write_str("├─ ")?;
                }

                write!(f, "{} duplicate tracing frame(s): [", repeated_idx)?;

                for idx in 0..repeated_idx - 1 {
                    write!(f, "{}, ", repeated[idx].expect("Previously set as Some"))?;
                }

                write!(
                    f,
                    "{}]",
                    repeated[repeated_idx - 1].expect("Previously set as Some")
                )?;
            }
        }
        Ok(())
    }

    #[allow(unused_mut)]
    fn fmt<W>(mut self, f: &mut W) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
    {
        self.tree.apply_msg(LeadingLineFormatter::new(
            &mut *f,
            Self::front_lines_str(self.front_lines, self.scratch_fill),
        ))?;

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
             #[cfg(feature = "tracing")] found_traces: &mut [Option<T::TraceSpanId>],
             source: T::Source<'_>,
             last: bool| {
                Self::write_front_lines(front_lines, f, scratch_fill)?;
                f.write_char('│')?;
                Self::write_front_lines(front_lines, f, scratch_fill)?;

                if last {
                    f.write_str("╰─▶ ")?;
                } else {
                    f.write_str("├─▶ ")?;
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

/// Injects the newline leader
struct LeadingLineFormatter<'a, F> {
    formatter: F,
    leading: &'a str,
}

impl<'a, F> LeadingLineFormatter<'a, F> {
    pub fn new(formatter: F, leading: &'a str) -> Self {
        Self { formatter, leading }
    }
}

impl<F: Write> Write for LeadingLineFormatter<'_, F> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.chars().all(|c| c != '\n') {
            self.formatter.write_str(s)?
        } else {
            for c in s.chars() {
                self.write_char(c)?;
            }
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.formatter.write_char(c)?;

        if c == '\n' {
            self.formatter.write_str(self.leading)?;
            self.formatter.write_str("│ ")?;
        }

        Ok(())
    }
}
