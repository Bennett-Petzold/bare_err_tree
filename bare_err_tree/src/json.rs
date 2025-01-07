//! Error tree output to/from JSON.

extern crate alloc;

use core::{
    borrow::Borrow,
    error::Error,
    fmt::{self, Display, Formatter, Write},
};

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use crate::{AsErrTree, ErrTree, ErrTreeFmtWrap, ErrTreeFormattable};

/// Error during JSON (de)serialization.
#[derive(Debug)]
pub enum JsonErr {
    Serde(serde_json::Error),
    Formatting(fmt::Error),
}

impl Display for JsonErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serde(x) => x.fmt(f),
            Self::Formatting(x) => x.fmt(f),
        }
    }
}

impl Error for JsonErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serde(x) => Some(x),
            Self::Formatting(x) => Some(x),
        }
    }
}

impl From<serde_json::Error> for JsonErr {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<fmt::Error> for JsonErr {
    fn from(value: fmt::Error) -> Self {
        Self::Formatting(value)
    }
}

/// Produces JSON to store [`ErrTree`] formatted output.
///
/// JSON output can be used to display with [`ErrTree`] format with
/// [`reconstruct_output`], but the [`ErrTree`] itself cannot be reconstructed.
#[track_caller]
pub fn tree_to_json<E, S, F>(tree: S, formatter: &mut F) -> Result<(), JsonErr>
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
    F: fmt::Write,
{
    let mut res = Ok(());
    tree.borrow().as_err_tree(&mut |tree| {
        res = json_fmt(tree, formatter);
    });
    res?;
    Ok(())
}

/// Custom JSON format outputter
fn json_fmt<F: fmt::Write>(tree: ErrTree<'_>, formatter: &mut F) -> fmt::Result {
    formatter.write_str("{\"msg\":\"")?;
    write!(JsonEscapeFormatter { formatter }, "{}", tree.inner)?;
    formatter.write_char('"')?;

    #[cfg(feature = "source_line")]
    if let Some(loc) = tree.location {
        formatter.write_str(",\"location\":\"")?;
        write!(JsonEscapeFormatter { formatter }, "{}", loc)?;
        formatter.write_char('"')?;
    }

    #[cfg(feature = "tracing")]
    if let Some(trace) = tree.trace {
        formatter.write_str(",\"trace\":[")?;
        let mut res = Ok(());
        let mut first_trace = true;
        trace.with_spans(|metadata, fields| {
            res = json_trace_fmt(metadata, fields, first_trace, formatter);
            first_trace = false;
            res.is_ok()
        });
        res?;
        formatter.write_char(']')?;
    }

    if tree.sources_len() > 0 {
        formatter.write_str(",\"sources\":[")?;
        let mut sources = tree
            .sources()
            .iter()
            .flat_map(|source_line| source_line.iter());
        if let Some(first_source) = sources.next() {
            let mut res = Ok(());
            first_source.as_err_tree(&mut |subtree| {
                res = json_fmt(subtree, formatter);
            });
            res?
        }
        for source in sources {
            formatter.write_char(',')?;
            let mut res = Ok(());
            source.as_err_tree(&mut |subtree| {
                res = json_fmt(subtree, formatter);
            });
            res?
        }
        formatter.write_char(']')?;
    }

    formatter.write_char('}')
}

/// Escapes strings according to JSON
struct JsonEscapeFormatter<'a, F> {
    formatter: &'a mut F,
}

impl<F: Write> Write for JsonEscapeFormatter<'_, F> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        const BACKSPACE: char = 8 as char;
        const FORM_FEED: char = 12 as char;
        const JSON_ESCAPE: [char; 7] = ['"', '\\', BACKSPACE, FORM_FEED, '\n', '\r', '\t'];

        if JSON_ESCAPE.contains(&c) {
            self.formatter.write_char('\\')?;
        }

        match c {
            BACKSPACE => self.formatter.write_char('b'),
            FORM_FEED => self.formatter.write_char('f'),
            '\n' => self.formatter.write_char('n'),
            '\r' => self.formatter.write_char('r'),
            '\t' => self.formatter.write_char('t'),
            x => self.formatter.write_char(x),
        }
    }
}

