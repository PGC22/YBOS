# ybos-proto

This crate provides a single source of truth for gRPC and Protobuf definitions shared across the YBOS workspace.

## Why this crate exists

Without this crate, every consumer crate (like `l0` or `orchestrator`) would have to re-run `tonic-build` independently. This leads to several issues:
1. **Duplicate Compilation**: Protobuf code is compiled multiple times, increasing build duration.
2. **Type Mismatches**: Even if generated from the same `.proto` files, the resulting Rust types would be distinct in each crate, preventing them from being shared across crate boundaries.

By centralizing the generated code here, we ensure type consistency and faster builds.

## Consumers

- **`l0`**: Uses this for the identity and session gRPC services.
- **`orchestrator`**: Uses this to communicate with `l0`.
- **Other workspace crates**: Can consume these definitions by adding `ybos-proto = { workspace = true }` to their `Cargo.toml` and importing `ybos_proto::l0::*` or `ybos_proto::orchestrator::*`.

## Adding new proto files

1. Add your `.proto` file to the `proto/` directory.
2. Update `build.rs` to include the new proto file in the `compile_protos` call.
3. Export the generated module in `src/lib.rs` using `tonic::include_proto!`.
