use piece_lib::{card::Card, load_protos};
use protobuf::CodedOutputStream;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/binpb")?;

    for (card, card_file) in cards {
        let _: Card = (&card).try_into()?;

        let mut file = std::fs::File::create(
            std::path::Path::new("experimental/binpb")
                .join(card_file.path().file_name().unwrap())
                .with_extension("binpb"),
        )?;
        let mut output = CodedOutputStream::new(&mut file);
        output.write_message_no_tag(&card)?;
        output.flush()?;
    }

    Ok(())
}
