use piece_lib::load_protos;
use protobuf::CodedOutputStream;

fn main() {
    println!("cargo:rerun-if-changed=../piece-lib/src/protos");
    println!("cargo:rerun-if-changed=../piece-lib/cards");

    let cards = load_protos().expect("Failed to load cards");

    if std::path::Path::new("cards_binpb").exists() {
        std::fs::remove_dir_all("cards_binpb").expect("Failed to remove directory");
    }
    std::fs::create_dir_all("cards_binpb").expect("Failed to create directory");

    for (card, file) in cards {
        let file_path = std::path::Path::new(&*file);

        let path = std::path::Path::new("cards_binpb").join(file_path.parent().unwrap());
        std::fs::create_dir_all(path.clone()).expect("Failed to create directory");
        let mut file = std::fs::File::create(
            path.join(file_path.file_name().unwrap())
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
