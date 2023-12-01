fn main() {
    for path in std::fs::read_dir("src/protos")
        .unwrap()
        .map(|f| f.unwrap().path())
    {
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());
    }

    let out_dir_env = std::env::var_os("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir_env);

    protobuf_codegen::Codegen::new()
        .protoc()
        // All inputs and imports from the inputs must reside in `includes` directories.
        .includes(["src/protos"])
        // Inputs must reside in some of include paths.
        .inputs(
            std::fs::read_dir("src/protos")
                .unwrap()
                .map(|f| f.unwrap().path()),
        )
        // Specify output directory relative to Cargo output directory.
        .cargo_out_dir("protos")
        .run_from_script();

    for path in std::fs::read_dir(out_dir.join("protos"))
        .unwrap()
        .map(|f| f.unwrap().path())
    {
        let filename = format!(
            "src/protogen/{}",
            path.file_name().unwrap().to_string_lossy()
        );
        std::fs::File::create(&filename).unwrap();
        std::fs::copy(path, filename).unwrap();
    }
}
