//! token-actuary — Privacy-first LLM input firewall & cost actuary.
//!
//! Core library exposing token counting, local redaction, safe token-level
//! truncation, and structural jailbreak-token detection. Designed to run
//! natively and in WebAssembly.

use aho_corasick::AhoCorasick;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokenizers::Tokenizer;

/// A loaded tokenizer plus local audit configuration.
pub struct Actuary {
    tokenizer: Tokenizer,
    redactor: Option<AhoCorasick>,
    redaction_replacements: Vec<String>,
    jailbreak_tokens: HashSet<u32>,
    control_token_prefixes: Vec<String>,
}

/// Options controlling the `audit` pass.
#[derive(Debug, Clone, Default)]
pub struct AuditOptions {
    /// Maximum number of tokens to keep. `None` means no truncation.
    pub max_tokens: Option<usize>,
    /// Whether to add special tokens during encoding.
    pub add_special_tokens: bool,
    /// Whether to skip decoding the truncated token stream back to text.
    pub skip_decode: bool,
}

/// Result of an audit pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Total tokens before truncation.
    pub tokens_before: usize,
    /// Total tokens after truncation (if any).
    pub tokens_after: usize,
    /// Whether truncation occurred.
    pub truncated: bool,
    /// Number of redaction hits.
    pub redaction_hits: usize,
    /// Number of detected jailbreak/control tokens.
    pub jailbreak_hits: usize,
    /// Human-readable redacted text (after truncation if requested).
    pub text: String,
    /// Token ids after all processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_ids: Option<Vec<u32>>,
    /// Informational messages / warnings.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

impl Actuary {
    /// Load a tokenizer from a `tokenizer.json` file.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(path).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            tokenizer,
            redactor: None,
            redaction_replacements: Vec::new(),
            jailbreak_tokens: HashSet::new(),
            control_token_prefixes: Vec::new(),
        })
    }

    /// Load a tokenizer from in-memory `tokenizer.json` bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let tokenizer = Tokenizer::from_bytes(bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            tokenizer,
            redactor: None,
            redaction_replacements: Vec::new(),
            jailbreak_tokens: HashSet::new(),
            control_token_prefixes: Vec::new(),
        })
    }

    /// Configure local redaction patterns.
    ///
    /// `patterns[i]` is replaced by `replacements[i]`. The replacement happens
    /// before tokenization, so patterns are matched on raw text.
    pub fn with_redactions(mut self, patterns: &[&str], replacements: &[&str]) -> Result<Self> {
        anyhow::ensure!(
            patterns.len() == replacements.len(),
            "patterns and replacements must have the same length"
        );
        self.redactor = Some(
            AhoCorasick::new(patterns).context("failed to build redaction automaton")?,
        );
        self.redaction_replacements = replacements.iter().map(|s| (*s).to_string()).collect();
        Ok(self)
    }

    /// Configure tokens that should trigger a jailbreak warning.
    ///
    /// Any token whose decoded text starts with one of the prefixes is flagged.
    pub fn with_control_token_prefixes(mut self, prefixes: &[&str]) -> Self {
        self.control_token_prefixes = prefixes.iter().map(|s| (*s).to_string()).collect();
        self
    }

    /// Encode text into token ids.
    pub fn encode(&self, text: &str, add_special_tokens: bool) -> Result<Vec<u32>> {
        let encoding = self
            .tokenizer
            .encode(text, add_special_tokens)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(encoding.get_ids().to_vec())
    }

    /// Decode token ids back to text.
    pub fn decode(&self, ids: &[u32], skip_special_tokens: bool) -> Result<String> {
        self.tokenizer
            .decode(ids, skip_special_tokens)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Count tokens for a given text.
    pub fn count(&self, text: &str, add_special_tokens: bool) -> Result<usize> {
        Ok(self.encode(text, add_special_tokens)?.len())
    }

    /// Compute a simple character/token density score.
    ///
    /// Returns characters per token. Higher is better (fewer tokens per character).
    pub fn density(&self, text: &str, add_special_tokens: bool) -> Result<f64> {
        let chars = text.chars().count();
        let tokens = self.count(text, add_special_tokens)?;
        if tokens == 0 {
            return Ok(0.0);
        }
        Ok(chars as f64 / tokens as f64)
    }

    /// Audit input text: redact, encode, truncate, detect jailbreak tokens.
    pub fn audit(&self, text: &str, options: &AuditOptions) -> Result<AuditReport> {
        // 1. Local redaction on raw text.
        let (redacted, redaction_hits) = self.redact(text);

        // 2. Encode.
        let mut ids = self.encode(&redacted, options.add_special_tokens)?;
        let tokens_before = ids.len();

        // 3. Truncate at token boundary.
        let truncated = if let Some(max) = options.max_tokens {
            if ids.len() > max {
                ids.truncate(max);
                true
            } else {
                false
            }
        } else {
            false
        };
        let tokens_after = ids.len();

        // 4. Jailbreak / control-token scan.
        let jailbreak_hits = self.scan_jailbreak(&ids)?;

        // 5. Decode back unless skipped.
        let text_out = if options.skip_decode {
            String::new()
        } else {
            self.decode(&ids, options.add_special_tokens)?
        };

        let mut warnings = Vec::new();
        if truncated {
            warnings.push(format!(
                "input truncated from {} to {} tokens",
                tokens_before, tokens_after
            ));
        }
        if jailbreak_hits > 0 {
            warnings.push(format!(
                "detected {} jailbreak/control token(s)",
                jailbreak_hits
            ));
        }

        Ok(AuditReport {
            tokens_before,
            tokens_after,
            truncated,
            redaction_hits,
            jailbreak_hits,
            text: text_out,
            token_ids: Some(ids),
            warnings,
        })
    }

    /// Redact raw text using the configured patterns.
    fn redact(&self, text: &str) -> (String, usize) {
        let Some(ref redactor) = self.redactor else {
            return (text.to_string(), 0);
        };
        let mut hits = 0;
        let mut result = String::with_capacity(text.len());
        redactor.replace_all_with(text, &mut result, |m, _, dst| {
            hits += 1;
            let pid = m.pattern().as_usize();
            if pid < self.redaction_replacements.len() {
                dst.push_str(&self.redaction_replacements[pid]);
            }
            true
        });
        (result, hits)
    }

    /// Scan token ids for jailbreak / control tokens.
    fn scan_jailbreak(&self, ids: &[u32]) -> Result<usize> {
        let mut hits = 0;
        // Direct token-id matches.
        for id in ids {
            if self.jailbreak_tokens.contains(id) {
                hits += 1;
            }
        }
        // Prefix matches on decoded token text.
        if !self.control_token_prefixes.is_empty() {
            for id in ids {
                let text = self.decode(&[*id], true).unwrap_or_default();
                for prefix in &self.control_token_prefixes {
                    if text.starts_with(prefix) {
                        hits += 1;
                        break;
                    }
                }
            }
        }
        Ok(hits)
    }
}

