use std::env;
use std::fs;
use std::time::Instant;
use token_actuary::Actuary;

const RUNS: usize = 3;

fn main() {
    let tokenizer_path = env::var("TOKENIZER_JSON")
        .unwrap_or_else(|_| "../tokenizer-json/data/gpt_4o.tokenizer.json".into());
    let text = fs::read_to_string("bench/gtb_1257.txt").expect("bench/gtb_1257.txt not found");

    // Cold: load + first encode
    let t0 = Instant::now();
    let actuary = Actuary::from_file(&tokenizer_path).unwrap();
    let ids = actuary.encode(&text, false).unwrap();
    let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Hot loop
    let mut best = f64::MAX;
    let mut total = 0.0;
    for _ in 0..RUNS {
        let t0 = Instant::now();
        let ids = actuary.encode(&text, false).unwrap();
        let dt = t0.elapsed().as_secs_f64();
        total += dt;
        if dt < best {
            best = dt;
        }
        // avoid unused warning by touching ids
        let _ = ids.len();
    }
    let avg_ms = total / RUNS as f64 * 1000.0;
    let best_ms = best * 1000.0;
    let token_count = ids.len();

    println!("tokenizer:       {}", tokenizer_path);
    println!("chars:           {}", text.len());
    println!("tokens:          {}", token_count);
    println!("cold (load+enc): {:.1} ms", cold_ms);
    println!("hot best:        {:.1} ms  ({:.0} tokens/s)", best_ms, token_count as f64 / best);
    println!("hot avg:         {:.1} ms  ({:.0} tokens/s)", avg_ms, token_count as f64 / (avg_ms / 1000.0));
}
