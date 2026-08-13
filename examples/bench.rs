use std::env;
use std::fs;
use std::time::Instant;
use token_actuary::Actuary;

const FILES: &[&str] = &["short", "medium", "long", "huge"];
const WARMUP: usize = 3;
const RUNS: usize = 10;

fn main() {
    let tokenizer_path = env::var("TOKENIZER_JSON")
        .unwrap_or_else(|_| "../tokenizer-json/data/gpt_4o.tokenizer.json".into());
    let actuary = Actuary::from_file(&tokenizer_path).unwrap();

    println!("file\tchars\ttokens\tload_ms\thot_min_ms\thot_avg_ms\ttokens_per_sec");

    for name in FILES {
        let text = fs::read_to_string(&format!("bench/{}.txt", name)).unwrap();

        // measure load separately once
        let t0 = Instant::now();
        let _ = Actuary::from_file(&tokenizer_path).unwrap();
        let load_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // warmup
        for _ in 0..WARMUP {
            let _ = actuary.encode(&text, false);
        }

        let mut min = f64::MAX;
        let mut total = 0.0;
        let mut token_count = 0;
        for _ in 0..RUNS {
            let t0 = Instant::now();
            let ids = actuary.encode(&text, false).unwrap();
            let dt = t0.elapsed().as_secs_f64();
            token_count = ids.len();
            total += dt;
            if dt < min {
                min = dt;
            }
        }
        let avg = total / RUNS as f64;
        let tps = token_count as f64 / avg;
        println!(
            "{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.0}",
            name,
            text.len(),
            token_count,
            load_ms,
            min * 1000.0,
            avg * 1000.0,
            tps
        );
    }
}
