use core::{
    borrow::Borrow,
    error::Error,
    fmt,
    fmt::{Debug, Display, Formatter},
};
use std::panic::Location;

/// Simple single-linked list, tracking from children
///
/// The node with no parent is treated as empty.
#[derive(Debug, Clone, Copy)]
struct FmtDepthNode<'a> {
    val: bool,
    parent: Option<&'a FmtDepthNode<'a>>,
}

impl<'a> FmtDepthNode<'a> {
    fn new(val: bool, parent: Option<&'a FmtDepthNode<'a>>) -> Self {
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

pub struct ErrTreeFmt<'a> {
    tree: &'a ErrTree<'a>,
    node: FmtDepthNode<'a>,
}

impl Display for ErrTreeFmt<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.tree, f)?;

        let mut front_lines = String::new();
        for parent_display in &self.node {
            if parent_display {
                front_lines = "│   ".to_string() + &front_lines;
            } else {
                front_lines = "    ".to_string() + &front_lines;
            }
        }

        if let Some(location) = self.tree.location {
            writeln!(f)?;
            write!(f, "{}", front_lines)?;
            if self.tree.sources().is_empty() {
                write!(f, "╰─ ")?;
            } else {
                write!(f, "├─ ")?;
            }
            write!(f, "at \x1b[3m{}\x1b[0m", location)?;
        }

        let mut source_fmt = |source: &ErrTree, last: bool| {
            // Preamble arrow connections
            write!(f, "\n{0}│\n{0}", front_lines)?;
            if !last {
                write!(f, "├─▶ ")?;
            } else {
                write!(f, "╰─▶ ")?;
            }

            let next_node = FmtDepthNode::new(!last, Some(&self.node));
            ErrTreeFmt {
                tree: source,
                node: next_node,
            }
            .fmt(f)
        };

        for source in &self.tree.sources()[..self.tree.sources().len().saturating_sub(1)] {
            source_fmt(source, false)?;
        }
        if let Some(last_source) = self.tree.sources().last() {
            source_fmt(last_source, true)?;
        }
        Ok(())
    }
}

/// Alternative to [`Result::unwrap`] that formats the error as a tree.
///
/// Errors must define [`Error::source`] correctly for the tree to display.
/// The derive macros for [`ErrTree`] track extra information and handle
/// multiple sources ([`Error::source`] is designed around a single error
/// source).
#[track_caller]
pub fn tree_unwrap<T, E, S>(res: Result<T, S>) -> T
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
{
    match res {
        Ok(x) => x,
        Err(tree) => {
            panic!(
                "{}",
                ErrTreeFmt {
                    tree: &tree.borrow().as_err_tree(),
                    node: FmtDepthNode::new(false, None),
                }
            );
        }
    }
}

/// Produces [`ErrTree`] formatted output for an error.
#[track_caller]
pub fn print_tree<E, S>(tree: S) -> String
where
    S: Borrow<E>,
    E: AsErrTree + ?Sized,
{
    format!(
        "{}",
        ErrTreeFmt {
            tree: &tree.borrow().as_err_tree(),
            node: FmtDepthNode::new(false, None),
        }
    )
}

#[derive(Debug, Clone)]
pub struct ErrTree<'a> {
    pub inner: &'a dyn Error,
    pub sources: Box<[ErrTree<'a>]>,
    pub location: Option<&'a Location<'a>>,
}

/// Defines an [`Error`]'s conversion into [`ErrTree`].
///
/// This is automatically defined for `dyn` [`Error`], but the macro derives
/// provide more information.
pub trait AsErrTree {
    fn as_err_tree(&self) -> ErrTree<'_>;
}

impl<'a> AsErrTree for ErrTree<'a> {
    fn as_err_tree(&self) -> ErrTree<'_> {
        Self {
            inner: self.inner,
            sources: self.sources.clone(),
            location: self.location,
        }
    }
}

impl AsErrTree for dyn Error {
    fn as_err_tree(&self) -> ErrTree<'_> {
        let sources = match self.source() {
            Some(e) => vec![e.as_err_tree()],
            None => vec![],
        }
        .into_boxed_slice();
        ErrTree {
            inner: self,
            sources,
            location: None,
        }
    }
}

impl ErrTree<'_> {
    fn sources(&self) -> &[ErrTree<'_>] {
        &self.sources
    }
}

impl Error for ErrTree<'_> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl Display for ErrTree<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
