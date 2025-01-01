/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use bare_err_tree::{err_tree, print_tree};
use thiserror::Error;
use tracing_error::ErrorLayer;
use tracing_subscriber::{field::MakeExt, layer::SubscriberExt};

#[allow(dead_code)]
fn main() {
    run_fatal()
}

fn run_fatal() {
    let formatted = gen_print();
    println!("{formatted}")
}

#[tracing::instrument]
fn gen_print() -> String {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(tracing_subscriber::fmt::layer().map_fmt_fields(|f| f.debug_alt()))
        // any number of other subscriber layers may be added before or
        // after the `ErrorLayer`...
        .with(ErrorLayer::default());

    // set the subscriber as the default for the application
    let _ = tracing::subscriber::set_global_default(subscriber);

    let fatal = Empty::_tree();
    let mut formatted = String::new();
    print_tree::<60, _, _, _>(fatal, &mut formatted).unwrap();
    formatted
}

#[err_tree]
#[derive(Debug, Error)]
#[error("EMPTY")]
struct Empty {}
