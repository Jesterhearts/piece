use piece::{card::Card, load_protos};
use protobuf::text_format::print_to_string_pretty;

fn main() -> anyhow::Result<()> {
    let cards = load_protos()?;

    for (card, card_file) in cards {
        let _: Card = (&card).try_into()?;

        std::fs::write(
            std::path::Path::new("cards").join(card_file.path()),
            print_to_string_pretty(&card),
        )?;
    }

    Ok(())
}
