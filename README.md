# token-actuary

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> Privacy-First LLM Input Firewall & Cost Actuary.  
> Local token counting, redaction, truncation, and jailbreak-token detection — no network, no data leakage.

`token-actuary` (binary: `ta`) is a tiny, self-contained Rust tool that audits LLM prompts before they leave the machine. It loads a Hugging Face `tokenizer.json`, counts tokens, redacts sensitive patterns, safely truncates at token boundaries, and flags control tokens that may indicate prompt injection or jailbreak attempts.

## Why

Existing cloud-based token gateways force you to ship prompts to a third party. `token-actuary` keeps everything local:

- **Zero network**: tokenizer is loaded from a local file or fetched once by your frontend; no prompt text ever leaves the process.
- **Token-level safety**: truncation happens on `Vec<u32>`, never by slicing raw strings, so multi-byte characters and code blocks stay intact.
- **Agent-friendly**: plain stdout, structured JSON/TSV options, no TUI, no progress bars.
- **WASM-ready**: the same core compiles to WebAssembly for browser-side audit sandboxes.

## Install

### Cargo

```bash
cargo install token-actuary
```

### Build from source

```bash
git clone https://github.com/ljh-sh/token-actuary
cd token-actuary
cargo build --release   # binary at target/release/ta
```

## Usage

All commands need a `tokenizer.json`. Either pass `--tokenizer` or set `TOKENIZER_JSON`:

```bash
export TOKENIZER_JSON=/path/to/tokenizer.json
```

### Count tokens

```bash
echo "hello world" | ta count
# 2
```

### Audit (redact + truncate + detect)

```bash
echo "my secret password is here" | ta audit --redact secret,password --replace [REDACTED],[SECRET] --max-tokens 10
```

Output:

```text
tokens_before: 6
tokens_after:  6
truncated:     false
redactions:    2
jailbreak:     0
---
my [REDACTED] [SECRET] is here
```

JSON mode for agents:

```bash
cat prompt.txt | ta audit --max-tokens 2048 --format json
```

### Encode / decode

```bash
echo "hello world" | ta encode
# 15496,995

ta decode 15496,995
# hello world
```

### Heatmap

```bash
echo "hello world" | ta heatmap
```

## Library

```rust
use token_actuary::{Actuary, AuditOptions};

let actuary = Actuary::from_file("tokenizer.json")?
    .with_redactions(&["secret", "password"], &["[REDACTED]", "[SECRET_ID_1]"])?
    .with_control_token_prefixes(&["<|im_start|>", "<|endoftext|>"]);

let report = actuary.audit("my secret is safe", &AuditOptions::default())?;
println!("{} tokens", report.tokens_after);
```

## WebAssembly

Build with `wasm-pack`:

```bash
wasm-pack build --target web --features wasm
```

The `WasmActuary` class exposes `count`, `encode`, `decode`, and `audit` to JavaScript.

## Model support

`token-actuary` uses the Hugging Face `tokenizers` Rust library. It supports any model shipped with a `tokenizer.json` (Qwen2.5, Llama3, DeepSeek, etc.). Large tokenizer files can be Brotli/Gzip compressed for web delivery and decompressed in the browser before instantiation.

GGUF support is currently not bundled; we are evaluating whether the existing `tokenizers` BPE/WordPiece/Unigram implementation can cover GGUF vocabulary loading without adding a dedicated dependency.

## Security

See [SECURITY.md](SECURITY.md). For vulnerabilities, email [lijunhao@x-cmd.com](mailto:lijunhao@x-cmd.com).

## License

Apache 2.0 — see [LICENSE](LICENSE).
