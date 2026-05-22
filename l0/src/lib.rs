//! Library facade for ybos-l0.
//! Allows integration tests and (later) workspace consumers (e.g. orchestrator
//! tests) to spawn parts of L0 in-process without going through the binary's
//! full boot sequence. Production runtime always uses the binary in src/main.rs.

pub mod bus;
pub mod grpc;
pub mod hw;
pub mod identity;
pub mod reflex;
