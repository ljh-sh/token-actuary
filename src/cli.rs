//! CLI argument definitions for the `ta` binary.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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
        /// Path to tokenizer.json. Defaults to TOKENIZER_JSON env var.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
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
        /// Path to tokenizer.json.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
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
        /// Path to tokenizer.json.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// Add special tokens.
        #[arg(long)]
        special: bool,
        /// Input text. If omitted, read from stdin.
        text: Option<String>,
    },
    /// Decode token ids back to text.
    Decode {
        /// Path to tokenizer.json.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
        /// Comma-separated token ids.
        #[arg(value_delimiter = ',')]
        ids: Vec<u32>,
    },
    /// Print a per-token heatmap for terminal debugging.
    Heatmap {
        /// Path to tokenizer.json.
        #[arg(short, long)]
        tokenizer: Option<PathBuf>,
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

impl Command {
    /// Resolve tokenizer path from flag or environment.
    pub fn tokenizer_path(&self) -> Option<PathBuf> {
        let opt = match self {
            Command::Count { tokenizer, .. } => tokenizer.clone(),
            Command::Audit { tokenizer, .. } => tokenizer.clone(),
            Command::Encode { tokenizer, .. } => tokenizer.clone(),
            Command::Decode { tokenizer, .. } => tokenizer.clone(),
            Command::Heatmap { tokenizer, .. } => tokenizer.clone(),
        };
        opt.or_else(|| std::env::var("TOKENIZER_JSON").ok().map(PathBuf::from))
    }
}
