use piece_lib::{initialize_assets, Cards};
use protobuf::CodedInputStream;

#[iftree::include_file_tree(
    "
paths = 'cards_binpb/**'
template.identifiers = false
template.initializer = 'initialize_assets'
"
)]
pub struct CardDefs {
    pub relative_path: &'static str,
    pub get_bytes: fn() -> std::borrow::Cow<'static, [u8]>,
}
pub fn load_protos() -> anyhow::Result<Vec<piece_lib::protogen::card::Card>> {
    let mut results = vec![];
    for card_file in ASSETS.iter() {
        let contents = (card_file.get_bytes)();

        let mut input = CodedInputStream::from_bytes(&contents);
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
