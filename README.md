# token-actuary

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> Privacy-First LLM Input Firewall & Cost Actuary.  
> Local token counting, redaction, truncation, and jailbreak-token detection — no network, no data leakage.

`token-actuary` (binary: `token-actuary`) is a tiny, self-contained Rust tool that audits LLM prompts before they leave the machine. It counts tokens, redacts sensitive patterns, safely truncates at token boundaries, and flags control tokens that may indicate prompt injection or jailbreak attempts.

The native binary ships with an embedded OpenAI `tiktoken` backend, so it works out of the box for GPT-4o / GPT-4 / GPT-3.5-style models without downloading tokenizer files. For open-source models (Qwen2.5, Llama3, DeepSeek, etc.), pass a Hugging Face `tokenizer.json`.

## Why

Existing cloud-based token gateways force you to ship prompts to a third party. `token-actuary` keeps everything local:

- **Zero network**: native builds embed OpenAI tokenizers; no prompt text ever leaves the process.
- **Token-level safety**: truncation happens on `Vec<u32>`, never by slicing raw strings, so multi-byte characters and code blocks stay intact.
- **Agent-friendly**: plain stdout, structured JSON/TSV options, no TUI, no progress bars.
- **WASM-ready**: the same core compiles to WebAssembly for browser-side audit sandboxes (uses HF `tokenizer.json` bytes).

## Install

### Prebuilt binaries

Download the latest release for your platform:

```bash
# macOS (Apple Silicon + Intel universal)
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/token-actuary-darwin-universal.tar.xz | tar xJ

# Linux (x86_64)
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/token-actuary-linux-x64.tar.xz | tar xJ

# Windows (x86_64 / arm64)
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/token-actuary-windows-x64.tar.xz | tar xJ
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/token-actuary-windows-arm64.tar.xz | tar xJ
# token-actuary.exe is now available
```

All archives are compressed with `xz`.

> Note: releases before the rename used `ta-...` asset names. Current and future releases use `token-actuary-...`.

### Cargo

```bash
cargo install token-actuary
```

### Build from source

```bash
git clone https://github.com/ljh-sh/token-actuary
cd token-actuary
cargo build --release   # binary at target/release/token-actuary
```

To build a smaller binary without the embedded OpenAI tokenizers:

```bash
cargo build --release --no-default-features
```

## Usage

Native builds default to the embedded `gpt-4o` tokenizer, so the simplest usage needs no extra files:

```bash
echo "hello world" | token-actuary count
# 2
```

You can also select a specific OpenAI model:

```bash
token-actuary count --model gpt-4
```

Or load a Hugging Face `tokenizer.json`:

```bash
token-actuary count --tokenizer /path/to/tokenizer.json
export TOKENIZER_JSON=/path/to/tokenizer.json
token-actuary count
```

Precedence:

1. `--tokenizer` flag
2. `TOKENIZER_JSON` environment variable
3. `--model` flag
4. `TOKENIZER_MODEL` environment variable
5. default model `gpt-4o`

### TL;DR

```bash
# Count tokens (uses embedded gpt-4o by default)
echo "hello world" | token-actuary count

# Count with an open-source tokenizer
echo "你好世界" | token-actuary count --tokenizer qwen2_5.tokenizer.json

# Audit: redact secrets, truncate to budget, output JSON for agents
cat prompt.txt | token-actuary audit \
  --redact password,secret,token \
  --replace [REDACTED],[REDACTED],[REDACTED] \
  --max-tokens 4096 --format json

# Compare token counts across models
echo "hello world" | token-actuary compare

# Download recommended open-source tokenizers
token-actuary download --recommend

# Download from mainland China (mirrors first)
TA_CHINA=1 token-actuary download --recommend

# Encode / decode (default separator is `,`)
echo "hello world" | token-actuary encode
token-actuary decode 24912,2375

# Roundtrip: encode then decode
echo "hello world" | token-actuary encode | token-actuary decode

# Use a custom separator
echo "hello world" | token-actuary encode -s " | " | token-actuary decode -s " | "

# Print per-token heatmap
echo "hello world" | token-actuary heatmap
```

### Count tokens

```bash
echo "hello world" | token-actuary count
# 2
```

### Audit (redact + truncate + detect)

```bash
echo "my secret password is here" | token-actuary audit --redact secret,password --replace [REDACTED],[SECRET] --max-tokens 10
```

Output:

```text
tokens_before: 13
tokens_after:  10
truncated:     true
redactions:    2
jailbreak:     0
---
my [REDACTED] [SECRET]
warning: input truncated from 13 to 10 tokens
```

JSON mode for agents:

```bash
cat prompt.txt | token-actuary audit --max-tokens 2048 --format json
```

### Encode / decode

```bash
echo "hello world" | token-actuary encode
# 24912,2375,198

token-actuary decode 24912,2375,198
# hello world
```

### Heatmap

```bash
echo "hello world" | token-actuary heatmap
```

### Download open-source tokenizers

