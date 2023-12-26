use piece::load_protos;
use protobuf::CodedOutputStream;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    for (card, card_file) in cards {
        let mut file = std::fs::File::create(
            std::path::Path::new("cards_protos")
                .join(card_file.path().file_name().unwrap())
                .with_extension("binpb"),
        )?;
        let mut output = CodedOutputStream::new(&mut file);
        output.write_message_no_tag(&card)?;
        output.flush()?;
    }

    Ok(())
}
