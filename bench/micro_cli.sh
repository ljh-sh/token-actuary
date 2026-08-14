#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TA="${ROOT_DIR}/target/release/token-actuary"
TOKENIZER="${TOKENIZER_JSON:-${ROOT_DIR}/../tokenizer-json/data/gpt_4o.tokenizer.json}"
TEXT="x gtb show         1257"
RUNS=100

if [ ! -f "$TA" ]; then
  echo "Building token-actuary..."
  cargo build --release --quiet
fi
if [ ! -f "$TOKENIZER" ]; then
  echo "Tokenizer not found: $TOKENIZER"
  echo "Set TOKENIZER_JSON or place tokenizer-json next to token-actuary."
  exit 1
fi

now_us() {
  perl -MTime::HiRes=time -e 'printf "%d", time * 1e6'
}

# Single cold-ish CLI call
t0=$(now_us)
"$TA" encode --tokenizer "$TOKENIZER" "$TEXT" >/dev/null
t1=$(now_us)
cold_us=$((t1 - t0))

# Hot loop: repeated subprocess
start=$(now_us)
for _ in $(seq 1 $RUNS); do
  "$TA" encode --tokenizer "$TOKENIZER" "$TEXT" >/dev/null
done
end=$(now_us)
total_us=$((end - start))
avg_us=$(python3 -c "print(f'{($total_us / $RUNS):.2f}')")
ops=$(python3 -c "print(f'{$RUNS / ($total_us / 1e6):.0f}')")

echo "token-actuary CLI (subprocess per call)"
echo "  text:          '$TEXT'"
echo "  single call:   ${cold_us} µs"
echo "  hot loop:      ${avg_us} µs/op  (${ops} ops/s)"
