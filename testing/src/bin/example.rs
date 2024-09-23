/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{self, Display, Formatter};

use bare_err_tree::{err_tree, print_tree};
use thiserror::Error;

fn main() {
    let fatal: MissedClassTree = MissedClass::Overslept(Overslept::new(BedTime::new(
        2,
        vec![
            ClassProject::new("proving 1 == 2".to_string()).into(),
            BedTimeReasons::ExamStressed,
            BedTimeReasons::PlayingGames,
        ],
    )))
    .into();
    let mut formatted = String::new();
    print_tree::<60, _, _, _>(fatal, &mut formatted).unwrap();
    println!("{formatted}")
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

#[derive(Debug, Error, Default)]
#[error("bed is comfortable")]
struct BedComfy;

#[err_tree]
#[derive(Debug, Error, Default)]
#[error("stayed in bed too long")]
struct Overslept {
    #[tree_err]
    #[source]
    bed_time: BedTime,
    #[dyn_err]
    comfy: BedComfy,
}

impl Overslept {
    #[track_caller]
    fn new(bed_time: BedTime) -> Self {
        Overslept::_tree(bed_time, BedComfy)
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
