fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/l0.proto", "proto/orchestrator.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/l0.proto");
    println!("cargo:rerun-if-changed=proto/orchestrator.proto");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
