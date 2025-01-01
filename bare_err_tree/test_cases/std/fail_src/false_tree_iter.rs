/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

use bare_err_tree::err_tree;

fn main() {}

#[allow(dead_code)]
#[err_tree]
#[derive(Debug)]
struct ErrStruct<'a> {
    #[tree_iter_err]
    err: &'a [std::io::Error; 1],
}

impl Error for ErrStruct<'_> {}
impl Display for ErrStruct<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.err[0], f)
    }
}
