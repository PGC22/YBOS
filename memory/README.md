# ybos-memory

Memory layer for YBOS, providing vector storage and embedding capabilities.

## Features

- `mock_store`: In-memory vector store for testing.
- `mock_embedder`: Deterministic mock embedder for testing.
- `sqlite_vec`: Vector storage using SQLite and the `sqlite-vec` extension.
- `fastembed`: Local embedding generation using the `fastembed` crate.

## Usage

Add `ybos-memory` to your `Cargo.toml`:

```toml
[dependencies]
ybos-memory = { workspace = true }
```

By default, only mock implementations are enabled. To use real backends:

```toml
[dependencies]
ybos-memory = { workspace = true, features = ["sqlite_vec", "fastembed"] }
```

## Chosen Libraries

- **fastembed**: version 4.9.1. Chosen for its ease of use and local ONNX runtime integration for generating embeddings.
- **sqlite-vec**: version 0.1.0-alpha.4. Chosen for its mature SQLite-based KNN search capabilities.

## How to run tests

To run the mock tests:
```bash
cargo test -p ybos-memory
```

To run the full smoke test (requires downloading the embedding model):
```bash
cargo test -p ybos-memory --features fastembed,sqlite_vec
```
