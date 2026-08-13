//! CLI argument definitions for the `ta` binary.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Default OpenAI model used when neither `--tokenizer` nor `--model` is given.
pub const DEFAULT_MODEL: &str = "gpt-4o";

#[derive(Parser, Debug)]
#[command(
    name = "ta",
    about = "Privacy-first LLM input firewall & cost actuary",
    version,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Count tokens for the given input.
    Count {
        /// Path to tokenizer.json (Hugging Face format). Overrides --model.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// OpenAI model name to use the embedded tiktoken backend.
        #[arg(short, long)]
        model: Option<String>,
        /// Add special tokens during encoding.
        #[arg(long)]
        special: bool,
        /// Output JSON instead of plain text.
        #[arg(long)]
        json: bool,
        /// Input text. If omitted, read from stdin.
        text: Option<String>,
    },
    /// Audit input: redact, count, truncate, detect jailbreak tokens.
    Audit {
        /// Path to tokenizer.json. Overrides --model.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// OpenAI model name to use the embedded tiktoken backend.
        #[arg(short, long)]
        model: Option<String>,
        /// Maximum tokens to keep.
        #[arg(short, long)]
        max_tokens: Option<usize>,
        /// Comma-separated sensitive patterns to redact.
        #[arg(long, value_delimiter = ',')]
        redact: Vec<String>,
        /// Comma-separated replacement strings for redacted patterns.
        #[arg(long, value_delimiter = ',')]
        replace: Vec<String>,
        /// Comma-prefixes of control tokens to flag.
        #[arg(long, value_delimiter = ',')]
        control: Vec<String>,
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        /// Input text. If omitted, read from stdin.
        text: Option<String>,
    },
    /// Encode text into token ids.
    Encode {
        /// Path to tokenizer.json. Overrides --model.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// OpenAI model name to use the embedded tiktoken backend.
        #[arg(short, long)]
        model: Option<String>,
        /// Add special tokens.
        #[arg(long)]
        special: bool,
        /// Input text. If omitted, read from stdin.
        text: Option<String>,
    },
    /// Decode token ids back to text.
    Decode {
        /// Path to tokenizer.json. Overrides --model.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// OpenAI model name to use the embedded tiktoken backend.
        #[arg(short, long)]
        model: Option<String>,
        /// Comma-separated token ids.
        #[arg(value_delimiter = ',')]
        ids: Vec<u32>,
    },
    /// Print a per-token heatmap for terminal debugging.
    Heatmap {
        /// Path to tokenizer.json. Overrides --model.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// OpenAI model name to use the embedded tiktoken backend.
        #[arg(short, long)]
        model: Option<String>,
        /// Add special tokens.
        #[arg(long)]
        special: bool,
        /// Input text. If omitted, read from stdin.
        text: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Tsv,
}

/// Source of the tokenizer selected by the user.
pub enum TokenizerSource {
    /// Hugging Face `tokenizer.json` path.
    File(PathBuf),
    /// OpenAI model name resolved by tiktoken-rs.
    #[allow(dead_code)]
    Model(String),
}

impl Command {
    /// Resolve tokenizer source from flags/environment.
    ///
    /// Precedence:
    /// 1. `--tokenizer` flag
    /// 2. `TOKENIZER_JSON` environment variable
    /// 3. `--model` flag
    /// 4. `TOKENIZER_MODEL` environment variable
    /// 5. Default model (`gpt-4o`) when the `tiktoken` feature is enabled
    pub fn tokenizer_source(&self) -> TokenizerSource {
        let (tokenizer, model) = match self {
            Command::Count {
                tokenizer, model, ..
            } => (tokenizer.clone(), model.clone()),
            Command::Audit {
                tokenizer, model, ..
            } => (tokenizer.clone(), model.clone()),
            Command::Encode {
                tokenizer, model, ..
            } => (tokenizer.clone(), model.clone()),
            Command::Decode {
                tokenizer, model, ..
            } => (tokenizer.clone(), model.clone()),
            Command::Heatmap {
                tokenizer, model, ..
            } => (tokenizer.clone(), model.clone()),
        };

        if let Some(path) = tokenizer {
            return TokenizerSource::File(path);
        }
        if let Ok(path) = std::env::var("TOKENIZER_JSON") {
            return TokenizerSource::File(PathBuf::from(path));
        }

        let model = model
            .or_else(|| std::env::var("TOKENIZER_MODEL").ok())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        TokenizerSource::Model(model)
    }
}
