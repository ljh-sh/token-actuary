#!/usr/bin/env python3
"""Micro-benchmark: tokenize 'x gtb show         1257' with tiktoken."""
import os
import time
import tiktoken

TEXT = "x gtb show         1257"
ENCODING = "o200k_base"
RUNS = 10000

# Cold start: get_encoding + first encode in this process
t0 = time.perf_counter()
enc = tiktoken.get_encoding(ENCODING)
ids = enc.encode(TEXT)
t1 = time.perf_counter()
cold_us = (t1 - t0) * 1e6

# Hot loop
start = time.perf_counter()
for _ in range(RUNS):
    enc.encode(TEXT)
elapsed = time.perf_counter() - start

print("tiktoken Python")
print(f"  text:          {repr(TEXT)}")
print(f"  tokens:        {len(ids)}")
print(f"  token IDs:     {ids}")
print(f"  cold start:    {cold_us:.1f} µs")
print(f"  hot loop:      {elapsed*1e6/RUNS:.2f} µs/op  ({RUNS/elapsed:,.0f} ops/s)")
