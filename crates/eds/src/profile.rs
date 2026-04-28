//! `eds profile` subcommands — profile validation and listing.

use clap::Subcommand;
use std::path::PathBuf;

use edgesentry_profile::{load_profile, validate_profile};

#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    /// Validate a profile directory.
    Validate {
        /// Profile directory containing rules.json (and optionally kb/).
        #[arg(long)]
        profile: PathBuf,
    },

    /// List rule IDs defined in a profile.
    List {
        /// Profile directory containing rules.json.
        #[arg(long)]
        profile: PathBuf,
    },
}

pub fn run(cmd: ProfileCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ProfileCommand::Validate { profile } => run_validate(profile),
        ProfileCommand::List { profile } => run_list(profile),
    }
}

fn run_validate(profile: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let report = validate_profile(&profile);

    for w in &report.warnings {
        eprintln!("warning: {w}");
    }
    for e in &report.errors {
        eprintln!("error: {e}");
    }

    if report.is_valid() {
        if report.warnings.is_empty() {
            println!("Profile is valid.");
        } else {
            println!(
                "Profile is valid with {} warning(s).",
                report.warnings.len()
            );
        }
        Ok(())
    } else {
        Err(format!(
            "{} error(s), {} warning(s). Profile is invalid.",
            report.errors.len(),
            report.warnings.len()
        ).into())
    }
}

fn run_list(profile: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let rules = load_profile(&profile)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    for rule in &rules {
        println!("{}", rule.rule_id);
    }
    Ok(())
}
