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
    let formatted = gen_print();
    println!("{formatted}")
}

fn gen_print() -> String {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(tracing_subscriber::fmt::layer().map_fmt_fields(|f| f.debug_alt()))
        // any number of other subscriber layers may be added before or
        // after the `ErrorLayer`...
        .with(ErrorLayer::default());

    // set the subscriber as the default for the application
    let _ = tracing::subscriber::set_global_default(subscriber);

    gen_print_inner()
}

#[tracing::instrument]
fn gen_print_inner() -> String {
    let fatal: MissedClassTree = MissedClass::Overslept(Overslept::new(ClassProject::new(
        "proving 1 == 2".to_string(),
    )))
    .into();
    let mut formatted = String::new();
    print_tree::<60, _, _>(fatal, &mut formatted).unwrap();
    formatted
}

#[err_tree]
#[derive(Debug, Error)]
#[error("{desc}")]
struct ClassProject {
    desc: String,
}

impl ClassProject {
    #[track_caller]
    pub fn new(desc: String) -> Self {
        Self::_tree(desc)
    }
}

#[err_tree]
#[derive(Debug, Error)]
#[error("stayed in bed too long")]
struct Overslept {
    #[tree_err]
    #[source]
    project: ClassProject,
}

impl Overslept {
    #[track_caller]
    fn new(project: ClassProject) -> Self {
        Overslept::_tree(project)
    }
}

#[err_tree(MissedClassTree)]
#[derive(Debug, Error)]
#[error("missed class")]
enum MissedClass {
    #[tree_err]
    Overslept(#[source] Overslept),
    #[expect(unused)]
    NuclearWar,
}
