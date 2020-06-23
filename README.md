# substrate-client
Implementation of substrate client in rust compilable to wasm. It is written to be integrated with CosmWasm readily.

## Compilation

### Prerequisites
1. Rust 1.42.0 or higher
2. Two target need to be installed
    1. `wasm32-unknown-unknown` to compile it into wasm and integrate it with CosmWasm
    2. `x86_64-apple-darwin` to run tests

### Compile in wasm
Run `cargo wasm` in project directory

### Testing
1. Run all the tests:
`cargo test`
2. Run the test tool:
Test tool is a bash script that run two tests with `-- --nocapture` flag, which makes them print out execution trace.
```$bash
$chmod +x test-tool.sh
$./test-tool.sh
```
