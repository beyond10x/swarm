use clap::Parser;
use std::{path::PathBuf, process::ExitCode};
#[derive(Parser)]
#[command(
    about = "Report the seven acceptance clauses of story:a-recorded-run-shows-two-agents-working against a finished swarm's records."
)]
struct Args {
    /// A path to a swarm's directory, or its slug.
    swarm: PathBuf,
    /// Where swarms live, when the first argument is a slug.
    #[arg(long, default_value = "data/swarms")]
    data: PathBuf,
    /// The report as JSON.
    #[arg(long)]
    json: bool,
}
fn main() -> ExitCode {
    let args = Args::parse();
    let mut root = args.swarm;
    if !root.is_dir() {
        let by_slug = args.data.join(&root);
        if by_slug.is_dir() {
            root = by_slug;
        }
    }
    let report = swarm_check::check(&root);
    println!(
        "{}",
        if args.json {
            serde_json::to_string_pretty(&report).expect("serializable report")
        } else {
            report.render()
        }
    );
    ExitCode::from(u8::from(!report.ok))
}
