#!/usr/bin/env python3
"""Benchmark Python tiktoken encode performance."""
import time
import tiktoken

ENCODING_NAME = "o200k_base"
FILES = ["short", "medium", "long", "huge"]
WARMUP = 3
RUNS = 10


def measure(fn, runs):
    for _ in range(WARMUP):
        fn()
    times = []
    for _ in range(runs):
        t0 = time.perf_counter()
        fn()
        t1 = time.perf_counter()
        times.append(t1 - t0)
    return min(times), sum(times) / len(times)


print("file", "chars", "tokens", "cold_ms", "hot_min_ms", "hot_avg_ms", "tokens_per_sec", sep="\t")
enc = tiktoken.get_encoding(ENCODING_NAME)
for name in FILES:
    with open(f"bench/{name}.txt") as f:
        text = f.read()

    # Cold start: get_encoding + first encode
    t0 = time.perf_counter()
    enc_cold = tiktoken.get_encoding(ENCODING_NAME)
    enc_cold.encode(text)
    cold_ms = (time.perf_counter() - t0) * 1000

    tokens = enc.encode(text)
    token_count = len(tokens)

    hot_min, hot_avg = measure(lambda: enc.encode(text), RUNS)

    tps = token_count / hot_avg
    print(name, len(text), token_count, f"{cold_ms:.3f}", f"{hot_min*1000:.3f}", f"{hot_avg*1000:.3f}", f"{tps:,.0f}", sep="\t")
