use std::env;
use std::fs;
use std::time::Instant;
use token_actuary::Actuary;

const RUNS: usize = 5;

fn bench(actuary: &Actuary, text: &str, label: &str) {
    // Cold: first encode
    let t0 = Instant::now();
    let ids = actuary.encode(text, false).unwrap();
    let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Hot loop
    let mut best = f64::MAX;
    let mut total = 0.0;
    for _ in 0..RUNS {
        let t0 = Instant::now();
        let ids = actuary.encode(text, false).unwrap();
        let dt = t0.elapsed().as_secs_f64();
        total += dt;
        if dt < best {
            best = dt;
        }
        let _ = ids.len();
    }
    let avg_ms = total / RUNS as f64 * 1000.0;
    let best_ms = best * 1000.0;
    let token_count = ids.len();

    println!(
        "{:<30} cold={:>7.1} ms  best={:>7.1} ms  avg={:>7.1} ms  {:>8.0} tokens/s",
        label,
        cold_ms,
        best_ms,
        avg_ms,
        token_count as f64 / (avg_ms / 1000.0)
    );
}

fn main() {
    let text = fs::read_to_string("bench/gtb_1257.txt").expect("bench/gtb_1257.txt not found");
    let tokenizer_json = env::var("TOKENIZER_JSON")
        .unwrap_or_else(|_| "../tokenizer-json/data/gpt_4o.tokenizer.json".into());

    println!("Input: {} bytes ({} chars)", text.len(), text.chars().count());
    println!();

    // HF backend via tokenizer.json
    let hf = Actuary::from_file(&tokenizer_json).unwrap();
    bench(&hf, &text, "HF tokenizer.json");

    // Tiktoken backend via model name
    #[cfg(feature = "tiktoken")]
    {
        let tiktoken = Actuary::from_model("gpt-4o").unwrap();
        bench(&tiktoken, &text, "tiktoken-rs (gpt-4o)");
    }
}
