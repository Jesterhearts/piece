use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::Database, load_cards, pending_results::ResolutionResult,
    player::AllPlayers, protogen::ids::CardId, stack::Stack, turns::Phase,
};

#[test]
fn exiles_until_leaves_battlefield() -> anyhow::Result<()> {
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
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);
    let mut db = Database::new(all_players);

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player1, "Dusk Rose Reliquary");
    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card3 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");
    let card4 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card5 = CardId::upload(&mut db, &cards, player1, "Deconstruction Hammer");

    card2.move_to_battlefield(&mut db);
    card3.move_to_battlefield(&mut db);
    card4.move_to_battlefield(&mut db);
    card5.move_to_battlefield(&mut db);

    // Get rid of summoning sickness
    db.turn.turn_count += db.turn.turns_per_round();

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card.clone(), true);
    let result = results.resolve(&mut db, None);
    // Pay mana
    assert_eq!(result, ResolutionResult::TryAgain);
    // Compute sacrifice cost
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay sacrifice
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    //resolve casting the reliquary
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // resolve the etb
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.exile[player2], IndexSet::from([card3.clone()]));
    assert_eq!(
        db.battlefield[player1],
        IndexSet::from([card4.clone(), card5.clone(), card.clone()]),
    );
    assert_eq!(db.graveyard[player1], IndexSet::from([card2.clone()]));

    // Equip deconstruction hammer
    let mut results = Battlefields::activate_ability(&mut db, &None, player1, &card5, 0);
    // Pay the costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay costs
    // Target the bear
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the equip
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Activate the ability
    let mut results = Battlefields::activate_ability(&mut db, &None, player1, &card4, 0);
    // Pay the white
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose the reliquary as the default only target
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Pay for ward
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.battlefield[player2], IndexSet::from([card3]));
    assert_eq!(db.battlefield[player1], IndexSet::from([card4]));
    assert_eq!(db.graveyard[player1], IndexSet::from([card2, card5, card]));

    Ok(())
}

#[test]
fn destroyed_during_etb_does_not_exile() -> anyhow::Result<()> {
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
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);
    let mut db = Database::new(all_players);

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let card = CardId::upload(&mut db, &cards, player1, "Dusk Rose Reliquary");
    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card3 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");
    let card4 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card5 = CardId::upload(&mut db, &cards, player1, "Deconstruction Hammer");

    card2.move_to_battlefield(&mut db);
    card3.move_to_battlefield(&mut db);
    card4.move_to_battlefield(&mut db);
    card5.move_to_battlefield(&mut db);

    // Get rid of summoning sickness
    db.turn.turn_count += db.turn.turns_per_round();

    // Equip deconstruction hammer
    let mut results = Battlefields::activate_ability(&mut db, &None, player1, &card5, 0);
    // Pay the costs
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay costs
    // Target the bear
    let result = results.resolve(&mut db, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the equip
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card.clone(), true);
    // Pay mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Compute sacrifice cost
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay sacrifice
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    //resolve casting the reliquary
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Activate the ability
    let mut results = Battlefields::activate_ability(&mut db, &None, player1, &card4, 0);
    // Pay the mana
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Target the reliquary
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Pay for ward
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the deconstruction hammer
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the etb
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.battlefield[player2], IndexSet::from([card3]));
    assert_eq!(db.battlefield[player1], IndexSet::from([card4]));
    assert_eq!(db.graveyard[player1], IndexSet::from([card2, card5, card]));

    Ok(())
}
