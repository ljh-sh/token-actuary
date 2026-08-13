# Changelog

## 0.1.0 — 2026-08-14

First release of `token-actuary`.

### Highlights

- Native CLI `ta` with subcommands: `count`, `encode`, `decode`, `audit`, `heatmap`.
- Embedded OpenAI tokenizer backend via `tiktoken-rs`: no external `tokenizer.json` needed for GPT-4o / GPT-4 / GPT-3.5-turbo / o-series models.
- Hugging Face `tokenizer.json` backend for open-source models (Qwen2.5, Llama3, DeepSeek, Mistral, etc.).
- Privacy-first local audit: redaction, token-level truncation, and jailbreak/control-token detection.
- Rust library API (`Actuary`) and WebAssembly bindings (`WasmActuary`).
- Zero network usage in the hot path.

### Performance

On an Apple Silicon Mac, processing the full output of `x gtb show 1257` (~322k tokens with `gpt-4o`):

- `ta --model gpt-4o`: ~10.5 M tokens/s
- Python `tiktoken`: ~4.0 M tokens/s
- `ta --tokenizer gpt_4o.tokenizer.json`: ~1.8 M tokens/s

### Binary size

- Default build (with embedded OpenAI tokenizers): ~9.6 MB uncompressed, ~2.6 MB with `xz -9`.
- HF-only build (`--no-default-features`): ~2.7 MB uncompressed, ~830 KB with `xz -9`.

### License

- `token-actuary`: Apache-2.0
- Bundled `tiktoken-rs`: MIT (see `NOTICE`)
