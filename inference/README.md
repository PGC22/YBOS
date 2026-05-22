# ybos-inference

YBOS Inference Layer provides a unified `Inference` trait for LLM interactions, supporting local and remote backends.

## Features

- **Inference Trait**: Unified API for completion and streaming.
- **MockInference**: Lightweight implementation for testing without real LLMs.
- **LocalLlama**: High-performance local inference via `llama-cpp-2` (CPU-only in Y4).
- **RemoteAPI**: Stub for future cloud burst integration.

## Usage

### Feature Flags

- `mock` (default): Enables `MockInference`.
- `local_llama`: Enables `LocalLlama` (requires `llama-cpp-2` and its build dependencies).
- `remote_api`: Enables `RemoteAPI` stub.

### Running Local LLM

To run real LLM inference locally, you need `cmake` and `clang` installed.

```bash
cargo test -p ybos-inference --features local_llama
```

The smoke test will download a small (~600MB) TinyLlama model to `target/test-models/` if it's not already present.

## Architecture

The `Inference` trait defines two primary methods:

- `complete`: For synchronous-like request/response.
- `complete_stream`: For token-by-token streaming (returns a `Stream`).

Implementations are gated by feature flags to minimize dependency weight in the default workspace build.
