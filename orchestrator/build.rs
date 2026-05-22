fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/orchestrator.proto"], &["proto"])?;

    // Also need l0 proto for the client
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["../l0/proto/l0.proto"], &["../l0/proto"])?;

    println!("cargo:rerun-if-changed=proto/orchestrator.proto");
    println!("cargo:rerun-if-changed=../l0/proto/l0.proto");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
