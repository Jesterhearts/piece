use anyhow::Context;
use include_dir::{include_dir, Dir, File};
use piece_lib::Cards;
use protobuf::CodedInputStream;

static CARD_DEFINITIONS: Dir = include_dir!("cards_binpb");

pub fn load_protos(
) -> anyhow::Result<Vec<(piece_lib::protogen::card::Card, &'static File<'static>)>> {
    fn dir_to_files(dir: &'static Dir) -> Vec<&'static File<'static>> {
        let mut results = vec![];
        for entry in dir.entries() {
            match entry {
                include_dir::DirEntry::Dir(dir) => results.extend(dir_to_files(dir)),
                include_dir::DirEntry::File(file) => {
                    results.push(file);
                }
            }
        }

        results
    }

    let mut results = vec![];
    for card_file in CARD_DEFINITIONS
        .entries()
        .iter()
        .flat_map(|entry| match entry {
            include_dir::DirEntry::Dir(dir) => dir_to_files(dir).into_iter(),
            include_dir::DirEntry::File(file) => vec![file].into_iter(),
        })
    {
        let contents = card_file.contents();

        let mut input = CodedInputStream::from_bytes(contents);
        let card = input.read_message::<piece_lib::protogen::card::Card>()?;

        results.push((card, card_file));
    }

    Ok(results)
}

pub fn load_cards() -> anyhow::Result<Cards> {
    let timer = std::time::Instant::now();
    let protos = load_protos()?;
    info!(
        "Loaded {} cards in {}ms",
        protos.len(),
        timer.elapsed().as_millis()
    );

    let timer = std::time::Instant::now();
    let mut cards = Cards::with_capacity(protos.len());
    for (card, card_file) in protos {
        if cards
            .insert(
                card.name.clone(),
                (&card)
                    .try_into()
                    .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
            )
            .is_some()
        {
            warn!("Overwriting card {}", card.name);
        };
    }

    info!(
        "Converted {} cards in {}ms",
        cards.len(),
        timer.elapsed().as_millis()
    );

    Ok(cards)
}
