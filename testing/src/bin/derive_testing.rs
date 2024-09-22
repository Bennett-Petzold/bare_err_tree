/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
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
#[derive(Debug)]
struct ErrStruct<
    'a,
    'b: 'a,
    Tree: AsErrTree + Debug,
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
    #[dyn_iter_err]
    err_slice_static: &'a [std::io::Error; 3],
    #[tree_iter_err]
    err2_slice_static: [InnerErrWrap; THREE],
    _phantom: PhantomData<&'a ()>,
    _phantom_2: PhantomData<&'b ()>,
}

const THREE: usize = 3;

impl<const C: usize, Tree: AsErrTree + Debug, Err: Error> Error
    for ErrStruct<'_, '_, Tree, C, Err>
{
}
impl<const C: usize, Tree: AsErrTree + Debug, Err: Error> Display
    for ErrStruct<'_, '_, Tree, C, Err>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.err, f)
    }
}

#[expect(dead_code)]
#[err_tree(ErrEnumWrap)]
#[derive(Debug)]
enum ErrEnum {
    #[dyn_err]
    Local(InnerErrWrap),
    #[dyn_iter_err]
    IoGroup([std::io::Error; 7]),
    #[dyn_iter_err]
    IoVec(Vec<std::io::Error>),
}

impl Error for ErrEnum {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Local(x) => x.source(),
            Self::IoGroup(x) => Some(&x[0]),
            Self::IoVec(x) => x.first().map(|x| x as &dyn Error),
        }
    }
}

impl Display for ErrEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}
