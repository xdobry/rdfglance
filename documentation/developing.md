# Test Coverage


Install it once:

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

Then you have two ways to run tests:

```powershell
# Ordinary tests, without coverage
cargo test --workspace

# Tests with an HTML coverage report
cargo llvm-cov --workspace --all-targets -j 2 --html --open
```

The HTML report highlights uncovered code; the function coverage summaries help identify functions tests never reached. For a text summary instead, run:

```powershell
cargo llvm-cov --workspace --all-targets
```
