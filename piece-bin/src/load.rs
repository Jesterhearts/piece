use piece_lib::Cards;
use protobuf::CodedInputStream;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "cards_binpb/"]
pub struct CardDefs;

pub fn load_protos() -> anyhow::Result<Vec<piece_lib::protogen::card::Card>> {
    let mut results = vec![];
    for card_file in CardDefs::iter() {
        let contents = CardDefs::get(&card_file).unwrap();

        let mut input = CodedInputStream::from_bytes(&contents.data);
        let card = input.read_message::<piece_lib::protogen::card::Card>()?;

        results.push(card);
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
    for card in protos {
        if let Some(overwritten) = cards.insert(card.name.clone(), card) {
            warn!("Overwriting card {}", overwritten.name);
        };
    }

    info!(
        "Converted {} cards in {}ms",
        cards.len(),
        timer.elapsed().as_millis()
    );

    Ok(cards)
}
