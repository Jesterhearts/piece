fn main() {
    println!("cargo:rerun-if-changed=src/protos");

    protobuf_codegen::Codegen::new()
        .pure()
        // All inputs and imports from the inputs must reside in `includes` directories.
        .includes(["src/protos"])
        // Inputs must reside in some of include paths.
        .inputs(
            std::fs::read_dir("src/protos")
                .unwrap()
                .map(|f| f.unwrap().path()),
        )
        .out_dir("src/protogen")
        .run_from_script();
}
