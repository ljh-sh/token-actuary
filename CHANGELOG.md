# Changelog

## 0.1.1 — 2026-08-14

### Added

- `ta download` subcommand for downloading open-source tokenizer files.
  - `ta download --recommend` downloads `qwen2_5`, `llama3`, and `deepseek_v3`.
  - Custom IDs: `ta download <id>...`.
  - Download strategy mirrors `x-bash/eget`:
    1. Direct `github.com` with stall detection.
    2. eget hosted mirror (`https://eget.ljh.sh/gh/...`).
    3. User-configured `GHPROXY_ENDPOINT` mirror.
  - Files are decompressed and cached under `~/.local/data/tokenizer-json/`.
- `ta compare` subcommand for comparing token counts across models.
  - `ta compare --recommend` compares against `gpt-4o`, `qwen2_5`, `llama3`, and `deepseek_v3`.
  - Accepts stdin, files, or inline `--text`.
  - Outputs TSV by default; `--format text` gives a table.

### Changed

- All release archives now use `xz` compression (Windows included).
- Added Windows arm64 build.

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

### Release artifacts

- macOS: `ta-darwin-arm64.tar.xz`, `ta-darwin-x64.tar.xz`, `ta-darwin-universal.tar.xz`
- Linux (musl): `ta-linux-arm64.tar.xz`, `ta-linux-x64.tar.xz`
- Windows: `ta-windows-arm64.tar.xz`, `ta-windows-x64.tar.xz`
- `checksums.txt` (SHA-256)

### License

- `token-actuary`: Apache-2.0
- Bundled `tiktoken-rs`: MIT (see `NOTICE`)
