#!/usr/bin/env python3
"""Benchmark Python tiktoken cold start per file in separate process."""
import time
import tiktoken
import sys

name = sys.argv[1]
with open(f"bench/{name}.txt") as f:
    text = f.read()

t0 = time.perf_counter()
enc = tiktoken.get_encoding("o200k_base")
ids = enc.encode(text)
t1 = time.perf_counter()
print(f"{name}\t{len(text)}\t{len(ids)}\t{((t1-t0)*1000):.3f}")
