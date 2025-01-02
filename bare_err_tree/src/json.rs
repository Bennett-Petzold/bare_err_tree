//! Error tree output to/from JSON

extern crate alloc;

use core::{
    borrow::Borrow,
    cell::RefCell,
    fmt::{self, Display, Formatter, Write},
};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use serde::{
    de::Visitor,
    ser::{SerializeSeq, SerializeStruct},
    Deserialize, Serialize,
};
use serde_json::from_str;

use crate::{AsErrTree, ErrTree, ErrTreeFmt};

#[track_caller]
pub fn tree_to_json<E, S>(tree: S) -> Result<String, serde_json::Error>
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
{
    let mut res = Ok(String::new());
    tree.borrow().as_err_tree(&mut |tree| {
        let found_traces = &RefCell::new(Vec::new());
        res = serde_json::to_string(&ErrTreeFmtSerde { tree, found_traces });
    });
    res
}

#[track_caller]
pub fn reconstruct_output<S>(json: S) -> Result<String, serde_json::Error>
where
    S: AsRef<str>,
{
    from_str::<ErrTreeReconstruct>(json.as_ref()).map(|x| {
        let mut out = String::new();
        x.fmt(&mut out, String::new()).unwrap();
        out.trim_end().to_string()
    })
}

struct SourcesIterSer<'a> {
    sources: &'a [&'a [&'a dyn AsErrTree]],
    found_traces: &'a RefCell<Vec<tracing_core::callsite::Identifier>>,
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
                res = seq_serialize.serialize_element(&ErrTreeFmtSerde {
                    tree,
                    found_traces: self.found_traces,
                });
            });
            res?
        }

        seq_serialize.end()
    }
}

struct ErrTreeFmtTraceSerde<'a> {
    pub tree: ErrTree<'a>,
    pub found_traces: &'a RefCell<Vec<tracing_core::callsite::Identifier>>,
}

impl Display for ErrTreeFmtTraceSerde<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        ErrTreeFmt::<0> {
            tree: self.tree.clone(),
            scratch_fill: 0,
            front_lines: &mut [],
            found_traces: &mut self.found_traces.borrow_mut(),
        }
        .tracing(f)
    }
}

struct ErrTreeFmtSerde<'a> {
    pub tree: ErrTree<'a>,
    pub found_traces: &'a RefCell<Vec<tracing_core::callsite::Identifier>>,
}

impl Serialize for ErrTreeFmtSerde<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Color", 3)?;
        state.serialize_field("msg", &self.tree.inner.to_string())?;

        #[cfg(feature = "source_line")]
        if let Some(loc) = self.tree.location {
            state.serialize_field("location", &loc.to_string())?;
        }

        #[cfg(feature = "tracing")]
        if self.tree.trace.is_some() {
            let mut trace = String::new();
            write!(
                &mut trace,
                "{}",
                ErrTreeFmtTraceSerde {
                    tree: self.tree.clone(),
                    found_traces: self.found_traces
                }
            )
            .unwrap();
            state.serialize_field("trace", &trace.to_string())?;
        }

        state.serialize_field(
            "sources",
            &SourcesIterSer {
                sources: self.tree.sources,
                found_traces: self.found_traces,
            },
        )?;

        state.end()
    }
}

#[derive(Debug, Deserialize)]
struct ErrTreeReconstruct {
    msg: String,
    #[cfg(feature = "source_line")]
    location: Option<String>,
    #[cfg(feature = "tracing")]
    trace: Option<String>,
    #[cfg(feature = "tracing")]
    sources: Vec<Self>,
}

impl ErrTreeReconstruct {
    fn fmt(&self, f: &mut String, leading: String) -> fmt::Result {
        f.push_str(&self.msg);
        f.push('\n');

        #[cfg(feature = "source_line")]
        if let Some(loc) = &self.location {
            f.push_str(&leading);

            #[cfg(not(feature = "tracing"))]
            let last_entry = self.sources.is_empty();
            #[cfg(feature = "tracing")]
            let last_entry = self.sources.is_empty() && self.trace.is_none();

            if last_entry {
                f.push_str("╰─ ");
            } else {
                f.push_str("├─ ");
            }
            f.push_str("at ");
            f.push_str(loc);
        }

        #[cfg(feature = "tracing")]
        if let Some(trace) = &self.trace {
            if let Some(line) = trace.lines().next() {
                f.push_str(line);
                f.push('\n');
            }
            for line in trace.lines().skip(1) {
                f.push_str(&leading);
                f.push_str(line);
                f.push('\n');
            }
            f.push_str(&leading);
            f.push_str("│\n");
        }

        {
            let mut new_lead = leading.clone();
            new_lead.push_str("│    ");
            for source in &self.sources[..self.sources.len().saturating_sub(1)] {
                f.push_str(&leading);
                f.push_str("├─▶ ");
                source.fmt(f, new_lead.clone())?;
                f.push_str(&leading);
                f.push('│');
                f.push('\n');
            }
        }

        if let Some(last) = self.sources.last() {
            let mut new_lead = leading.clone();
            new_lead.push_str("    ");
            f.push_str(&leading);
            f.push_str("╰─▶ ");
            last.fmt(f, new_lead)?;
        }
        Ok(())
    }
}
