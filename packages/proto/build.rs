fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    // First pass: generate `agentspay.common.v1` with no extern_path so the
    // types actually get written to OUT_DIR. include_proto!("agentspay.common.v1")
    // in lib.rs reads this file.
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&["proto/agentspay/v1/common.proto"], &["proto"])?;

    // Second pass: for auth/payment/metering, point cross-package references
    // to `crate::common` instead of letting prost compute a (wrong) relative
    // module path. Without this we get "too many leading super keywords"
    // because the module layout in lib.rs is flat (mod common, mod payment,
    // ...) rather than nested (agentspay::payment::v1).
    let dependent_protos = [
        "proto/agentspay/v1/auth.proto",
        "proto/agentspay/v1/payment.proto",
        "proto/agentspay/v1/metering.proto",
    ];
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .extern_path(".agentspay.common.v1", "crate::common")
        .compile_protos(&dependent_protos, &["proto"])?;

    println!("cargo:rerun-if-changed=proto/agentspay/v1/common.proto");
    for proto in dependent_protos {
        println!("cargo:rerun-if-changed={proto}");
    }

    Ok(())
}
