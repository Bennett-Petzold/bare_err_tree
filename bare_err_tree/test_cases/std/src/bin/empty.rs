/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use bare_err_tree::ErrTreeDisplay;
use std::fmt::Write;
use thiserror::Error;

#[allow(dead_code)]
fn main() {
    let formatted = gen_print();
    println!("{formatted}")
}

fn gen_print() -> String {
    let fatal = Empty;
    let mut formatted = String::new();
    write!(
        formatted,
        "{}",
        ErrTreeDisplay::<_, 60>(&fatal as &dyn std::error::Error)
    )
    .unwrap();
    formatted
}

#[derive(Debug, Error)]
#[error("EMPTY")]
struct Empty;
