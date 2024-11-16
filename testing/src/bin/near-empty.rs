/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use bare_err_tree::{err_tree, print_tree, AsErrTree};
use thiserror::Error;

fn main() {
    let fatal = Empty::_tree();
    let mut formatted = String::new();
    print_tree::<60, _, _, _>(fatal, &mut formatted).unwrap();
    println!("{formatted}")
}

#[err_tree]
#[derive(Debug, Error)]
#[error("EMPTY")]
struct Empty {}
