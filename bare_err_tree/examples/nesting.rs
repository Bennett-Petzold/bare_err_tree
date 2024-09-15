use std::{
    error::Error,
    fmt::{Display, Formatter},
    panic::Location,
};

use bare_err_tree::{print_tree, tree_unwrap, AsErrTree, ErrTree};

fn main() {
    println!("Single error:\n{}\n", single_leaf());
    println!("Nested error:\n{}\n", nested_leaf());
    println!("Doubly nested error:\n{}\n", double_nested_leaf());
    println!(
        "Triple nested error with lines:\n{}\n",
        triple_nested_leaf()
    );

    quad_nested_leaf();
}

#[derive(Debug)]
struct Err1 {}
impl Error for Err1 {}
impl Display for Err1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "error 1")
    }
}

#[derive(Debug)]
enum Err2 {
    Underlying(Err1),
    This(Box<Err2>),
    Origin,
}
impl Error for Err2 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Underlying(x) => Some(x),
            Self::This(x) => Some(x),
            Self::Origin => None,
        }
    }
}
impl Display for Err2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Underlying(_) => write!(f, "error 1 caused error 2"),
            Self::This(_) => write!(f, "error 2 caused by itself"),
            Self::Origin => write!(f, "error 2"),
        }
    }
}

#[derive(Debug)]
enum Err3Enum {
    Underlying(Vec<Err2>),
    Origin,
}

#[derive(Debug)]
struct Err3 {
    inner: Err3Enum,
    location: &'static Location<'static>,
}

impl From<Err3Enum> for Err3 {
    #[track_caller]
    fn from(inner: Err3Enum) -> Self {
        Self {
            inner,
            location: Location::caller(),
        }
    }
}

impl Error for Err3 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.inner {
            // Arbitrarily return only the first error
            Err3Enum::Underlying(x) => Some(&x[0]),
            Err3Enum::Origin => None,
        }
    }
}
impl Display for Err3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Err3Enum::Underlying(_) => write!(f, "multiple error 2"),
            Err3Enum::Origin => write!(f, "error 3"),
        }
    }
}
impl AsErrTree for Err3 {
    fn as_err_tree(&self) -> ErrTree<'_> {
        let sources = match &self.inner {
            Err3Enum::Underlying(x) => x.iter().map(|v| (v as &dyn Error).as_err_tree()).collect(),
            Err3Enum::Origin => vec![],
        }
        .into_boxed_slice();
        ErrTree {
            inner: self,
            sources,
            location: Some(self.location),
        }
    }
}

#[derive(Debug)]
enum Err4 {
    Underlying(Vec<Err3>),
}
impl Error for Err4 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            // Arbitrarily return only the first error
            Self::Underlying(x) => Some(&x[0]),
        }
    }
}
impl Display for Err4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Underlying(_) => write!(f, "multiple error 3"),
        }
    }
}
impl AsErrTree for Err4 {
    #[track_caller]
    fn as_err_tree(&self) -> ErrTree<'_> {
        let sources = match self {
            Err4::Underlying(x) => x.iter().map(AsErrTree::as_err_tree).collect::<Vec<_>>(),
        }
        .into_boxed_slice();
        ErrTree {
            inner: self,
            sources,
            location: Some(Location::caller()),
        }
    }
}

fn single_leaf() -> String {
    let leaf = &Err1 {} as &dyn Error;
    print_tree::<dyn Error, _>(leaf)
}

fn nested_leaf() -> String {
    let leaf = &Err2::Underlying(Err1 {}) as &dyn Error;
    print_tree::<dyn Error, _>(leaf)
}

fn double_nested_leaf() -> String {
    let err = Err2::This(Box::new(Err2::Underlying(Err1 {})));
    let leaf = &err as &dyn Error;
    print_tree::<dyn Error, _>(leaf)
}

fn triple_nested_leaf() -> String {
    let err = Err3::from(Err3Enum::Underlying(vec![
        Err2::This(Box::new(Err2::Underlying(Err1 {}))),
        Err2::Origin,
    ]));
    let leaf = err.as_err_tree();
    print_tree(leaf)
}

fn quad_nested_leaf() {
    let err_0 = Err3Enum::Underlying(vec![
        Err2::This(Box::new(Err2::Underlying(Err1 {}))),
        Err2::Origin,
    ])
    .into();
    let err_1 = Err3Enum::Origin.into();

    let err = Err4::Underlying(vec![err_0, err_1]);
    tree_unwrap(Err::<(), _>(err));
}
