/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{self, Display, Formatter};

use bare_err_tree::{err_tree, print_tree};
use thiserror::Error;

fn main() {
    let fatal = Empty;
    let mut formatted = String::new();
    print_tree::<60, &dyn std::error::Error, _, _>(
        &fatal as &dyn std::error::Error,
        &mut formatted,
    )
    .unwrap();
    println!("{formatted}")
}

#[derive(Debug, Error)]
#[error("EMPTY")]
struct Empty;
