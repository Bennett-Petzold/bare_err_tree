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
union ErrUnion {
    f1: i32,
    f2: i32,
}

impl Error for ErrUnion {}
impl Display for ErrUnion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}
impl Debug for ErrUnion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}
