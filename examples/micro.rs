use std::env;
use std::time::Instant;
use token_actuary::Actuary;

const TEXT: &str = "x gtb show         1257";
const RUNS: usize = 100_000;

fn main() {
    let tokenizer_path = env::var("TOKENIZER_JSON")
        .unwrap_or_else(|_| "../tokenizer-json/data/gpt_4o.tokenizer.json".into());

    // Cold: load + first encode
    let t0 = Instant::now();
    let actuary = Actuary::from_file(&tokenizer_path).unwrap();
    let ids = actuary.encode(TEXT, false).unwrap();
    let cold = t0.elapsed().as_micros();

    // Hot loop
    let t0 = Instant::now();
    for _ in 0..RUNS {
        let _ = actuary.encode(TEXT, false).unwrap();
    }
    let hot_total = t0.elapsed().as_micros() as f64;
    let hot_avg = hot_total / RUNS as f64;

    println!("ta Rust library");
    println!("  text:          {:?}", TEXT);
    println!("  tokens:        {}", ids.len());
    println!("  token IDs:     {:?}", ids);
    println!("  cold start:    {} µs", cold);
    println!("  hot loop:      {:.2} µs/op  ({:.0} ops/s)", hot_avg, 1e6 / hot_avg);
}
