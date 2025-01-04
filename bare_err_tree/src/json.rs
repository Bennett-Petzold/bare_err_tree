//! Error tree output to/from JSON

extern crate alloc;

use core::{
    borrow::Borrow,
    cell::RefCell,
    error::Error,
    fmt::{self, Display, Formatter, Write},
};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Serialize,
};
use serde_json::from_str;

use crate::{AsErrTree, ErrTree, ErrTreeFmtWrap, ErrTreeFormattable};

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

#[track_caller]
pub fn tree_to_json<E, S, F>(tree: S, formatter: &mut F) -> Result<(), JsonErr>
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
    F: fmt::Write,
{
    let mut res = Ok(String::new());
    tree.borrow().as_err_tree(&mut |tree| {
        res = serde_json::to_string(&ErrTreeFmtSerde { tree });
    });
    Ok(formatter.write_str(&res?)?)
}

pub fn reconstruct_output<const FRONT_MAX: usize, R, F>(
    json: R,
    formatter: &mut F,
) -> Result<(), JsonErr>
where
    R: Iterator<Item = char>,
    F: fmt::Write,
{
    let tree = from_str::<ErrTreeReconstruct>(json.collect::<String>().as_ref())?;
    write!(
        formatter,
        "{}",
        ErrTreeFmtWrap::<FRONT_MAX, _> { tree: &tree }
    )?;
    Ok(())
}

struct SourcesIterSer<'a> {
    sources: &'a [&'a [&'a dyn AsErrTree]],
}

impl Serialize for SourcesIterSer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let len = self.sources.iter().map(|subsource| subsource.len()).sum();
        let mut seq_serialize = serializer.serialize_seq(Some(len))?;

        for source in self.sources.iter().flat_map(|subsource| subsource.iter()) {
            let mut res = Ok(());
            source.as_err_tree(&mut |tree| {
                res = seq_serialize.serialize_element(&ErrTreeFmtSerde { tree });
            });
            res?
        }

        seq_serialize.end()
    }
}

struct ErrTreeFmtSerde<'a> {
    pub tree: ErrTree<'a>,
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

struct SerializeSpan<'a> {
    trace: &'a tracing_error::SpanTrace,
}

impl Serialize for SerializeSpan<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        let mut res = Ok(());
        self.trace.with_spans(|metadata, fields| {
            res = seq.serialize_element(&JsonSpan {
                target: metadata.target(),
                name: metadata.name(),
                fields,
                location: metadata
                    .file()
                    .and_then(|file| metadata.line().map(|line| (file, line))),
            });
            res.is_ok()
        });
        seq.end()
    }
}

impl Serialize for ErrTreeFmtSerde<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(None)?;
        state.serialize_entry("msg", &self.tree.inner.to_string())?;

        #[cfg(feature = "source_line")]
        if let Some(loc) = self.tree.location {
            state.serialize_entry("location", &loc.to_string())?;
        }

        #[cfg(feature = "tracing")]
        if let Some(trace) = &self.tree.trace {
            state.serialize_entry("trace", &SerializeSpan { trace })?;
        }

        state.serialize_entry(
            "sources",
            &SourcesIterSer {
                sources: self.tree.sources,
            },
        )?;

        state.end()
    }
}

#[derive(Deserialize)]
struct ErrTreeReconstruct {
    msg: String,
    #[cfg(feature = "source_line")]
    location: Option<String>,
    #[cfg(feature = "tracing")]
    #[serde(default)]
    trace: Vec<JsonSpanOwned>,
    #[cfg(feature = "tracing")]
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
    fn trace_unique(&self, found_traces: &[Self::TraceSpanType]) -> bool {
        !self
            .trace
            .iter()
            .any(|trace_line| found_traces.contains(&trace_line))
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
