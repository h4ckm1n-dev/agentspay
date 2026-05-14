fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    let proto_files = [
        "proto/agentspay/v1/common.proto",
        "proto/agentspay/v1/auth.proto",
        "proto/agentspay/v1/payment.proto",
        "proto/agentspay/v1/metering.proto",
    ];

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .extern_path(".agentspay.common.v1", "crate::common")
        .compile_protos(&proto_files, &["proto"])?;

    for proto in proto_files {
        println!("cargo:rerun-if-changed={proto}");
    }

    Ok(())
}
