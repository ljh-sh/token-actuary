# token-actuary

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> Privacy-First LLM Input Firewall & Cost Actuary.  
> Local token counting, redaction, truncation, and jailbreak-token detection — no network, no data leakage.

`token-actuary` (binary: `ta`) is a tiny, self-contained Rust tool that audits LLM prompts before they leave the machine. It counts tokens, redacts sensitive patterns, safely truncates at token boundaries, and flags control tokens that may indicate prompt injection or jailbreak attempts.

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
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/ta-darwin-universal.tar.xz | tar xJ

# Linux (x86_64)
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/ta-linux-x64.tar.xz | tar xJ

# Windows (x86_64 / arm64)
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/ta-windows-x64.tar.xz | tar xJ
curl -L https://github.com/ljh-sh/token-actuary/releases/download/v0.1.1/ta-windows-arm64.tar.xz | tar xJ
# ta.exe is now available
```

All archives are compressed with `xz`.

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

To build a smaller binary without the embedded OpenAI tokenizers:

```bash
cargo build --release --no-default-features
```

## Usage

Native builds default to the embedded `gpt-4o` tokenizer, so the simplest usage needs no extra files:

```bash
echo "hello world" | ta count
# 2
```

You can also select a specific OpenAI model:

```bash
ta count --model gpt-4
```

Or load a Hugging Face `tokenizer.json`:

```bash
ta count --tokenizer /path/to/tokenizer.json
export TOKENIZER_JSON=/path/to/tokenizer.json
ta count
```

Precedence:

1. `--tokenizer` flag
2. `TOKENIZER_JSON` environment variable
3. `--model` flag
4. `TOKENIZER_MODEL` environment variable
5. default model `gpt-4o`

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
cat prompt.txt | ta audit --max-tokens 2048 --format json
```

### Encode / decode

```bash
echo "hello world" | ta encode
# 24912,2375

ta decode 24912,2375
# hello world
```

### Heatmap

```bash
echo "hello world" | ta heatmap
```

### Download open-source tokenizers

`ta` can download Hugging Face-format tokenizers from the companion repo [`ljh-sh/tokenizer-json`](https://github.com/ljh-sh/tokenizer-json). Downloads are opt-in and cached locally under `~/.local/data/tokenizer-json/`.

```bash
# Download the recommended set (qwen2_5, llama3, deepseek_v3)
ta download

# Download specific IDs
ta download claude qwen2_5

# Force re-download
ta download --recommend --force
```

Download strategy (mirrors `x-bash/eget`):

1. Try `github.com` directly with stall detection.
2. Fall back to the eget hosted mirror (`https://eget.ljh.sh/gh/...`).
3. If `GHPROXY_ENDPOINT` is set, try that mirror too.

### Compare token counts across models

```bash
# Compare stdin across recommended models
echo "hello world" | ta compare

# Compare a file
ta compare prompt.txt

# Compare inline text
ta compare --text "hello world"

# Include extra models or local tokenizers
ta compare --model gpt-4 --tokenizer /path/to/custom.tokenizer.json prompt.txt

# Table output
ta compare --format text prompt.txt
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
| `ta --model gpt-4o` (tiktoken-rs) | ~35 ms | ~31 ms | **~10.5 M tokens/s** |
| Python `tiktoken` | ~99 ms | ~80 ms | ~4.0 M tokens/s |
| `ta --tokenizer gpt_4o.tokenizer.json` (HF) | ~190 ms | ~177 ms | ~1.8 M tokens/s |

The HF backend is intentionally slower because it deserializes a 6.7 MB `tokenizer.json` and goes through the general-purpose HF `tokenizers` pipeline. For OpenAI models the `--model` path is therefore both faster and requires no external file.

The two backends produce identical token ids for the same OpenAI encoding.

## Binary size

The default native binary embeds OpenAI BPE tables so it works offline. The extra size is data, not code: `gpt_4o.tokenizer.json` alone is ~6.7 MB, and `tiktoken-rs` packs several OpenAI encodings.

| Build | Uncompressed | `gzip` | `xz -9` |
|---|---|---|---|
| Default (with OpenAI tokenizers) | ~9.6 MB | ~4.4 MB | ~2.6 MB |
| `--no-default-features` (HF only) | ~2.7 MB | ~1.2 MB | ~830 KB |

Use `--no-default-features` when you only need open-source HF tokenizers and want the smallest binary (e.g. constrained containers or custom distributions).

## Security

See [SECURITY.md](SECURITY.md). For vulnerabilities, email [lijunhao@x-cmd.com](mailto:lijunhao@x-cmd.com).

## License

Apache 2.0 — see [LICENSE](LICENSE).

`token-actuary` optionally includes `tiktoken-rs`, which is licensed under the MIT license. Its license text is included in the crate's source distribution.