`token-actuary` can download Hugging Face-format tokenizers from the companion repo [`ljh-sh/tokenizer-json`](https://github.com/ljh-sh/tokenizer-json). Downloads are opt-in and cached locally under `~/.local/data/tokenizer-json/`.

```bash
# Download the recommended set (qwen2_5, llama3, deepseek_v3)
token-actuary download

# Download specific IDs
token-actuary download claude qwen2_5

# Force re-download
token-actuary download --recommend --force
```

Download strategy:

1. Try `github.com` directly with stall detection.
2. Fall back to the eget hosted mirror (`https://eget.ljh.sh/gh/...`).
3. If `GHPROXY_ENDPOINT` is set, try that mirror too.

For networks where GitHub is slow or unstable (common in mainland China), use `--china` or set `TA_CHINA=1`. This flips the order to try China-accessible mirrors first:

```bash
TA_CHINA=1 token-actuary download
# or
export TA_CHINA=1
token-actuary download

# explicit flag
token-actuary download --china
```

`token-actuary` does not depend on x-cmd or the `eget` CLI; the fallback uses the same mirror endpoints that `x-bash/eget` uses.

### Compare token counts across models

```bash
# Compare stdin across recommended models
echo "hello world" | token-actuary compare

# Compare a file
token-actuary compare prompt.txt

# Compare inline text
token-actuary compare --text "hello world"

# Include extra models or local tokenizers
token-actuary compare --model gpt-4 --tokenizer /path/to/custom.tokenizer.json prompt.txt

# Table output
token-actuary compare --format text prompt.txt
```

Default TSV output:

```tsv
input	model	tokens
stdin	gpt-4o	3
stdin	qwen2_5.tokenizer	3
stdin	llama3.tokenizer	3
stdin	deepseek_v3.tokenizer	3
```

## Library

```rust
use token_actuary::{Actuary, AuditOptions};

// Embedded OpenAI tokenizer — no external file needed.
let actuary = Actuary::from_model("gpt-4o")?
    .with_redactions(&["secret", "password"], &["[REDACTED]", "[SECRET_ID_1]"])?
    .with_control_token_prefixes(&["<|im_start|>", "<|endoftext|>"]);

let report = actuary.audit("my secret is safe", &AuditOptions::default())?;
println!("{} tokens", report.tokens_after);
```

For open-source models, use `Actuary::from_file("tokenizer.json")` or `Actuary::from_bytes(&bytes)`.

## WebAssembly

Build with `wasm-pack`. The `wasm` feature disables the embedded OpenAI backend because `tiktoken-rs` currently relies on the `regex` crate, which does not target `wasm32-unknown-unknown`.

```bash
wasm-pack build --target web --no-default-features --features wasm
```

The `WasmActuary` class exposes `count`, `encode`, `decode`, and `audit` to JavaScript. It is constructed from `tokenizer.json` bytes:

```js
const actuary = new WasmActuary(tokenizerJsonBytes);
const report = JSON.parse(actuary.audit(text, 512));
```

## Model support

| Backend | Source | Models |
|---|---|---|
| `tiktoken-rs` | Embedded BPE tables | OpenAI: `gpt-4o`, `gpt-4`, `gpt-3.5-turbo`, `o1`, `o3`, embeddings, etc. |
| Hugging Face `tokenizers` | External `tokenizer.json` | Qwen2.5, Llama3, DeepSeek, Mistral, Claude-converted, etc. |

OpenAI models use the exact same BPE tables as `tiktoken` and produce identical token ids.

GGUF support is currently not bundled; we are evaluating whether the existing `tokenizers` BPE/WordPiece/Unigram implementation can cover GGUF vocabulary loading without adding a dedicated dependency.

## Performance

Measured on an Apple Silicon Mac using the full output of `x gtb show 1257` (1,337,848 bytes, ~322k tokens with `gpt-4o`).

| Backend | Cold (first encode) | Hot avg | Throughput |
|---|---|---|---|
| `token-actuary --model gpt-4o` (tiktoken-rs) | ~35 ms | ~31 ms | **~10.5 M tokens/s** |
| Python `tiktoken` | ~99 ms | ~80 ms | ~4.0 M tokens/s |
| `token-actuary --tokenizer gpt_4o.tokenizer.json` (HF) | ~190 ms | ~177 ms | ~1.8 M tokens/s |

The HF backend is intentionally slower because it deserializes a 6.7 MB `tokenizer.json` and goes through the general-purpose HF `tokenizers` pipeline. For OpenAI models the `--model` path is therefore both faster and requires no external file.

The two backends produce identical token ids for the same OpenAI encoding.

## Binary size

The default native binary embeds OpenAI BPE tables so it works offline. The extra size is data, not code: `gpt_4o.tokenizer.json` alone is ~6.7 MB, and `tiktoken-rs` packs several OpenAI encodings.

| Build | Uncompressed | `gzip` | `xz -9` |
|---|---|---|---|
| Default (`tiktoken` + `download`) | ~11 MB | ~5.0 MB | ~3.6 MB |
| `--no-default-features --features tiktoken` (no network) | ~9.6 MB | ~4.4 MB | ~3.4 MB |
| `--no-default-features` (HF only) | ~2.7 MB | ~1.2 MB | ~830 KB |

The `download` feature (`ureq` + `lzma-rs` + root certificates) adds about **~1.4 MB** to the uncompressed binary. Use `--no-default-features --features tiktoken` when you only need embedded OpenAI tokenizers and want to avoid the network stack, or `--no-default-features` for the smallest HF-only binary.

## Security

See [SECURITY.md](SECURITY.md). For vulnerabilities, email [lijunhao@x-cmd.com](mailto:lijunhao@x-cmd.com).

## License

Apache 2.0 — see [LICENSE](LICENSE).

`token-actuary` optionally includes `tiktoken-rs`, which is licensed under the MIT license. Its license text is included in the crate's source distribution.
