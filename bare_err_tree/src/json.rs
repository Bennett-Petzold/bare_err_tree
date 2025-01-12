//! Error tree output to/from JSON.

use core::{
    borrow::Borrow,
    fmt::{self, Write},
    iter::FusedIterator,
    str::Chars,
};

use crate::{fmt_tree, AsErrTree, ErrTree, ErrTreeFormattable};

/// Produces JSON to store [`ErrTree`] formatted output.
///
/// JSON output can be used to display with [`ErrTree`] format with
/// [`reconstruct_output`], but the [`ErrTree`] itself cannot be reconstructed.
#[track_caller]
pub fn tree_to_json<E, S, F>(tree: S, formatter: &mut F) -> fmt::Result
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
    F: fmt::Write,
{
    let mut res = Ok(());
    tree.borrow().as_err_tree(&mut |tree| {
        res = json_fmt(tree, formatter);
    });
    res
}

/// Custom JSON format outputter
fn json_fmt<F: fmt::Write>(mut tree: ErrTree<'_>, formatter: &mut F) -> fmt::Result {
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

    if let Some(first_source) = tree.sources.next() {
        formatter.write_str(",\"sources\":[")?;
        let mut res = Ok(());
        first_source.as_err_tree(&mut |subtree| {
            res = json_fmt(subtree, formatter);
        });
        res?;

        for source in tree.sources {
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

const BACKSPACE: char = 8 as char;
const FORM_FEED: char = 12 as char;
const JSON_ESCAPE: [char; 7] = ['"', '\\', BACKSPACE, FORM_FEED, '\n', '\r', '\t'];

impl<F: Write> Write for JsonEscapeFormatter<'_, F> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
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
        formatter.write_str(",\"source_loc\":[\"file\":\"")?;
        write!(JsonEscapeFormatter { formatter }, "{}", file)?;
        write!(formatter, "\",\"line\":{line}]")?;
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
pub fn reconstruct_output<const FRONT_MAX: usize, S, F>(json: S, formatter: &mut F) -> fmt::Result
where
    S: AsRef<str>,
    F: fmt::Write,
{
    fmt_tree::<FRONT_MAX, _, _>(JsonReconstruct::new(json.as_ref()), formatter)
}

const EMPTY_STR: &str = "";

struct JsonReconstruct<'f> {
    msg: &'f str,
    #[cfg(feature = "source_line")]
    source_line: &'f str,
    #[cfg(feature = "tracing")]
    trace: &'f str,
    sources: &'f str,
}

const BRACE_LEN: usize = '{'.len_utf8();
const BRACKET_LEN: usize = '['.len_utf8();

impl<'f> JsonReconstruct<'f> {
    pub fn new(json_body: &'f str) -> Self {
        const SOURCES_KEY: &str = "\"sources\"";
        const MSG_KEY: &str = "\"msg\"";
        #[cfg(feature = "source_line")]
        const LOCATION_KEY: &str = "\"location\"";
        #[cfg(feature = "tracing")]
        const TRACE_KEY: &str = "\"trace\"";

        let first_brace = json_meta_char_idx('{', json_body).unwrap_or(json_body.len());
        let last_brace =
            json_char_idx('{', json_body.char_indices().rev()).unwrap_or(json_body.len());
        let json_body = &json_body[(first_brace + BRACE_LEN)..(last_brace - BRACE_LEN)];

        let (before_sources, sources, after_sources) =
            if let Some(sources_colon) = find_json_key(SOURCES_KEY, json_body) {
                let sources_start_slice = &json_body[sources_colon..];
                if let Some(end_idx) = json_char_idx(']', sources_start_slice.char_indices()) {
                    (
                        &json_body[..sources_colon - SOURCES_KEY.len()],
                        &sources_start_slice[BRACKET_LEN..end_idx],
                        &sources_start_slice[end_idx + BRACKET_LEN..],
                    )
                } else {
                    (EMPTY_STR, EMPTY_STR, EMPTY_STR)
                }
            } else {
                (json_body, EMPTY_STR, EMPTY_STR)
            };

        let msg = [before_sources, after_sources]
            .iter()
            .map(|sub_body| find_json_str(MSG_KEY, sub_body))
            .find(|s| !s.is_empty())
            .unwrap_or(EMPTY_STR);

        #[cfg(feature = "source_line")]
        let source_line = [before_sources, after_sources]
            .iter()
            .map(|sub_body| find_json_str(LOCATION_KEY, sub_body))
            .find(|s| !s.is_empty())
            .unwrap_or(EMPTY_STR);

        #[cfg(feature = "tracing")]
        let trace = [before_sources, after_sources]
            .iter()
            .flat_map(|sub_body| {
                let trace_start = find_json_key(TRACE_KEY, sub_body)?;
                let slice_start = &sub_body[trace_start..];
                let trace_sub_end = json_char_idx(']', slice_start.char_indices())?;

                let trace_end = trace_start + trace_sub_end;

                let trace_adjusted_start = BRACKET_LEN + trace_start;

                Some(&sub_body[trace_adjusted_start..trace_end])
            })
            .next()
            .unwrap_or(EMPTY_STR);

        Self {
            msg,
            #[cfg(feature = "source_line")]
            source_line,
            #[cfg(feature = "tracing")]
            trace,
            sources,
        }
    }
}

impl<'f> ErrTreeFormattable for JsonReconstruct<'f> {
    fn apply_msg<W: fmt::Write + ?Sized>(&self, f: &mut W) -> fmt::Result {
        apply_json_str(self.msg, f)
    }

    type Source<'a> = JsonReconstruct<'f>;
    fn sources_empty(&mut self) -> bool {
        SourcesIter::new(self.sources).next().is_none()
    }

    fn apply_to_leading_sources<F>(&mut self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        let mut iter = SourcesIter::new(self.sources);
        if let Some(mut prev_source) = iter.next() {
            // Skips the last source by operating one behind
            for next_source in iter {
                (func)(Self::new(prev_source))?;
                prev_source = next_source;
            }
        }
        Ok(())
    }
    fn apply_to_last_source<F>(&mut self, mut func: F) -> fmt::Result
    where
        F: FnMut(Self::Source<'_>) -> fmt::Result,
    {
        if let Some(last_source) = SourcesIter::new(self.sources).next_back() {
            (func)(Self::new(last_source))?;
        }
        Ok(())
    }

    #[cfg(feature = "source_line")]
    fn has_source_line(&self) -> bool {
        !self.source_line.is_empty()
    }
    #[cfg(feature = "source_line")]
    fn apply_source_line<W: fmt::Write + ?Sized>(&self, f: &mut W) -> fmt::Result {
        apply_json_str(self.source_line, f)
    }

    #[cfg(feature = "tracing")]
    fn trace_empty(&self) -> bool {
        self.trace.is_empty()
    }

    type TraceSpanId = &'f str;
    type TraceSpanIter<'a> = JsonStrChars<'a>;

    #[cfg(feature = "tracing")]
    fn apply_trace<F>(&self, mut func: F) -> fmt::Result
    where
        F: FnMut(crate::TraceSpan<Self::TraceSpanId, Self::TraceSpanIter<'_>>) -> fmt::Result,
    {
        use crate::TraceSpan;

        const TARGET: &str = "\"target\"";
        const NAME: &str = "\"name\"";
        const FIELDS: &str = "\"fields\"";
        const LOCATION: &str = "\"source_loc\"";
        const FILE: &str = "\"file\"";
        const LINE: &str = "\"line\"";

        for trace_line in SourcesIter::new(self.trace) {
            let trace_line_start =
                json_meta_char_idx('{', trace_line).unwrap_or(trace_line.len()) + BRACE_LEN;
            let trace_line = &trace_line[trace_line_start..];

            let iter = |id| JsonStrChars::new(find_json_str(id, trace_line));

            let location = find_json_key(LOCATION, trace_line).and_then(|location_start| {
                let slice_start = &trace_line[location_start..];

                let loc_start_idx = json_meta_char_idx('[', slice_start)? + BRACKET_LEN;
                let slice_inner = &slice_start[loc_start_idx..];

                let file = find_json_str(FILE, slice_inner);
                if file.is_empty() {
                    None
                } else {
                    let line_start = find_json_key(LINE, slice_inner).unwrap_or(slice_inner.len());
                    let line_end = json_char_idx(']', slice_start.char_indices())? - loc_start_idx;

                    let line = str::parse(&slice_inner[line_start..line_end]).ok()?;

                    Some((JsonStrChars::new(file), line))
                }
            });

            (func)(TraceSpan {
                identifier: trace_line,
                target: iter(TARGET),
                name: iter(NAME),
                fields: iter(FIELDS),
                location,
            })?;
        }

        Ok(())
    }
}

/// Returns the index after `field` in `json_body`.
fn json_field_idx(field: &str, json_body: &str) -> Option<usize> {
    // Count these fields separately, to return None on malformed input
    let mut brace_counter = 0_usize;
    let mut bracket_counter = 0_usize;

    // Presence of a backslash effectively cancels the next character
    let mut prev_backslash = false;
    // Quote starts and stops are identical, so this bool just flips whenever
    // one is encountered
    let mut in_quote = false;

    for (idx, json_char) in json_body.char_indices() {
        if prev_backslash {
            prev_backslash = false;
        } else {
            match json_char {
                '\\' => prev_backslash = true,
                '{' if !in_quote => brace_counter = brace_counter.checked_add(1)?,
                '[' if !in_quote => bracket_counter = bracket_counter.checked_add(1)?,
                '}' if !in_quote => brace_counter = brace_counter.checked_sub(1)?,
                ']' if !in_quote => bracket_counter = bracket_counter.checked_sub(1)?,
                '"' => in_quote = !in_quote,
                _ => (),
            }
        }

        let regular_region = (!in_quote) && (brace_counter == 0) && (bracket_counter == 0);

        if regular_region && json_body[..=idx].ends_with(field) {
            return Some(idx);
        }
    }

    None
}

/// Returns the first index of `target` in `json_body`.
fn json_char_idx<I>(target: char, json_body_iter: I) -> Option<usize>
where
    I: IntoIterator<Item = (usize, char)>,
{
    // Count these fields separately, to return None on malformed input
    // Counting with isize allows for running in reverse
    let mut brace_counter = if target == '{' { -1 } else { 0_isize };
    let mut bracket_counter = if target == '[' { -1 } else { 0_isize };

    // Presence of a backslash effectively cancels the next character
    let mut prev_backslash = false;
    // Quote starts and stops are identical, so this bool just flips whenever
    // one is encountered
    let mut in_quote = false;

    for (idx, json_char) in json_body_iter.into_iter() {
        if prev_backslash {
            prev_backslash = false;
        } else {
            match json_char {
                '\\' => prev_backslash = true,
                '{' if !in_quote => brace_counter = brace_counter.checked_add(1)?,
                '[' if !in_quote => bracket_counter = bracket_counter.checked_add(1)?,
                '}' if !in_quote => brace_counter = brace_counter.checked_sub(1)?,
                ']' if !in_quote => bracket_counter = bracket_counter.checked_sub(1)?,
                '"' => in_quote = !in_quote,
                _ => (),
            }

            // Special casing when the character is '"'.
            let unquoted = (json_char == '"') || (!in_quote);
            let regular_region = unquoted && (brace_counter == 0) && (bracket_counter == 0);

            if regular_region && (json_char == target) {
                return Some(idx);
            }
        }
    }

    None
}

/// Returns the first index of `meta` in `json_body`, returning `None` if a
/// meta character is prior.
fn json_meta_char_idx(meta: char, json_body: &str) -> Option<usize> {
    // As per JSON spec
    const META_CHARS: [char; 6] = ['"', ':', '{', '}', '[', ']'];

    // Presence of a backslash effectively cancels the next character
    let mut prev_backslash = false;

    for (idx, json_char) in json_body.char_indices() {
        if prev_backslash {
            prev_backslash = false;
        } else {
            match json_char {
                // Handle this check first since meta is a special character
                x if x == meta => return Some(idx),
                x if META_CHARS.contains(&x) => return None,
                '\\' => prev_backslash = true,
                _ => (),
            }
        }
    }

    None
}

/// Returns the first index with an unescaped quote.
fn json_quote_end(json_body: &str) -> Option<usize> {
    // Presence of a backslash effectively cancels the next character
    let mut prev_backslash = false;

    for (idx, json_char) in json_body.char_indices() {
        if prev_backslash {
            prev_backslash = false;
        } else {
            match json_char {
                '"' => return Some(idx),
                '\\' => prev_backslash = true,
                _ => (),
            }
        }
    }

    None
}

#[inline]
fn next_char_idx(s: &str) -> Option<usize> {
    Some(s.char_indices().nth(1)?.0)
}

/// Returns the idx after the JSON field colon, if it exists.
///
/// Field must include its JSON field quotes (e.g. `let field = "\"foo\"";`)
fn find_json_key(field: &str, json_body: &str) -> Option<usize> {
    // ID the key, if it exists
    if let Some(field_end) = json_field_idx(field, json_body) {
        if let Some(colon_search) = next_char_idx(&json_body[field_end..]) {
            let colon_offset = field_end + colon_search;
            // ID the colon following the key, if it exists
            if let Some(colon_loc) = json_meta_char_idx(':', &json_body[colon_offset..]) {
                let total_offset = colon_loc + colon_offset;
                return next_char_idx(&json_body[total_offset..]).map(|x| x + total_offset);
            }
        }
    }

    None
}

/// Returns `field`'s string, or an empty string.
///
/// Field must include its JSON field quotes (e.g. `let field = "\"foo\"";`)
fn find_json_str<'a>(field: &str, json_body: &'a str) -> &'a str {
    if let Some(quote_search) = find_json_key(field, json_body) {
        // There cannot be a meta character before the string start quote
        if let Some(opening_quote) = json_meta_char_idx('"', &json_body[quote_search..]) {
            let opening_quote_offset = opening_quote + quote_search;
            if let Some(quote_start) = next_char_idx(&json_body[opening_quote_offset..]) {
                let quote_start_offset = quote_start + opening_quote_offset;
                if let Some(closing_quote) = json_quote_end(&json_body[quote_start_offset..]) {
                    let closing_quote_offset = closing_quote + quote_start_offset;
                    return &json_body[quote_start_offset..closing_quote_offset];
                }
            }
        }
    }

    EMPTY_STR
}

struct JsonStrChars<'a> {
    prev_backslash: bool,
    iter: Chars<'a>,
}

impl<'a> JsonStrChars<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            prev_backslash: false,
            iter: s.chars(),
        }
    }
}

