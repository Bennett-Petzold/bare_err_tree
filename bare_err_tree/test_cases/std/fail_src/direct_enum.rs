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
enum ErrEnum {
    Only(std::io::Error),
}

impl Error for ErrEnum {}
impl Display for ErrEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Only(err) => Display::fmt(&err, f),
        }
    }
}
