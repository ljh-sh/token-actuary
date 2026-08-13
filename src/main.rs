//! Native CLI entry point for token-actuary (`ta`).

use anyhow::{Context, Result};
use clap::Parser;
use std::io::{self, Read};
use token_actuary::{Actuary, AuditOptions};

mod cli;
#[cfg(feature = "download")]
mod compare;
#[cfg(feature = "download")]
mod download;

use cli::{Cli, Command, OutputFormat, TokenizerSource};

fn main() -> Result<()> {
    let cli = Cli::parse();

    #[cfg(feature = "download")]
    match cli.cmd {
        Command::Download {
            recommend,
            ids,
            force,
        } => {
            println!("downloading to {}", download::data_dir().display());
            let results = if ids.is_empty() && recommend {
                download::download_recommended(force)?
            } else if ids.is_empty() {
                anyhow::bail!("no tokenizer IDs specified; use --recommend or pass IDs");
            } else {
                download::download_ids(&ids, force)?
            };
            for r in results {
                println!("{}\t{}\t{}\t{}", r.id, r.method, r.bytes, r.path.display());
            }
            return Ok(());
        }

        Command::Compare {
            recommend,
            stdin,
            model,
            tokenizer,
            format,
            text,
            files,
        } => {
            let tokenizers = if recommend && model.is_empty() && tokenizer.is_empty() {
                compare::recommended_tokenizers()
            } else {
                let mut t = compare::tokenizers_from_flags(&model, &tokenizer);
                if recommend {
                    t.extend(compare::recommended_tokenizers());
                }
                t
            };

            if tokenizers.is_empty() {
                anyhow::bail!("no tokenizers to compare; use --recommend or pass --model/--tokenizer");
            }

            let inputs = compare::collect_inputs(stdin, &files, text)?;
            let rows = compare::compare(&tokenizers, &inputs)?;

            match format {
                OutputFormat::Tsv | OutputFormat::Json => {
                    print!("{}", compare::render_tsv(&rows));
                }
                OutputFormat::Text => {
                    print!("{}", compare::render_table(&rows));
                }
            }
            return Ok(());
        }

        _ => {}
    }

    let source = cli.cmd.tokenizer_source();
    let actuary = match source {
        TokenizerSource::File(path) => Actuary::from_file(&path)?,
        #[cfg(feature = "tiktoken")]
        TokenizerSource::Model(model) => Actuary::from_model(&model)?,
        #[cfg(not(feature = "tiktoken"))]
        TokenizerSource::Model(_) => {
            anyhow::bail!("--model requires the `tiktoken` feature (enabled by default on native builds)")
        }
    };

    match cli.cmd {
        Command::Count {
            special,
            json,
            text,
            ..
        } => {
            let input = read_input(text)?;
            let count = actuary.count(&input, special)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "tokens": count,
                        "characters": input.chars().count(),
                        "density": actuary.density(&input, special)?,
                    })
                );
            } else {
                println!("{}", count);
            }
        }
        Command::Audit {
            max_tokens,
            redact,
            replace,
            control,
            format,
            text,
            ..
        } => {
            let input = read_input(text)?;
            let replacements: Vec<&str> = replace.iter().map(|s| s.as_str()).collect();
            let patterns: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
            let actuary = if patterns.is_empty() {
                actuary
            } else {
                actuary.with_redactions(&patterns, &replacements)?
            };
            let control_ref: Vec<&str> = control.iter().map(|s| s.as_str()).collect();
            let actuary = actuary.with_control_token_prefixes(&control_ref);

            let opts = AuditOptions {
                max_tokens,
                add_special_tokens: false,
                skip_decode: false,
            };
            let report = actuary.audit(&input, &opts)?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
                OutputFormat::Tsv => {
                    println!(
                        "tokens_before\ttokens_after\ttruncated\tredaction_hits\tjailbreak_hits"
                    );
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        report.tokens_before,
                        report.tokens_after,
                        report.truncated,
                        report.redaction_hits,
                        report.jailbreak_hits
                    );
                }
                OutputFormat::Text => {
                    println!("tokens_before: {}", report.tokens_before);
                    println!("tokens_after:  {}", report.tokens_after);
                    println!("truncated:     {}", report.truncated);
                    println!("redactions:    {}", report.redaction_hits);
                    println!("jailbreak:     {}", report.jailbreak_hits);
                    for w in &report.warnings {
                        eprintln!("warning: {}", w);
                    }
                    println!("---");
                    println!("{}", report.text);
                }
            }
        }
        Command::Encode { special, text, .. } => {
            let input = read_input(text)?;
            let ids = actuary.encode(&input, special)?;
            println!("{}", ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","));
        }
        Command::Decode { ids, .. } => {
            let text = actuary.decode(&ids, true)?;
            println!("{}", text);
        }
        Command::Heatmap { special, text, .. } => {
            let input = read_input(text)?;
            let heat = token_actuary::heatmap(&actuary, &input, special)?;
            for t in heat {
                println!("{}\t{}\t{}", t.start, t.end, t.token);
            }
        }
        #[cfg(feature = "download")]
        Command::Download { .. } | Command::Compare { .. } => {
            unreachable!("handled above")
        }
    }

    Ok(())
}

/// Read input from argument, stdin, or fail.
fn read_input(text: Option<String>) -> Result<String> {
    if let Some(t) = text {
        return Ok(t);
    }
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .context("failed to read stdin")?;
    Ok(buf)
}
