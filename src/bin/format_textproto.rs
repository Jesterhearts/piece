use anyhow::{anyhow, Context};
use include_dir::{include_dir, Dir};

use piece::protogen;
use protobuf::text_format::print_to_string_pretty;

static CARD_DEFINITIONS: Dir = include_dir!("cards");

fn main() -> anyhow::Result<()> {
    for card in CARD_DEFINITIONS.entries().iter() {
        let card_file = card
            .as_file()
            .ok_or_else(|| anyhow!("Non-file entry in cards directory"))?;

        let card: protogen::card::Card = protobuf::text_format::parse_from_str(
            card_file
                .contents_utf8()
                .ok_or_else(|| anyhow!("Non utf-8 text proto"))?,
        )
        .with_context(|| format!("Parsing file: {}", card_file.path().display()))?;

        std::fs::write(
            std::path::Path::new("cards").join(card_file.path()),
            print_to_string_pretty(&card),
        )?;
    }

    Ok(())
}
