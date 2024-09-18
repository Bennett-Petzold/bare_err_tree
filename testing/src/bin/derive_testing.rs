/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    marker::PhantomData,
};

use bare_err_tree::{err_tree, AsErrTree};

fn main() {}

#[err_tree(InnerErrWrap)]
#[derive(Default, Debug, Clone, Copy)]
struct InnerErrStruct;

impl Error for InnerErrStruct {}
impl Display for InnerErrStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

#[expect(dead_code)]
#[err_tree]
#[derive(Default, Debug)]
struct ErrStruct<
    'a,
    'b: 'a,
    Tree: AsErrTree,
    const C: usize = 5,
    Err: Error + 'static = std::io::Error,
> {
    #[dyn_err]
    err: Err,
    #[tree_err]
    err2: Tree,
    #[dyn_iter_err]
    err_slice: Vec<std::io::Error>,
    #[tree_iter_err]
    err2_slice: Vec<InnerErrWrap>,
    _phantom: PhantomData<&'a ()>,
    _phantom_2: PhantomData<&'b ()>,
}

impl<const C: usize, Tree: AsErrTree, Err: Error> Error for ErrStruct<'_, '_, Tree, C, Err> {}
impl<const C: usize, Tree: AsErrTree, Err: Error> Display for ErrStruct<'_, '_, Tree, C, Err> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.err, f)
    }
}

#[expect(dead_code)]
#[err_tree(ErrEnumWrap)]
#[derive(Debug, Clone, Copy)]
enum ErrEnum {
    #[dyn_err]
    Local(InnerErrWrap),
}

impl Error for ErrEnum {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Local(x) => x.source(),
        }
    }
}

impl Display for ErrEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}
