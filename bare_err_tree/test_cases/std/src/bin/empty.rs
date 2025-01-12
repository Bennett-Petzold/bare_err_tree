/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use bare_err_tree::print_tree;
use thiserror::Error;

#[allow(dead_code)]
fn main() {
    let formatted = gen_print();
    println!("{formatted}")
}

fn gen_print() -> String {
    let fatal = Empty;
    let mut formatted = String::new();
    print_tree::<60, _, _>(&fatal as &dyn std::error::Error, &mut formatted).unwrap();
    formatted
}

#[derive(Debug, Error)]
#[error("EMPTY")]
struct Empty;