impl Iterator for JsonStrChars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.iter.next()?;

        if self.prev_backslash {
            self.prev_backslash = false;
            match c {
                '"' => Some('"'),
                '\\' => Some('\\'),
                '/' => Some('/'),
                'b' => Some(BACKSPACE),
                'f' => Some(FORM_FEED),
                'n' => Some('\n'),
                'r' => Some('\r'),
                't' => Some('\t'),
                // Invalid backslash, quit processing
                _ => {
                    // Drain the inner iterator to fuse
                    let _ = self.iter.by_ref().last();
                    None
                }
            }
        } else if c != '\\' {
            Some(c)
        // c == '\\'
        } else {
            self.prev_backslash = true;
            self.next()
        }
    }
}

impl FusedIterator for JsonStrChars<'_> {}

fn apply_json_str<F: fmt::Write + ?Sized>(s: &str, formatter: &mut F) -> fmt::Result {
    for c in JsonStrChars::new(s) {
        formatter.write_char(c)?;
    }
    Ok(())
}

struct SourcesIter<'f> {
    json_body: &'f str,
}

impl<'f> SourcesIter<'f> {
    pub fn new(json_body: &'f str) -> Self {
        Self { json_body }
    }
}

impl<'f> Iterator for SourcesIter<'f> {
    type Item = &'f str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(comma_idx) = json_char_idx(',', self.json_body.char_indices()) {
            let (res, trailing) = self.json_body.split_at(comma_idx);
            let new_start_idx = trailing
                .char_indices()
                .nth(1)
                .map(|(idx, _)| idx)
                .unwrap_or(trailing.len());
            self.json_body = &trailing[new_start_idx..];
            Some(res)
        } else if self.json_body.is_empty() {
            None
        } else {
            let res = self.json_body;
            self.json_body = EMPTY_STR;
            Some(res)
        }
    }
}

impl FusedIterator for SourcesIter<'_> {}

impl DoubleEndedIterator for SourcesIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(comma_idx) = json_char_idx(',', self.json_body.char_indices().rev()) {
            let (before, res) = self.json_body.split_at(comma_idx);
            self.json_body = before;
            let new_start_idx = res
                .char_indices()
                .nth(1)
                .map(|(idx, _)| idx)
                .unwrap_or(res.len());
            Some(&res[new_start_idx..])
        } else if self.json_body.is_empty() {
            None
        } else {
            let res = self.json_body;
            self.json_body = EMPTY_STR;
            Some(res)
        }
    }
}
