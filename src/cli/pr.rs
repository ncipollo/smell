//! View layer for the `pr` subcommand: resolves a PR reference to a commit
//! pair and forwards straight to the `diff` subcommand's CLI glue — no
//! rendering logic of its own.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::cli::diff;
use crate::feature::diff::delta::DeltaOptions;
use crate::feature::pr;

pub struct PrRunArgs {
    pub repo: PathBuf,
    pub reference: String,
    pub paths: Vec<PathBuf>,
    pub delta: DeltaOptions,
    pub json: bool,
}

pub fn run(args: PrRunArgs) -> ExitCode {
    let (old_ref, new_ref) = match pr::resolve(&args.repo, &args.reference) {
        Ok(refs) => refs,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    diff::run(diff::DiffRunArgs {
        repo: args.repo,
        old_ref,
        new_ref,
        paths: args.paths,
        delta: args.delta,
        json: args.json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_forwards_an_explicit_range_to_the_diff_engine() {
        let args = PrRunArgs {
            repo: PathBuf::from("."),
            reference: "HEAD~1..HEAD".to_string(),
            paths: vec![PathBuf::from("does-not-exist.rs")],
            delta: DeltaOptions::default(),
            json: false,
        };
        // Exercises the same code path as `diff::run` directly; a missing
        // path is a soft per-path error there, not a crash here.
        let _ = run(args);
    }

    #[test]
    fn run_reports_an_error_for_an_unparseable_reference() {
        let args = PrRunArgs {
            repo: PathBuf::from("."),
            reference: "not a reference".to_string(),
            paths: vec![PathBuf::from("src")],
            delta: DeltaOptions::default(),
            json: false,
        };
        assert_eq!(run(args), ExitCode::FAILURE);
    }
}
