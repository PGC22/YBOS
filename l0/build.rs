//! Build script — compileaza `proto/l0.proto` cu tonic-build.
//!
//! Pe Windows + dev mediu mixt, NU presupunem ca user-ul are `protoc` in PATH.
//! Folosim binar vendat (`protoc-bin-vendored`) si exportam PROTOC env var.
//!
//! Rezultatul codegen este disponibil in cod via:
//!     tonic::include_proto!("ybos.l0.v1");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["proto/l0.proto"], &["proto"])?;

    println!("cargo:rerun-if-changed=proto/l0.proto");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
