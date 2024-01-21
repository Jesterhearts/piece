use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    library::Library,
    load_cards,
    pending_results::ResolutionResult,
    player::{AllPlayers, Player},
    protogen::ids::CardId,
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
    all_players[&player].infinite_mana();

    let mut db = Database::new(all_players);

    let deck1 = CardId::upload(&mut db, &cards, player.clone(), "Annul");
    let deck2 = CardId::upload(&mut db, &cards, player.clone(), "Annul");
    Library::place_on_top(&mut db, &player, deck1.clone());
    Library::place_on_top(&mut db, &player, deck2.clone());

    let card = CardId::upload(&mut db, &cards, player.clone(), "Blood Scrivener");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &card, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Hand is empty
    let mut results = Player::draw(&mut db, &player, 1);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(db.all_players[&player].life_total, 19);

    assert_eq!(db.hand[&player], IndexSet::from([deck2, deck1]));

    Ok(())
}
