//! Materialize portable synthetic evidence without running a swarm.
#[path = "../tests/support/fixtures.rs"]
mod fixtures;
use clap::Parser;
use std::{path::PathBuf, process::ExitCode};
#[derive(Parser)]
struct Args {
    /// A new directory for the synthetic swarm.
    directory: PathBuf,
    /// Remove the evidence for this clause (1 through 7).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=7))]
    unmet: Option<u8>,
}
fn main() -> ExitCode {
    let args = Args::parse();
    if args.directory.exists() {
        eprintln!(
            "fixture destination already exists: {}",
            args.directory.display()
        );
        return ExitCode::from(2);
    }
    let corpus = fixtures::corpus();
    let case = if let Some(number) = args.unmet {
        &corpus["test_checker.TheShapeOfTheReport.test_each_clause_can_be_taken_the_other_way_on_its_own"]
            [usize::from(number) - 1]
    } else {
        &corpus["test_checker.TheShapeOfTheReport.test_a_swarm_that_meets_everything_meets_all_seven"]
            [0]
    };
    fixtures::materialize(&case["input"], &args.directory);
    println!("{}", args.directory.display());
    ExitCode::SUCCESS
}
