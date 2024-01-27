use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    effects::SelectionResult,
    in_play::{CardId, Database},
    library::Library,
    load_cards,
    player::{AllPlayers, Player},
};

#[test]
fn replacement() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .with_writer(std::io::stderr)
        .try_init();

    let cards = load_cards()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let deck1 = CardId::upload(&mut db, &cards, player, "Annul");
    let deck2 = CardId::upload(&mut db, &cards, player, "Annul");
    Library::place_on_top(&mut db, player, deck1);
    Library::place_on_top(&mut db, player, deck2);

    let card = CardId::upload(&mut db, &cards, player, "Blood Scrivener");
    card.move_to_battlefield(&mut db);

    // Hand is empty
    let mut results = Player::draw(player, 1);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);
    assert_eq!(db.all_players[player].life_total, 19);

    assert_eq!(db.hand[player], IndexSet::from([deck2, deck1]));

    Ok(())
}
