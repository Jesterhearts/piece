use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::SelectionResult,
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn ability() -> anyhow::Result<()> {
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
    let player = all_players.new_player("name".to_string(), 20);
    all_players[player].infinite_mana();
    let mut db = Database::new(all_players);

    let card = CardId::upload(&mut db, &cards, player, "Deadapult");
    card.move_to_battlefield(&mut db);

    let sac = CardId::upload(&mut db, &cards, player, "Blood Scrivener");
    sac.move_to_battlefield(&mut db);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, card, 0);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Choose to sacrifice the zombie
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    // Recompute targets
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Pay the generic mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    // Choose the bear as the target
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(bear.marked_damage(&db), 2);

    Ok(())
}
