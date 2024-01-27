use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, effects::SelectionResult, in_play::CardId, in_play::Database,
    load_cards, player::AllPlayers, protogen::color::Color,
};

#[test]
fn aura_works() -> anyhow::Result<()> {
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

    let mut db = Database::new(all_players);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, creature, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let aura = CardId::upload(&mut db, &cards, player, "Sinister Strength");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, aura, Some(creature));
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(7));
    assert_eq!(creature.toughness(&db), Some(3));
    assert_eq!(db[creature].modified_colors, HashSet::from([Color::BLACK]));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(card2.power(&db), Some(4));
    assert_eq!(card2.toughness(&db), Some(2));

    let results = Battlefields::permanent_to_graveyard(&mut db, aura);
    assert!(results.is_empty());
    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));
    assert_eq!(db[creature].modified_colors, HashSet::from([Color::GREEN]));

    assert!(Battlefields::no_modifiers(&db));

    Ok(())
}
