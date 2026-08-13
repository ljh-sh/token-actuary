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
    /// Download tokenizer.json files from ljh-sh/tokenizer-json.
    #[cfg(feature = "download")]
    Download {
        /// Download the recommended set of open-source tokenizers.
        #[arg(long, default_value_t = true)]
        recommend: bool,
        /// Specific tokenizer IDs to download (e.g. qwen2_5, llama3).
        #[arg(value_name = "ID")]
        ids: Vec<String>,
        /// Force re-download even if the file already exists.
        #[arg(short, long)]
        force: bool,
    },
    /// Compare token counts across multiple tokenizers.
    #[cfg(feature = "download")]
    Compare {
        /// Use the recommended tokenizer set (default).
        #[arg(long, default_value_t = true)]
        recommend: bool,
        /// Read input from stdin instead of files.
        #[arg(long)]
        stdin: bool,
        /// Additional OpenAI model(s) to include.
        #[arg(short, long, value_delimiter = ',')]
        model: Vec<String>,
        /// Additional tokenizer.json path(s) to include.
        #[arg(short, long, value_delimiter = ',')]
        tokenizer: Vec<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value = "tsv")]
        format: OutputFormat,
        /// Input text (single inline argument).
        #[arg(long)]
        text: Option<String>,
        /// Input files. If omitted, read from stdin.
        files: Vec<PathBuf>,
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
            #[cfg(feature = "download")]
            Command::Download { .. } | Command::Compare { .. } => {
                unreachable!("tokenizer_source called on non-tokenization command")
            }
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
