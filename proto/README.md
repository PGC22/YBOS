# ybos-proto

This crate provides a single source of truth for gRPC types shared across `l0` and `orchestrator`.

## Adding new proto files

1. Add your `.proto` file to the `proto/` directory.
2. Update `build.rs` to include the new proto file in the `compile_protos` call.
3. Export the generated module in `src/lib.rs` using `tonic::include_proto!`.