/// Heatmap data: each token with its byte span and a heat value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatToken {
    pub token: String,
    pub start: usize,
    pub end: usize,
    /// 0 = cool (short/common), higher = hot (long/expensive).
    pub heat: u32,
}

/// Produce a simple per-token heatmap for terminal display.
pub fn heatmap(actuary: &Actuary, text: &str, add_special_tokens: bool) -> Result<Vec<HeatToken>> {
    let encoding = actuary
        .tokenizer
        .encode(text, add_special_tokens)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let tokens = encoding.get_tokens();
    let offsets = encoding.get_offsets();
    let mut out = Vec::with_capacity(tokens.len());
    for (i, (token, (start, end))) in tokens.iter().zip(offsets.iter()).enumerate() {
        // Simple heat heuristic: longer tokens and later tokens get warmer.
        let heat = (token.chars().count() as u32).saturating_add((i as u32).saturating_mul(2) / 10);
        out.push(HeatToken {
            token: token.to_string(),
            start: *start,
            end: *end,
            heat,
        });
    }
    Ok(out)
}

#[cfg(feature = "wasm")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub struct WasmActuary {
        inner: Actuary,
    }

    #[wasm_bindgen]
    impl WasmActuary {
        #[wasm_bindgen(constructor)]
        pub fn new(tokenizer_json: &[u8]) -> Result<WasmActuary, JsValue> {
            let inner = Actuary::from_bytes(tokenizer_json)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            Ok(WasmActuary { inner })
        }

        pub fn count(&self, text: &str, add_special_tokens: bool) -> Result<usize, JsValue> {
            self.inner
                .count(text, add_special_tokens)
                .map_err(|e| JsValue::from_str(&format!("{e}")))
        }

        pub fn encode(&self, text: &str, add_special_tokens: bool) -> Result<Vec<u32>, JsValue> {
            self.inner
                .encode(text, add_special_tokens)
                .map_err(|e| JsValue::from_str(&format!("{e}")))
        }

        pub fn decode(&self, ids: &[u32], skip_special_tokens: bool) -> Result<String, JsValue> {
            self.inner
                .decode(ids, skip_special_tokens)
                .map_err(|e| JsValue::from_str(&format!("{e}")))
        }

        pub fn audit(&self, text: &str, max_tokens: Option<usize>) -> Result<String, JsValue> {
            let opts = AuditOptions {
                max_tokens,
                add_special_tokens: false,
                skip_decode: false,
            };
            let report = self
                .inner
                .audit(text, &opts)
                .map_err(|e| JsValue::from_str(&format!("{e}")))?;
            serde_json::to_string(&report)
                .map_err(|e| JsValue::from_str(&format!("serialization failed: {e}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_actuary() -> Actuary {
        Actuary::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/minimal-tokenizer.json")).unwrap()
    }

    #[test]
    fn count_basic() {
        let actuary = test_actuary();
        // Whitespace pre-tokenizer splits "hello world" into 2 tokens.
        assert_eq!(actuary.count("hello world", false).unwrap(), 2);
    }

    #[test]
    fn redaction_counts_hits() {
        let actuary = test_actuary()
            .with_redactions(&["secret", "password"], &["[REDACTED]", "[SECRET_ID_1]"])
            .unwrap();

        let report = actuary
            .audit(
                "my secret password here",
                &AuditOptions {
                    max_tokens: None,
                    add_special_tokens: false,
                    skip_decode: false,
                },
            )
            .unwrap();
        assert_eq!(report.redaction_hits, 2);
        assert!(!report.text.contains("secret"));
        assert!(!report.text.contains("password"));
    }

    #[test]
    fn truncation_preserves_decoding() {
        let actuary = test_actuary();
        let report = actuary
            .audit(
                "one two three four five",
                &AuditOptions {
                    max_tokens: Some(3),
                    add_special_tokens: false,
                    skip_decode: false,
                },
            )
            .unwrap();
        assert!(report.truncated);
        assert_eq!(report.tokens_after, 3);
    }
}
