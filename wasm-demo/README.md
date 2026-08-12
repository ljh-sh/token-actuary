# token-actuary WASM sandbox

A minimal browser demo that loads the token-actuary core compiled to WebAssembly, fetches a compressed `tokenizer.json`, and audits prompts locally — no prompt text leaves the browser.

## Build

From the repo root:

```bash
wasm-pack build --target web --features wasm
```

This produces `pkg/` with the `.wasm` and JavaScript glue.

## Run locally

You need a static file server because ES modules and `SharedArrayBuffer`/WASM loading do not work from `file://` URLs reliably. Serve from the repo root so the demo can reach `../pkg/`:

```bash
python3 -m http.server 8080
```

Then open http://localhost:8080/wasm-demo/ .

## How it works

1. `index.html` fetches `minimal-tokenizer.json.gz`.
2. The browser's `DecompressionStream` decompresses it in memory.
3. `WasmActuary` is instantiated with the raw tokenizer bytes.
4. User pastes a prompt and clicks **Audit**.
5. The core returns token count, truncation status, and jailbreak-token hits.

For this demo, simple redaction patterns are applied in JavaScript before the core audit. A future version will expose `with_redactions` directly on `WasmActuary` so the Aho-Corasick automaton also runs inside WASM.

## Production tokenizer

Replace `minimal-tokenizer.json.gz` with a real model tokenizer (e.g. Qwen2.5, DeepSeek, Llama3). Compress with:

```bash
gzip -k -c tokenizer.json > tokenizer.json.gz
```

A typical 7-11 MB `tokenizer.json` compresses to ~1-1.8 MB with gzip.
