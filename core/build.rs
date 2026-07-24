// Build script for zaplivre-core
// 1. Compiles Protocol Buffers definitions into Rust code
// 2. Generates UniFFI scaffolding from UDL

fn main() {
    // 1. Compile Protocol Buffers
    let proto_files = ["../proto/messages.proto"];
    let proto_include = ["../proto"];

    prost_build::Config::new()
        .out_dir("src/protocol/generated")
        .compile_protos(&proto_files, &proto_include)
        .expect("Failed to compile protobuf");

    // Tell Cargo to rerun if proto files change
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file);
    }

    // 2. Generate UniFFI scaffolding
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let udl_path = std::path::PathBuf::from(manifest_dir).join("src/zaplivre.udl");
    let udl_path =
        camino::Utf8PathBuf::from_path_buf(udl_path).expect("UDL path must be valid UTF-8");
    println!("cargo:rerun-if-changed={}", udl_path);
    uniffi::generate_scaffolding(udl_path).expect("Failed to generate UniFFI scaffolding");
}
