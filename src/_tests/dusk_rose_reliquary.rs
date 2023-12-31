use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database, InExile, InGraveyard, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
};

#[test]
fn exiles_until_leaves_battlefield() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);

    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player1, "Dusk Rose Reliquary");
    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card3 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");
    let card4 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card5 = CardId::upload(&mut db, &cards, player1, "Deconstruction Hammer");

    card2.move_to_battlefield(&mut db);
    card3.move_to_battlefield(&mut db);
    card4.move_to_battlefield(&mut db);
    card5.move_to_battlefield(&mut db);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    // Pay mana
    assert_eq!(result, ResolutionResult::TryAgain);
    // Compute sacrifice cost
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay sacrifice
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    //resolve casting the reliquary
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // resolve the etb
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player2.get_cards::<InExile>(&mut db), [card3]);
    assert_eq!(
        player1.get_cards::<OnBattlefield>(&mut db),
        [card4, card5, card]
    );
    assert_eq!(player1.get_cards::<InGraveyard>(&mut db), [card2]);

    // Equip deconstruction hammer
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player1, card5, 0);
    // Pay the costs
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay costs
    // Target the bear
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the equip
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Activate the ability
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player1, card4, 0);
    // Pay the genric mana
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose the reliquary as the default only target
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay for ward
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player2.get_cards::<OnBattlefield>(&mut db), [card3]);
    assert_eq!(player1.get_cards::<OnBattlefield>(&mut db), [card4]);
    assert_eq!(
        player1.get_cards::<InGraveyard>(&mut db),
        [card2, card5, card]
    );

    Ok(())
}

#[test]
fn destroyed_during_etb_does_not_exile() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);

    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player1, "Dusk Rose Reliquary");
    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card3 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");
    let card4 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card5 = CardId::upload(&mut db, &cards, player1, "Deconstruction Hammer");

    card2.move_to_battlefield(&mut db);
    card3.move_to_battlefield(&mut db);
    card4.move_to_battlefield(&mut db);
    card5.move_to_battlefield(&mut db);

    // Equip deconstruction hammer
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player1, card5, 0);
    // Pay the costs
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay costs
    // Target the bear
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the equip
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, card, true);
    // Pay mana
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Compute sacrifice cost
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay sacrifice
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    //resolve casting the reliquary
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Activate the ability
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player1, card4, 0);
    // Pay the mana
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay mana
    // Target the reliquary
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay for ward
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay for ward
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the etb
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player2.get_cards::<OnBattlefield>(&mut db), [card3]);
    assert_eq!(player1.get_cards::<OnBattlefield>(&mut db), [card4]);
    assert_eq!(
        player1.get_cards::<InGraveyard>(&mut db),
        [card2, card5, card]
    );

    Ok(())
}
