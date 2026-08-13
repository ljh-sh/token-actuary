//! Compare token counts across multiple tokenizers.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use token_actuary::Actuary;

use crate::download::{data_dir, recommended_ids};

/// A tokenizer source for comparison.
pub enum CompareTokenizer {
    /// Embedded OpenAI model.
    Model(String),
    /// Local Hugging Face tokenizer.json.
    File(PathBuf),
}

impl CompareTokenizer {
    fn load(&self) -> anyhow::Result<Actuary> {
        match self {
            CompareTokenizer::Model(name) => Actuary::from_model(name),
            CompareTokenizer::File(path) => Actuary::from_file(path),
        }
    }

    fn label(&self) -> String {
        match self {
            CompareTokenizer::Model(name) => name.clone(),
            CompareTokenizer::File(path) => {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("custom")
                    .to_string()
            }
        }
    }
}

/// Resolve the default set of tokenizers for `ta compare --recommend`.
pub fn recommended_tokenizers() -> Vec<CompareTokenizer> {
    let mut out = vec![CompareTokenizer::Model("gpt-4o".to_string())];

    let dir = data_dir();
    for id in recommended_ids() {
        let path = dir.join(format!("{}.tokenizer.json", id));
        if path.exists() {
            out.push(CompareTokenizer::File(path));
        }
    }

    out
}

/// Build the tokenizer list from explicit `--model` / `--tokenizer` flags.
pub fn tokenizers_from_flags(
    models: &[String],
    tokenizers: &[PathBuf],
) -> Vec<CompareTokenizer> {
    let mut out = Vec::new();
    for m in models {
        out.push(CompareTokenizer::Model(m.clone()));
    }
    for t in tokenizers {
        out.push(CompareTokenizer::File(t.clone()));
    }
    out
}

/// Read input from stdin, a list of files, or a single inline text argument.
pub fn collect_inputs(
    stdin: bool,
    files: &[PathBuf],
    text: Option<String>,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut out = Vec::new();

    if stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        out.push(("stdin".to_string(), buf));
        return Ok(out);
    }

    if let Some(t) = text {
        out.push(("text".to_string(), t));
        return Ok(out);
    }

    if files.is_empty() {
        // Default to stdin when nothing else is given.
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        out.push(("stdin".to_string(), buf));
        return Ok(out);
    }

    for file in files {
        let label = file.to_string_lossy().to_string();
        let content = fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", file.display(), e))?;
        out.push((label, content));
    }

    Ok(out)
}

/// Compare token counts across tokenizers and return TSV rows.
pub fn compare(
    tokenizers: &[CompareTokenizer],
    inputs: &[(String, String)],
) -> anyhow::Result<Vec<CompareRow>> {
    // Load all tokenizers once.
    let loaded: Vec<(String, Actuary)> = tokenizers
        .iter()
        .map(|t| {
            let label = t.label();
            let actuary = t.load()?;
            Ok((label, actuary))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut rows = Vec::new();
    for (input_label, text) in inputs {
        for (model_label, actuary) in &loaded {
            let count = actuary.count(text, false).map_err(|e| {
                anyhow::anyhow!(
                    "failed to count {} with {}: {}",
                    input_label,
                    model_label,
                    e
                )
            })?;
            rows.push(CompareRow {
                input: input_label.clone(),
                model: model_label.clone(),
                tokens: count,
            });
        }
    }

    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct CompareRow {
    pub input: String,
    pub model: String,
    pub tokens: usize,
}

/// Render rows as TSV.
pub fn render_tsv(rows: &[CompareRow]) -> String {
    let mut out = "input\tmodel\ttokens\n".to_string();
    for row in rows {
        out.push_str(&format!("{}\t{}\t{}\n", row.input, row.model, row.tokens));
    }
    out
}

/// Render rows as a human-readable table with one row per input and one
/// column per model.
pub fn render_table(rows: &[CompareRow]) -> String {
    let mut inputs: Vec<String> = Vec::new();
    let mut models: Vec<String> = Vec::new();
    let mut values: HashMap<(String, String), usize> = HashMap::new();

    for row in rows {
        if !inputs.contains(&row.input) {
            inputs.push(row.input.clone());
        }
        if !models.contains(&row.model) {
            models.push(row.model.clone());
        }
        values.insert((row.input.clone(), row.model.clone()), row.tokens);
    }

    let mut out = String::new();
    out.push_str("input");
    for m in &models {
        out.push('\t');
        out.push_str(m);
    }
    out.push('\n');

    for input in &inputs {
        out.push_str(input);
        for m in &models {
            out.push('\t');
            out.push_str(&values.get(&(input.clone(), m.clone())).unwrap_or(&0).to_string());
        }
        out.push('\n');
    }

    out
}
