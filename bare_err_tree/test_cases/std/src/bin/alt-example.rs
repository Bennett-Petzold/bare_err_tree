/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{self, Display, Formatter, Write};

use bare_err_tree::{err_tree, ErrTreeDisplay};
use thiserror::Error;

#[allow(dead_code)]
fn main() {
    let formatted = gen_print();
    println!("{formatted}")
}

fn gen_print() -> String {
    let fatal: MissedClassTree = MissedClass::Overslept(Overslept::new(BedTime::new(
        2,
        vec![
            BedTimeReasons::ExamStressed,
            BedTimeReasons::PlayingGames,
            ClassProject::new("proving 1 == 2".to_string()).into(),
        ],
    )))
    .into();
    let mut formatted = String::new();
    write!(formatted, "{}", ErrTreeDisplay::<_, 60>(fatal)).unwrap();
    formatted
}

#[derive(Debug, Error)]
#[error("{desc}")]
struct ClassProject {
    desc: String,
}

impl ClassProject {
    pub fn new(desc: String) -> Self {
        Self { desc }
    }
}

#[derive(Debug, Error)]
enum BedTimeReasons {
    #[error("finishing a project")]
    FinishingProject(#[from] ClassProject),
    #[error("stressed about exams")]
    ExamStressed,
    #[error("playing video games")]
    PlayingGames,
}

#[err_tree]
#[derive(Debug, Default, Error)]
struct BedTime {
    hour: u8,
    #[dyn_iter_err]
    reasons: Vec<BedTimeReasons>,
}

impl BedTime {
    #[track_caller]
    pub fn new(hour: u8, reasons: Vec<BedTimeReasons>) -> Self {
        Self::_tree(hour, reasons)
    }
}

impl Display for BedTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let half = if self.hour < 12 { "A.M." } else { "P.M." };
        let hour = if self.hour > 12 {
            self.hour - 12
        } else {
            self.hour
        };
        write!(f, "went to sleep at {hour} {half}")
    }
}

#[err_tree]
#[derive(Debug, Error)]
#[error("bed is comfortable")]
struct BedComfy;

#[err_tree]
#[derive(Debug, Error, Default)]
#[error("stayed in bed too long")]
struct Overslept {
    #[tree_err]
    #[source]
    bed_time: BedTime,
    #[tree_err]
    comfy: BedComfy,
}

impl Overslept {
    #[track_caller]
    fn new(bed_time: BedTime) -> Self {
        Overslept::_tree(bed_time, BedComfy::_tree())
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
