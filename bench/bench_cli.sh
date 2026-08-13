#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TA="${ROOT_DIR}/target/release/ta"
TOKENIZER="${TOKENIZER_JSON:-${ROOT_DIR}/../tokenizer-json/data/gpt_4o.tokenizer.json}"

build() {
  if [ ! -f "$TA" ]; then
    cargo build --release --quiet
  fi
}

measure_once() {
  local file=$1
  local t0=$(perl -MTime::HiRes=time -e 'printf "%.6f", time')
  "$TA" count --tokenizer "$TOKENIZER" < "bench/$file.txt" >/dev/null
  local t1=$(perl -MTime::HiRes=time -e 'printf "%.6f", time')
  python3 -c "print($t1 - $t0)"
}

bench() {
  local name=$1
  local file="bench/$name.txt"
  local chars=$(wc -c < "$file" | tr -d ' ')
  local tokens=$("$TA" count --tokenizer "$TOKENIZER" < "$file")

  local total=0
  local min=999999
  for i in $(seq 1 5); do
    local t=$(measure_once "$name")
    total=$(python3 -c "print($total + $t)")
    min=$(python3 -c "print(min($min, $t))")
  done
  local avg=$(python3 -c "print($total / 5)")

  printf "%s\t%d\t%d\t%.3f\t%.3f\t%s\n" \
    "$name" "$chars" "$tokens" \
    "$(python3 -c "print($min*1000)")" \
    "$(python3 -c "print($avg*1000)")" \
    "$(python3 -c "print(int($tokens / $avg))")"
}

build
printf "file\tchars\ttokens\tcli_min_ms\tcli_avg_ms\ttokens_per_sec\n"
for name in short medium long huge; do
  bench "$name"
done
