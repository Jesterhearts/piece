use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::CardId, in_play::Database, load_cards,
    pending_results::ResolutionResult, player::AllPlayers,
};

#[test]
fn remove_abilities() -> anyhow::Result<()> {
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

    let elesh = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, elesh, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, bear, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(elesh.power(&db), Some(4));
    assert_eq!(elesh.toughness(&db), Some(7));

    assert_eq!(bear.power(&db), Some(6));
    assert_eq!(bear.toughness(&db), Some(4));

    let enchant = CardId::upload(&mut db, &cards, player, "Eaten by Piranhas");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, enchant, Some(elesh));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(db[elesh].modified_static_abilities.is_empty());
    assert_eq!(bear.power(&db), Some(4));
    assert_eq!(bear.toughness(&db), Some(2));

    let mut results = Battlefields::permanent_to_graveyard(&mut db, enchant);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.power(&db), Some(6));
    assert_eq!(bear.toughness(&db), Some(4));

    Ok(())
}
