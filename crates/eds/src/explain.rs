//! `eds explain` subcommands — LLM-powered explanation of RiskEvents.

use clap::{Subcommand, ValueEnum};
use std::fs;
use std::path::PathBuf;

use edgesentry_evaluate::RiskEvent;
use edgesentry_explain::{pick_events, Explainer, KnowledgeBase, LlmClient, PickStrategy};
use edgesentry_ingest::jsonl::{JsonlReader, JsonlWriter};

#[derive(Debug, Clone, ValueEnum)]
pub enum PickStrategyArg {
    Severity,
    Time,
    Random,
}

impl From<PickStrategyArg> for PickStrategy {
    fn from(a: PickStrategyArg) -> Self {
        match a {
            PickStrategyArg::Severity => PickStrategy::Severity,
            PickStrategyArg::Time => PickStrategy::Time,
            PickStrategyArg::Random => PickStrategy::Random,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ExplainCommand {
    /// Generate LLM-powered plain-language explanations for RiskEvents.
    Run {
        /// Input RiskEvent JSONL file.
        #[arg(long)]
        input: PathBuf,

        /// Number of events to explain.
        #[arg(long, default_value = "5")]
        n: usize,

        /// Event selection strategy.
        #[arg(long, value_enum, default_value = "severity")]
        pick: PickStrategyArg,

        /// LLM server base URL (OpenAI-compatible).
        #[arg(long, default_value = "http://localhost:8080")]
        llm_url: String,

        /// Model name (auto-discovered if omitted).
        #[arg(long)]
        model: Option<String>,

        /// Output Explanation JSONL file.
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run(cmd: ExplainCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ExplainCommand::Run { input, n, pick, llm_url, model, out } => {
            run_explain(&input, n, pick.into(), &llm_url, model.as_deref(), &out)
        }
    }
}

fn run_explain(
    input: &PathBuf,
    n: usize,
    strategy: PickStrategy,
    llm_url: &str,
    model: Option<&str>,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(input)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let events: Vec<RiskEvent> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let picked = pick_events(&events, n, strategy);

    let kb = KnowledgeBase::from_map(std::collections::HashMap::new());
    let llm = match model {
        Some(m) => LlmClient::new(llm_url, m),
        None => LlmClient::new_autodiscover(llm_url),
    };
    let explainer = Explainer::new(kb, llm);

    let out_file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.explanation", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut count = 0usize;
    for event in picked {
        match explainer.explain(event) {
            Ok(explanation) => {
                writer.write_record(&explanation)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                count += 1;
            }
            Err(e) => {
                eprintln!("explain: warning — failed to explain event {}: {e}", event.rule_id);
            }
        }
    }

    eprintln!("explain run: {} explanation(s) written to {}", count, out.display());
    Ok(())
}
