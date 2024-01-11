use piece_lib::{card::Card, load_protos};
use protobuf::text_format::print_to_string_pretty;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    std::fs::create_dir_all("experimental/textproto")?;

    for (card, card_file) in cards {
        let _: Card = (&card).try_into()?;

        std::fs::write(
            std::path::Path::new("experimental/textproto")
                .join(card_file.path().file_name().unwrap())
                .with_extension("textproto"),
            print_to_string_pretty(&card),
        )?;
    }

    Ok(())
}
