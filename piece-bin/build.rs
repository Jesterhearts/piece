use piece_lib::load_protos;
use protobuf::CodedOutputStream;

fn main() {
    println!("cargo:rerun-if-changed=../piece-lib/src/protos");
    println!("cargo:rerun-if-changed=../cards");

    let cards = load_protos().expect("Failed to load cards");

    std::fs::create_dir_all("../cards_binpb").expect("Failed to create directory");

    for (card, file) in cards {
        let mut file = std::fs::File::create(
            std::path::Path::new("../cards_binpb")
                .join(file.path())
                .with_extension("binpb"),
        )
        .expect("Failed to create file");
        let mut output = CodedOutputStream::new(&mut file);

        output
            .write_message_no_tag(&card)
            .expect("Failed to write proto");
        output.flush().expect("Failed to flush data");
    }
}
