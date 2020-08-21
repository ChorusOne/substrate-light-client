# Substrate light client
Implementation of substrate light client in rust compilable to wasm. It is written to be integrated with CosmWasm readily, and is optimized to run in a constrained environment of a smart contract. Refer [here](#how-it-works) to know how it works under the hood.

## Compilation

### Prerequisites
1. Rust 1.42.0 or higher
2. Two target need to be installed
    1. `wasm32-unknown-unknown` to compile it into wasm and integrate it with CosmWasm
    2. `x86_64-apple-darwin` to run tests

### Compile in wasm
Run `make wasm` in project directory. This will produce a file `/target/wasm32-unknown-unknown/release/substrate_client.wasm`
To produce a size optimized build, you need to run `make wasm-optimized`.

### Testing
1. Run all the tests:
`cargo test`
2. Run the test tool:
Test tool is a bash script that run two tests with `-- --nocapture` flag, which makes them print out execution trace.
```commandline
chmod +x test-tool.sh
./test-tool.sh
```

## Upload optimized wasm bytecode
Setup CosmWasm enabled `Gaia` from [here](https://github.com/ChorusOne/gaia/tree/41eb8accd684aeea680d406da9126c9721f24a2c), and run following command in project directory.
```commandline
make wasm-optimized
gaiacli tx ibc wasm store "target/wasm32-unknown-unknown/release/substrate_client.wasm"  --gas=2000000  --from=<account with $ for gas> --chain-id "<your chain id>" --yes
```

## How it works?
Light client is in form of CosmWasm contract, with three entry points: 
1. `init`: As the name suggests, init method initializes new light client instance. It requires a root header and grandpa authority set who signed that header along with some configuration parameters.
2. `update`: update method ingests incoming header with optional justification. Header ingestion first validates incoming header (optionally with justification), and contains mainly two checks: a. Header is a child of the last header we successfully ingested b. If justification is provided, it is valid against current authority set and its target hash is equal to header's hash. Upon successful validation, if a scheduled authority set change is contained in the header, it is extracted and stored along with the header. Lastly, if valid justification is provided, the header and its ascendants are marked as finalized.
3. `query`: Query method is a read-only method that reads light client storage and returns data like last ingested header, last finalized header, etc.
