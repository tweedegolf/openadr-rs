# OpenADR 3.0 in Rust

This is a work-in-progress implementation of the OpenADR 3.0 specification.
OpenADR is a protocol for automatic demand-response in electricity grids, like dynamic pricing or load shedding.

## Limitations
This repository contains only OpenADR 3.0, older versions are not supported.
Currently, only the `/programs`, `/reports`, `/events` endpints are supported.
Also no authentication is supported yet.

## How to use

Starting the VTN server
```bash
RUST_LOG=trace cargo run --bin vtn
```

Running the client
```bash
cargo run --bin openadr
```
