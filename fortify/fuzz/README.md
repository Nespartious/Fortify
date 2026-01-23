# Fortify Fuzz Testing

This directory contains fuzz testing targets for security-critical parsing code in Fortify.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly Rust)
cargo install cargo-fuzz

# Ensure nightly is installed
rustup install nightly
```

## Available Fuzz Targets

| Target | Description | Risk Level |
|--------|-------------|------------|
| `fuzz_token_decode` | SessionToken base64 decoding | 🔴 Critical |
| `fuzz_token_verify` | HMAC signature verification | 🔴 Critical |
| `fuzz_cookie_parse` | Cookie header parsing | 🔴 Critical |
| `fuzz_ip_extraction` | X-Forwarded-For IP parsing | 🟡 High |

## Running Fuzz Tests

### Run a specific target

```bash
cd /path/to/fortify

# Run token decoding fuzzer for 5 minutes
cargo +nightly fuzz run fuzz_token_decode -- -max_total_time=300

# Run with more verbosity
cargo +nightly fuzz run fuzz_token_decode -- -max_total_time=300 -verbosity=1
```

### Run all targets

```bash
# Run each target for 1 minute
for target in fuzz_token_decode fuzz_token_verify fuzz_cookie_parse fuzz_ip_extraction; do
    echo "Fuzzing $target..."
    cargo +nightly fuzz run $target -- -max_total_time=60
done
```

### List available targets

```bash
cargo +nightly fuzz list
```

## CI Integration

The fuzz tests are integrated into GitHub Actions via `.github/workflows/fuzz-testing.yml`.

They run:
- On manual trigger (workflow_dispatch)
- Daily at 3 AM UTC (scheduled)

## Interpreting Results

### Crash found
If a crash is found, it will be saved to `fuzz/artifacts/<target>/`. 
To reproduce:

```bash
cargo +nightly fuzz run fuzz_token_decode fuzz/artifacts/fuzz_token_decode/crash-<hash>
```

### No crashes
A successful run means no panics were found for the given input corpus.

## Adding New Targets

1. Create a new file in `fuzz_targets/`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Your parsing code here - should never panic
});
```

2. Add the binary to `Cargo.toml`:

```toml
[[bin]]
name = "fuzz_new_target"
path = "fuzz_targets/fuzz_new_target.rs"
test = false
doc = false
bench = false
```

3. Run: `cargo +nightly fuzz run fuzz_new_target`

## Coverage

To generate coverage reports:

```bash
cargo +nightly fuzz coverage fuzz_token_decode
```

## Seed Corpus

For better fuzzing efficiency, add known-valid inputs to `fuzz/corpus/<target>/`:

```bash
mkdir -p fuzz/corpus/fuzz_token_decode
echo -n "valid_base64_token_here" > fuzz/corpus/fuzz_token_decode/seed1
```

## Related Documentation

- [02-PANIC-AUDIT-SPRINT.md](../docs/Dev_Progress/02-PANIC-AUDIT-SPRINT.md) - Sprint documentation
- [cargo-fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html) - Official documentation