#[cfg(feature = "tracing")]
fn json_trace_fmt<F: fmt::Write>(
    metadata: &tracing_core::Metadata<'static>,
    fields: &str,
    first_trace: bool,
    formatter: &mut F,
) -> fmt::Result {
    if !first_trace {
        formatter.write_char(',')?;
    }
    formatter.write_str("{\"target\":\"")?;
    write!(JsonEscapeFormatter { formatter }, "{}", metadata.target())?;
    formatter.write_str("\",\"name\":\"")?;
    write!(JsonEscapeFormatter { formatter }, "{}", metadata.name())?;
    formatter.write_str("\",\"fields\":\"")?;
    write!(JsonEscapeFormatter { formatter }, "{}", fields)?;
    formatter.write_char('"')?;

    if let Some((file, line)) = metadata
        .file()
        .and_then(|file| metadata.line().map(|line| (file, line)))
    {
        formatter.write_str(",\"location\":[\"")?;
        write!(JsonEscapeFormatter { formatter }, "{}", file)?;
        write!(formatter, "\",{line}]")?;
    }
    formatter.write_char('}')?;
    Ok(())
}

/// Reconstructs [`ErrTree`] formatted output from JSON.
///
/// Only the output produced by [`tree_to_json`] is valid for this function.
///
/// `FRONT_MAX` limits the number of leading bytes. Each deeper error requires 6
/// bytes to fit "│   ". So for a max depth of 3 errors, `FRONT_MAX` == 18.
/// By default, `FRONT_MAX` bytes are allocated on stack. When `heap_buffer` is
/// enabled, the bytes are allocated on stack and `FRONT_MAX` only acts as a
/// depth limit. When `tracing` is enabled, at most `FRONT_MAX` stack traces
/// will be tracked for duplicates.
pub fn reconstruct_output<const FRONT_MAX: usize, S, F>(
    json: S,
    formatter: &mut F,
) -> Result<(), JsonErr>
where
    S: AsRef<str>,
    F: fmt::Write,
{
    let tree = from_str::<ErrTreeReconstruct>(json.as_ref())?;
    write!(
        formatter,
        "{}",
        ErrTreeFmtWrap::<FRONT_MAX, _> { tree: &tree }
    )?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct JsonSpan<'a> {
    target: &'a str,
    name: &'a str,
    fields: &'a str,
    location: Option<(&'a str, u32)>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct JsonSpanOwned {
    target: String,
    name: String,
    fields: String,
    location: Option<(String, u32)>,
}

#[derive(Deserialize)]
struct ErrTreeReconstruct {
    msg: String,
    #[cfg(feature = "source_line")]
    location: Option<String>,
    #[cfg(feature = "tracing")]
    #[serde(default)]
    trace: Vec<JsonSpanOwned>,
    #[serde(default)]
    sources: Vec<Self>,
}

impl<'de> ErrTreeFormattable for &'de ErrTreeReconstruct {
    fn apply_msg(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }

    type Source<'a> = Self;
    fn sources_len(&self) -> usize {
        self.sources.len()
    }
    fn apply_to_leading_sources<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        for source in &self.sources[0..self.sources.len().saturating_sub(1)] {
            (func)(source)?
        }
        Ok(())
    }
    fn apply_to_last_source<F>(&self, func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        self.sources.last().map(func).unwrap_or(Ok(()))
    }

    #[cfg(feature = "source_line")]
    fn has_source_line(&self) -> bool {
        self.location.is_some()
    }

    #[cfg(feature = "source_line")]
    fn apply_source_line(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            f.write_str(location)
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool {
        self.trace.is_empty()
    }

    #[cfg(not(feature = "tracing"))]
    type TraceSpanType = ();

    #[cfg(feature = "tracing")]
    type TraceSpanType = &'de JsonSpanOwned;

    #[cfg(feature = "tracing")]
    fn trace_unique(&self, found_traces: &[Option<Self::TraceSpanType>]) -> bool {
        let search_trace_end = found_traces.partition_point(|x| x.is_some());
        let search_traces = &found_traces[0..search_trace_end];
        !self.trace.iter().any(|trace_line| {
            search_traces
                .iter()
                .flatten()
                .any(|found| *found == trace_line)
        })
    }

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(crate::TraceSpan<'_, Self::TraceSpanType>) -> fmt::Result,
    {
        use crate::TraceSpan;

        for trace_line in &self.trace {
            (func)(TraceSpan {
                identifier: trace_line,
                target: &trace_line.target,
                name: &trace_line.name,
                fields: &trace_line.fields,
                location: trace_line
                    .location
                    .as_ref()
                    .map(|(file, line)| (file.as_str(), *line)),
            })?;
        }

        Ok(())
    }
}
