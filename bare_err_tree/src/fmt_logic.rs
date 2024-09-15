use core::{
    fmt,
    fmt::{Debug, Display, Formatter},
};

use alloc::string::{String, ToString};

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

pub(crate) struct ErrTreeFmt<'a> {
    pub tree: &'a ErrTree<'a>,
    pub node: FmtDepthNode<'a>,
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
