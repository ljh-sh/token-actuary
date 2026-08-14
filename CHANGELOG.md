# Changelog

## 0.1.2 — 2026-08-14

### Added

- New downloadable tokenizer IDs in the upstream `tokenizer-json` data release:
  - `qwen3` (Qwen 3 family, including 0.6B through 235B-A22B, Coder, and MoE variants).
  - `glm` (Zhipu GLM-4 / GLM-4-9B / GLM-4-9B-Chat, sourced from the HF-compatible release).
- Encode/decode roundtrip support: `-s/--sep` on `encode` and `decode` lets you pipe ids between the two commands.
- China network fallback for `token-actuary download`:
  - `--china` flag and `TA_CHINA=1` environment variable.
  - Fallback order: user `GHPROXY_ENDPOINT` → eget mirror → built-in ghproxy mirrors → direct GitHub.

### Changed

- CLI binary renamed from `ta` to `token-actuary` to avoid conflicting with x-cmd's `ta` (tsv-awk).
  - Release archives now use the `token-actuary-*` prefix.
- `README.md` and `README.cn.md` refreshed with a TL;DR cheat sheet.

## 0.1.1 — 2026-08-14

### Added

- `token-actuary download` subcommand for downloading open-source tokenizer files.
  - `token-actuary download --recommend` downloads `qwen2_5`, `llama3`, and `deepseek_v3`.
  - Custom IDs: `token-actuary download <id>...`.
  - Download strategy uses the same mirror endpoints as `x-bash/eget`:
    1. Direct `github.com` with stall detection.
    2. eget hosted mirror (`https://eget.ljh.sh/gh/...`).
    3. User-configured `GHPROXY_ENDPOINT` mirror.
  - No dependency on x-cmd or the `eget` CLI; fallback is handled inside `token-actuary`.
  - Files are decompressed and cached under `~/.local/data/tokenizer-json/`.
- `token-actuary compare` subcommand for comparing token counts across models.
  - `token-actuary compare --recommend` compares against `gpt-4o`, `qwen2_5`, `llama3`, and `deepseek_v3`.
  - Accepts stdin, files, or inline `--text`.
  - Outputs TSV by default; `--format text` gives a table.

### Changed

- All release archives now use `xz` compression (Windows included).
- Added Windows arm64 build.

## 0.1.0 — 2026-08-14

First release of `token-actuary`.

### Highlights

- Native CLI `token-actuary` with subcommands: `count`, `encode`, `decode`, `audit`, `heatmap`.
- Embedded OpenAI tokenizer backend via `tiktoken-rs`: no external `tokenizer.json` needed for GPT-4o / GPT-4 / GPT-3.5-turbo / o-series models.
- Hugging Face `tokenizer.json` backend for open-source models (Qwen2.5, Llama3, DeepSeek, Mistral, etc.).
- Privacy-first local audit: redaction, token-level truncation, and jailbreak/control-token detection.
- Rust library API (`Actuary`) and WebAssembly bindings (`WasmActuary`).
- Zero network usage in the hot path.

### Performance

On an Apple Silicon Mac, processing the full output of `x gtb show 1257` (~322k tokens with `gpt-4o`):

- `token-actuary --model gpt-4o`: ~10.5 M tokens/s
- Python `tiktoken`: ~4.0 M tokens/s
- `token-actuary --tokenizer gpt_4o.tokenizer.json`: ~1.8 M tokens/s

### Binary size

- Default build (with embedded OpenAI tokenizers): ~9.6 MB uncompressed, ~2.6 MB with `xz -9`.
- HF-only build (`--no-default-features`): ~2.7 MB uncompressed, ~830 KB with `xz -9`.

### Release artifacts

- macOS: `token-actuary-darwin-arm64.tar.xz`, `token-actuary-darwin-x64.tar.xz`, `token-actuary-darwin-universal.tar.xz`
- Linux (musl): `token-actuary-linux-arm64.tar.xz`, `token-actuary-linux-x64.tar.xz`
- Windows: `token-actuary-windows-arm64.tar.xz`, `token-actuary-windows-x64.tar.xz`
- `checksums.txt` (SHA-256)

### License

- `token-actuary`: Apache-2.0
- Bundled `tiktoken-rs`: MIT (see `NOTICE`)
