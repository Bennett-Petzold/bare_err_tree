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
struct ErrStruct {
    #[dyn_iter_err]
    err: std::io::Error,
}

impl Error for ErrStruct {}
impl Display for ErrStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.err, f)
    }
}
