use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};

use edgesentry_document::{
    check, fill, fill_clearance, parse_clearance_facts_json, render_html, ComplianceAlert,
    FilledDocument, PORT_CYBER_CLEARANCE_HTML,
};
use edgesentry_ingest::jsonl::{JsonlReader, JsonlWriter};
use edgesentry_parse::DocumentEntity;

const FAL_FORM_1: &str = include_str!("../../edgesentry-document/templates/fal-form-1.html");
const FAL_FORM_5: &str = include_str!("../../edgesentry-document/templates/fal-form-5.html");
const SG_PORT_ENTRY: &str = include_str!("../../edgesentry-document/templates/sg-port-entry.html");

#[derive(Debug, Subcommand)]
pub enum DocumentCommand {
    /// Fill a document template from DocumentEntity JSONL.
    Fill {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        template: String,
        #[arg(long)]
        llm_url: Option<String>,
        #[arg(long, default_value = "0.5")]
        confidence_threshold: f64,
        #[arg(long)]
        out: PathBuf,
    },
    /// Check filled documents against compliance rules.
    Check {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render a filled document as HTML.
    Gen {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        template: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render port cyber clearance HTML from indago `*_facts.json` (Cap Vista W5).
    RenderClearance(RenderClearanceArgs),
}

#[derive(Debug, clap::Args)]
pub struct RenderClearanceArgs {
    #[arg(long)]
    pub facts: PathBuf,
    #[arg(long, default_value = "https://verify.edgesentry.io/clearance/poc")]
    pub verify_url: String,
    #[arg(long)]
    pub operator_explanation: Option<PathBuf>,
    #[arg(long)]
    pub out: PathBuf,
}

pub fn run(cmd: DocumentCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        DocumentCommand::Fill { input, template, llm_url, confidence_threshold, out } => {
            run_fill(&input, &template, llm_url.as_deref(), confidence_threshold, &out)
        }
        DocumentCommand::Check { input, profile, out } => {
            run_check(&input, &profile, &out)
        }
        DocumentCommand::Gen { input, template, out } => {
            run_gen(&input, &template, &out)
        }
        DocumentCommand::RenderClearance(args) => run_render_clearance(
            &args.facts,
            &args.verify_url,
            args.operator_explanation.as_ref(),
            &args.out,
        ),
    }
}

fn read_entities(path: &PathBuf) -> Result<Vec<DocumentEntity>, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let entities: Vec<DocumentEntity> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(entities)
}

fn read_filled_docs(path: &PathBuf) -> Result<Vec<FilledDocument>, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let docs: Vec<FilledDocument> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(docs)
}

fn run_fill(
    input: &PathBuf,
    template: &str,
    llm_url: Option<&str>,
    confidence_threshold: f64,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let entities = read_entities(input)?;

    let out_file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.filled-document", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut count = 0usize;
    for entity in &entities {
        let doc = fill(entity, template, llm_url, confidence_threshold)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        writer.write_record(&doc)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        count += 1;
    }

    eprintln!("document fill: {} document(s) written to {}", count, out.display());
    Ok(())
}

fn run_check(
    input: &PathBuf,
    profile: &Path,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let docs = read_filled_docs(input)?;

    let rules_path = profile.join("rules.json");
    let rules_json = fs::read_to_string(&rules_path)?;

    let out_file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.compliance-alert", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut total_alerts = 0usize;
    for doc in &docs {
        let alerts: Vec<ComplianceAlert> = check(doc, &rules_json)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        for alert in &alerts {
            writer.write_record(alert)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            total_alerts += 1;
        }
    }

    eprintln!("document check: {} alert(s) written to {}", total_alerts, out.display());
    Ok(())
}

fn run_gen(
    input: &PathBuf,
    template: &str,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let docs = read_filled_docs(input)?;
    let doc = docs
        .into_iter()
        .next()
        .ok_or_else(|| -> Box<dyn std::error::Error> { "input JSONL has no records".into() })?;

    let template_html = match template {
        "fal-form-1" => FAL_FORM_1,
        "fal-form-5" => FAL_FORM_5,
        "sg-port-entry" => SG_PORT_ENTRY,
        "port-cyber-clearance" => PORT_CYBER_CLEARANCE_HTML,
        other => {
            return Err(format!(
                "unknown template '{}'; choices: fal-form-1, fal-form-5, sg-port-entry, port-cyber-clearance",
                other
            )
            .into());
        }
    };

    let rendered = render_html(&doc, template_html);
    fs::write(out, rendered)?;
    eprintln!("document gen: rendered '{}' → {}", template, out.display());
    Ok(())
}

fn run_render_clearance(
    facts_path: &PathBuf,
    verify_url: &str,
    operator_explanation_path: Option<&PathBuf>,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(facts_path)?;
    let facts = parse_clearance_facts_json(&content)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let operator_explanation = operator_explanation_path
        .map(|p| fs::read_to_string(p))
        .transpose()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let doc = fill_clearance(
        &facts,
        verify_url,
        operator_explanation.as_deref(),
    );
    let rendered = render_html(&doc, PORT_CYBER_CLEARANCE_HTML);
    fs::write(out, rendered)?;
    eprintln!(
        "document render-clearance: {} → {} ({})",
        facts_path.display(),
        out.display(),
        doc.fields.get("OUTCOME").map(|f| f.value.as_str()).unwrap_or("?")
    );
    Ok(())
}
