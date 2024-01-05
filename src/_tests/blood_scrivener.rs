use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{self, CardId, Database, InHand},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    turns::Turn,
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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let deck1 = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, deck1);
    let deck2 = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, deck2);

    let card = CardId::upload(&mut db, &cards, player, "Blood Scrivener");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Hand is empty
    let mut results = all_players[player].draw(&mut db, 1);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    assert_eq!(all_players[player].life_total, 19);

    assert_eq!(in_play::cards::<InHand>(&mut db), [deck2, deck1]);

    Ok(())
}
